//! Region collision data cache
//!
//! Caches faces per region to avoid repeated octree traversal during collision queries.

use super::region_id::RegionId;
use super::triangle_gen::{face_aabb, face_to_triangle};
use cube::{visit_faces_in_region, Cube, FaceInfo};
use rapier3d::parry::bounding_volume::Aabb;
use rapier3d::parry::shape::Triangle;

/// Cached collision data for a region
///
/// Stores faces queried via `visit_faces_in_region()`, avoiding repeated
/// octree traversal for the same region.
pub struct RegionCollisionData {
    /// The region this data belongs to
    pub region: RegionId,
    /// World-space AABB of the region
    pub aabb: Aabb,
    /// Faces found in this region
    pub faces: Vec<FaceInfo>,
    /// Terrain version when this cache was built
    pub version: u64,
}

impl RegionCollisionData {
    /// Build from octree using cube crate traversal
    ///
    /// # Arguments
    /// * `cube` - The octree to query
    /// * `region` - The region to cache
    /// * `world_size` - World size in units
    /// * `border_materials` - Material IDs for border layers
    pub fn from_octree(
        cube: &Cube<u8>,
        region: RegionId,
        world_size: f32,
        border_materials: [u8; 4],
    ) -> Self {
        let bounds = region.to_region_bounds();
        let mut faces = Vec::new();

        visit_faces_in_region(
            cube,
            &bounds,
            |face_info| {
                faces.push(face_info.clone());
            },
            border_materials,
        );

        let aabb = region.world_aabb(world_size);

        Self {
            region,
            aabb,
            faces,
            version: 0,
        }
    }

    /// Number of triangles (2 per face)
    #[inline]
    pub fn triangle_count(&self) -> usize {
        self.faces.len() * 2
    }

    /// Check if the region has any collision geometry
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.faces.is_empty()
    }

    /// Get triangle by index
    ///
    /// # Arguments
    /// * `idx` - Triangle index (0 to triangle_count - 1)
    /// * `world_size` - World size in units
    ///
    /// # Returns
    /// The triangle if index is valid, None otherwise
    pub fn get_triangle(&self, idx: u16, world_size: f32) -> Option<Triangle> {
        let face_idx = idx as usize / 2;
        let tri_in_face = (idx % 2) as u8;

        self.faces
            .get(face_idx)
            .map(|face| face_to_triangle(face, tri_in_face, world_size))
    }

    /// Get triangle AABB by index
    ///
    /// This returns the AABB of the face containing the triangle, which is
    /// slightly larger than the exact triangle AABB but faster to compute.
    ///
    /// # Arguments
    /// * `idx` - Triangle index (0 to triangle_count - 1)
    /// * `world_size` - World size in units
    ///
    /// # Returns
    /// The AABB if index is valid, None otherwise
    pub fn get_triangle_aabb(&self, idx: u16, world_size: f32) -> Option<Aabb> {
        let face_idx = idx as usize / 2;
        self.faces
            .get(face_idx)
            .map(|face| face_aabb(face, world_size))
    }

    /// Get all face information for iteration
    pub fn faces(&self) -> &[FaceInfo] {
        &self.faces
    }

    /// Update the version number
    pub fn set_version(&mut self, version: u64) {
        self.version = version;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_region() {
        // Create an empty cube (all air)
        let cube = Cube::Solid(0);
        let region = RegionId::new(glam::IVec3::ZERO, 1);

        let data = RegionCollisionData::from_octree(&cube, region, 100.0, [0, 0, 0, 0]);

        assert!(data.is_empty());
        assert_eq!(data.triangle_count(), 0);
        assert!(data.get_triangle(0, 100.0).is_none());
    }

    #[test]
    fn test_solid_region() {
        // Create a solid cube
        let cube = Cube::Solid(1);
        let region = RegionId::new(glam::IVec3::ZERO, 1);

        let data = RegionCollisionData::from_octree(&cube, region, 100.0, [0, 0, 0, 0]);

        // A solid cube at depth 1 should have some exposed faces
        // (faces at boundaries of the region)
        assert!(!data.is_empty());
        assert!(data.triangle_count() > 0);
    }

    #[test]
    fn test_triangle_retrieval() {
        // Create a simple subdivided cube with one solid octant
        let cube = Cube::tabulate(|i| {
            if i == 0 {
                Cube::Solid(1) // Only octant 0 is solid
            } else {
                Cube::Solid(0)
            }
        });

        // Query the region containing octant 0
        let region = RegionId::new(glam::IVec3::ZERO, 1);
        let data = RegionCollisionData::from_octree(&cube, region, 100.0, [0, 0, 0, 0]);

        if !data.is_empty() {
            // Should be able to retrieve triangles
            for idx in 0..data.triangle_count() as u16 {
                let tri = data.get_triangle(idx, 100.0);
                assert!(tri.is_some(), "Triangle {} should exist", idx);

                let aabb = data.get_triangle_aabb(idx, 100.0);
                assert!(aabb.is_some(), "AABB {} should exist", idx);
            }

            // Beyond triangle count should return None
            let out_of_bounds = data.get_triangle(data.triangle_count() as u16 + 1, 100.0);
            assert!(out_of_bounds.is_none());
        }
    }

    #[test]
    fn test_region_aabb() {
        let cube = Cube::Solid(1);
        let region = RegionId::new(glam::IVec3::ZERO, 2);

        let data = RegionCollisionData::from_octree(&cube, region, 100.0, [0, 0, 0, 0]);

        // Region (0,0,0) at depth 2 should cover [0, 0.25) in local space
        // In world space with size 100: [-50, -25) on each axis
        let aabb = data.aabb;
        assert!((aabb.mins.x - (-50.0)).abs() < 1e-6);
        assert!((aabb.mins.y - (-50.0)).abs() < 1e-6);
        assert!((aabb.mins.z - (-50.0)).abs() < 1e-6);
        assert!((aabb.maxs.x - (-25.0)).abs() < 1e-6);
        assert!((aabb.maxs.y - (-25.0)).abs() < 1e-6);
        assert!((aabb.maxs.z - (-25.0)).abs() < 1e-6);
    }

    #[test]
    fn test_version_tracking() {
        let cube = Cube::Solid(1);
        let region = RegionId::new(glam::IVec3::ZERO, 1);

        let mut data = RegionCollisionData::from_octree(&cube, region, 100.0, [0, 0, 0, 0]);

        assert_eq!(data.version, 0);
        data.set_version(42);
        assert_eq!(data.version, 42);
    }
}
