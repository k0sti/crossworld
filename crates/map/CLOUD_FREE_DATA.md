# Getting Cloud-Free Map Data

## The Cloud Problem

Raw satellite imagery from Sentinel-2 captures whatever is in the sky at that moment. If there are clouds over your region, they'll appear in the image. This is normal for satellite imagery!

## Solutions

### ✅ Option 1: Better Sentinel-2 Filtering (RECOMMENDED)

**What I did:** Updated the code to search for scenes with <5% cloud cover instead of <20%.

```bash
cargo run --example cloud_free
```

**Results for Madeira:**
- Found scene from: August 16, 2021
- Cloud cover: **0.0%** ✓
- Resolution: 10m
- File size: ~285 MB

**How it works:**
1. Searches 50 recent scenes via STAC API
2. Filters for <5% cloud cover
3. Sorts by lowest cloud cover first
4. Downloads the clearest image available

**Pros:**
- FREE, no API keys
- High resolution (10m)
- Real satellite imagery
- Usually finds cloud-free scenes

**Cons:**
- May not have recent imagery for all locations
- Depends on weather conditions

---

### Option 2: Pre-Processed Basemaps (ALWAYS Cloud-Free)

Basemaps are **composites** of multiple satellite images, pre-processed to remove clouds. They're ALWAYS cloud-free.

#### A. ESRI World Imagery

```rust
use map::basemap;
basemap::fetch_esri_world_imagery(output_dir).await?;
```

- Free, no API key
- Pre-composited satellite imagery
- Always cloud-free
- Lower resolution for free tier

#### B. Mapbox Satellite

- Requires free API key from mapbox.com
- High quality, cloud-free composite
- Good global coverage

```bash
export MAPBOX_TOKEN="pk.your_token"
cargo run --bin map-fetcher
```

---

### Option 3: PMTiles Basemaps

PMTiles are single-file archives of map tiles. Great for vector maps (OpenStreetMap-style), but less useful for raw satellite imagery.

**For satellite basemaps**, use options 1 or 2 above.

**For vector maps** (roads, buildings, labels):
- Protomaps: https://protomaps.com
- OpenMapTiles: https://openmaptiles.org

PMTiles are best suited for:
- Offline mapping applications
- Vector basemaps (roads, buildings)
- Custom styled maps

---

## Comparison Table

| Source | Cloud-Free? | Resolution | API Key? | Best For |
|--------|-------------|------------|----------|----------|
| **Sentinel-2 (<5% filter)** | Usually ✓ | 10m | No | Recent, high-res imagery |
| **ESRI World Imagery** | Always ✓ | Varies | No | Reliable basemap |
| **Mapbox Satellite** | Always ✓ | 1-10m | Free tier | High-quality composite |
| **OpenStreetMap/PMTiles** | N/A | Vector | No | Maps, not imagery |
| **Elevation (SRTM)** | Always ✓ | 90m | No | Height data |

---

## What Data Did You Download?

The files in `map_data/madeira_cloud_free/`:

### 1. Elevation (always cloud-free)
- `N32W017.tif` (0.12 MB)
- `N32W018.tif` (0.11 MB)
- Radar-based height data
- Resolution: ~90m

### 2. Satellite Imagery
- `sentinel2_tci.tif` (285 MB)
- **0% cloud cover** ✓
- True color (RGB) image
- Resolution: 10m
- Date: August 16, 2021

---

## How to View

All files are GeoTIFF format:

```bash
# View in QGIS
qgis map_data/madeira_cloud_free/sentinel2_tci.tif

# Or get info with GDAL
gdalinfo map_data/madeira_cloud_free/sentinel2_tci.tif
```

---

## Why Was My First Download Cloudy?

The first download (11.8% cloud cover) was selected because:
1. It was the most recent scene
2. The old filter allowed up to 20% clouds

The new version prioritizes **cloud-free** over **recent**, giving you clearer imagery!

---

## Summary

**For cloud-free data:**
1. ✅ Run `cargo run --example cloud_free` (finds <5% cloud scenes)
2. ✅ Use basemaps for guaranteed cloud-free (ESRI, Mapbox)
3. ❌ Don't use PMTiles for satellite imagery (they're for vector maps)

**The data you have now is cloud-free!** (0% cloud cover)
