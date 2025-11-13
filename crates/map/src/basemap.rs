use anyhow::{Context, Result};
use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::Region;

/// Download cloud-free basemap tiles
/// These are pre-processed map tiles (not raw satellite imagery)
/// They are ALWAYS cloud-free because they're rendered from vector data or composited imagery
pub async fn fetch_basemap_pmtiles(_region: &Region, output_dir: &Path) -> Result<String> {
    println!("Fetching cloud-free basemap data...");
    println!("Note: Basemaps are pre-processed maps (OpenStreetMap-based), not raw satellite imagery");

    // Option 1: Download from Protomaps (free PMTiles of global OSM data)
    fetch_protomaps_pmtiles(output_dir).await
}

/// Download PMTiles from Protomaps
/// Protomaps provides free PMTiles archives of OpenStreetMap data
pub async fn fetch_protomaps_pmtiles(output_dir: &Path) -> Result<String> {
    println!("\nDownloading Protomaps basemap (OpenStreetMap data)...");
    println!("This is a pre-rendered map, always cloud-free!");

    // Protomaps provides a global PMTiles file
    // For a specific region, you'd want to extract or generate a smaller PMTiles file
    let _url = "https://build.protomaps.com/20241025.pmtiles";

    println!("Note: This downloads the GLOBAL PMTiles file (~100GB)");
    println!("For production, you'd want to:");
    println!("  1. Extract just your region from the global file");
    println!("  2. Or use a tile server to serve from the PMTiles");
    println!("  3. Or download regional extracts\n");

    println!("Instead, let me download a regional raster basemap...");
    fetch_esri_world_imagery(output_dir).await
}

/// Download ESRI World Imagery (cloud-free composite basemap)
/// ESRI World Imagery is a composite of multiple satellite sources, pre-processed to be cloud-free
pub async fn fetch_esri_world_imagery(output_dir: &Path) -> Result<String> {
    println!("\nDownloading cloud-free basemap from ESRI World Imagery...");
    println!("This uses Mapbox satellite tiles (cloud-free composite)");

    // For Madeira, we can download tiles from ESRI's free service
    // These are pre-composited and cloud-free
    let center_lat: f64 = 32.74;
    let center_lon: f64 = -16.975;
    let zoom: i32 = 12;

    // Calculate tile coordinates
    let n = 2f64.powi(zoom);
    let x = ((center_lon + 180.0) / 360.0 * n).floor() as i32;
    let y = ((1.0 - (center_lat.to_radians().tan() + 1.0 / center_lat.to_radians().cos()).ln()
        / std::f64::consts::PI) / 2.0 * n).floor() as i32;

    println!("Downloading tile z={}/x={}/y={}", zoom, x, y);

    // ESRI World Imagery (free)
    let url = format!(
        "https://services.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{}/{}/{}",
        zoom, y, x
    );

    println!("URL: {}", url);

    let client = reqwest::Client::builder()
        .user_agent("map-cw/0.1.0")
        .build()?;

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to fetch basemap tile")?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to fetch tile: {}", response.status());
    }

    let bytes = response.bytes().await?;
    let output_path = output_dir.join("basemap_esri.jpg");
    let mut file = File::create(&output_path)?;
    file.write_all(&bytes)?;

    println!("✓ Saved: {:?} ({:.2} KB)", output_path, bytes.len() as f64 / 1024.0);
    println!("\nNote: This is a single tile. For full coverage:");
    println!("  - Download multiple tiles and stitch them together");
    println!("  - Or use a proper tile downloading tool");
    println!("  - Or use a WMS service to get a georeferenced image");

    Ok(output_path.to_string_lossy().to_string())
}

/// Alternative: Download from OpenAerialMap (community-contributed aerial imagery)
#[allow(dead_code)]
pub async fn fetch_openaerialmap(region: &Region, _output_dir: &Path) -> Result<String> {
    println!("Searching OpenAerialMap for imagery...");

    let client = reqwest::Client::new();

    // OpenAerialMap API
    let api_url = format!(
        "https://api.openaerialmap.org/meta?bbox={},{},{},{}",
        region.west, region.south, region.east, region.north
    );

    let response = client.get(&api_url).send().await?;

    if !response.status().is_success() {
        anyhow::bail!("OpenAerialMap API error: {}", response.status());
    }

    let result: serde_json::Value = response.json().await?;

    if let Some(results) = result["results"].as_array() {
        if let Some(image) = results.first() {
            if let Some(uuid) = image["_id"].as_str() {
                if let Some(tms_url) = image["properties"]["tms"].as_str() {
                    println!("Found image: {} ({})",
                        image["title"].as_str().unwrap_or("Unknown"),
                        image["acquisition_end"].as_str().unwrap_or("Unknown date")
                    );
                    println!("TMS URL: {}", tms_url);
                    println!("UUID: {}", uuid);

                    // Note: OpenAerialMap provides TMS endpoints, not direct downloads
                    // You'd need to download tiles from the TMS server
                    return Ok(tms_url.to_string());
                }
            }
        }
    }

    anyhow::bail!("No imagery found in OpenAerialMap for this region")
}
