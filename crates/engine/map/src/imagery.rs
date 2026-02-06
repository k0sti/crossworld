//! Satellite and terrain imagery fetching
//!
//! This module provides traits and types for fetching map imagery tiles
//! from various sources (satellite imagery, terrain maps, etc.)

use crate::area::Area;
use crate::coords::GeoCoord;

/// Result type for imagery operations
pub type ImageResult<T> = Result<T, ImageError>;

/// Errors that can occur during imagery operations
#[derive(Debug, Clone)]
pub enum ImageError {
    /// The requested coordinates are outside the available data bounds
    OutOfBounds { coord: GeoCoord },
    /// The image source is not available or failed to load
    SourceUnavailable { message: String },
    /// The requested zoom level is not supported
    UnsupportedZoom { requested: u8, min: u8, max: u8 },
    /// Network error when fetching remote imagery
    NetworkError { message: String },
    /// Image decoding error
    DecodeError { message: String },
}

impl std::fmt::Display for ImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageError::OutOfBounds { coord } => {
                write!(
                    f,
                    "Coordinates out of bounds: ({}, {})",
                    coord.lat, coord.lon
                )
            }
            ImageError::SourceUnavailable { message } => {
                write!(f, "Image source unavailable: {}", message)
            }
            ImageError::UnsupportedZoom {
                requested,
                min,
                max,
            } => {
                write!(
                    f,
                    "Unsupported zoom level {}: valid range is {}-{}",
                    requested, min, max
                )
            }
            ImageError::NetworkError { message } => {
                write!(f, "Network error: {}", message)
            }
            ImageError::DecodeError { message } => {
                write!(f, "Image decode error: {}", message)
            }
        }
    }
}

impl std::error::Error for ImageError {}

/// Represents a map tile coordinate in the standard Web Mercator tile scheme
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileCoord {
    /// X tile coordinate (0 to 2^zoom - 1)
    pub x: u32,
    /// Y tile coordinate (0 to 2^zoom - 1)
    pub y: u32,
    /// Zoom level (0 to ~22 for most providers)
    pub zoom: u8,
}

impl TileCoord {
    /// Create a new tile coordinate
    pub fn new(x: u32, y: u32, zoom: u8) -> Self {
        Self { x, y, zoom }
    }

    /// Convert geographic coordinates to tile coordinates at a given zoom level
    ///
    /// Uses the standard Web Mercator tile scheme (same as Google Maps, OpenStreetMap, etc.)
    pub fn from_geo(coord: &GeoCoord, zoom: u8) -> Self {
        let n = 2_u32.pow(zoom as u32) as f64;
        let x = ((coord.lon + 180.0) / 360.0 * n).floor() as u32;

        let lat_rad = coord.lat.to_radians();
        let y = ((1.0 - (lat_rad.tan() + 1.0 / lat_rad.cos()).ln() / std::f64::consts::PI) / 2.0
            * n)
            .floor() as u32;

        Self {
            x: x.min(n as u32 - 1),
            y: y.min(n as u32 - 1),
            zoom,
        }
    }

    /// Get the geographic bounds of this tile
    pub fn bounds(&self) -> Area {
        let n = 2_u32.pow(self.zoom as u32) as f64;

        let lon_min = self.x as f64 / n * 360.0 - 180.0;
        let lon_max = (self.x + 1) as f64 / n * 360.0 - 180.0;

        let lat_max = (std::f64::consts::PI * (1.0 - 2.0 * self.y as f64 / n))
            .sinh()
            .atan()
            .to_degrees();
        let lat_min = (std::f64::consts::PI * (1.0 - 2.0 * (self.y + 1) as f64 / n))
            .sinh()
            .atan()
            .to_degrees();

        Area::new(
            GeoCoord::new(lat_min, lon_min),
            GeoCoord::new(lat_max, lon_max),
        )
    }

    /// Get the center of this tile in geographic coordinates
    pub fn center(&self) -> GeoCoord {
        self.bounds().center()
    }

    /// Get the number of tiles at this zoom level (per axis)
    pub fn tiles_at_zoom(zoom: u8) -> u32 {
        2_u32.pow(zoom as u32)
    }
}

/// Represents a tile image with its data and metadata
#[derive(Debug, Clone)]
pub struct TileImage {
    /// Tile coordinate
    pub coord: TileCoord,
    /// Image data as RGBA bytes (row-major, top-to-bottom)
    pub data: Vec<u8>,
    /// Width of the image in pixels
    pub width: u32,
    /// Height of the image in pixels
    pub height: u32,
}

impl TileImage {
    /// Create a new tile image
    pub fn new(coord: TileCoord, data: Vec<u8>, width: u32, height: u32) -> Self {
        Self {
            coord,
            data,
            width,
            height,
        }
    }

    /// Create an empty (transparent) tile image
    pub fn empty(coord: TileCoord, width: u32, height: u32) -> Self {
        let data = vec![0u8; (width * height * 4) as usize];
        Self {
            coord,
            data,
            width,
            height,
        }
    }

    /// Create a solid color tile image
    pub fn solid(coord: TileCoord, width: u32, height: u32, rgba: [u8; 4]) -> Self {
        let pixel_count = (width * height) as usize;
        let mut data = Vec::with_capacity(pixel_count * 4);
        for _ in 0..pixel_count {
            data.extend_from_slice(&rgba);
        }
        Self {
            coord,
            data,
            width,
            height,
        }
    }

    /// Get the pixel at a given position
    ///
    /// # Arguments
    /// * `x` - X coordinate (0 to width-1)
    /// * `y` - Y coordinate (0 to height-1)
    ///
    /// # Returns
    /// RGBA color values, or None if out of bounds
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let idx = ((y * self.width + x) * 4) as usize;
        Some([
            self.data[idx],
            self.data[idx + 1],
            self.data[idx + 2],
            self.data[idx + 3],
        ])
    }
}

/// Trait for imagery tile providers
///
/// Implement this trait to provide map tiles from different sources
/// (OpenStreetMap, satellite imagery, custom tile servers, etc.)
pub trait ImageProvider {
    /// Get a tile image at the given coordinates
    ///
    /// # Arguments
    /// * `coord` - Tile coordinate to fetch
    ///
    /// # Returns
    /// The tile image, or an error
    fn get_tile(&self, coord: &TileCoord) -> ImageResult<TileImage>;

    /// Get the minimum zoom level supported by this provider
    fn min_zoom(&self) -> u8;

    /// Get the maximum zoom level supported by this provider
    fn max_zoom(&self) -> u8;

    /// Get the tile size in pixels (usually 256 or 512)
    fn tile_size(&self) -> u32 {
        256
    }

    /// Check if a zoom level is supported
    fn supports_zoom(&self, zoom: u8) -> bool {
        zoom >= self.min_zoom() && zoom <= self.max_zoom()
    }

    /// Get the attribution string for this provider (required for most tile providers)
    fn attribution(&self) -> &str;
}

/// Placeholder image provider that returns solid color tiles
///
/// This is a simple implementation for testing and development.
/// Replace with actual tile sources for production.
#[derive(Debug, Clone)]
pub struct PlaceholderImageProvider {
    /// Color to fill tiles with (RGBA)
    pub color: [u8; 4],
    /// Tile size in pixels
    pub tile_size: u32,
}

impl PlaceholderImageProvider {
    /// Create a new placeholder provider with a given color
    pub fn new(color: [u8; 4]) -> Self {
        Self {
            color,
            tile_size: 256,
        }
    }

    /// Create a provider with gray placeholder tiles
    pub fn gray() -> Self {
        Self::new([128, 128, 128, 255])
    }
}

impl Default for PlaceholderImageProvider {
    fn default() -> Self {
        Self::gray()
    }
}

impl ImageProvider for PlaceholderImageProvider {
    fn get_tile(&self, coord: &TileCoord) -> ImageResult<TileImage> {
        Ok(TileImage::solid(
            *coord,
            self.tile_size,
            self.tile_size,
            self.color,
        ))
    }

    fn min_zoom(&self) -> u8 {
        0
    }

    fn max_zoom(&self) -> u8 {
        22
    }

    fn tile_size(&self) -> u32 {
        self.tile_size
    }

    fn attribution(&self) -> &str {
        "Placeholder tiles"
    }
}

/// Get an image tile at the given geographic coordinate and zoom level
///
/// This is a convenience function that uses the default placeholder provider.
/// In production, this should be replaced with actual tile sources.
///
/// # Arguments
/// * `coord` - Geographic coordinate
/// * `zoom` - Zoom level (0-22)
///
/// # Returns
/// The tile image containing the given coordinate
pub fn get_image(coord: &GeoCoord, zoom: u8) -> ImageResult<TileImage> {
    let tile_coord = TileCoord::from_geo(coord, zoom);
    let provider = PlaceholderImageProvider::default();
    provider.get_tile(&tile_coord)
}

/// Get all tiles needed to cover a given area at a specific zoom level
///
/// # Arguments
/// * `area` - Geographic area to cover
/// * `zoom` - Zoom level
///
/// # Returns
/// Vector of tile coordinates needed to cover the area
pub fn tiles_for_area(area: &Area, zoom: u8) -> Vec<TileCoord> {
    let min_tile = TileCoord::from_geo(&area.min, zoom);
    let max_tile = TileCoord::from_geo(&area.max, zoom);

    // Note: In web mercator, y increases downward, so min_tile might have larger y
    let (y_start, y_end) = if min_tile.y <= max_tile.y {
        (min_tile.y, max_tile.y)
    } else {
        (max_tile.y, min_tile.y)
    };

    let mut tiles = Vec::new();
    for x in min_tile.x..=max_tile.x {
        for y in y_start..=y_end {
            tiles.push(TileCoord::new(x, y, zoom));
        }
    }
    tiles
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_coord_from_geo() {
        // Test a known location (Portland, OR)
        let portland = GeoCoord::new(45.5155, -122.6789);
        let tile = TileCoord::from_geo(&portland, 10);

        // At zoom 10, Portland should be around tile (163, 353)
        assert!(tile.x > 150 && tile.x < 180);
        assert!(tile.y > 340 && tile.y < 370);
        assert_eq!(tile.zoom, 10);
    }

    #[test]
    fn test_tile_coord_bounds() {
        let tile = TileCoord::new(0, 0, 0);
        let bounds = tile.bounds();

        // Zoom 0 should cover the whole world
        assert!(bounds.min.lat < -80.0);
        assert!(bounds.max.lat > 80.0);
        assert!((bounds.min.lon - (-180.0)).abs() < 0.01);
        assert!((bounds.max.lon - 180.0).abs() < 0.01);
    }

    #[test]
    fn test_tile_image_creation() {
        let coord = TileCoord::new(0, 0, 0);
        let img = TileImage::solid(coord, 256, 256, [255, 0, 0, 255]);

        assert_eq!(img.width, 256);
        assert_eq!(img.height, 256);
        assert_eq!(img.data.len(), 256 * 256 * 4);
        assert_eq!(img.get_pixel(0, 0), Some([255, 0, 0, 255]));
    }

    #[test]
    fn test_placeholder_provider() {
        let provider = PlaceholderImageProvider::gray();
        let coord = TileCoord::new(100, 100, 10);
        let tile = provider.get_tile(&coord).unwrap();

        assert_eq!(tile.width, 256);
        assert_eq!(tile.height, 256);
        assert_eq!(tile.get_pixel(128, 128), Some([128, 128, 128, 255]));
    }

    #[test]
    fn test_get_image_function() {
        let coord = GeoCoord::new(45.0, -122.0);
        let tile = get_image(&coord, 10).unwrap();
        assert_eq!(tile.width, 256);
        assert_eq!(tile.height, 256);
    }

    #[test]
    fn test_tiles_for_area() {
        let area = Area::new(GeoCoord::new(45.0, -122.5), GeoCoord::new(45.5, -122.0));

        let tiles = tiles_for_area(&area, 10);
        assert!(!tiles.is_empty());

        // All tiles should be at zoom 10
        for tile in &tiles {
            assert_eq!(tile.zoom, 10);
        }
    }
}
