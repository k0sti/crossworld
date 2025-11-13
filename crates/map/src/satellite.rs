use anyhow::{Context, Result};
use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::Region;

/// Fetch satellite imagery using Sentinel-2 L2A data (10m resolution)
/// Priority: Sentinel-2 AWS (FREE) > Mapbox > Sentinel Hub > Fallback tiles
pub async fn fetch_satellite_imagery(region: &Region, output_dir: &Path) -> Result<String> {
    println!("Fetching satellite imagery for region: {:?}", region);

    // Try Sentinel-2 from AWS first (FREE, no API key!)
    println!("\nTrying Sentinel-2 from AWS (free, 10m resolution)...");
    match crate::sentinel2::fetch_sentinel2_imagery_aws(region, output_dir).await {
        Ok(path) => return Ok(path),
        Err(e) => {
            println!("Sentinel-2 not available: {}", e);
            println!("Falling back to other sources...\n");
        }
    }

    // Try Mapbox if token is available
    if std::env::var("MAPBOX_TOKEN").is_ok() {
        match fetch_mapbox_satellite(region, output_dir).await {
            Ok(path) => return Ok(path),
            Err(e) => println!("Mapbox failed: {}\n", e),
        }
    }

    // Try Sentinel Hub if credentials available
    if std::env::var("SENTINEL_HUB_CLIENT_ID").is_ok() {
        match fetch_sentinel_hub_imagery(region, output_dir).await {
            Ok(path) => return Ok(path),
            Err(e) => println!("Sentinel Hub failed: {}\n", e),
        }
    }

    // Last resort: simple tile fallback
    fetch_tile_imagery(region, output_dir).await
}

/// Fetch satellite imagery from Mapbox Static Images API
/// This provides decent resolution satellite imagery (1m-10m depending on location)
pub async fn fetch_mapbox_satellite(region: &Region, output_dir: &Path) -> Result<String> {
    println!("Fetching satellite imagery from Mapbox...");

    // Calculate center and bounds
    let _center_lon = (region.west + region.east) / 2.0;
    let _center_lat = (region.north + region.south) / 2.0;

    // Calculate approximate zoom level for the region
    // For Madeira (roughly 0.65 degrees wide), zoom 11-12 is appropriate
    let zoom = 12;
    let width = 1280;
    let height = 1280;

    // Note: Requires Mapbox API token
    // Users should set MAPBOX_TOKEN environment variable
    let token = std::env::var("MAPBOX_TOKEN")
        .unwrap_or_else(|_| "pk.YOUR_MAPBOX_TOKEN".to_string());

    let url = format!(
        "https://api.mapbox.com/styles/v1/mapbox/satellite-v9/static/[{},{},{},{}]/{}/{}x{}?access_token={}",
        region.west, region.south, region.east, region.north,
        zoom, width, height, token
    );

    println!("Requesting satellite imagery from Mapbox...");
    if token == "pk.YOUR_MAPBOX_TOKEN" {
        println!("Note: Set MAPBOX_TOKEN environment variable for actual data");
        println!("Get a free token at https://account.mapbox.com/");
    }

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to fetch satellite imagery")?;

    if !response.status().is_success() {
        println!(
            "Mapbox request failed with status: {}. Trying alternative source...",
            response.status()
        );
        return fetch_sentinel_hub_imagery(region, output_dir).await;
    }

    let bytes = response
        .bytes()
        .await
        .context("Failed to read satellite imagery")?;

    let output_path = output_dir.join("satellite.jpg");
    let mut file = File::create(&output_path)
        .context("Failed to create satellite output file")?;

    file.write_all(&bytes)
        .context("Failed to write satellite imagery")?;

    println!("Satellite imagery saved to: {:?}", output_path);
    Ok(output_path.to_string_lossy().to_string())
}

/// Alternative: Fetch from Sentinel Hub (requires API key)
pub async fn fetch_sentinel_hub_imagery(region: &Region, output_dir: &Path) -> Result<String> {
    println!("Fetching Sentinel-2 imagery...");
    println!("Note: Sentinel Hub requires registration at https://www.sentinel-hub.com/");

    // For demonstration, we'll provide the structure
    // Real implementation requires OAuth and proper API calls
    let client_id = std::env::var("SENTINEL_HUB_CLIENT_ID")
        .unwrap_or_else(|_| "YOUR_CLIENT_ID".to_string());
    let client_secret = std::env::var("SENTINEL_HUB_CLIENT_SECRET")
        .unwrap_or_else(|_| "YOUR_CLIENT_SECRET".to_string());

    if client_id == "YOUR_CLIENT_ID" {
        anyhow::bail!(
            "Sentinel Hub credentials not configured. \
             Set SENTINEL_HUB_CLIENT_ID and SENTINEL_HUB_CLIENT_SECRET environment variables.\n\
             Alternative: Use MAPBOX_TOKEN for Mapbox satellite imagery."
        );
    }

    // OAuth token request
    let token_url = "https://services.sentinel-hub.com/oauth/token";
    let client = reqwest::Client::new();

    let params = [
        ("grant_type", "client_credentials"),
        ("client_id", &client_id),
        ("client_secret", &client_secret),
    ];

    let token_response = client
        .post(token_url)
        .form(&params)
        .send()
        .await
        .context("Failed to get Sentinel Hub token")?;

    let token_data: serde_json::Value = token_response.json().await?;
    let access_token = token_data["access_token"]
        .as_str()
        .context("No access token in response")?;

    // Process API request (simplified)
    let process_url = "https://services.sentinel-hub.com/api/v1/process";

    let request_body = serde_json::json!({
        "input": {
            "bounds": {
                "bbox": [region.west, region.south, region.east, region.north],
                "properties": {
                    "crs": "http://www.opengis.net/def/crs/EPSG/0/4326"
                }
            },
            "data": [{
                "type": "sentinel-2-l2a",
                "dataFilter": {
                    "timeRange": {
                        "from": "2024-01-01T00:00:00Z",
                        "to": "2024-12-31T23:59:59Z"
                    },
                    "maxCloudCoverage": 10
                }
            }]
        },
        "output": {
            "width": 2560,
            "height": 2560,
            "responses": [{
                "identifier": "default",
                "format": {
                    "type": "image/jpeg"
                }
            }]
        },
        "evalscript": "//VERSION=3\nfunction setup() {return {input: [\"B04\", \"B03\", \"B02\"], output: {bands: 3}}}\nfunction evaluatePixel(sample) {return [2.5 * sample.B04, 2.5 * sample.B03, 2.5 * sample.B02]}"
    });

    let response = client
        .post(process_url)
        .bearer_auth(access_token)
        .json(&request_body)
        .send()
        .await
        .context("Failed to fetch Sentinel imagery")?;

    if !response.status().is_success() {
        anyhow::bail!("Sentinel Hub request failed: {}", response.status());
    }

    let bytes = response.bytes().await?;
    let output_path = output_dir.join("satellite_sentinel.jpg");
    let mut file = File::create(&output_path)?;
    file.write_all(&bytes)?;

    println!("Sentinel-2 imagery saved to: {:?}", output_path);
    Ok(output_path.to_string_lossy().to_string())
}

/// Simple fallback: Download from a tile server
pub async fn fetch_tile_imagery(region: &Region, output_dir: &Path) -> Result<String> {
    println!("Fetching imagery from tile server...");

    // Calculate center
    let center_lon = (region.west + region.east) / 2.0;
    let center_lat = (region.north + region.south) / 2.0;
    let zoom = 12;

    // Convert lat/lon to tile coordinates
    let n = 2f64.powi(zoom);
    let x = ((center_lon + 180.0) / 360.0 * n).floor() as i32;
    let y = ((1.0 - (center_lat.to_radians().tan() + 1.0 / center_lat.to_radians().cos()).ln() / std::f64::consts::PI) / 2.0 * n).floor() as i32;

    // OpenStreetMap tile server (for demonstration - use with caution, has rate limits)
    let url = format!(
        "https://tile.openstreetmap.org/{}/{}/{}.png",
        zoom, x, y
    );

    println!("Downloading tile: {}", url);

    let client = reqwest::Client::builder()
        .user_agent("map-cw/0.1.0")
        .build()?;

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to fetch tile")?;

    if response.status().is_success() {
        let bytes = response.bytes().await?;
        let output_path = output_dir.join("tile.png");
        let mut file = File::create(&output_path)?;
        file.write_all(&bytes)?;

        println!("Tile saved to: {:?}", output_path);
        Ok(output_path.to_string_lossy().to_string())
    } else {
        anyhow::bail!("Failed to fetch tile: {}", response.status())
    }
}
