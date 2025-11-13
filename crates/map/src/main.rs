use anyhow::Result;
use map::{fetch_region_data, Region};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Map Data Fetcher ===\n");

    // Define the Madeira region
    let madeira = Region::madeira();
    println!("Fetching data for Madeira Island");
    println!("Region bounds:");
    println!("  North: {}", madeira.north);
    println!("  South: {}", madeira.south);
    println!("  East:  {}", madeira.east);
    println!("  West:  {}\n", madeira.west);

    // Set output directory
    let output_dir = PathBuf::from("./map_data/madeira");
    println!("Output directory: {:?}\n", output_dir);

    // Fetch the data
    match fetch_region_data(madeira, &output_dir).await {
        Ok(data) => {
            println!("\n=== Success! ===");
            if let Some(elev) = &data.elevation_path {
                println!("Elevation data: {}", elev);
            }
            if let Some(sat) = &data.satellite_path {
                println!("Satellite imagery: {}", sat);
            }
            println!("\nData resolution:");
            println!("  Elevation: ~30m (SRTM)");
            println!("  Satellite: ~1-10m (depending on source)");
        }
        Err(e) => {
            eprintln!("\n=== Error ===");
            eprintln!("Failed to fetch data: {}", e);
            eprintln!("\nMake sure to set up API credentials:");
            eprintln!("  - MAPBOX_TOKEN for satellite imagery");
            eprintln!("  - Or use OpenTopography API key");
            eprintln!("  - Or SENTINEL_HUB_CLIENT_ID and SENTINEL_HUB_CLIENT_SECRET");
            return Err(e);
        }
    }

    Ok(())
}
