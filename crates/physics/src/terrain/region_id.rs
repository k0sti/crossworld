//! Region and triangle identification for terrain collision
//!
//! This module provides compact encodings for identifying collision regions and
//! individual triangles within the octree coordinate system.

use cube::RegionBounds;
use glam::{IVec3, Vec3};
use rapier3d::parry::bounding_volume::Aabb;

/// Identifies a collision region in the octree
///
/// Uses corner-based coordinates at a fixed depth, compatible with
/// cube::RegionBounds for efficient octree queries.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct RegionId {
    /// Corner position at region_depth (in [0, 2^depth) per axis)
    pub pos: IVec3,
    /// Depth level (determines region granularity)
    pub depth: u32,
}

impl RegionId {
    /// Create a new region ID
    pub fn new(pos: IVec3, depth: u32) -> Self {
        Self { pos, depth }
    }

    /// Convert to cube::RegionBounds for octree queries
    pub fn to_region_bounds(&self) -> RegionBounds {
        RegionBounds::new(self.pos, self.depth, IVec3::ONE)
    }

    /// Compute AABB in local [0,1] space
    pub fn local_aabb(&self) -> (Vec3, Vec3) {
        let scale = 1.0 / (1 << self.depth) as f32;
        let min = self.pos.as_vec3() * scale;
        let max = min + Vec3::splat(scale);
        (min, max)
    }

    /// Compute AABB in world space
    pub fn world_aabb(&self, world_size: f32) -> Aabb {
        let (local_min, local_max) = self.local_aabb();
        let half_world = world_size / 2.0;

        // Convert from [0,1] to centered world coordinates
        let world_min = local_min * world_size - Vec3::splat(half_world);
        let world_max = local_max * world_size - Vec3::splat(half_world);

        Aabb::new(world_min.to_array().into(), world_max.to_array().into())
    }

    /// Create from world AABB (returns all regions that intersect)
    pub fn from_world_aabb(
        world_aabb: &Aabb,
        world_size: f32,
        depth: u32,
    ) -> Vec<RegionId> {
        let half_world = world_size / 2.0;

        // Convert world AABB to local [0,1] space
        let local_min =
            (Vec3::new(world_aabb.mins.x, world_aabb.mins.y, world_aabb.mins.z)
                + Vec3::splat(half_world))
                / world_size;
        let local_max =
            (Vec3::new(world_aabb.maxs.x, world_aabb.maxs.y, world_aabb.maxs.z)
                + Vec3::splat(half_world))
                / world_size;

        // Convert to region coordinates at given depth
        let scale = (1 << depth) as f32;
        let min_pos = (local_min * scale).floor().as_ivec3().max(IVec3::ZERO);
        let max_pos = (local_max * scale)
            .ceil()
            .as_ivec3()
            .min(IVec3::splat((1 << depth) - 1));

        // Handle case where AABB is outside [0,1] bounds
        if min_pos.x > max_pos.x || min_pos.y > max_pos.y || min_pos.z > max_pos.z {
            return Vec::new();
        }

        let mut result = Vec::new();
        for x in min_pos.x..=max_pos.x {
            for y in min_pos.y..=max_pos.y {
                for z in min_pos.z..=max_pos.z {
                    result.push(RegionId::new(IVec3::new(x, y, z), depth));
                }
            }
        }
        result
    }

    /// Number of regions at a given depth
    pub fn count_at_depth(depth: u32) -> usize {
        1 << (depth * 3) // 2^(3*depth) = 8^depth
    }
}

/// Identifies a specific triangle within the terrain
///
/// Encodes region coordinates and face/triangle index compactly.
/// Each face produces 2 triangles, so triangle_idx encodes:
/// - face_index * 2 + triangle_within_face (0 or 1)
///
/// # Bit Layout
/// Bits: [depth:4][x:16][y:16][z:16][tri_idx:12]
/// - depth: 4 bits (0-15)
/// - x, y, z: 16 bits each (0-65535)
/// - tri_idx: 12 bits (0-4095, supports 2048 faces per region)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TerrainPartId(u64);

impl TerrainPartId {
    const DEPTH_SHIFT: u32 = 60;
    const X_SHIFT: u32 = 44;
    const Y_SHIFT: u32 = 28;
    const Z_SHIFT: u32 = 12;
    const TRI_MASK: u64 = 0xFFF; // 12 bits = 4096 triangles per region

    /// Create a new terrain part ID
    ///
    /// # Panics
    /// Debug-panics if depth > 15 or triangle_idx >= 4096
    pub fn new(region: RegionId, triangle_idx: u16) -> Self {
        debug_assert!(region.depth <= 15, "depth must fit in 4 bits");
        debug_assert!(triangle_idx < 4096, "triangle_idx must fit in 12 bits");

        let packed = ((region.depth as u64) << Self::DEPTH_SHIFT)
            | ((region.pos.x as u64 & 0xFFFF) << Self::X_SHIFT)
            | ((region.pos.y as u64 & 0xFFFF) << Self::Y_SHIFT)
            | ((region.pos.z as u64 & 0xFFFF) << Self::Z_SHIFT)
            | (triangle_idx as u64);
        Self(packed)
    }

    /// Extract the region ID
    pub fn region(&self) -> RegionId {
        RegionId {
            pos: IVec3::new(
                ((self.0 >> Self::X_SHIFT) & 0xFFFF) as i32,
                ((self.0 >> Self::Y_SHIFT) & 0xFFFF) as i32,
                ((self.0 >> Self::Z_SHIFT) & 0xFFFF) as i32,
            ),
            depth: ((self.0 >> Self::DEPTH_SHIFT) & 0xF) as u32,
        }
    }

    /// Extract the triangle index within the region
    pub fn triangle_idx(&self) -> u16 {
        (self.0 & Self::TRI_MASK) as u16
    }

    /// Get face index (each face has 2 triangles)
    pub fn face_idx(&self) -> u16 {
        self.triangle_idx() / 2
    }

    /// Get triangle within face (0 or 1)
    pub fn triangle_in_face(&self) -> u8 {
        (self.triangle_idx() % 2) as u8
    }

    /// Get the raw u64 value
    pub fn raw(&self) -> u64 {
        self.0
    }
}

impl From<u64> for TerrainPartId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<TerrainPartId> for u64 {
    fn from(value: TerrainPartId) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_id_to_region_bounds() {
        let region = RegionId::new(IVec3::new(1, 2, 3), 3);
        let bounds = region.to_region_bounds();

        assert_eq!(bounds.pos, IVec3::new(1, 2, 3));
        assert_eq!(bounds.depth, 3);
        assert_eq!(bounds.size, IVec3::ONE);
    }

    #[test]
    fn test_region_id_local_aabb() {
        // At depth 2, each region is 0.25 units
        let region = RegionId::new(IVec3::new(1, 2, 3), 2);
        let (min, max) = region.local_aabb();

        assert!((min.x - 0.25).abs() < 1e-6);
        assert!((min.y - 0.5).abs() < 1e-6);
        assert!((min.z - 0.75).abs() < 1e-6);
        assert!((max.x - 0.5).abs() < 1e-6);
        assert!((max.y - 0.75).abs() < 1e-6);
        assert!((max.z - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_region_id_world_aabb() {
        let region = RegionId::new(IVec3::ZERO, 1);
        let aabb = region.world_aabb(100.0);

        // At depth 1, region 0,0,0 covers [0, 0.5) in local space
        // In world space with size 100: [-50, 0)
        assert!((aabb.mins.x - (-50.0)).abs() < 1e-6);
        assert!((aabb.maxs.x - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_region_id_from_world_aabb() {
        let world_size = 100.0;
        let depth = 2;

        // AABB covering center of world
        let aabb = Aabb::new([-10.0, -10.0, -10.0].into(), [10.0, 10.0, 10.0].into());

        let regions = RegionId::from_world_aabb(&aabb, world_size, depth);

        // Should cover some regions around the center
        assert!(!regions.is_empty());

        // All regions should be at the correct depth
        for region in &regions {
            assert_eq!(region.depth, depth);
        }
    }

    #[test]
    fn test_region_id_from_world_aabb_outside() {
        let world_size = 100.0;
        let depth = 2;

        // AABB completely outside world bounds
        let aabb = Aabb::new([100.0, 100.0, 100.0].into(), [200.0, 200.0, 200.0].into());

        let regions = RegionId::from_world_aabb(&aabb, world_size, depth);
        assert!(regions.is_empty());
    }

    #[test]
    fn test_terrain_part_id_roundtrip() {
        let region = RegionId::new(IVec3::new(7, 5, 3), 4);
        let tri_idx = 42;

        let part_id = TerrainPartId::new(region, tri_idx);

        assert_eq!(part_id.region(), region);
        assert_eq!(part_id.triangle_idx(), tri_idx);
    }

    #[test]
    fn test_terrain_part_id_face_helpers() {
        let region = RegionId::new(IVec3::ZERO, 1);

        // Triangle 0 and 1 are from face 0
        let part0 = TerrainPartId::new(region, 0);
        let part1 = TerrainPartId::new(region, 1);

        assert_eq!(part0.face_idx(), 0);
        assert_eq!(part0.triangle_in_face(), 0);
        assert_eq!(part1.face_idx(), 0);
        assert_eq!(part1.triangle_in_face(), 1);

        // Triangle 2 and 3 are from face 1
        let part2 = TerrainPartId::new(region, 2);
        let part3 = TerrainPartId::new(region, 3);

        assert_eq!(part2.face_idx(), 1);
        assert_eq!(part2.triangle_in_face(), 0);
        assert_eq!(part3.face_idx(), 1);
        assert_eq!(part3.triangle_in_face(), 1);
    }

    #[test]
    fn test_terrain_part_id_max_values() {
        // Test with maximum values
        let region = RegionId::new(IVec3::new(0xFFFF, 0xFFFF, 0xFFFF), 15);
        let tri_idx = 4095;

        let part_id = TerrainPartId::new(region, tri_idx);

        assert_eq!(part_id.region().depth, 15);
        assert_eq!(part_id.region().pos.x, 0xFFFF);
        assert_eq!(part_id.region().pos.y, 0xFFFF);
        assert_eq!(part_id.region().pos.z, 0xFFFF);
        assert_eq!(part_id.triangle_idx(), 4095);
    }

    #[test]
    fn test_region_count_at_depth() {
        assert_eq!(RegionId::count_at_depth(0), 1);
        assert_eq!(RegionId::count_at_depth(1), 8);
        assert_eq!(RegionId::count_at_depth(2), 64);
        assert_eq!(RegionId::count_at_depth(3), 512);
    }
}
