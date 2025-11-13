use anyhow::Result;
use map::{basemap, elevation, sentinel2, Region};
use std::path::PathBuf;

/// Example: Get cloud-free map data
///
/// This shows different options for getting cloud-free data:
/// 1. Elevation data (always cloud-free - it's radar)
/// 2. Sentinel-2 with strict cloud filtering (<5%)
/// 3. Pre-processed basemaps (always cloud-free)
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Downloading Cloud-Free Map Data ===\n");

    let madeira = Region::madeira();
    let output_dir = PathBuf::from("./map_data/madeira_cloud_free");
    std::fs::create_dir_all(&output_dir)?;

    println!("Region: Madeira Island\n");

    // 1. Elevation data - always cloud-free (radar-based)
    println!("=== 1. ELEVATION DATA (always cloud-free) ===");
    match elevation::fetch_elevation_data(&madeira, &output_dir).await {
        Ok(path) => println!("✓ Elevation: {}\n", path),
        Err(e) => eprintln!("✗ Elevation failed: {}\n", e),
    }

    // 2. Try Sentinel-2 with strict cloud filtering
    println!("=== 2. SENTINEL-2 SATELLITE (filtering for <5% clouds) ===");
    match sentinel2::fetch_sentinel2_imagery_aws(&madeira, &output_dir).await {
        Ok(path) => {
            println!("✓ Sentinel-2: {}\n", path);
            println!("SUCCESS! Found cloud-free Sentinel-2 imagery.\n");
        }
        Err(e) => {
            eprintln!("✗ Sentinel-2 failed: {}", e);
            eprintln!("No scenes with <5% cloud cover found.");
            eprintln!("Trying alternative: pre-processed basemap...\n");

            // 3. Fall back to cloud-free basemap
            println!("=== 3. CLOUD-FREE BASEMAP (composite imagery) ===");
            match basemap::fetch_basemap_pmtiles(&madeira, &output_dir).await {
                Ok(path) => {
                    println!("✓ Basemap: {}\n", path);
                    println!("This basemap is ALWAYS cloud-free (pre-processed).");
                }
                Err(e) => eprintln!("✗ Basemap failed: {}\n", e),
            }
        }
    }

    println!("\n=== SUMMARY ===");
    println!("Output: {:?}", output_dir);
    println!("\nData comparison:");
    println!("  • Elevation: Radar data, always cloud-free, ~90m resolution");
    println!("  • Sentinel-2: Raw satellite, may have clouds, 10m resolution");
    println!("  • Basemaps: Pre-processed composite, always cloud-free, varies");
    println!("\nFor cloud-free imagery, basemaps are most reliable!");

    Ok(())
}
