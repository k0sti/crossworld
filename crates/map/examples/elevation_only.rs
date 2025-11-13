use anyhow::Result;
use map::{elevation, Region};
use std::path::PathBuf;

/// Example: Download only elevation data (no API keys required)
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Downloading Elevation Data (No API Keys Required) ===\n");

    let madeira = Region::madeira();
    println!("Region: Madeira Island");
    println!("Bounds: N{} S{} E{} W{}\n", madeira.north, madeira.south, madeira.east, madeira.west);

    let output_dir = PathBuf::from("./map_data/madeira");
    std::fs::create_dir_all(&output_dir)?;

    // Download elevation data from OpenTopography S3 (no authentication required)
    println!("Downloading SRTM elevation data from OpenTopography...");
    match elevation::fetch_elevation_data(&madeira, &output_dir).await {
        Ok(path) => {
            println!("\n✓ Success!");
            println!("Downloaded elevation data to: {}", path);
            println!("\nResolution: ~90m (SRTM GL3)");
            println!("Format: GeoTIFF (.tif)");
            println!("\nYou can view these files with QGIS or any GIS software.");
        }
        Err(e) => {
            eprintln!("\n✗ Error: {}", e);
            return Err(e);
        }
    }

    Ok(())
}
