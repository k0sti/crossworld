//! Area definitions for querying regions of the world
//!
//! This module provides types for defining rectangular areas of the world,
//! used when querying height maps, imagery, or OSM data.

use serde::{Deserialize, Serialize};

use crate::coords::{GeoCoord, WorldCoord};

/// A rectangular area defined in geographic coordinates (bounding box)
///
/// The area is defined by its southwest (minimum) and northeast (maximum) corners.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Area {
    /// Southwest corner (minimum lat/lon)
    pub min: GeoCoord,
    /// Northeast corner (maximum lat/lon)
    pub max: GeoCoord,
}

impl Area {
    /// Create a new area from southwest and northeast corners
    ///
    /// # Arguments
    /// * `min` - Southwest corner (minimum latitude and longitude)
    /// * `max` - Northeast corner (maximum latitude and longitude)
    pub fn new(min: GeoCoord, max: GeoCoord) -> Self {
        Self { min, max }
    }

    /// Create an area from center point and size in degrees
    ///
    /// # Arguments
    /// * `center` - Center point of the area
    /// * `width_deg` - Width in degrees (longitude span)
    /// * `height_deg` - Height in degrees (latitude span)
    pub fn from_center(center: GeoCoord, width_deg: f64, height_deg: f64) -> Self {
        let half_w = width_deg / 2.0;
        let half_h = height_deg / 2.0;

        Self {
            min: GeoCoord::new(center.lat - half_h, center.lon - half_w),
            max: GeoCoord::new(center.lat + half_h, center.lon + half_w),
        }
    }

    /// Create an area from center point and radius in meters
    ///
    /// This approximates a square area with the given radius from center to edge.
    ///
    /// # Arguments
    /// * `center` - Center point of the area
    /// * `radius_m` - Distance from center to each edge in meters
    pub fn from_center_radius(center: GeoCoord, radius_m: f64) -> Self {
        // Approximate degrees per meter at this latitude
        let lat_rad = center.lat.to_radians();
        let deg_per_m_lat = 1.0 / 111_320.0;
        let deg_per_m_lon = 1.0 / (111_320.0 * lat_rad.cos());

        let dlat = radius_m * deg_per_m_lat;
        let dlon = radius_m * deg_per_m_lon;

        Self {
            min: GeoCoord::new(center.lat - dlat, center.lon - dlon),
            max: GeoCoord::new(center.lat + dlat, center.lon + dlon),
        }
    }

    /// Check if the area bounds are valid
    pub fn is_valid(&self) -> bool {
        self.min.is_valid()
            && self.max.is_valid()
            && self.min.lat <= self.max.lat
            && self.min.lon <= self.max.lon
    }

    /// Get the center point of the area
    pub fn center(&self) -> GeoCoord {
        GeoCoord::new(
            (self.min.lat + self.max.lat) / 2.0,
            (self.min.lon + self.max.lon) / 2.0,
        )
    }

    /// Get the width of the area in degrees (longitude span)
    pub fn width_deg(&self) -> f64 {
        self.max.lon - self.min.lon
    }

    /// Get the height of the area in degrees (latitude span)
    pub fn height_deg(&self) -> f64 {
        self.max.lat - self.min.lat
    }

    /// Approximate width of the area in meters
    pub fn width_m(&self) -> f64 {
        let center_lat = self.center().lat.to_radians();
        let meters_per_deg = 111_320.0 * center_lat.cos();
        self.width_deg() * meters_per_deg
    }

    /// Approximate height of the area in meters
    pub fn height_m(&self) -> f64 {
        self.height_deg() * 111_320.0
    }

    /// Check if a point is contained within this area
    pub fn contains(&self, point: &GeoCoord) -> bool {
        point.lat >= self.min.lat
            && point.lat <= self.max.lat
            && point.lon >= self.min.lon
            && point.lon <= self.max.lon
    }

    /// Check if this area intersects with another
    pub fn intersects(&self, other: &Area) -> bool {
        !(self.max.lat < other.min.lat
            || self.min.lat > other.max.lat
            || self.max.lon < other.min.lon
            || self.min.lon > other.max.lon)
    }

    /// Expand the area by a given amount in meters on all sides
    pub fn expand(&self, meters: f64) -> Area {
        let center = self.center();
        let lat_rad = center.lat.to_radians();
        let deg_per_m_lat = 1.0 / 111_320.0;
        let deg_per_m_lon = 1.0 / (111_320.0 * lat_rad.cos());

        let dlat = meters * deg_per_m_lat;
        let dlon = meters * deg_per_m_lon;

        Area {
            min: GeoCoord::new(self.min.lat - dlat, self.min.lon - dlon),
            max: GeoCoord::new(self.max.lat + dlat, self.max.lon + dlon),
        }
    }
}

impl Default for Area {
    fn default() -> Self {
        // Default to a 1km x 1km area centered on null island
        Self::from_center_radius(GeoCoord::default(), 500.0)
    }
}

/// A rectangular area defined in world coordinates
///
/// Used for querying data within the Crossworld coordinate system.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WorldArea {
    /// Minimum corner (lowest x, y, z values)
    pub min: WorldCoord,
    /// Maximum corner (highest x, y, z values)
    pub max: WorldCoord,
}

impl WorldArea {
    /// Create a new world area from min and max corners
    pub fn new(min: WorldCoord, max: WorldCoord) -> Self {
        Self { min, max }
    }

    /// Create a world area from center and half-extents
    pub fn from_center(center: WorldCoord, half_extents: WorldCoord) -> Self {
        Self {
            min: WorldCoord::new(
                center.x - half_extents.x,
                center.y - half_extents.y,
                center.z - half_extents.z,
            ),
            max: WorldCoord::new(
                center.x + half_extents.x,
                center.y + half_extents.y,
                center.z + half_extents.z,
            ),
        }
    }

    /// Get the center of the area
    pub fn center(&self) -> WorldCoord {
        WorldCoord::new(
            (self.min.x + self.max.x) / 2.0,
            (self.min.y + self.max.y) / 2.0,
            (self.min.z + self.max.z) / 2.0,
        )
    }

    /// Get the size of the area
    pub fn size(&self) -> WorldCoord {
        WorldCoord::new(
            self.max.x - self.min.x,
            self.max.y - self.min.y,
            self.max.z - self.min.z,
        )
    }

    /// Check if a point is contained within this area
    pub fn contains(&self, point: &WorldCoord) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
    }
}

impl Default for WorldArea {
    fn default() -> Self {
        Self {
            min: WorldCoord::new(-100.0, -100.0, -100.0),
            max: WorldCoord::new(100.0, 100.0, 100.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_area_creation() {
        let area = Area::new(GeoCoord::new(45.0, -123.0), GeoCoord::new(46.0, -122.0));
        assert!(area.is_valid());
        assert_eq!(area.width_deg(), 1.0);
        assert_eq!(area.height_deg(), 1.0);
    }

    #[test]
    fn test_area_from_center() {
        let center = GeoCoord::new(45.5, -122.5);
        let area = Area::from_center(center, 1.0, 0.5);

        assert!((area.center().lat - center.lat).abs() < 0.0001);
        assert!((area.center().lon - center.lon).abs() < 0.0001);
        assert!((area.width_deg() - 1.0).abs() < 0.0001);
        assert!((area.height_deg() - 0.5).abs() < 0.0001);
    }

    #[test]
    fn test_area_contains() {
        let area = Area::new(GeoCoord::new(45.0, -123.0), GeoCoord::new(46.0, -122.0));

        assert!(area.contains(&GeoCoord::new(45.5, -122.5)));
        assert!(area.contains(&GeoCoord::new(45.0, -123.0))); // Corner
        assert!(!area.contains(&GeoCoord::new(44.9, -122.5))); // Outside
    }

    #[test]
    fn test_area_intersects() {
        let area1 = Area::new(GeoCoord::new(45.0, -123.0), GeoCoord::new(46.0, -122.0));
        let area2 = Area::new(GeoCoord::new(45.5, -122.5), GeoCoord::new(46.5, -121.5));
        let area3 = Area::new(GeoCoord::new(47.0, -120.0), GeoCoord::new(48.0, -119.0));

        assert!(area1.intersects(&area2));
        assert!(!area1.intersects(&area3));
    }

    #[test]
    fn test_world_area_contains() {
        let area = WorldArea::new(
            WorldCoord::new(0.0, 0.0, 0.0),
            WorldCoord::new(100.0, 50.0, 100.0),
        );

        assert!(area.contains(&WorldCoord::new(50.0, 25.0, 50.0)));
        assert!(!area.contains(&WorldCoord::new(150.0, 25.0, 50.0)));
    }
}
