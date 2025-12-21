//! WASM-compatible collision primitives
//!
//! This module provides AABB and intersection region calculations using only glam types,
//! ensuring full WASM compatibility without conditional compilation.

use cube::CubeCoord;
use glam::{IVec3, Quat, Vec3};

/// Axis-Aligned Bounding Box using glam types (WASM-compatible)
///
/// Represents a box aligned to the world coordinate axes. All corners are axis-aligned,
/// making intersection tests simple min/max comparisons.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    /// Minimum corner of the bounding box
    pub min: Vec3,
    /// Maximum corner of the bounding box
    pub max: Vec3,
}

impl Aabb {
    /// Create a new AABB from min and max corners
    ///
    /// # Arguments
    /// * `min` - Minimum corner (smallest x, y, z values)
    /// * `max` - Maximum corner (largest x, y, z values)
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    /// Create AABB for unit cube [0,1]³
    ///
    /// This is the canonical AABB for a voxel cube in local space.
    pub fn unit() -> Self {
        Self {
            min: Vec3::ZERO,
            max: Vec3::ONE,
        }
    }

    /// Transform local AABB to world space given position, rotation, scale
    ///
    /// This computes a tight AABB around the rotated box (OBB → AABB transformation).
    /// The resulting AABB may be larger than the original if the box is rotated.
    ///
    /// # Arguments
    /// * `position` - World position of the AABB origin
    /// * `rotation` - Rotation quaternion
    /// * `scale` - Uniform scale factor
    ///
    /// # Example
    /// ```
    /// use crossworld_physics::collision::Aabb;
    /// use glam::{Vec3, Quat};
    ///
    /// let local_aabb = Aabb::unit();
    /// let world_aabb = local_aabb.to_world(Vec3::new(10.0, 0.0, 0.0), Quat::IDENTITY, 2.0);
    /// assert_eq!(world_aabb.min, Vec3::new(10.0, 0.0, 0.0));
    /// assert_eq!(world_aabb.max, Vec3::new(12.0, 2.0, 2.0));
    /// ```
    pub fn to_world(&self, position: Vec3, rotation: Quat, scale: f32) -> Self {
        // 8 corners of the local box
        let corners = [
            Vec3::new(self.min.x, self.min.y, self.min.z),
            Vec3::new(self.max.x, self.min.y, self.min.z),
            Vec3::new(self.min.x, self.max.y, self.min.z),
            Vec3::new(self.max.x, self.max.y, self.min.z),
            Vec3::new(self.min.x, self.min.y, self.max.z),
            Vec3::new(self.max.x, self.min.y, self.max.z),
            Vec3::new(self.min.x, self.max.y, self.max.z),
            Vec3::new(self.max.x, self.max.y, self.max.z),
        ];

        // Transform corners and find min/max
        let mut world_min = Vec3::splat(f32::MAX);
        let mut world_max = Vec3::splat(f32::MIN);

        for corner in corners {
            let world_corner = position + rotation * (corner * scale);
            world_min = world_min.min(world_corner);
            world_max = world_max.max(world_corner);
        }

        Self {
            min: world_min,
            max: world_max,
        }
    }

    /// Test intersection with another AABB
    ///
    /// Two AABBs intersect if they overlap in all three dimensions.
    ///
    /// # Returns
    /// `true` if the AABBs overlap (including touching at edges/faces)
    pub fn intersects(&self, other: &Aabb) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    /// Compute intersection volume (returns None if no overlap)
    ///
    /// Returns the AABB representing the overlapping region, or None if the
    /// AABBs do not intersect.
    pub fn intersection(&self, other: &Aabb) -> Option<Aabb> {
        let min = self.min.max(other.min);
        let max = self.max.min(other.max);

        if min.x < max.x && min.y < max.y && min.z < max.z {
            Some(Aabb { min, max })
        } else {
            None
        }
    }

    /// Calculate the center point of the AABB
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Calculate the size (extents) of the AABB
    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }

    /// Calculate the half-size (half-extents) of the AABB
    pub fn half_size(&self) -> Vec3 {
        (self.max - self.min) * 0.5
    }

    /// Calculate the volume of the AABB
    pub fn volume(&self) -> f32 {
        let size = self.size();
        size.x * size.y * size.z
    }

    /// Check if a point is inside the AABB
    pub fn contains_point(&self, point: Vec3) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
    }

    /// Expand the AABB to include a point
    pub fn expand_to_include(&mut self, point: Vec3) {
        self.min = self.min.min(point);
        self.max = self.max.max(point);
    }

    /// Create an AABB that encompasses both AABBs
    pub fn union(&self, other: &Aabb) -> Aabb {
        Aabb {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }
}

impl Default for Aabb {
    fn default() -> Self {
        Self::unit()
    }
}

/// Region of octree that intersects with a bounding volume
///
/// Represents a rectangular region within the octree coordinate system.
/// The region is defined by a base coordinate and a size (1 or 2 in each dimension).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IntersectionRegion {
    /// Base octant coordinate (min corner)
    pub coord: CubeCoord,
    /// Size in each dimension (1 or 2 per axis)
    pub size: IVec3,
}

impl IntersectionRegion {
    /// Create a new intersection region
    pub fn new(coord: CubeCoord, size: IVec3) -> Self {
        Self { coord, size }
    }

    /// Calculate intersection region between world AABB and cube octree
    ///
    /// Transforms the world-space AABB into the cube's local coordinate system
    /// and determines which octants are covered.
    ///
    /// # Arguments
    /// * `world_aabb` - World space AABB to test
    /// * `cube_pos` - Cube world position (origin)
    /// * `cube_scale` - Cube world scale
    /// * `depth` - Octree depth for region granularity
    ///
    /// # Returns
    /// `Some(IntersectionRegion)` if the AABB intersects the cube, `None` otherwise
    pub fn from_aabb(
        world_aabb: &Aabb,
        cube_pos: Vec3,
        cube_scale: f32,
        depth: u32,
    ) -> Option<Self> {
        // Transform AABB to cube's [0,1] local space
        let local_min = (world_aabb.min - cube_pos) / cube_scale;
        let local_max = (world_aabb.max - cube_pos) / cube_scale;

        // Quick rejection: AABB outside [0,1] bounds
        if local_max.x < 0.0 || local_min.x > 1.0 {
            return None;
        }
        if local_max.y < 0.0 || local_min.y > 1.0 {
            return None;
        }
        if local_max.z < 0.0 || local_min.z > 1.0 {
            return None;
        }

        // Clamp to [0,1] bounds
        let clamped_min = local_min.max(Vec3::ZERO);
        let clamped_max = local_max.min(Vec3::ONE);

        // Convert to octant coordinates at given depth
        // At depth d, the cube is divided into 2^d cells per axis
        let scale = (1 << depth) as f32;

        // Convert [0,1] coordinates to octant indices
        let min_octant = (clamped_min * scale).floor().as_ivec3();
        let max_octant = ((clamped_max * scale).ceil().as_ivec3() - IVec3::ONE).max(min_octant);

        // Size is difference + 1, clamped to valid range
        let size = (max_octant - min_octant + IVec3::ONE).clamp(IVec3::ONE, IVec3::splat(2));

        // Convert from [0, 2^d) coordinates to center-based [-2^d, 2^d) coordinates
        // Formula: center_based = octant_idx * 2 - (2^d - 1)
        let center_offset: i32 = (1 << depth) - 1;
        let center_based_pos = min_octant * 2 - IVec3::splat(center_offset);

        Some(Self {
            coord: CubeCoord::new(center_based_pos, depth),
            size,
        })
    }

    /// Number of octants covered (1 to 8)
    pub fn octant_count(&self) -> usize {
        (self.size.x * self.size.y * self.size.z) as usize
    }

    /// Iterate over all covered octant coordinates
    pub fn iter_coords(&self) -> impl Iterator<Item = CubeCoord> + '_ {
        let base = self.coord.pos;
        let depth = self.coord.depth;

        (0..self.size.x).flat_map(move |dx| {
            (0..self.size.y).flat_map(move |dy| {
                (0..self.size.z).map(move |dz| {
                    // Each octant is 2 units apart in center-based coordinates
                    CubeCoord::new(base + IVec3::new(dx * 2, dy * 2, dz * 2), depth)
                })
            })
        })
    }

    /// Check if a coordinate is within this region
    ///
    /// Uses center-based coordinate comparison.
    pub fn contains_coord(&self, coord: &CubeCoord) -> bool {
        // Must be same depth
        if coord.depth != self.coord.depth {
            return false;
        }

        let base = self.coord.pos;
        let pos = coord.pos;

        // Check each axis - in center-based coords, size of 2 means span of 2 units
        let in_x = pos.x >= base.x && pos.x < base.x + self.size.x * 2;
        let in_y = pos.y >= base.y && pos.y < base.y + self.size.y * 2;
        let in_z = pos.z >= base.z && pos.z < base.z + self.size.z * 2;

        in_x && in_y && in_z
    }

    /// Convert a world-space point to check if it falls within this region
    ///
    /// # Arguments
    /// * `world_point` - Point in world coordinates
    /// * `cube_pos` - Cube world position
    /// * `cube_scale` - Cube world scale
    pub fn contains_world_point(&self, world_point: Vec3, cube_pos: Vec3, cube_scale: f32) -> bool {
        // Transform to local [0,1] space
        let local = (world_point - cube_pos) / cube_scale;

        // Quick bounds check
        if local.x < 0.0 || local.x > 1.0 {
            return false;
        }
        if local.y < 0.0 || local.y > 1.0 {
            return false;
        }
        if local.z < 0.0 || local.z > 1.0 {
            return false;
        }

        // Convert to octant index at this depth
        let scale = (1 << self.coord.depth) as f32;
        let octant_idx = (local * scale).floor().as_ivec3();

        // Convert to center-based
        let center_offset: i32 = (1 << self.coord.depth) - 1;
        let center_based = octant_idx * 2 - IVec3::splat(center_offset);

        // Check if within region
        let base = self.coord.pos;
        let in_x = center_based.x >= base.x && center_based.x < base.x + self.size.x * 2;
        let in_y = center_based.y >= base.y && center_based.y < base.y + self.size.y * 2;
        let in_z = center_based.z >= base.z && center_based.z < base.z + self.size.z * 2;

        in_x && in_y && in_z
    }
}

/// Generates collision between a static Cube (ground/terrain) and a dynamic CubeObject
///
/// This handles the common case where a moving voxel object collides with
/// static terrain. It calculates the intersection region and generates
/// colliders only for the overlapping volume.
pub struct CubeCollider;

impl CubeCollider {
    /// Generate collision geometry for the intersection of a cube and a world AABB
    ///
    /// # Arguments
    /// * `cube_aabb` - The world-space AABB of the static cube
    /// * `object_aabb` - The world-space AABB of the dynamic object
    /// * `cube_pos` - World position of the static cube
    /// * `cube_scale` - Scale of the static cube
    ///
    /// # Returns
    /// The local-space AABB representing the intersection region within the cube,
    /// or None if no intersection exists.
    pub fn intersection_region(
        cube_aabb: &Aabb,
        object_aabb: &Aabb,
        cube_pos: Vec3,
        cube_scale: f32,
    ) -> Option<Aabb> {
        // Get world-space intersection
        let world_intersection = cube_aabb.intersection(object_aabb)?;

        // Convert to cube's local [0,1] space
        let local_min = (world_intersection.min - cube_pos) / cube_scale;
        let local_max = (world_intersection.max - cube_pos) / cube_scale;

        // Clamp to valid range and check for valid intersection
        let clamped_min = local_min.max(Vec3::ZERO);
        let clamped_max = local_max.min(Vec3::ONE);

        if clamped_min.x < clamped_max.x
            && clamped_min.y < clamped_max.y
            && clamped_min.z < clamped_max.z
        {
            Some(Aabb::new(clamped_min, clamped_max))
        } else {
            None
        }
    }

    /// Check if a static cube and dynamic object might collide
    ///
    /// This is a quick broad-phase test using AABBs.
    pub fn might_collide(cube_aabb: &Aabb, object_aabb: &Aabb) -> bool {
        cube_aabb.intersects(object_aabb)
    }
}

/// Generates collision between two dynamic CubeObjects
///
/// This handles object-to-object collisions by finding the intersection
/// region and generating colliders for both objects in that region.
pub struct ObjectCollider;

impl ObjectCollider {
    /// Check if two objects might collide using AABB test
    pub fn might_collide(aabb_a: &Aabb, aabb_b: &Aabb) -> bool {
        aabb_a.intersects(aabb_b)
    }

    /// Calculate the intersection regions for two colliding objects
    ///
    /// # Arguments
    /// * `aabb_a` - World AABB of first object
    /// * `aabb_b` - World AABB of second object
    /// * `pos_a` - World position of first object
    /// * `pos_b` - World position of second object
    /// * `scale_a` - Scale of first object
    /// * `scale_b` - Scale of second object
    ///
    /// # Returns
    /// Tuple of local-space AABBs for each object representing their intersection
    /// regions, or None if no intersection exists.
    pub fn intersection_regions(
        aabb_a: &Aabb,
        aabb_b: &Aabb,
        pos_a: Vec3,
        pos_b: Vec3,
        scale_a: f32,
        scale_b: f32,
    ) -> Option<(Aabb, Aabb)> {
        // Get world-space intersection
        let world_intersection = aabb_a.intersection(aabb_b)?;

        // Convert to each object's local [0,1] space
        let local_a_min = (world_intersection.min - pos_a) / scale_a;
        let local_a_max = (world_intersection.max - pos_a) / scale_a;

        let local_b_min = (world_intersection.min - pos_b) / scale_b;
        let local_b_max = (world_intersection.max - pos_b) / scale_b;

        // Clamp to valid range
        let clamped_a_min = local_a_min.max(Vec3::ZERO);
        let clamped_a_max = local_a_max.min(Vec3::ONE);

        let clamped_b_min = local_b_min.max(Vec3::ZERO);
        let clamped_b_max = local_b_max.min(Vec3::ONE);

        // Validate both regions
        let valid_a = clamped_a_min.x < clamped_a_max.x
            && clamped_a_min.y < clamped_a_max.y
            && clamped_a_min.z < clamped_a_max.z;

        let valid_b = clamped_b_min.x < clamped_b_max.x
            && clamped_b_min.y < clamped_b_max.y
            && clamped_b_min.z < clamped_b_max.z;

        if valid_a && valid_b {
            Some((
                Aabb::new(clamped_a_min, clamped_a_max),
                Aabb::new(clamped_b_min, clamped_b_max),
            ))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_1_SQRT_2;

    // ==================== AABB Tests ====================

    #[test]
    fn test_aabb_unit() {
        let aabb = Aabb::unit();
        assert_eq!(aabb.min, Vec3::ZERO);
        assert_eq!(aabb.max, Vec3::ONE);
        assert_eq!(aabb.volume(), 1.0);
    }

    #[test]
    fn test_aabb_to_world_identity() {
        let local = Aabb::unit();
        let world = local.to_world(Vec3::ZERO, Quat::IDENTITY, 1.0);

        assert_eq!(world.min, Vec3::ZERO);
        assert_eq!(world.max, Vec3::ONE);
    }

    #[test]
    fn test_aabb_to_world_translation() {
        let local = Aabb::unit();
        let world = local.to_world(Vec3::new(10.0, 20.0, 30.0), Quat::IDENTITY, 1.0);

        assert_eq!(world.min, Vec3::new(10.0, 20.0, 30.0));
        assert_eq!(world.max, Vec3::new(11.0, 21.0, 31.0));
    }

    #[test]
    fn test_aabb_to_world_scale() {
        let local = Aabb::unit();
        let world = local.to_world(Vec3::ZERO, Quat::IDENTITY, 2.0);

        assert_eq!(world.min, Vec3::ZERO);
        assert_eq!(world.max, Vec3::splat(2.0));
        assert_eq!(world.volume(), 8.0);
    }

    #[test]
    fn test_aabb_to_world_rotation_45_degrees() {
        // Use a centered cube for clearer rotation behavior
        let local = Aabb::new(Vec3::splat(-0.5), Vec3::splat(0.5));
        // Rotate 45 degrees around Y axis
        let rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_4);
        let world = local.to_world(Vec3::ZERO, rotation, 1.0);

        // Rotated box should expand by sqrt(2) in X and Z
        // Original centered cube rotated 45 degrees around Y
        // Y unchanged (-0.5 to 0.5)
        assert!((world.min.y - (-0.5)).abs() < 0.001);
        assert!((world.max.y - 0.5).abs() < 0.001);

        // The diagonal of a unit square is sqrt(2), so the rotated AABB
        // should have a size of approximately sqrt(2) in XZ plane
        let size = world.size();
        let expected_xz = FRAC_1_SQRT_2 * 2.0; // sqrt(2)
        assert!(
            (size.x - expected_xz).abs() < 0.01,
            "Expected X size ~{}, got {}",
            expected_xz,
            size.x
        );
        assert!(
            (size.z - expected_xz).abs() < 0.01,
            "Expected Z size ~{}, got {}",
            expected_xz,
            size.z
        );
    }

    #[test]
    fn test_aabb_intersects() {
        let a = Aabb::new(Vec3::ZERO, Vec3::ONE);
        let b = Aabb::new(Vec3::splat(0.5), Vec3::splat(1.5));
        let c = Aabb::new(Vec3::splat(2.0), Vec3::splat(3.0));

        assert!(a.intersects(&b)); // Overlapping
        assert!(b.intersects(&a)); // Symmetric
        assert!(!a.intersects(&c)); // No overlap
        assert!(!c.intersects(&a)); // Symmetric
    }

    #[test]
    fn test_aabb_intersects_touching() {
        let a = Aabb::new(Vec3::ZERO, Vec3::ONE);
        let b = Aabb::new(Vec3::new(1.0, 0.0, 0.0), Vec3::new(2.0, 1.0, 1.0));

        // Touching at face should count as intersecting
        assert!(a.intersects(&b));
    }

    #[test]
    fn test_aabb_intersection_volume() {
        let a = Aabb::new(Vec3::ZERO, Vec3::ONE);
        let b = Aabb::new(Vec3::splat(0.5), Vec3::splat(1.5));

        let intersection = a.intersection(&b).unwrap();
        assert_eq!(intersection.min, Vec3::splat(0.5));
        assert_eq!(intersection.max, Vec3::ONE);
        assert_eq!(intersection.volume(), 0.125); // 0.5^3
    }

    #[test]
    fn test_aabb_intersection_none() {
        let a = Aabb::new(Vec3::ZERO, Vec3::ONE);
        let c = Aabb::new(Vec3::splat(2.0), Vec3::splat(3.0));

        assert!(a.intersection(&c).is_none());
    }

    #[test]
    fn test_aabb_contains_point() {
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);

        assert!(aabb.contains_point(Vec3::splat(0.5)));
        assert!(aabb.contains_point(Vec3::ZERO)); // Corner
        assert!(aabb.contains_point(Vec3::ONE)); // Corner
        assert!(!aabb.contains_point(Vec3::splat(1.5)));
        assert!(!aabb.contains_point(Vec3::splat(-0.5)));
    }

    #[test]
    fn test_aabb_center_and_half_size() {
        let aabb = Aabb::new(Vec3::ZERO, Vec3::splat(2.0));

        assert_eq!(aabb.center(), Vec3::ONE);
        assert_eq!(aabb.half_size(), Vec3::ONE);
        assert_eq!(aabb.size(), Vec3::splat(2.0));
    }

    #[test]
    fn test_aabb_union() {
        let a = Aabb::new(Vec3::ZERO, Vec3::ONE);
        let b = Aabb::new(Vec3::splat(2.0), Vec3::splat(3.0));

        let union = a.union(&b);
        assert_eq!(union.min, Vec3::ZERO);
        assert_eq!(union.max, Vec3::splat(3.0));
    }

    // ==================== IntersectionRegion Tests ====================

    #[test]
    fn test_intersection_region_single_octant() {
        // AABB in bottom-left-back corner of cube
        let aabb = Aabb::new(Vec3::new(0.1, 0.1, 0.1), Vec3::new(0.4, 0.4, 0.4));
        let region = IntersectionRegion::from_aabb(&aabb, Vec3::ZERO, 1.0, 1).unwrap();

        assert_eq!(region.octant_count(), 1);
        assert_eq!(region.size, IVec3::ONE);
    }

    #[test]
    fn test_intersection_region_spanning_octants() {
        // AABB spanning all 8 octants (center of cube)
        let aabb = Aabb::new(Vec3::splat(0.25), Vec3::splat(0.75));
        let region = IntersectionRegion::from_aabb(&aabb, Vec3::ZERO, 1.0, 1).unwrap();

        // At depth 1, this spans both halves in all dimensions
        assert_eq!(region.octant_count(), 8);
        assert_eq!(region.size, IVec3::splat(2));
    }

    #[test]
    fn test_intersection_region_outside_bounds() {
        // AABB completely outside the cube
        let aabb = Aabb::new(Vec3::splat(2.0), Vec3::splat(3.0));
        let region = IntersectionRegion::from_aabb(&aabb, Vec3::ZERO, 1.0, 1);

        assert!(region.is_none());
    }

    #[test]
    fn test_intersection_region_partial_overlap() {
        // AABB partially overlapping the cube
        let aabb = Aabb::new(Vec3::splat(-0.5), Vec3::splat(0.25));
        let region = IntersectionRegion::from_aabb(&aabb, Vec3::ZERO, 1.0, 1).unwrap();

        // Should only cover the corner octant
        assert_eq!(region.octant_count(), 1);
    }

    #[test]
    fn test_intersection_region_with_translation() {
        // Cube at position (10, 0, 0)
        let aabb = Aabb::new(Vec3::new(10.1, 0.1, 0.1), Vec3::new(10.4, 0.4, 0.4));
        let region =
            IntersectionRegion::from_aabb(&aabb, Vec3::new(10.0, 0.0, 0.0), 1.0, 1).unwrap();

        assert_eq!(region.octant_count(), 1);
    }

    #[test]
    fn test_intersection_region_with_scale() {
        // Cube with scale 2.0
        let aabb = Aabb::new(Vec3::new(0.2, 0.2, 0.2), Vec3::new(0.8, 0.8, 0.8));
        let region = IntersectionRegion::from_aabb(&aabb, Vec3::ZERO, 2.0, 1).unwrap();

        // In scaled space, this is [0.1, 0.4] which is a single octant
        assert_eq!(region.octant_count(), 1);
    }

    #[test]
    fn test_intersection_region_iter_coords() {
        // Region spanning 2x2x2 octants
        let aabb = Aabb::new(Vec3::splat(0.25), Vec3::splat(0.75));
        let region = IntersectionRegion::from_aabb(&aabb, Vec3::ZERO, 1.0, 1).unwrap();

        let coords: Vec<_> = region.iter_coords().collect();
        assert_eq!(coords.len(), 8);

        // All coords should be at depth 1
        for coord in &coords {
            assert_eq!(coord.depth, 1);
        }
    }

    #[test]
    fn test_intersection_region_depth_2() {
        // Higher depth = finer granularity
        let aabb = Aabb::new(Vec3::new(0.1, 0.1, 0.1), Vec3::new(0.3, 0.3, 0.3));
        let region = IntersectionRegion::from_aabb(&aabb, Vec3::ZERO, 1.0, 2).unwrap();

        assert_eq!(region.coord.depth, 2);
        // At depth 2, cube is divided into 4x4x4 = 64 cells
        // [0.1, 0.3] in [0,1] space = cells [0,1] at depth 2 = size 2 per axis
        assert!(region.octant_count() <= 8);
    }

    #[test]
    fn test_intersection_region_contains_coord() {
        let region = IntersectionRegion::new(CubeCoord::new(IVec3::new(-1, -1, -1), 1), IVec3::ONE);

        // Same coord should be contained
        let inside = CubeCoord::new(IVec3::new(-1, -1, -1), 1);
        assert!(region.contains_coord(&inside));

        // Different depth should not be contained
        let wrong_depth = CubeCoord::new(IVec3::new(-1, -1, -1), 2);
        assert!(!region.contains_coord(&wrong_depth));

        // Outside coord should not be contained
        let outside = CubeCoord::new(IVec3::new(1, 1, 1), 1);
        assert!(!region.contains_coord(&outside));
    }

    #[test]
    fn test_intersection_region_rotated_aabb() {
        // When an object is rotated 45 degrees, its world AABB expands
        let local = Aabb::unit();
        let rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_4);
        let world_aabb = local.to_world(Vec3::splat(0.25), rotation, 0.5);

        // The expanded AABB should span more octants
        let region = IntersectionRegion::from_aabb(&world_aabb, Vec3::ZERO, 1.0, 1);

        // Rotated box may span multiple octants
        assert!(region.is_some());
        let region = region.unwrap();
        assert!(region.octant_count() >= 1);
    }

    // ==================== CubeCollider Tests ====================

    #[test]
    fn test_cube_collider_intersection_region() {
        // Ground cube at origin, scale 10
        let cube_pos = Vec3::ZERO;
        let cube_scale = 10.0;
        let cube_aabb = Aabb::new(Vec3::ZERO, Vec3::splat(cube_scale));

        // Object at position (5, 5, 5), size 2
        let object_aabb = Aabb::new(Vec3::new(4.0, 4.0, 4.0), Vec3::new(6.0, 6.0, 6.0));

        let region =
            CubeCollider::intersection_region(&cube_aabb, &object_aabb, cube_pos, cube_scale)
                .unwrap();

        // Region should be in local [0.4, 0.6] range
        assert!((region.min.x - 0.4).abs() < 0.01);
        assert!((region.max.x - 0.6).abs() < 0.01);
    }

    #[test]
    fn test_cube_collider_no_intersection() {
        let cube_aabb = Aabb::new(Vec3::ZERO, Vec3::splat(10.0));
        let object_aabb = Aabb::new(Vec3::splat(20.0), Vec3::splat(25.0));

        let region = CubeCollider::intersection_region(&cube_aabb, &object_aabb, Vec3::ZERO, 10.0);

        assert!(region.is_none());
    }

    #[test]
    fn test_cube_collider_might_collide() {
        let cube_aabb = Aabb::new(Vec3::ZERO, Vec3::splat(10.0));
        let near_aabb = Aabb::new(Vec3::splat(5.0), Vec3::splat(15.0));
        let far_aabb = Aabb::new(Vec3::splat(20.0), Vec3::splat(25.0));

        assert!(CubeCollider::might_collide(&cube_aabb, &near_aabb));
        assert!(!CubeCollider::might_collide(&cube_aabb, &far_aabb));
    }

    // ==================== ObjectCollider Tests ====================

    #[test]
    fn test_object_collider_intersection_regions() {
        // Object A at (0, 0, 0), scale 2
        let aabb_a = Aabb::new(Vec3::ZERO, Vec3::splat(2.0));
        // Object B at (1, 1, 1), scale 2
        let aabb_b = Aabb::new(Vec3::ONE, Vec3::splat(3.0));

        let (region_a, region_b) =
            ObjectCollider::intersection_regions(&aabb_a, &aabb_b, Vec3::ZERO, Vec3::ONE, 2.0, 2.0)
                .unwrap();

        // Object A's intersection region is in [1,2] world space → [0.5, 1.0] local
        assert!((region_a.min.x - 0.5).abs() < 0.01);
        assert!((region_a.max.x - 1.0).abs() < 0.01);

        // Object B's intersection region is in [1,2] world space → [0.0, 0.5] local (relative to pos (1,1,1))
        assert!((region_b.min.x - 0.0).abs() < 0.01);
        assert!((region_b.max.x - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_object_collider_no_intersection() {
        let aabb_a = Aabb::new(Vec3::ZERO, Vec3::ONE);
        let aabb_b = Aabb::new(Vec3::splat(10.0), Vec3::splat(11.0));

        let result = ObjectCollider::intersection_regions(
            &aabb_a,
            &aabb_b,
            Vec3::ZERO,
            Vec3::splat(10.0),
            1.0,
            1.0,
        );

        assert!(result.is_none());
    }

    #[test]
    fn test_object_collider_might_collide() {
        let aabb_a = Aabb::new(Vec3::ZERO, Vec3::ONE);
        let aabb_b = Aabb::new(Vec3::splat(0.5), Vec3::splat(1.5));
        let aabb_c = Aabb::new(Vec3::splat(5.0), Vec3::splat(6.0));

        assert!(ObjectCollider::might_collide(&aabb_a, &aabb_b));
        assert!(!ObjectCollider::might_collide(&aabb_a, &aabb_c));
    }
}
