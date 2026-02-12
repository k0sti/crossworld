//! MAPG binary format reader and height/image provider
//!
//! Reads `.mapg` files produced by the `mapdata` CLI tool.
//! The format stores a georeferenced grid of height + color data
//! in ETRS-TM35FIN projected coordinates (EPSG:3067).
//!
//! # Binary Format (v1)
//!
//! | Offset | Size | Type   | Field          |
//! |--------|------|--------|----------------|
//! | 0      | 4    | bytes  | Magic "MAPG"   |
//! | 4      | 4    | u32 LE | Version (1)    |
//! | 8      | 4    | u32 LE | Width (cols)   |
//! | 12     | 4    | u32 LE | Height (rows)  |
//! | 16     | 8    | f64 LE | Resolution m/px|
//! | 24     | 4    | f32 LE | Min height (m) |
//! | 28     | 4    | f32 LE | Max height (m) |
//! | 32     | 8    | f64 LE | Bounds min_x   |
//! | 40     | 8    | f64 LE | Bounds max_x   |
//! | 48     | 8    | f64 LE | Bounds min_y   |
//! | 56     | 8    | f64 LE | Bounds max_y   |
//! | 64     | 7×N  | mixed  | Cells: f32 height + 3×u8 RGB |

use std::io::{BufReader, Read};
use std::path::Path;

use crate::area::Area;
use crate::coords::GeoCoord;
use crate::height::{HeightError, HeightProvider, HeightResult};

/// Magic bytes for .mapg format identification
const MAGIC: &[u8; 4] = b"MAPG";
/// Supported format version
const VERSION: u32 = 1;

/// A single cell in the map grid
#[derive(Clone, Debug, Default)]
pub struct MapgCell {
    /// Height in meters above sea level
    pub height: f32,
    /// RGB color from orthophoto
    pub color: [u8; 3],
}

/// Georeferenced bounds in ETRS-TM35FIN (EPSG:3067) projected coordinates
#[derive(Clone, Debug, Default)]
pub struct MapgBounds {
    /// Minimum easting (meters)
    pub min_x: f64,
    /// Maximum easting (meters)
    pub max_x: f64,
    /// Minimum northing (meters)
    pub min_y: f64,
    /// Maximum northing (meters)
    pub max_y: f64,
}

impl MapgBounds {
    /// Width in meters (easting extent)
    pub fn width(&self) -> f64 {
        self.max_x - self.min_x
    }

    /// Height in meters (northing extent)
    pub fn height(&self) -> f64 {
        self.max_y - self.min_y
    }

    /// Center point (easting, northing)
    pub fn center(&self) -> (f64, f64) {
        (
            (self.min_x + self.max_x) / 2.0,
            (self.min_y + self.max_y) / 2.0,
        )
    }

    /// Convert bounds to approximate WGS84 Area using ETRS-TM35FIN inverse projection
    pub fn to_wgs84_area(&self) -> Area {
        let min_geo = etrs_tm35fin_to_wgs84(self.min_x, self.min_y);
        let max_geo = etrs_tm35fin_to_wgs84(self.max_x, self.max_y);
        Area::new(min_geo, max_geo)
    }
}

/// Loaded .mapg file with height and color grid data
pub struct MapgFile {
    /// Grid width in cells
    pub width: usize,
    /// Grid height in cells
    pub height: usize,
    /// Resolution in meters per cell
    pub resolution: f64,
    /// Minimum height value in the dataset (meters)
    pub min_height: f32,
    /// Maximum height value in the dataset (meters)
    pub max_height: f32,
    /// Georeferenced bounds in ETRS-TM35FIN
    pub bounds: MapgBounds,
    /// Grid cells in row-major order
    pub cells: Vec<MapgCell>,
}

impl MapgFile {
    /// Load a .mapg file from disk
    pub fn load(path: &Path) -> Result<Self, MapgError> {
        let file = std::fs::File::open(path)
            .map_err(|e| MapgError::Io(format!("{}: {}", path.display(), e)))?;
        let mut reader = BufReader::new(file);

        // Verify magic
        let mut magic = [0u8; 4];
        reader
            .read_exact(&mut magic)
            .map_err(|e| MapgError::Io(e.to_string()))?;
        if &magic != MAGIC {
            return Err(MapgError::InvalidFormat("bad magic bytes".into()));
        }

        // Read header
        let version = read_u32(&mut reader)?;
        if version != VERSION {
            return Err(MapgError::InvalidFormat(format!(
                "unsupported version: {}",
                version
            )));
        }

        let width = read_u32(&mut reader)? as usize;
        let height = read_u32(&mut reader)? as usize;
        let resolution = read_f64(&mut reader)?;
        let min_height = read_f32(&mut reader)?;
        let max_height = read_f32(&mut reader)?;

        let bounds = MapgBounds {
            min_x: read_f64(&mut reader)?,
            max_x: read_f64(&mut reader)?,
            min_y: read_f64(&mut reader)?,
            max_y: read_f64(&mut reader)?,
        };

        // Read cells
        let cell_count = width * height;
        let mut cells = Vec::with_capacity(cell_count);

        for _ in 0..cell_count {
            let h = read_f32(&mut reader)?;
            let mut color = [0u8; 3];
            reader
                .read_exact(&mut color)
                .map_err(|e| MapgError::Io(e.to_string()))?;
            cells.push(MapgCell { height: h, color });
        }

        Ok(Self {
            width,
            height,
            resolution,
            min_height,
            max_height,
            bounds,
            cells,
        })
    }

    /// Get cell at grid position (x, y)
    pub fn get(&self, x: usize, y: usize) -> Option<&MapgCell> {
        if x < self.width && y < self.height {
            Some(&self.cells[y * self.width + x])
        } else {
            None
        }
    }

    /// Sample height at ETRS-TM35FIN coordinates with bilinear interpolation
    pub fn sample_height(&self, easting: f64, northing: f64) -> Option<f32> {
        // Convert projected coordinates to grid position
        let gx = (easting - self.bounds.min_x) / self.resolution;
        // Y is flipped: max_y is row 0 (top of image = north)
        let gy = (self.bounds.max_y - northing) / self.resolution;

        if gx < 0.0 || gy < 0.0 {
            return None;
        }

        let x0 = gx.floor() as usize;
        let y0 = gy.floor() as usize;

        if x0 >= self.width || y0 >= self.height {
            return None;
        }

        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);

        let fx = (gx - x0 as f64) as f32;
        let fy = (gy - y0 as f64) as f32;

        let v00 = self.cells[y0 * self.width + x0].height;
        let v10 = self.cells[y0 * self.width + x1].height;
        let v01 = self.cells[y1 * self.width + x0].height;
        let v11 = self.cells[y1 * self.width + x1].height;

        // Skip NaN cells
        if !v00.is_finite() || !v10.is_finite() || !v01.is_finite() || !v11.is_finite() {
            // Fall back to nearest valid sample
            if v00.is_finite() {
                return Some(v00);
            }
            return None;
        }

        let v0 = v00 * (1.0 - fx) + v10 * fx;
        let v1 = v01 * (1.0 - fx) + v11 * fx;
        Some(v0 * (1.0 - fy) + v1 * fy)
    }

    /// Sample color at ETRS-TM35FIN coordinates (nearest neighbor)
    pub fn sample_color(&self, easting: f64, northing: f64) -> Option<[u8; 3]> {
        let gx = ((easting - self.bounds.min_x) / self.resolution).round() as usize;
        let gy = ((self.bounds.max_y - northing) / self.resolution).round() as usize;

        self.get(gx, gy).map(|c| c.color)
    }
}

/// Height provider backed by a .mapg file
///
/// Provides terrain elevation data from preprocessed .mapg files.
/// Coordinates are converted between WGS84 (GeoCoord) and ETRS-TM35FIN
/// internally.
pub struct MapgHeightProvider {
    file: MapgFile,
    /// Cached WGS84 bounds for the `bounds()` method
    wgs84_area: Area,
}

impl MapgHeightProvider {
    /// Create a new provider by loading a .mapg file
    pub fn load(path: &Path) -> Result<Self, MapgError> {
        let file = MapgFile::load(path)?;
        let wgs84_area = file.bounds.to_wgs84_area();
        Ok(Self { file, wgs84_area })
    }

    /// Get a reference to the underlying MapgFile
    pub fn file(&self) -> &MapgFile {
        &self.file
    }
}

impl HeightProvider for MapgHeightProvider {
    fn get_height(&self, coord: &GeoCoord) -> HeightResult<f32> {
        let (easting, northing) = wgs84_to_etrs_tm35fin(coord);
        self.file
            .sample_height(easting, northing)
            .ok_or(HeightError::OutOfBounds { coord: *coord })
    }

    fn bounds(&self) -> Option<Area> {
        Some(self.wgs84_area)
    }

    fn resolution(&self) -> f32 {
        self.file.resolution as f32
    }
}

// ============================================================================
// ETRS-TM35FIN (EPSG:3067) ↔ WGS84 approximate conversion
// ============================================================================
//
// ETRS-TM35FIN is a Transverse Mercator projection:
//   - Central meridian: 27°E
//   - Scale factor: 0.9996
//   - False easting: 500000
//   - False northing: 0
//   - Ellipsoid: GRS80 (practically identical to WGS84)

/// Approximate conversion from ETRS-TM35FIN (easting, northing) to WGS84 (lat, lon).
///
/// Uses a simplified inverse Transverse Mercator. Accuracy is ~1m for Finland,
/// which is sufficient for terrain rendering.
pub fn etrs_tm35fin_to_wgs84(easting: f64, northing: f64) -> GeoCoord {
    // GRS80 ellipsoid parameters
    const A: f64 = 6_378_137.0; // semi-major axis
    const F: f64 = 1.0 / 298.257_222_101; // flattening
    const K0: f64 = 0.9996; // scale factor
    const E0: f64 = 500_000.0; // false easting
    const LON0: f64 = 27.0; // central meridian (degrees)

    let e2 = 2.0 * F - F * F; // eccentricity squared
    let e_prime2 = e2 / (1.0 - e2);

    let x = easting - E0;
    let y = northing;

    let m = y / K0;
    let mu = m / (A * (1.0 - e2 / 4.0 - 3.0 * e2 * e2 / 64.0 - 5.0 * e2 * e2 * e2 / 256.0));

    let e1 = (1.0 - (1.0 - e2).sqrt()) / (1.0 + (1.0 - e2).sqrt());

    let phi1 = mu
        + (3.0 * e1 / 2.0 - 27.0 * e1 * e1 * e1 / 32.0) * (2.0 * mu).sin()
        + (21.0 * e1 * e1 / 16.0 - 55.0 * e1 * e1 * e1 * e1 / 32.0) * (4.0 * mu).sin()
        + (151.0 * e1 * e1 * e1 / 96.0) * (6.0 * mu).sin();

    let sin_phi1 = phi1.sin();
    let cos_phi1 = phi1.cos();
    let tan_phi1 = phi1.tan();

    let n1 = A / (1.0 - e2 * sin_phi1 * sin_phi1).sqrt();
    let t1 = tan_phi1 * tan_phi1;
    let c1 = e_prime2 * cos_phi1 * cos_phi1;
    let r1 = A * (1.0 - e2) / (1.0 - e2 * sin_phi1 * sin_phi1).powf(1.5);
    let d = x / (n1 * K0);

    let lat = phi1
        - (n1 * tan_phi1 / r1)
            * (d * d / 2.0
                - (5.0 + 3.0 * t1 + 10.0 * c1 - 4.0 * c1 * c1 - 9.0 * e_prime2) * d * d * d * d
                    / 24.0
                + (61.0 + 90.0 * t1 + 298.0 * c1 + 45.0 * t1 * t1
                    - 252.0 * e_prime2
                    - 3.0 * c1 * c1)
                    * d.powi(6)
                    / 720.0);

    let lon = (d - (1.0 + 2.0 * t1 + c1) * d * d * d / 6.0
        + (5.0 - 2.0 * c1 + 28.0 * t1 - 3.0 * c1 * c1 + 8.0 * e_prime2 + 24.0 * t1 * t1)
            * d.powi(5)
            / 120.0)
        / cos_phi1;

    GeoCoord::new(lat.to_degrees(), LON0 + lon.to_degrees())
}

/// Approximate conversion from WGS84 (lat, lon) to ETRS-TM35FIN (easting, northing).
///
/// Uses a simplified forward Transverse Mercator.
pub fn wgs84_to_etrs_tm35fin(coord: &GeoCoord) -> (f64, f64) {
    const A: f64 = 6_378_137.0;
    const F: f64 = 1.0 / 298.257_222_101;
    const K0: f64 = 0.9996;
    const E0: f64 = 500_000.0;
    const LON0: f64 = 27.0;

    let e2 = 2.0 * F - F * F;
    let e_prime2 = e2 / (1.0 - e2);

    let lat = coord.lat.to_radians();
    let lon = coord.lon.to_radians();
    let lon0 = LON0.to_radians();

    let sin_lat = lat.sin();
    let cos_lat = lat.cos();
    let tan_lat = lat.tan();

    let n = A / (1.0 - e2 * sin_lat * sin_lat).sqrt();
    let t = tan_lat * tan_lat;
    let c = e_prime2 * cos_lat * cos_lat;
    let a_coeff = (lon - lon0) * cos_lat;

    let m = A
        * ((1.0 - e2 / 4.0 - 3.0 * e2 * e2 / 64.0 - 5.0 * e2 * e2 * e2 / 256.0) * lat
            - (3.0 * e2 / 8.0 + 3.0 * e2 * e2 / 32.0 + 45.0 * e2 * e2 * e2 / 1024.0)
                * (2.0 * lat).sin()
            + (15.0 * e2 * e2 / 256.0 + 45.0 * e2 * e2 * e2 / 1024.0) * (4.0 * lat).sin()
            - (35.0 * e2 * e2 * e2 / 3072.0) * (6.0 * lat).sin());

    let easting = E0
        + K0 * n
            * (a_coeff
                + (1.0 - t + c) * a_coeff.powi(3) / 6.0
                + (5.0 - 18.0 * t + t * t + 72.0 * c - 58.0 * e_prime2) * a_coeff.powi(5) / 120.0);

    let northing = K0
        * (m + n
            * tan_lat
            * (a_coeff * a_coeff / 2.0
                + (5.0 - t + 9.0 * c + 4.0 * c * c) * a_coeff.powi(4) / 24.0
                + (61.0 - 58.0 * t + t * t + 600.0 * c - 330.0 * e_prime2) * a_coeff.powi(6)
                    / 720.0));

    (easting, northing)
}

// ============================================================================
// Errors
// ============================================================================

/// Errors that can occur when loading .mapg files
#[derive(Debug)]
pub enum MapgError {
    /// I/O error reading the file
    Io(String),
    /// Invalid file format (bad magic, unsupported version, etc.)
    InvalidFormat(String),
}

impl std::fmt::Display for MapgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MapgError::Io(msg) => write!(f, "I/O error: {}", msg),
            MapgError::InvalidFormat(msg) => write!(f, "invalid .mapg format: {}", msg),
        }
    }
}

impl std::error::Error for MapgError {}

// ============================================================================
// Binary reading helpers
// ============================================================================

fn read_u32(reader: &mut impl Read) -> Result<u32, MapgError> {
    let mut buf = [0u8; 4];
    reader
        .read_exact(&mut buf)
        .map_err(|e| MapgError::Io(e.to_string()))?;
    Ok(u32::from_le_bytes(buf))
}

fn read_f32(reader: &mut impl Read) -> Result<f32, MapgError> {
    let mut buf = [0u8; 4];
    reader
        .read_exact(&mut buf)
        .map_err(|e| MapgError::Io(e.to_string()))?;
    Ok(f32::from_le_bytes(buf))
}

fn read_f64(reader: &mut impl Read) -> Result<f64, MapgError> {
    let mut buf = [0u8; 8];
    reader
        .read_exact(&mut buf)
        .map_err(|e| MapgError::Io(e.to_string()))?;
    Ok(f64::from_le_bytes(buf))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_etrs_tm35fin_roundtrip() {
        // Helsinki: approx 60.17°N, 24.94°E
        let geo = GeoCoord::new(60.17, 24.94);
        let (easting, northing) = wgs84_to_etrs_tm35fin(&geo);

        // Known approximate values for Helsinki in ETRS-TM35FIN
        assert!((easting - 385_000.0).abs() < 5000.0, "easting: {}", easting);
        assert!(
            (northing - 6_675_000.0).abs() < 5000.0,
            "northing: {}",
            northing
        );

        // Roundtrip
        let back = etrs_tm35fin_to_wgs84(easting, northing);
        assert!(
            (back.lat - geo.lat).abs() < 0.001,
            "lat diff: {}",
            (back.lat - geo.lat).abs()
        );
        assert!(
            (back.lon - geo.lon).abs() < 0.001,
            "lon diff: {}",
            (back.lon - geo.lon).abs()
        );
    }

    #[test]
    fn test_etrs_tm35fin_roundtrip_oulu() {
        // Oulu: approx 65.01°N, 25.47°E
        let geo = GeoCoord::new(65.01, 25.47);
        let (easting, northing) = wgs84_to_etrs_tm35fin(&geo);

        let back = etrs_tm35fin_to_wgs84(easting, northing);
        assert!(
            (back.lat - geo.lat).abs() < 0.001,
            "lat diff: {}",
            (back.lat - geo.lat).abs()
        );
        assert!(
            (back.lon - geo.lon).abs() < 0.001,
            "lon diff: {}",
            (back.lon - geo.lon).abs()
        );
    }

    #[test]
    fn test_mapg_bounds_to_wgs84() {
        // A small area in Helsinki region
        let bounds = MapgBounds {
            min_x: 384_000.0,
            max_x: 386_000.0,
            min_y: 6_674_000.0,
            max_y: 6_676_000.0,
        };

        let area = bounds.to_wgs84_area();
        assert!(area.is_valid());
        // Should be roughly around Helsinki
        assert!(area.min.lat > 59.0 && area.min.lat < 61.0);
        assert!(area.min.lon > 24.0 && area.min.lon < 26.0);
    }
}
