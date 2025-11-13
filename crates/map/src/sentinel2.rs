use anyhow::{Context, Result};
use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::Region;

/// Sentinel-2 tile information
#[derive(Debug, Clone)]
pub struct Sentinel2Tile {
    pub mgrs_tile: String, // e.g., "28RCS"
    pub utm_zone: u8,
    pub latitude_band: char,
    pub grid_square: String,
}

/// Get Sentinel-2 tile ID for a region
/// For Madeira: 28RCS (UTM zone 28, latitude band R, grid square CS)
pub fn get_sentinel2_tile(region: &Region) -> Result<Sentinel2Tile> {
    let center_lat = (region.north + region.south) / 2.0;
    let center_lon = (region.west + region.east) / 2.0;

    // Calculate UTM zone from longitude
    let utm_zone = ((center_lon + 180.0) / 6.0).floor() as u8 + 1;

    // Calculate latitude band (C-X, skipping I and O)
    let lat_bands = [
        'C', 'D', 'E', 'F', 'G', 'H', 'J', 'K', 'L', 'M', 'N', 'P', 'Q', 'R', 'S', 'T', 'U', 'V',
        'W', 'X',
    ];
    let band_index = ((center_lat + 80.0) / 8.0).floor() as usize;
    let latitude_band = lat_bands.get(band_index).copied().unwrap_or('N');

    // For simplicity, we'll use a lookup table for common regions
    // In production, you'd implement full MGRS grid square calculation
    let grid_square = match (center_lat.round() as i32, center_lon.round() as i32) {
        (33, -17) | (32, -17) => "RCS".to_string(), // Madeira
        _ => {
            // Fallback: try to guess based on coordinates
            // This is a simplified approach
            format!("{}{}",
                ((center_lon + 180.0) % 6.0 / 6.0 * 26.0) as u8 as char,
                ((center_lat + 80.0) % 8.0 / 8.0 * 26.0) as u8 as char
            )
        }
    };

    let mgrs_tile = format!("{}{}{}", utm_zone, latitude_band, grid_square);

    Ok(Sentinel2Tile {
        mgrs_tile,
        utm_zone,
        latitude_band,
        grid_square,
    })
}

/// Download Sentinel-2 imagery from AWS S3 (free, no API key required)
/// Uses the Sentinel-2 L2A Cloud-Optimized GeoTIFF (COG) dataset
pub async fn fetch_sentinel2_imagery_aws(
    region: &Region,
    output_dir: &Path,
) -> Result<String> {
    println!("Fetching Sentinel-2 imagery from AWS S3 (FREE)...");

    // Determine the Sentinel-2 tile for this region
    let tile = get_sentinel2_tile(region)?;
    println!("Sentinel-2 tile: {}", tile.mgrs_tile);

    // For Madeira, hardcode the tile since MGRS calculation is complex
    let mgrs_tile = if region.north > 32.0 && region.north < 33.5 && region.west < -16.0 && region.west > -18.0 {
        "28SCB" // Madeira - corrected tile ID
    } else {
        &tile.mgrs_tile
    };

    println!("Using tile: {}", mgrs_tile);

    // Parse tile components
    let utm_zone = &mgrs_tile[0..2];
    let latitude_band = &mgrs_tile[2..3];
    let grid_square = &mgrs_tile[3..5];

    // Use Element 84's Earth Search STAC API to find available scenes
    println!("Searching for Sentinel-2 scenes using STAC API...");

    match search_sentinel2_stac(region, mgrs_tile).await {
        Ok(tci_url) => {
            println!("✓ Found scene via STAC API");
            return download_sentinel2_tci(&tci_url, output_dir).await;
        }
        Err(e) => {
            println!("STAC search failed: {}", e);
            println!("Trying direct S3 search (slower)...");
        }
    }

    // Fallback: direct S3 search with limited scope
    let year = 2024;
    let months = [10, 9, 8]; // Only try last 3 months
    let days_to_try = vec![15, 10, 20, 5, 25, 1]; // Try specific days

    for month in months {
        println!("\nTrying {}-{:02}...", year, month);

        for day in &days_to_try {
            for seq in 0..3 {
                let scene_url = format!(
                    "https://sentinel-cogs.s3.us-west-2.amazonaws.com/sentinel-s2-l2a-cogs/{}/{}/{}/{}/{}/{}/{}",
                    utm_zone, latitude_band, grid_square, year, month, day, seq
                );
                let test_url = format!("{}/TCI.tif", scene_url);

                let client = reqwest::Client::new();
                if let Ok(resp) = client.head(&test_url).send().await {
                    if resp.status().is_success() {
                        println!("✓ Found scene: {}-{:02}-{:02} seq {}", year, month, day, seq);
                        return download_sentinel2_tci(&test_url, output_dir).await;
                    }
                }
            }
        }
    }

    anyhow::bail!(
        "No Sentinel-2 scenes found for tile {}. Try manually specifying a scene URL.",
        mgrs_tile
    )
}

/// Search for Sentinel-2 scenes using Element 84's Earth Search STAC API
async fn search_sentinel2_stac(region: &Region, _mgrs_tile: &str) -> Result<String> {
    let client = reqwest::Client::new();

    // Earth Search STAC API endpoint
    let stac_url = "https://earth-search.aws.element84.com/v1/search";

    // Build search query - prioritize cloud-free imagery
    let query = serde_json::json!({
        "collections": ["sentinel-2-l2a"],
        "bbox": [region.west, region.south, region.east, region.north],
        "limit": 50,  // Search more scenes to find cloud-free ones
        "query": {
            "eo:cloud_cover": {
                "lt": 5  // Less than 5% cloud cover
            }
        },
        "sortby": [
            {"field": "properties.eo:cloud_cover", "direction": "asc"},  // Lowest cloud cover first
            {"field": "properties.datetime", "direction": "desc"}  // Then most recent
        ]
    });

    let response = client
        .post(stac_url)
        .json(&query)
        .send()
        .await
        .context("Failed to query STAC API")?;

    if !response.status().is_success() {
        anyhow::bail!("STAC API error: {}", response.status());
    }

    let result: serde_json::Value = response.json().await?;

    // Extract the first feature's assets
    if let Some(features) = result["features"].as_array() {
        if features.is_empty() {
            anyhow::bail!("No features returned from STAC API");
        }

        if let Some(feature) = features.first() {
            // Print scene info
            if let Some(date) = feature["properties"]["datetime"].as_str() {
                println!("Found scene from: {}", date);
            }
            if let Some(cloud) = feature["properties"]["eo:cloud_cover"].as_f64() {
                println!("Cloud cover: {:.1}%", cloud);
            }

            if let Some(visual_href) = feature["assets"]["visual"]["href"].as_str() {
                // The href IS the direct TCI.tif URL, return it directly
                return Ok(visual_href.to_string());
            } else {
                anyhow::bail!("No 'visual' asset found in feature");
            }
        }
    }

    anyhow::bail!("No suitable scenes found in STAC catalog")
}

/// Download TCI directly from URL
async fn download_sentinel2_tci(tci_url: &str, output_dir: &Path) -> Result<String> {
    println!("Downloading True Color Image (TCI) - 10m resolution...");
    println!("URL: {}", tci_url);

    let client = reqwest::Client::new();
    let response = client
        .get(tci_url)
        .send()
        .await
        .context("Failed to download Sentinel-2 TCI")?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to download: {}", response.status());
    }

    let bytes = response.bytes().await?;
    let output_path = output_dir.join("sentinel2_tci.tif");
    let mut file = File::create(&output_path)?;
    file.write_all(&bytes)?;

    println!("✓ Saved: {:?} ({:.2} MB)", output_path, bytes.len() as f64 / 1_048_576.0);
    Ok(output_path.to_string_lossy().to_string())
}

/// Alternative: Download individual RGB bands (B04=Red, B03=Green, B02=Blue)
#[allow(dead_code)]
async fn download_sentinel2_rgb_bands(scene_url: &str, output_dir: &Path) -> Result<Vec<String>> {
    println!("Downloading individual RGB bands...");

    let bands = [
        ("B04", "red"),   // Red - 10m
        ("B03", "green"), // Green - 10m
        ("B02", "blue"),  // Blue - 10m
    ];

    let mut downloaded = Vec::new();

    for (band, name) in bands {
        let band_url = format!("{}/{}.tif", scene_url, band);
        println!("Downloading {} band: {}", name, band_url);

        let client = reqwest::Client::new();
        let response = client.get(&band_url).send().await?;

        if !response.status().is_success() {
            println!("Warning: {} band not available", name);
            continue;
        }

        let bytes = response.bytes().await?;
        let output_path = output_dir.join(format!("sentinel2_{}.tif", name));
        let mut file = File::create(&output_path)?;
        file.write_all(&bytes)?;

        println!("✓ Saved: {:?}", output_path);
        downloaded.push(output_path.to_string_lossy().to_string());
    }

    Ok(downloaded)
}
