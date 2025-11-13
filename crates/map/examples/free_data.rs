use anyhow::Result;
use map::{elevation, sentinel2, Region};
use std::path::PathBuf;

/// Example: Download both elevation and satellite data (100% FREE, no API keys!)
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Downloading Map Data (100% FREE - No API Keys!) ===\n");

    let madeira = Region::madeira();
    println!("Region: Madeira Island");
    println!("Bounds: N{} S{} E{} W{}\n", madeira.north, madeira.south, madeira.east, madeira.west);

    let output_dir = PathBuf::from("./map_data/madeira_free");
    std::fs::create_dir_all(&output_dir)?;

    println!("=== 1. ELEVATION DATA ===");
    match elevation::fetch_elevation_data(&madeira, &output_dir).await {
        Ok(path) => {
            println!("✓ Elevation downloaded: {}", path);
            println!("  Resolution: ~90m (SRTM GL3)\n");
        }
        Err(e) => {
            eprintln!("✗ Elevation failed: {}\n", e);
        }
    }

    println!("=== 2. SATELLITE IMAGERY ===");
    match sentinel2::fetch_sentinel2_imagery_aws(&madeira, &output_dir).await {
        Ok(path) => {
            println!("✓ Satellite downloaded: {}", path);
            println!("  Resolution: 10m (Sentinel-2)\n");
        }
        Err(e) => {
            eprintln!("✗ Satellite failed: {}\n", e);
            eprintln!("This might happen if:");
            eprintln!("  - No recent cloud-free imagery available");
            eprintln!("  - Tile ID calculation needs adjustment");
            eprintln!("  - Network issues\n");
        }
    }

    println!("=== SUMMARY ===");
    println!("Output directory: {:?}", output_dir);
    println!("\nFiles downloaded:");
    if let Ok(entries) = std::fs::read_dir(&output_dir) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                println!("  - {} ({:.2} MB)",
                    entry.file_name().to_string_lossy(),
                    metadata.len() as f64 / 1_048_576.0
                );
            }
        }
    }

    println!("\nView with: qgis {}", output_dir.display());

    Ok(())
}
