use anyhow::Result;
use map::{elevation, satellite, Region};
use std::path::PathBuf;

/// Example of fetching data without API keys
/// This uses free public data sources that don't require registration
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Fetching Map Data (No API Keys Required) ===\n");

    let madeira = Region::madeira();
    let output_dir = PathBuf::from("./map_data/madeira_no_key");

    println!("Region: Madeira Island");
    println!("Output: {:?}\n", output_dir);

    std::fs::create_dir_all(&output_dir)?;

    // Fetch elevation data from AWS SRTM mirror (no API key needed)
    println!("1. Fetching elevation data from AWS...");
    match elevation::fetch_elevation_data_aws(&madeira, &output_dir).await {
        Ok(path) => println!("   ✓ Saved to: {}\n", path),
        Err(e) => println!("   ✗ Error: {}\n", e),
    }

    // Fetch a sample tile (note: OSM tiles have rate limits)
    println!("2. Fetching sample satellite tile...");
    match satellite::fetch_tile_imagery(&madeira, &output_dir).await {
        Ok(path) => println!("   ✓ Saved to: {}\n", path),
        Err(e) => println!("   ✗ Error: {}\n", e),
    }

    println!("Done!");
    println!("\nNote: For higher quality imagery, consider using:");
    println!("  - Mapbox (set MAPBOX_TOKEN)");
    println!("  - Sentinel Hub (set SENTINEL_HUB_CLIENT_ID/SECRET)");

    Ok(())
}
