use anyhow::{Context, Result};
use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::Region;

/// Fetch elevation data using OpenTopography S3 bucket (no API key required)
/// Uses SRTM GL1 (30m) or GL3 (90m) resolution data
pub async fn fetch_elevation_data(region: &Region, output_dir: &Path) -> Result<String> {
    println!("Fetching elevation data for region: {:?}", region);

    // Try GL1 (30m) first, fall back to GL3 (90m) if not available
    match fetch_opentopography_s3(region, output_dir, "SRTM_GL1").await {
        Ok(path) => Ok(path),
        Err(_) => {
            println!("GL1 (30m) not available, trying GL3 (90m)...");
            fetch_opentopography_s3(region, output_dir, "SRTM_GL3").await
        }
    }
}

/// Download SRTM data from OpenTopography's public S3 bucket
/// This requires no API key or authentication
/// Endpoint: https://opentopography.s3.sdsc.edu
///
/// Directory structure:
/// - SRTM_GL3/SRTM_GL3_srtm/North/North_30_60/N32W017.tif
/// - SRTM_GL1/SRTM_GL1_srtm/North/North_30_60/N32W017.tif (not all regions available)
pub async fn fetch_opentopography_s3(
    region: &Region,
    output_dir: &Path,
    dataset: &str, // "SRTM_GL1" (30m) or "SRTM_GL3" (90m)
) -> Result<String> {
    println!("Downloading {} data from OpenTopography S3...", dataset);

    // Calculate which SRTM tiles we need (1-degree tiles)
    let lat_min = region.south.floor() as i32;
    let lat_max = region.north.ceil() as i32;
    let lon_min = region.west.floor() as i32;
    let lon_max = region.east.ceil() as i32;

    let mut downloaded_files = Vec::new();

    for lat in lat_min..lat_max {
        for lon in lon_min..lon_max {
            let lat_str = if lat >= 0 {
                format!("N{:02}", lat.abs())
            } else {
                format!("S{:02}", lat.abs())
            };
            let lon_str = if lon >= 0 {
                format!("E{:03}", lon.abs())
            } else {
                format!("W{:03}", lon.abs())
            };

            let tile_name = format!("{}{}.tif", lat_str, lon_str);

            // Determine hemisphere and lat range directory
            let (hemisphere, lat_range) = if lat >= 0 {
                let range = if lat < 30 {
                    "North_0_29"
                } else {
                    "North_30_60"
                };
                ("North", range)
            } else {
                let range = if lat.abs() < 30 {
                    "South_0_29"
                } else {
                    "South_30_60"
                };
                ("South", range)
            };

            // OpenTopography S3 path structure
            let url = format!(
                "https://opentopography.s3.sdsc.edu/raster/{}/{}_srtm/{}/{}/{}",
                dataset, dataset, hemisphere, lat_range, tile_name
            );

            println!("Downloading tile: {}", tile_name);
            println!("URL: {}", url);

            let client = reqwest::Client::new();
            let response = client
                .get(&url)
                .send()
                .await
                .context("Failed to fetch SRTM tile")?;

            if !response.status().is_success() {
                println!("Warning: Tile {} not available ({})", tile_name, response.status());
                continue;
            }

            let bytes = response.bytes().await?;
            let output_path = output_dir.join(&tile_name);
            let mut file = File::create(&output_path)?;
            file.write_all(&bytes)?;

            println!("✓ Saved: {:?}", output_path);
            downloaded_files.push(output_path);
        }
    }

    if downloaded_files.is_empty() {
        anyhow::bail!("No SRTM tiles found for the specified region");
    }

    // Return the first file (or we could merge them later)
    Ok(downloaded_files[0].to_string_lossy().to_string())
}

/// Fallback method: Download SRTM tiles from public AWS mirror
/// SRTM 30m data is available in 1-degree tiles
pub async fn fetch_elevation_data_aws(region: &Region, output_dir: &Path) -> Result<String> {
    println!("Fetching elevation data from AWS SRTM mirror...");

    // Calculate which SRTM tiles we need
    let lat_min = region.south.floor() as i32;
    let lat_max = region.north.ceil() as i32;
    let lon_min = region.west.floor() as i32;
    let lon_max = region.east.ceil() as i32;

    let mut tiles = Vec::new();

    for lat in lat_min..lat_max {
        for lon in lon_min..lon_max {
            let lat_str = if lat >= 0 {
                format!("N{:02}", lat.abs())
            } else {
                format!("S{:02}", lat.abs())
            };
            let lon_str = if lon >= 0 {
                format!("E{:03}", lon.abs())
            } else {
                format!("W{:03}", lon.abs())
            };

            let tile_name = format!("{}{}.hgt", lat_str, lon_str);
            tiles.push((lat, lon, tile_name));
        }
    }

    println!("Need {} SRTM tiles: {:?}", tiles.len(), tiles);

    // Download first tile as example
    // AWS mirror: https://elevation-tiles-prod.s3.amazonaws.com/skadi/
    if let Some((_, _, tile_name)) = tiles.first() {
        let lat_str = &tile_name[0..3];
        let url = format!(
            "https://elevation-tiles-prod.s3.amazonaws.com/skadi/{}/{}.zip",
            lat_str, tile_name.trim_end_matches(".hgt")
        );

        println!("Downloading: {}", url);

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch SRTM tile")?;

        if response.status().is_success() {
            let bytes = response.bytes().await?;
            let output_path = output_dir.join(format!("{}.zip", tile_name.trim_end_matches(".hgt")));
            let mut file = File::create(&output_path)?;
            file.write_all(&bytes)?;

            println!("SRTM tile saved to: {:?}", output_path);
            return Ok(output_path.to_string_lossy().to_string());
        }
    }

    anyhow::bail!("Failed to download SRTM data")
}
