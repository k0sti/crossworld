# Map Data Fetcher

Rust library and CLI tool for fetching elevation (height map) and satellite imagery data for any world location.

## Features

- **100% FREE** - No API keys required! ✓
- **Elevation Data**: SRTM (Shuttle Radar Topography Mission) at ~90m resolution via OpenTopography S3
- **Satellite Imagery**: Sentinel-2 at 10m resolution via AWS S3 (completely free!)
- **Multiple Data Sources**: Supports OpenTopography, Sentinel-2 COGs, Mapbox (optional), Sentinel Hub (optional)
- **Easy to Use**: Pre-configured region for Madeira, easily extensible to other locations
- **Smart Scene Selection**: Automatically finds recent, low-cloud imagery via STAC API

## Data Resolution

- **Elevation**: ~90 meters (SRTM GL3, global coverage)
- **Satellite**: 10 meters (Sentinel-2 True Color Image)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
map = { path = "../map" }
```

## Usage

### As a Library

```rust
use map::{fetch_region_data, Region};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Use pre-configured Madeira region
    let madeira = Region::madeira();

    // Or create custom region
    let custom = Region::new(
        32.88,  // north
        32.60,  // south
        -16.65, // east
        -17.30  // west
    );

    let output_dir = PathBuf::from("./data");
    let result = fetch_region_data(madeira, &output_dir).await?;

    println!("Elevation: {:?}", result.elevation_path);
    println!("Satellite: {:?}", result.satellite_path);

    Ok(())
}
```

### As a CLI Tool

```bash
# Download CLOUD-FREE data (recommended!) ✓
cargo run --example cloud_free

# Or download any available satellite data
cargo run --example free_data

# Or download ONLY elevation data
cargo run --example elevation_only

# Optional: Use Mapbox for different satellite imagery
export MAPBOX_TOKEN="pk.your_token"
cargo run --bin map-fetcher
```

**Note:** Raw satellite imagery may have clouds. See [CLOUD_FREE_DATA.md](CLOUD_FREE_DATA.md) for solutions!

## Data Sources (100% FREE!)

### ✓ Elevation Data - FREE, No API Key!

Elevation data is fetched from **OpenTopography's public S3 bucket**:
- **SRTM GL3** (90m resolution) - global coverage
- **Direct download** from: `https://opentopography.s3.sdsc.edu`
- Format: GeoTIFF (.tif)

### ✓ Satellite Imagery - FREE, No API Key!

Satellite imagery is fetched from **Sentinel-2 on AWS**:
- **Sentinel-2 L2A** (10m resolution) - high quality RGB imagery
- **Cloud-Optimized GeoTIFFs** (COGs) for fast access
- **Smart scene search** via Element 84's STAC API
- Automatically finds recent, low-cloud coverage scenes
- Direct download from: `https://sentinel-cogs.s3.us-west-2.amazonaws.com`
- Format: GeoTIFF (.tif)

### Optional: Commercial Satellite Sources

If you want alternative imagery sources:

**Mapbox**
- Resolution: 1-10m
- Requires free account: https://account.mapbox.com/
- Set `MAPBOX_TOKEN` environment variable

**Sentinel Hub**
- High quality, custom processing
- Requires account: https://www.sentinel-hub.com/
- Set `SENTINEL_HUB_CLIENT_ID` and `SENTINEL_HUB_CLIENT_SECRET`

## Example: Madeira Island

The library comes with pre-configured coordinates for Madeira Island:

```rust
let madeira = Region::madeira();
// Coordinates:
// North: 32.88°
// South: 32.60°
// East: -16.65°
// West: -17.30°
```

## Output

Data is saved to the specified output directory:

```
map_data/madeira_free/
├── N32W017.tif          # GeoTIFF elevation tile 1 (~0.12 MB)
├── N32W018.tif          # GeoTIFF elevation tile 2 (~0.11 MB)
└── sentinel2_tci.tif    # Sentinel-2 True Color Image (~262 MB, 10m resolution)
```

- **Elevation tiles**: Each covers a 1°×1° area. Multiple tiles are downloaded automatically to cover your region.
- **Satellite image**: Single GeoTIFF covering the entire region at 10m resolution.

## Data Formats

- **Elevation**: GeoTIFF (.tif) or HGT (.hgt) format
- **Satellite**: JPEG (.jpg) or PNG (.png)

## Examples for Other Locations

```rust
// Iceland
let iceland = Region::new(66.5, 63.0, -13.5, -24.5);

// Hawaii
let hawaii = Region::new(22.5, 18.5, -154.5, -160.5);

// Alps
let alps = Region::new(47.5, 45.0, 8.5, 5.5);
```

## Error Handling

The library uses `anyhow::Result` for error handling. Common errors:

- Missing API credentials
- Network issues
- Invalid region coordinates
- Rate limiting from tile servers

## Contributing

To add new data sources, implement functions in:
- `src/elevation.rs` for elevation data
- `src/satellite.rs` for satellite imagery

## License

This project is part of the map-cw workspace.
