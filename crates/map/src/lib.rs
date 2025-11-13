use anyhow::{Context, Result};
use geo::{Coord, Rect};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub mod elevation;
pub mod satellite;
pub mod sentinel2;
pub mod basemap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Region {
    pub north: f64,
    pub south: f64,
    pub east: f64,
    pub west: f64,
}

impl Region {
    pub fn new(north: f64, south: f64, east: f64, west: f64) -> Self {
        Self {
            north,
            south,
            east,
            west,
        }
    }

    pub fn madeira() -> Self {
        // Madeira island coordinates
        Self {
            north: 32.88,
            south: 32.60,
            east: -16.65,
            west: -17.30,
        }
    }

    pub fn to_rect(&self) -> Rect<f64> {
        Rect::new(
            Coord {
                x: self.west,
                y: self.south,
            },
            Coord {
                x: self.east,
                y: self.north,
            },
        )
    }
}

#[derive(Debug)]
pub struct MapData {
    pub region: Region,
    pub elevation_path: Option<String>,
    pub satellite_path: Option<String>,
}

pub async fn fetch_region_data(region: Region, output_dir: &Path) -> Result<MapData> {
    std::fs::create_dir_all(output_dir)
        .context("Failed to create output directory")?;

    let elevation_path = elevation::fetch_elevation_data(&region, output_dir).await?;
    let satellite_path = satellite::fetch_satellite_imagery(&region, output_dir).await?;

    Ok(MapData {
        region,
        elevation_path: Some(elevation_path),
        satellite_path: Some(satellite_path),
    })
}
