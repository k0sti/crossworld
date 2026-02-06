//! Height map queries and terrain elevation data
//!
//! This module provides traits and implementations for querying terrain elevation.
//! Height data can come from various sources including DEM files, procedural generation,
//! or online elevation services.

use crate::area::Area;
use crate::coords::{GeoCoord, WorldCoord};

/// Result type for height queries
pub type HeightResult<T> = Result<T, HeightError>;

/// Errors that can occur during height queries
#[derive(Debug, Clone)]
pub enum HeightError {
    /// The requested coordinates are outside the available data bounds
    OutOfBounds { coord: GeoCoord },
    /// The data source is not available or failed to load
    DataUnavailable { message: String },
    /// The requested resolution is not supported
    UnsupportedResolution { requested: f32, available: f32 },
    /// Network error when fetching remote data
    NetworkError { message: String },
}

impl std::fmt::Display for HeightError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HeightError::OutOfBounds { coord } => {
                write!(
                    f,
                    "Coordinates out of bounds: ({}, {})",
                    coord.lat, coord.lon
                )
            }
            HeightError::DataUnavailable { message } => {
                write!(f, "Height data unavailable: {}", message)
            }
            HeightError::UnsupportedResolution {
                requested,
                available,
            } => {
                write!(
                    f,
                    "Unsupported resolution: requested {}m, available {}m",
                    requested, available
                )
            }
            HeightError::NetworkError { message } => {
                write!(f, "Network error: {}", message)
            }
        }
    }
}

impl std::error::Error for HeightError {}

/// Trait for height data providers
///
/// Implement this trait to provide elevation data from different sources
/// (DEM files, procedural generation, online APIs, etc.)
pub trait HeightProvider {
    /// Get the elevation at a geographic coordinate
    ///
    /// # Arguments
    /// * `coord` - Geographic coordinate to query
    ///
    /// # Returns
    /// Elevation in meters above sea level, or an error
    fn get_height(&self, coord: &GeoCoord) -> HeightResult<f32>;

    /// Get the elevation at a world coordinate
    ///
    /// Default implementation converts to geographic coordinates and queries.
    /// Override for providers that work directly in world coordinates.
    fn get_height_world(&self, _coord: &WorldCoord) -> HeightResult<f32> {
        Err(HeightError::DataUnavailable {
            message: "World coordinate queries not implemented".to_string(),
        })
    }

    /// Get the area covered by this height provider
    fn bounds(&self) -> Option<Area>;

    /// Get the native resolution of the data in meters per sample
    fn resolution(&self) -> f32;

    /// Check if this provider can provide data for a given area
    fn supports_area(&self, area: &Area) -> bool {
        if let Some(bounds) = self.bounds() {
            bounds.intersects(area)
        } else {
            true // Unbounded providers support all areas
        }
    }
}

/// A height map grid for a specific area
///
/// Contains elevation samples at regular intervals within a bounded area.
#[derive(Debug, Clone)]
pub struct HeightMap {
    /// The geographic area covered by this height map
    pub area: Area,
    /// Width of the grid in samples
    pub width: usize,
    /// Height of the grid in samples
    pub height: usize,
    /// Elevation values in row-major order (meters above sea level)
    pub data: Vec<f32>,
    /// Resolution in meters per sample
    pub resolution: f32,
}

impl HeightMap {
    /// Create a new height map with the given dimensions
    ///
    /// # Arguments
    /// * `area` - Geographic area covered
    /// * `width` - Grid width in samples
    /// * `height` - Grid height in samples
    pub fn new(area: Area, width: usize, height: usize) -> Self {
        let data = vec![0.0; width * height];
        let resolution = (area.width_m() / width as f64) as f32;
        Self {
            area,
            width,
            height,
            data,
            resolution,
        }
    }

    /// Create a height map from existing data
    ///
    /// # Panics
    /// Panics if data length doesn't match width * height
    pub fn from_data(area: Area, width: usize, height: usize, data: Vec<f32>) -> Self {
        assert_eq!(
            data.len(),
            width * height,
            "Data length must equal width * height"
        );
        let resolution = (area.width_m() / width as f64) as f32;
        Self {
            area,
            width,
            height,
            data,
            resolution,
        }
    }

    /// Get the elevation at a grid position
    ///
    /// # Arguments
    /// * `x` - Grid X coordinate (0 to width-1)
    /// * `y` - Grid Y coordinate (0 to height-1)
    pub fn get(&self, x: usize, y: usize) -> Option<f32> {
        if x < self.width && y < self.height {
            Some(self.data[y * self.width + x])
        } else {
            None
        }
    }

    /// Set the elevation at a grid position
    ///
    /// # Arguments
    /// * `x` - Grid X coordinate (0 to width-1)
    /// * `y` - Grid Y coordinate (0 to height-1)
    /// * `value` - Elevation in meters
    pub fn set(&mut self, x: usize, y: usize, value: f32) {
        if x < self.width && y < self.height {
            self.data[y * self.width + x] = value;
        }
    }

    /// Sample the height map at a geographic coordinate with bilinear interpolation
    ///
    /// # Arguments
    /// * `coord` - Geographic coordinate to sample
    pub fn sample(&self, coord: &GeoCoord) -> Option<f32> {
        if !self.area.contains(coord) {
            return None;
        }

        // Normalize coordinates to [0, 1] within the area
        let u = (coord.lon - self.area.min.lon) / self.area.width_deg();
        let v = (coord.lat - self.area.min.lat) / self.area.height_deg();

        // Convert to grid coordinates
        let gx = u * (self.width - 1) as f64;
        let gy = v * (self.height - 1) as f64;

        // Bilinear interpolation
        let x0 = gx.floor() as usize;
        let y0 = gy.floor() as usize;
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);

        let fx = (gx - x0 as f64) as f32;
        let fy = (gy - y0 as f64) as f32;

        let v00 = self.get(x0, y0)?;
        let v10 = self.get(x1, y0)?;
        let v01 = self.get(x0, y1)?;
        let v11 = self.get(x1, y1)?;

        // Bilinear interpolation
        let v0 = v00 * (1.0 - fx) + v10 * fx;
        let v1 = v01 * (1.0 - fx) + v11 * fx;
        Some(v0 * (1.0 - fy) + v1 * fy)
    }

    /// Get the minimum and maximum elevations in the height map
    pub fn min_max(&self) -> (f32, f32) {
        let mut min = f32::INFINITY;
        let mut max = f32::NEG_INFINITY;
        for &v in &self.data {
            if v < min {
                min = v;
            }
            if v > max {
                max = v;
            }
        }
        (min, max)
    }
}

/// Placeholder height provider that returns a flat surface
///
/// This is a simple implementation for testing and development.
/// Replace with actual data sources (DEM, online APIs, etc.) for production.
#[derive(Debug, Clone)]
pub struct FlatHeightProvider {
    /// Constant elevation to return (meters)
    pub elevation: f32,
}

impl FlatHeightProvider {
    /// Create a new flat height provider
    pub fn new(elevation: f32) -> Self {
        Self { elevation }
    }
}

impl Default for FlatHeightProvider {
    fn default() -> Self {
        Self { elevation: 0.0 }
    }
}

impl HeightProvider for FlatHeightProvider {
    fn get_height(&self, _coord: &GeoCoord) -> HeightResult<f32> {
        Ok(self.elevation)
    }

    fn get_height_world(&self, _coord: &WorldCoord) -> HeightResult<f32> {
        Ok(self.elevation)
    }

    fn bounds(&self) -> Option<Area> {
        None // Unbounded - covers the whole world
    }

    fn resolution(&self) -> f32 {
        0.0 // Infinite resolution (constant value)
    }
}

/// Get height at a geographic coordinate
///
/// This is a convenience function that uses the default flat height provider.
/// In production, this should be replaced with actual elevation data sources.
///
/// # Arguments
/// * `coord` - Geographic coordinate to query
///
/// # Returns
/// Elevation in meters above sea level
pub fn get_height(coord: &GeoCoord) -> HeightResult<f32> {
    // Placeholder: return 0 (sea level) for all coordinates
    // TODO: Implement actual height data sources
    let _ = coord;
    Ok(0.0)
}

/// Get height for an area as a height map grid
///
/// # Arguments
/// * `area` - Area to query
/// * `resolution` - Desired resolution in meters per sample
///
/// # Returns
/// A height map grid covering the area
pub fn get_height_map(area: &Area, resolution: f32) -> HeightResult<HeightMap> {
    let width = (area.width_m() / resolution as f64).ceil() as usize;
    let height = (area.height_m() / resolution as f64).ceil() as usize;

    // Placeholder: return flat terrain at sea level
    // TODO: Implement actual height data sources
    Ok(HeightMap::new(*area, width.max(1), height.max(1)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_height_provider() {
        let provider = FlatHeightProvider::new(100.0);
        let coord = GeoCoord::new(45.0, -122.0);
        assert_eq!(provider.get_height(&coord).unwrap(), 100.0);
        assert!(provider.bounds().is_none());
    }

    #[test]
    fn test_height_map_creation() {
        let area = Area::from_center(GeoCoord::new(45.0, -122.0), 0.01, 0.01);
        let map = HeightMap::new(area, 10, 10);
        assert_eq!(map.width, 10);
        assert_eq!(map.height, 10);
        assert_eq!(map.data.len(), 100);
    }

    #[test]
    fn test_height_map_get_set() {
        let area = Area::from_center(GeoCoord::new(45.0, -122.0), 0.01, 0.01);
        let mut map = HeightMap::new(area, 10, 10);

        map.set(5, 5, 100.0);
        assert_eq!(map.get(5, 5), Some(100.0));
        assert_eq!(map.get(0, 0), Some(0.0));
        assert_eq!(map.get(10, 10), None); // Out of bounds
    }

    #[test]
    fn test_height_map_sample() {
        let area = Area::new(GeoCoord::new(45.0, -122.0), GeoCoord::new(46.0, -121.0));
        let mut map = HeightMap::new(area, 2, 2);

        // Set corner values
        map.set(0, 0, 0.0);
        map.set(1, 0, 100.0);
        map.set(0, 1, 100.0);
        map.set(1, 1, 200.0);

        // Sample at center should interpolate
        let center = area.center();
        let height = map.sample(&center).unwrap();
        assert!((height - 100.0).abs() < 1.0); // Should be around 100 (average)

        // Sample outside should return None
        assert!(map.sample(&GeoCoord::new(44.0, -122.0)).is_none());
    }

    #[test]
    fn test_get_height_function() {
        let coord = GeoCoord::new(45.0, -122.0);
        let height = get_height(&coord).unwrap();
        assert_eq!(height, 0.0); // Placeholder returns sea level
    }

    #[test]
    fn test_get_height_map_function() {
        let area = Area::from_center(GeoCoord::new(45.0, -122.0), 0.01, 0.01);
        let map = get_height_map(&area, 100.0).unwrap();
        assert!(map.width > 0);
        assert!(map.height > 0);
    }
}
