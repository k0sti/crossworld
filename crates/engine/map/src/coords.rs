//! Coordinate types for geographic and projected positions
//!
//! This module provides types for representing geographic coordinates (latitude/longitude)
//! and world-space coordinates used in the Crossworld voxel engine.

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Geographic coordinate using WGS84 datum (latitude/longitude)
///
/// This is the standard coordinate system used by GPS and most mapping services.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GeoCoord {
    /// Latitude in degrees (-90 to 90, positive = north)
    pub lat: f64,
    /// Longitude in degrees (-180 to 180, positive = east)
    pub lon: f64,
}

impl GeoCoord {
    /// Create a new geographic coordinate
    ///
    /// # Arguments
    /// * `lat` - Latitude in degrees (-90 to 90)
    /// * `lon` - Longitude in degrees (-180 to 180)
    pub fn new(lat: f64, lon: f64) -> Self {
        Self { lat, lon }
    }

    /// Check if the coordinate is within valid ranges
    pub fn is_valid(&self) -> bool {
        (-90.0..=90.0).contains(&self.lat) && (-180.0..=180.0).contains(&self.lon)
    }

    /// Calculate approximate distance to another coordinate in meters
    /// using the Haversine formula
    pub fn distance_to(&self, other: &GeoCoord) -> f64 {
        const EARTH_RADIUS_M: f64 = 6_371_000.0;

        let lat1 = self.lat.to_radians();
        let lat2 = other.lat.to_radians();
        let dlat = (other.lat - self.lat).to_radians();
        let dlon = (other.lon - self.lon).to_radians();

        let a = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().asin();

        EARTH_RADIUS_M * c
    }
}

impl Default for GeoCoord {
    fn default() -> Self {
        // Default to null island (0, 0)
        Self { lat: 0.0, lon: 0.0 }
    }
}

/// World-space coordinate in Crossworld's voxel coordinate system
///
/// This represents a position in the game world, which may be derived from
/// geographic coordinates via projection or generated procedurally.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WorldCoord {
    /// X position in world units
    pub x: f32,
    /// Y position in world units (vertical/up)
    pub y: f32,
    /// Z position in world units
    pub z: f32,
}

impl WorldCoord {
    /// Create a new world coordinate
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Create from a glam Vec3
    pub fn from_vec3(v: Vec3) -> Self {
        Self {
            x: v.x,
            y: v.y,
            z: v.z,
        }
    }

    /// Convert to a glam Vec3
    pub fn to_vec3(&self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }

    /// Calculate distance to another world coordinate
    pub fn distance_to(&self, other: &WorldCoord) -> f32 {
        self.to_vec3().distance(other.to_vec3())
    }
}

impl Default for WorldCoord {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

impl From<Vec3> for WorldCoord {
    fn from(v: Vec3) -> Self {
        Self::from_vec3(v)
    }
}

impl From<WorldCoord> for Vec3 {
    fn from(c: WorldCoord) -> Self {
        c.to_vec3()
    }
}

/// Projection for converting between geographic and world coordinates
///
/// This trait allows different projection methods to be used depending on
/// the scale and location of the map being rendered.
pub trait Projection {
    /// Convert geographic coordinates to world coordinates
    fn geo_to_world(&self, geo: &GeoCoord) -> WorldCoord;

    /// Convert world coordinates to geographic coordinates
    fn world_to_geo(&self, world: &WorldCoord) -> GeoCoord;
}

/// Simple equirectangular projection for local areas
///
/// This projection works well for small areas (< 100km) where the curvature
/// of the Earth can be ignored.
#[derive(Debug, Clone)]
pub struct LocalProjection {
    /// Origin point in geographic coordinates
    pub origin: GeoCoord,
    /// Scale factor (meters per world unit)
    pub scale: f32,
}

impl LocalProjection {
    /// Create a new local projection centered on the given origin
    ///
    /// # Arguments
    /// * `origin` - Center point in geographic coordinates
    /// * `scale` - Meters per world unit (e.g., 1.0 for 1 meter = 1 world unit)
    pub fn new(origin: GeoCoord, scale: f32) -> Self {
        Self { origin, scale }
    }
}

impl Default for LocalProjection {
    fn default() -> Self {
        Self {
            origin: GeoCoord::default(),
            scale: 1.0,
        }
    }
}

impl Projection for LocalProjection {
    fn geo_to_world(&self, geo: &GeoCoord) -> WorldCoord {
        // Approximate meters per degree at the origin latitude
        let lat_rad = self.origin.lat.to_radians();
        let meters_per_deg_lat = 111_320.0; // Approximate
        let meters_per_deg_lon = 111_320.0 * lat_rad.cos();

        let dx = ((geo.lon - self.origin.lon) * meters_per_deg_lon) as f32 / self.scale;
        let dz = ((geo.lat - self.origin.lat) * meters_per_deg_lat) as f32 / self.scale;

        WorldCoord::new(dx, 0.0, dz)
    }

    fn world_to_geo(&self, world: &WorldCoord) -> GeoCoord {
        let lat_rad = self.origin.lat.to_radians();
        let meters_per_deg_lat = 111_320.0;
        let meters_per_deg_lon = 111_320.0 * lat_rad.cos();

        let dlon = (world.x * self.scale) as f64 / meters_per_deg_lon;
        let dlat = (world.z * self.scale) as f64 / meters_per_deg_lat;

        GeoCoord::new(self.origin.lat + dlat, self.origin.lon + dlon)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geo_coord_validity() {
        assert!(GeoCoord::new(45.0, -122.0).is_valid());
        assert!(GeoCoord::new(90.0, 180.0).is_valid());
        assert!(GeoCoord::new(-90.0, -180.0).is_valid());
        assert!(!GeoCoord::new(91.0, 0.0).is_valid());
        assert!(!GeoCoord::new(0.0, 181.0).is_valid());
    }

    #[test]
    fn test_geo_coord_distance() {
        // Test distance between two points
        let portland = GeoCoord::new(45.5155, -122.6789);
        let seattle = GeoCoord::new(47.6062, -122.3321);

        let distance = portland.distance_to(&seattle);
        // Approximately 233 km
        assert!((distance - 233_000.0).abs() < 5000.0);
    }

    #[test]
    fn test_world_coord_conversion() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        let coord = WorldCoord::from_vec3(v);
        assert_eq!(coord.x, 1.0);
        assert_eq!(coord.y, 2.0);
        assert_eq!(coord.z, 3.0);
        assert_eq!(coord.to_vec3(), v);
    }

    #[test]
    fn test_local_projection_roundtrip() {
        let origin = GeoCoord::new(45.0, -122.0);
        let projection = LocalProjection::new(origin, 1.0);

        let test_geo = GeoCoord::new(45.001, -121.999);
        let world = projection.geo_to_world(&test_geo);
        let back = projection.world_to_geo(&world);

        assert!((back.lat - test_geo.lat).abs() < 0.0001);
        assert!((back.lon - test_geo.lon).abs() < 0.0001);
    }
}
