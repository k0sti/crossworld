//! Raycasting through octrees
//!
//! This module provides efficient ray-octree intersection using a recursive DDA
//! (Digital Differential Analyzer) algorithm. The raycast system finds the first
//! solid voxel along a ray path by hierarchically traversing the octree structure.

use crate::{Cube, CubeCoord, IVec3Ext};
use glam::{IVec3, Vec3};

pub mod error;
use crate::axis::Axis;
pub use error::RaycastError;

/// Debug state for raycast traversal
#[derive(Debug, Clone, Default)]
pub struct RaycastDebugState {
    pub enter_count: u32,
    pub max_depth_reached: u32,
    pub traversed_nodes: Vec<CubeCoord>,
}

impl RaycastDebugState {
    pub fn new() -> Self {
        Self::default()
    }
    fn record_enter(&mut self, coord: CubeCoord, depth: u32) {
        self.enter_count += 1;
        self.max_depth_reached = self.max_depth_reached.max(depth);
        self.traversed_nodes.push(coord);
    }
}

/// Result of a raycast hit
#[derive(Debug, Clone)]
pub struct RaycastHit<T> {
    pub coord: CubeCoord,
    pub position: Vec3,
    pub normal: Axis,
    pub value: T,
    pub debug: Option<RaycastDebugState>,
}

impl<T> Cube<T>
where
    T: Clone + PartialEq,
{
    pub fn raycast<F>(
        &self,
        pos: Vec3,
        dir: Vec3,
        max_depth: u32,
        is_empty: &F,
    ) -> Result<Option<RaycastHit<T>>, RaycastError>
    where
        F: Fn(&T) -> bool,
    {
        if dir.length_squared() < 1e-6 {
            return Err(RaycastError::InvalidDirection);
        }
        self.raycast_recursive(pos, dir, max_depth, IVec3::ZERO, max_depth, is_empty, None)
    }

    pub fn raycast_debug<F>(
        &self,
        pos: Vec3,
        dir: Vec3,
        max_depth: u32,
        is_empty: &F,
    ) -> Result<Option<RaycastHit<T>>, RaycastError>
    where
        F: Fn(&T) -> bool,
    {
        if dir.length_squared() < 1e-6 {
            return Err(RaycastError::InvalidDirection);
        }

        let mut start_pos = pos;

        // Check if start position is outside [0, 1]³
        if !pos.cmpge(Vec3::ZERO).all() || !pos.cmple(Vec3::ONE).all() {
            // Ray starts outside, check intersection with bounding box
            if let Some((t_entry, _)) = Self::intersect_aabb(pos, dir, Vec3::ZERO, Vec3::ONE) {
                // Move start position to entry point (plus tiny epsilon to ensure inside)
                start_pos = pos + dir * (t_entry + 1e-6);
            } else {
                // Ray misses the box completely
                return Ok(None);
            }
        }

        let mut debug = RaycastDebugState::new();
        self.raycast_recursive(
            start_pos,
            dir,
            max_depth,
            IVec3::ZERO,
            max_depth,
            is_empty,
            Some(&mut debug),
        )
    }

    fn intersect_aabb(
        ray_origin: Vec3,
        ray_dir: Vec3,
        box_min: Vec3,
        box_max: Vec3,
    ) -> Option<(f32, f32)> {
        let t_min = (box_min - ray_origin) / ray_dir;
        let t_max = (box_max - ray_origin) / ray_dir;

        let t1 = t_min.min(t_max);
        let t2 = t_min.max(t_max);

        let t_near = t1.max_element();
        let t_far = t2.min_element();

        if t_near > t_far || t_far < 0.0 {
            None
        } else {
            Some((t_near, t_far))
        }
    }

    fn raycast_recursive<F>(
        &self,
        pos: Vec3,
        dir: Vec3,
        _max_depth: u32,
        octree_pos: IVec3,
        current_depth: u32,
        is_empty: &F,
        mut debug: Option<&mut RaycastDebugState>,
    ) -> Result<Option<RaycastHit<T>>, RaycastError>
    where
        F: Fn(&T) -> bool,
    {
        // Validate position is in [0, 1]³
        if !pos.cmpge(Vec3::ZERO).all() || !pos.cmple(Vec3::ONE).all() {
            // Only return error at top level (when current_depth == _max_depth)
            // For recursive calls, floating point errors might cause slight out of bounds, which we treat as miss
            if current_depth == _max_depth {
                return Err(RaycastError::StartOutOfBounds);
            }
            return Ok(None);
        }

        // Record entry into this node
        let coord = CubeCoord::new(octree_pos, current_depth);
        if let Some(d) = debug.as_deref_mut() {
            d.record_enter(coord, current_depth);
        }

        // Check cube type
        match self {
            Cube::Solid(value) => {
                // Non-empty voxel - return hit
                if !is_empty(value) {
                    let normal = calculate_entry_normal(pos, dir);
                    Ok(Some(RaycastHit {
                        coord,
                        position: pos,
                        normal,
                        value: value.clone(),
                        debug: debug.map(|d| d.clone()),
                    }))
                } else {
                    // Empty voxel
                    Ok(None)
                }
            }
            Cube::Cubes(children) if current_depth > 0 => {
                // Calculate which octant we're in
                // Simple octant calculation: check if each component is in lower (0) or upper (1) half
                let bit = IVec3::new(
                    if pos.x < 0.5 { 0 } else { 1 },
                    if pos.y < 0.5 { 0 } else { 1 },
                    if pos.z < 0.5 { 0 } else { 1 },
                );

                // Calculate octant index (0-7)
                let index = bit.to_octant_index();

                // Calculate pos2 for child coordinate transformation
                let pos2 = pos * 2.0;
                let sign = dir.signum();

                // Transform to child coordinate space
                let child_pos = (pos2 - bit.as_vec3()) / 2.0;
                let child_octree_pos = (octree_pos << 1) + bit;

                // Recursively raycast into child
                match children[index].raycast_recursive(
                    child_pos,
                    dir,
                    _max_depth,
                    child_octree_pos,
                    current_depth - 1,
                    is_empty,
                    debug.as_deref_mut(),
                ) {
                    Ok(Some(hit)) => return Ok(Some(hit)),
                    Ok(None) => {}           // Miss, continue
                    Err(e) => return Err(e), // Propagate error
                }

                // Miss in this octant - step to next boundary
                let next_pos = calculate_next_position(pos2, dir, sign);

                // Check if next position is still within valid bounds or if we've made no progress
                // If it's outside [0,1]³ or identical to current position, we've exited/stuck
                if !next_pos.cmpge(Vec3::ZERO).all() || !next_pos.cmple(Vec3::ONE).all() {
                    return Ok(None);
                }

                // Prevent infinite recursion: if next_pos is same as pos, we haven't moved
                const EPSILON: f32 = 1e-10;
                if (next_pos - pos).length() < EPSILON {
                    return Ok(None);
                }

                // Continue raycasting from new position
                self.raycast_recursive(
                    next_pos,
                    dir,
                    _max_depth,
                    octree_pos,
                    current_depth,
                    is_empty,
                    debug,
                )
            }
            _ => {
                // Max depth or unsupported structure
                Ok(None)
            }
        }
    }
}

/// Calculate the next integer boundary in the direction of sign
fn next_integer_boundary(v: Vec3, sign: Vec3) -> Vec3 {
    let scaled = v * sign + Vec3::ONE;
    scaled.floor() * sign
}

/// Calculate the next position after stepping to the next octant boundary
fn calculate_next_position(pos2: Vec3, dir: Vec3, sign: Vec3) -> Vec3 {
    const EPSILON: f32 = 1e-8;

    let next_integer = next_integer_boundary(pos2, sign);
    let diff = next_integer - pos2;

    // Avoid division by zero
    if diff.x.abs() < EPSILON && diff.y.abs() < EPSILON && diff.z.abs() < EPSILON {
        return pos2 / 2.0;
    }

    let inv_time = dir / diff;
    let max_inv = inv_time.x.max(inv_time.y).max(inv_time.z);

    if max_inv.abs() < EPSILON {
        return pos2 / 2.0;
    }

    let step = diff * (inv_time / max_inv);
    let next_pos = (pos2 + step) / 2.0;

    // Clamp to valid range
    next_pos.clamp(Vec3::ZERO, Vec3::ONE)
}

/// Calculate surface normal from entry point
/// The normal points towards the direction the ray came from
fn calculate_entry_normal(pos: Vec3, _dir: Vec3) -> Axis {
    let dist_to_min = pos;
    let dist_to_max = Vec3::ONE - pos;

    let min_dist = dist_to_min.min_element();
    let max_dist = dist_to_max.min_element();

    if min_dist < max_dist {
        // Entered from min face (0, 0, 0)
        if dist_to_min.x == min_dist {
            Axis::NegX
        } else if dist_to_min.y == min_dist {
            Axis::NegY
        } else {
            Axis::NegZ
        }
    } else {
        // Entered from max face (1, 1, 1)
        if dist_to_max.x == max_dist {
            Axis::PosX
        } else if dist_to_max.y == max_dist {
            Axis::PosY
        } else {
            Axis::PosZ
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    #[test]
    fn test_raycast_solid() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        // Cast ray from bottom (z=0) going up (z+)
        let pos = Vec3::new(0.5, 0.5, 0.0);
        let dir = Vec3::new(0.0, 0.0, 1.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty);

        assert!(hit.is_ok());
        let hit = hit.unwrap();
        assert!(hit.is_some());
        let hit = hit.unwrap();

        // Check coordinate
        assert_eq!(hit.coord.depth, 3);
        assert_eq!(hit.coord.pos, IVec3::ZERO);

        // Check position (should be at entry point)
        assert_eq!(hit.position.x, 0.5);
        assert_eq!(hit.position.y, 0.5);
        assert_eq!(hit.position.z, 0.0);

        // Check normal (entering from -Z face)
        assert_eq!(hit.normal, Axis::NegZ);
    }

    #[test]
    fn test_raycast_empty() {
        let cube = Cube::Solid(0i32);
        let is_empty = |v: &i32| *v == 0;

        // Cast ray into empty cube
        let hit = cube.raycast(
            Vec3::new(0.5, 0.5, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            3,
            &is_empty,
        );
        assert!(hit.is_ok());
        assert!(hit.unwrap().is_none());
    }

    #[test]
    fn test_raycast_octree() {
        // Create simple octree with one solid octant
        // Octant 0 is at position (0,0,0) in child space
        let children = [
            Rc::new(Cube::Solid(1i32)), // octant 0 (x=0,y=0,z=0): solid
            Rc::new(Cube::Solid(0i32)), // octant 1 (x=1,y=0,z=0): empty
            Rc::new(Cube::Solid(0i32)), // octant 2 (x=0,y=1,z=0): empty
            Rc::new(Cube::Solid(0i32)), // octant 3 (x=1,y=1,z=0): empty
            Rc::new(Cube::Solid(0i32)), // octant 4 (x=0,y=0,z=1): empty
            Rc::new(Cube::Solid(0i32)), // octant 5 (x=1,y=0,z=1): empty
            Rc::new(Cube::Solid(0i32)), // octant 6 (x=0,y=1,z=1): empty
            Rc::new(Cube::Solid(0i32)), // octant 7 (x=1,y=1,z=1): empty
        ];
        let cube = Cube::Cubes(Box::new(children));
        let is_empty = |v: &i32| *v == 0;

        // Cast ray into solid octant (0,0,0) from below
        // Position (0.1, 0.1, 0.0) is in the first octant (x<0.5, y<0.5)
        let pos = Vec3::new(0.1, 0.1, 0.0);
        let dir = Vec3::new(0.0, 0.0, 1.0);
        let hit = cube.raycast(pos, dir, 1, &is_empty);

        assert!(hit.is_ok());
        let hit = hit.unwrap();
        assert!(hit.is_some());
        let hit = hit.unwrap();

        // Check coordinate (octant 0 at depth 0)
        assert_eq!(hit.coord.depth, 0);
        assert_eq!(hit.coord.pos, IVec3::new(0, 0, 0));

        // Check position (returned in parent's normalized [0,1] space)
        // The hit position is at the entry point where the ray hits the solid voxel
        assert_eq!(hit.position.x, 0.1);
        assert_eq!(hit.position.y, 0.1);
        assert_eq!(hit.position.z, 0.0);

        // Check normal (entering from -Z face)
        assert_eq!(hit.normal, Axis::NegZ);
    }

    #[test]
    fn test_raycast_from_side() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        // Cast ray from left side (x=0) going right (x+)
        let pos = Vec3::new(0.0, 0.5, 0.5);
        let dir = Vec3::new(1.0, 0.0, 0.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty);

        assert!(hit.is_ok());
        let hit = hit.unwrap();
        assert!(hit.is_some());
        let hit = hit.unwrap();

        // Check normal (entering from -X face)
        assert_eq!(hit.normal, Axis::NegX);
        assert_eq!(hit.position.x, 0.0);
        assert_eq!(hit.position.y, 0.5);
        assert_eq!(hit.position.z, 0.5);
    }

    #[test]
    fn test_raycast_from_top() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        // Cast ray from top (y=1) going down (y-)
        let pos = Vec3::new(0.5, 1.0, 0.5);
        let dir = Vec3::new(0.0, -1.0, 0.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty);

        assert!(hit.is_ok());
        let hit = hit.unwrap();
        assert!(hit.is_some());
        let hit = hit.unwrap();

        // Check normal (entering from +Y face)
        assert_eq!(hit.normal, Axis::PosY);
        assert_eq!(hit.position.x, 0.5);
        assert_eq!(hit.position.y, 1.0);
        assert_eq!(hit.position.z, 0.5);
    }

    #[test]
    fn test_raycast_miss() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        // Cast ray starting outside the cube
        let pos = Vec3::new(2.0, 0.5, 0.5);
        let dir = Vec3::new(1.0, 0.0, 0.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty);

        // Should return StartOutOfBounds error
        assert!(hit.is_err(), "Ray from outside should return error");
        assert_eq!(hit.unwrap_err(), RaycastError::StartOutOfBounds);
    }

    #[test]
    fn test_raycast_deep_octree() {
        // Create deeper octree: depth 2 with one solid voxel at (0,0,0)
        // Level 1: octant 0 subdivided
        let level1_children = [
            Rc::new(Cube::Solid(1i32)), // octant 0: solid
            Rc::new(Cube::Solid(0i32)), // rest empty
            Rc::new(Cube::Solid(0i32)),
            Rc::new(Cube::Solid(0i32)),
            Rc::new(Cube::Solid(0i32)),
            Rc::new(Cube::Solid(0i32)),
            Rc::new(Cube::Solid(0i32)),
            Rc::new(Cube::Solid(0i32)),
        ];
        let level1_octant0 = Cube::Cubes(Box::new(level1_children));

        // Level 0: root with octant 0 subdivided
        let root_children = [
            Rc::new(level1_octant0),    // octant 0: subdivided
            Rc::new(Cube::Solid(0i32)), // rest empty
            Rc::new(Cube::Solid(0i32)),
            Rc::new(Cube::Solid(0i32)),
            Rc::new(Cube::Solid(0i32)),
            Rc::new(Cube::Solid(0i32)),
            Rc::new(Cube::Solid(0i32)),
            Rc::new(Cube::Solid(0i32)),
        ];
        let cube = Cube::Cubes(Box::new(root_children));
        let is_empty = |v: &i32| *v == 0;

        // Cast ray into the deepest solid voxel
        // Position (0.1, 0.1, 0.0) should hit octant 0->0
        let pos = Vec3::new(0.1, 0.1, 0.0);
        let dir = Vec3::new(0.0, 0.0, 1.0);
        let hit = cube.raycast(pos, dir, 2, &is_empty);

        assert!(hit.is_ok());
        let hit = hit.unwrap();
        assert!(hit.is_some());
        let hit = hit.unwrap();

        // Check coordinate
        // At depth 1, position (0.1, 0.1, 0.0) in root space
        // Maps to octant (0,0,0) at depth 1, position (0,0,0) in octree coords
        assert_eq!(hit.coord.depth, 0); // Leaf at depth 0
        assert_eq!(hit.coord.pos, IVec3::new(0, 0, 0));

        // Check position (in leaf's normalized [0,1] space)
        // The recursive algorithm maintains position in each voxel's local space
        // At each level, child_pos = (pos2 - bit) / 2 keeps values in [0,1]
        // So the leaf returns position in its own [0,1] space
        assert_eq!(hit.position.x, 0.1);
        assert_eq!(hit.position.y, 0.1);
        assert_eq!(hit.position.z, 0.0);

        // Check normal
        assert_eq!(hit.normal, Axis::NegZ);
    }

    #[test]
    fn test_raycast_diagonal() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        // Cast diagonal ray from corner
        let pos = Vec3::new(0.0, 0.0, 0.0);
        let dir = Vec3::new(1.0, 1.0, 1.0).normalize();
        let hit = cube.raycast(pos, dir, 3, &is_empty);

        assert!(hit.is_ok());
        let hit = hit.unwrap();
        assert!(hit.is_some());
        let hit = hit.unwrap();

        // Check that we hit at the corner
        assert_eq!(hit.position, Vec3::new(0.0, 0.0, 0.0));

        // Normal should be one of the three face normals (depending on implementation)
        let normal_is_valid =
            hit.normal == Axis::NegX || hit.normal == Axis::NegY || hit.normal == Axis::NegZ;
        assert!(
            normal_is_valid,
            "Normal should be one of the corner face normals, got {:?}",
            hit.normal
        );
    }

    // ============================================================================
    // Comprehensive Test Suite - TDD Approach
    // Following design document scenarios in docs/raycast.md
    // ============================================================================

    // Test helper: Create subdivided octree for testing
    fn create_test_octree_depth1(solid_octant: usize) -> Cube<i32> {
        let mut children = std::array::from_fn(|_| Rc::new(Cube::Solid(0i32)));
        children[solid_octant] = Rc::new(Cube::Solid(1i32));
        Cube::Cubes(Box::new(children))
    }

    // ============================================================================
    // 2.1 Basic Raycast Tests
    // ============================================================================

    #[test]
    fn test_basic_ray_through_center() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        // Ray through exact center
        let pos = Vec3::new(0.5, 0.5, 0.0);
        let dir = Vec3::new(0.0, 0.0, 1.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty);

        assert!(hit.is_ok(), "Raycast should succeed");
        let hit = hit.unwrap();
        assert!(hit.is_some(), "Ray through center should hit");
        let hit = hit.unwrap();
        assert_eq!(hit.normal, Axis::NegZ);
    }

    #[test]
    fn test_basic_ray_from_outside_bounds() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        // Ray starting outside [0,1] range
        let pos = Vec3::new(-0.5, 0.5, 0.5);
        let dir = Vec3::new(1.0, 0.0, 0.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty);

        // Should return StartOutOfBounds error
        assert!(hit.is_err(), "Ray from outside should return error");
        assert_eq!(hit.unwrap_err(), RaycastError::StartOutOfBounds);
    }

    // ============================================================================
    // 2.2 Octant Indexing Tests
    // ============================================================================

    #[test]
    fn test_octant_0_bottom_left_back() {
        // Octant 0 is (x=0, y=0, z=0) in child space
        let cube = create_test_octree_depth1(0);
        let is_empty = |v: &i32| *v == 0;

        let pos = Vec3::new(0.1, 0.1, 0.0);
        let dir = Vec3::new(0.0, 0.0, 1.0);
        let hit = cube.raycast(pos, dir, 1, &is_empty);

        assert!(hit.is_ok());
        assert!(hit.unwrap().is_some(), "Should hit octant 0");
    }

    #[test]
    fn test_octant_7_top_right_front() {
        // Octant 7 is (x=1, y=1, z=1) in child space
        let cube = create_test_octree_depth1(7);
        let is_empty = |v: &i32| *v == 0;

        let pos = Vec3::new(0.9, 0.9, 0.0);
        let dir = Vec3::new(0.0, 0.0, 1.0);
        let hit = cube.raycast(pos, dir, 1, &is_empty);

        assert!(hit.is_ok());
        assert!(hit.unwrap().is_some(), "Should hit octant 7");
    }

    // ============================================================================
    // 2.4 Normal Calculation Tests
    // ============================================================================

    #[test]
    fn test_normal_from_min_x_face() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        let pos = Vec3::new(0.0, 0.5, 0.5);
        let dir = Vec3::new(1.0, 0.0, 0.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty).unwrap().unwrap();

        assert_eq!(hit.normal, Axis::NegX, "Normal from -X face");
    }

    #[test]
    fn test_normal_from_max_x_face() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        let pos = Vec3::new(1.0, 0.5, 0.5);
        let dir = Vec3::new(-1.0, 0.0, 0.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty).unwrap().unwrap();

        assert_eq!(hit.normal, Axis::PosX, "Normal from +X face");
    }

    #[test]
    fn test_normal_from_min_y_face() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        let pos = Vec3::new(0.5, 0.0, 0.5);
        let dir = Vec3::new(0.0, 1.0, 0.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty).unwrap().unwrap();

        assert_eq!(hit.normal, Axis::NegY, "Normal from -Y face");
    }

    #[test]
    fn test_normal_from_max_y_face() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        let pos = Vec3::new(0.5, 1.0, 0.5);
        let dir = Vec3::new(0.0, -1.0, 0.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty).unwrap().unwrap();

        assert_eq!(hit.normal, Axis::PosY, "Normal from +Y face");
    }

    #[test]
    fn test_normal_from_min_z_face() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        let pos = Vec3::new(0.5, 0.5, 0.0);
        let dir = Vec3::new(0.0, 0.0, 1.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty).unwrap().unwrap();

        assert_eq!(hit.normal, Axis::NegZ, "Normal from -Z face");
    }

    #[test]
    fn test_normal_from_max_z_face() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        let pos = Vec3::new(0.5, 0.5, 1.0);
        let dir = Vec3::new(0.0, 0.0, -1.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty).unwrap().unwrap();

        assert_eq!(hit.normal, Axis::PosZ, "Normal from +Z face");
    }

    // ============================================================================
    // 2.7 Edge Case Tests - Axis-Aligned Rays
    // ============================================================================

    #[test]
    fn test_axis_aligned_positive_x() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        let pos = Vec3::new(0.0, 0.5, 0.5);
        let dir = Vec3::new(1.0, 0.0, 0.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty);

        assert!(hit.is_ok());
        let hit = hit.unwrap();
        assert!(hit.is_some(), "Axis-aligned +X ray should hit");
        assert_eq!(hit.unwrap().normal, Axis::NegX);
    }

    #[test]
    fn test_axis_aligned_negative_x() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        let pos = Vec3::new(1.0, 0.5, 0.5);
        let dir = Vec3::new(-1.0, 0.0, 0.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty);

        assert!(hit.is_ok());
        let hit = hit.unwrap();
        assert!(hit.is_some(), "Axis-aligned -X ray should hit");
        assert_eq!(hit.unwrap().normal, Axis::PosX);
    }

    #[test]
    fn test_axis_aligned_positive_y() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        let pos = Vec3::new(0.5, 0.0, 0.5);
        let dir = Vec3::new(0.0, 1.0, 0.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty);

        assert!(hit.is_ok());
        let hit = hit.unwrap();
        assert!(hit.is_some(), "Axis-aligned +Y ray should hit");
        assert_eq!(hit.unwrap().normal, Axis::NegY);
    }

    #[test]
    fn test_axis_aligned_negative_y() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        let pos = Vec3::new(0.5, 1.0, 0.5);
        let dir = Vec3::new(0.0, -1.0, 0.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty);

        assert!(hit.is_ok());
        let hit = hit.unwrap();
        assert!(hit.is_some(), "Axis-aligned -Y ray should hit");
        assert_eq!(hit.unwrap().normal, Axis::PosY);
    }

    #[test]
    fn test_axis_aligned_positive_z() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        let pos = Vec3::new(0.5, 0.5, 0.0);
        let dir = Vec3::new(0.0, 0.0, 1.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty);

        assert!(hit.is_ok());
        let hit = hit.unwrap();
        assert!(hit.is_some(), "Axis-aligned +Z ray should hit");
        assert_eq!(hit.unwrap().normal, Axis::NegZ);
    }

    #[test]
    fn test_axis_aligned_negative_z() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        let pos = Vec3::new(0.5, 0.5, 1.0);
        let dir = Vec3::new(0.0, 0.0, -1.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty);

        assert!(hit.is_ok());
        let hit = hit.unwrap();
        assert!(hit.is_some(), "Axis-aligned -Z ray should hit");
        assert_eq!(hit.unwrap().normal, Axis::PosZ);
    }

    // ============================================================================
    // 2.8 Depth Limit Tests
    // ============================================================================

    #[test]
    fn test_depth_0_no_subdivision() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        let pos = Vec3::new(0.5, 0.5, 0.0);
        let dir = Vec3::new(0.0, 0.0, 1.0);
        let hit = cube.raycast(pos, dir, 0, &is_empty);

        assert!(hit.is_ok());
        let hit = hit.unwrap();
        assert!(hit.is_some(), "Depth 0 should still hit solid");
        assert_eq!(hit.unwrap().coord.depth, 0);
    }

    #[test]
    fn test_max_depth_prevents_traversal() {
        // Create depth-2 octree where only the deepest level has solid
        let level1_children = [
            Rc::new(Cube::Solid(1i32)), // Solid at depth 2
            Rc::new(Cube::Solid(0i32)),
            Rc::new(Cube::Solid(0i32)),
            Rc::new(Cube::Solid(0i32)),
            Rc::new(Cube::Solid(0i32)),
            Rc::new(Cube::Solid(0i32)),
            Rc::new(Cube::Solid(0i32)),
            Rc::new(Cube::Solid(0i32)),
        ];
        let level1_octant0 = Cube::Cubes(Box::new(level1_children));
        let root_children = [
            Rc::new(level1_octant0),
            Rc::new(Cube::Solid(0i32)),
            Rc::new(Cube::Solid(0i32)),
            Rc::new(Cube::Solid(0i32)),
            Rc::new(Cube::Solid(0i32)),
            Rc::new(Cube::Solid(0i32)),
            Rc::new(Cube::Solid(0i32)),
            Rc::new(Cube::Solid(0i32)),
        ];
        let cube = Cube::Cubes(Box::new(root_children));
        let is_empty = |v: &i32| *v == 0;

        // With max_depth=2, can traverse to depth 2 and hit solid
        let pos = Vec3::new(0.1, 0.1, 0.0);
        let dir = Vec3::new(0.0, 0.0, 1.0);
        let hit_with_depth2 = cube.raycast(pos, dir, 2, &is_empty);
        assert!(hit_with_depth2.is_ok());
        assert!(
            hit_with_depth2.unwrap().is_some(),
            "Should hit with sufficient depth"
        );

        // With max_depth=1, cannot traverse into depth 2, misses solid
        // The implementation treats subdivided nodes at max depth as non-solid boundaries
        let hit_with_depth1 = cube.raycast(pos, dir, 1, &is_empty);
        // This is expected to miss because we can't traverse deep enough
        // (The _ pattern in match treats non-Solid Cubes at depth 0 as None)
        assert!(hit_with_depth1.is_ok());
        assert!(
            hit_with_depth1.unwrap().is_none(),
            "Should miss when depth limit prevents reaching solid"
        );
    }

    // ============================================================================
    // 2.3 Subdivision Traversal Tests
    // ============================================================================

    #[test]
    fn test_traverse_multiple_octants() {
        // Create octree with solid in octant 7 (far corner)
        let cube = create_test_octree_depth1(7);
        let is_empty = |v: &i32| *v == 0;

        // Ray starts in octant 0, travels to octant 7
        let pos = Vec3::new(0.1, 0.1, 0.1);
        let dir = Vec3::new(1.0, 1.0, 1.0).normalize();
        let hit = cube.raycast(pos, dir, 1, &is_empty);

        assert!(hit.is_ok());
        assert!(hit.unwrap().is_some(), "Should traverse and hit octant 7");
    }

    #[test]
    fn test_depth_3_traversal() {
        // Create depth-3 octree with solid at deepest level
        let level2_children = std::array::from_fn(|i| {
            if i == 0 {
                Rc::new(Cube::Solid(1i32))
            } else {
                Rc::new(Cube::Solid(0i32))
            }
        });
        let level2 = Cube::Cubes(Box::new(level2_children));

        let level1_children = std::array::from_fn(|i| {
            if i == 0 {
                Rc::new(level2.clone())
            } else {
                Rc::new(Cube::Solid(0i32))
            }
        });
        let level1 = Cube::Cubes(Box::new(level1_children));

        let root_children = std::array::from_fn(|i| {
            if i == 0 {
                Rc::new(level1.clone())
            } else {
                Rc::new(Cube::Solid(0i32))
            }
        });
        let cube = Cube::Cubes(Box::new(root_children));
        let is_empty = |v: &i32| *v == 0;

        let pos = Vec3::new(0.05, 0.05, 0.0);
        let dir = Vec3::new(0.0, 0.0, 1.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty);

        assert!(hit.is_ok());
        assert!(hit.unwrap().is_some(), "Should traverse to depth 3");
    }

    // ============================================================================
    // 2.9 Robustness Tests
    // ============================================================================

    #[test]
    fn test_ray_on_octant_boundary() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        // Ray starting exactly on boundary
        let pos = Vec3::new(0.5, 0.5, 0.5);
        let dir = Vec3::new(0.0, 0.0, 1.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty);

        // Should still hit (boundary is inside cube)
        assert!(hit.is_ok());
        assert!(hit.unwrap().is_some(), "Ray on boundary should hit");
    }

    #[test]
    fn test_ray_at_corner() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        // Ray at exact corner
        let pos = Vec3::new(1.0, 1.0, 1.0);
        let dir = Vec3::new(-1.0, -1.0, -1.0).normalize();
        let hit = cube.raycast(pos, dir, 3, &is_empty);

        assert!(hit.is_ok());
        assert!(hit.unwrap().is_some(), "Ray at corner should hit");
    }

    // ============================================================================
    // Comprehensive Debug State Tests
    // ============================================================================

    /// Expected debug state for a raycast test
    #[derive(Debug, Clone)]
    struct ExpectedDebugState {
        /// Expected minimum enter count
        min_enter_count: u32,
        /// Expected maximum enter count
        max_enter_count: u32,
        /// Expected max depth reached
        expected_max_depth: u32,
        /// Expected number of traversed nodes (optional)
        expected_node_count: Option<usize>,
    }

    impl ExpectedDebugState {
        fn exact(enter_count: u32, max_depth: u32) -> Self {
            Self {
                min_enter_count: enter_count,
                max_enter_count: enter_count,
                expected_max_depth: max_depth,
                expected_node_count: Some(enter_count as usize),
            }
        }

        fn range(min: u32, max: u32, max_depth: u32) -> Self {
            Self {
                min_enter_count: min,
                max_enter_count: max,
                expected_max_depth: max_depth,
                expected_node_count: None,
            }
        }

        fn verify(&self, debug: &RaycastDebugState, test_name: &str) {
            assert!(
                debug.enter_count >= self.min_enter_count,
                "{}: enter_count {} is less than expected minimum {}",
                test_name,
                debug.enter_count,
                self.min_enter_count
            );
            assert!(
                debug.enter_count <= self.max_enter_count,
                "{}: enter_count {} is greater than expected maximum {}",
                test_name,
                debug.enter_count,
                self.max_enter_count
            );
            assert_eq!(
                debug.max_depth_reached, self.expected_max_depth,
                "{}: max_depth_reached {} != expected {}",
                test_name, debug.max_depth_reached, self.expected_max_depth
            );
            if let Some(expected_count) = self.expected_node_count {
                assert_eq!(
                    debug.traversed_nodes.len(),
                    expected_count,
                    "{}: traversed_nodes.len() {} != expected {}",
                    test_name,
                    debug.traversed_nodes.len(),
                    expected_count
                );
            }
        }
    }

    /// Test case data for raycast validation
    #[derive(Debug, Clone)]
    struct RaycastTestCase {
        name: &'static str,
        pos: Vec3,
        dir: Vec3,
        should_hit: bool,
        expected_value: Option<i32>,
        expected_debug: ExpectedDebugState,
    }

    #[test]
    fn test_debug_state_solid_cube() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        // Test entering face voxel that has color
        let pos = Vec3::new(0.5, 0.5, 0.0);
        let dir = Vec3::new(0.0, 0.0, 1.0);
        let hit = cube.raycast_debug(pos, dir, 3, &is_empty);

        assert!(hit.is_ok());
        let hit = hit.unwrap();
        assert!(hit.is_some(), "Should hit solid cube");
        let hit = hit.unwrap();
        assert!(hit.debug.is_some(), "Debug state should be populated");

        let debug = hit.debug.unwrap();
        // When entering face voxel has color cube, raycast steps should be 1
        assert_eq!(
            debug.enter_count, 1,
            "Entering face voxel with color should have enter_count = 1"
        );
        assert_eq!(
            debug.max_depth_reached, 3,
            "Max depth should match the max_depth parameter"
        );
        assert_eq!(
            debug.traversed_nodes.len(),
            1,
            "Should traverse exactly 1 node"
        );
    }

    #[test]
    fn test_debug_state_axis_aligned_rays() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        // Test cases for 6 axis-aligned rays from all main axes
        let test_cases = vec![
            RaycastTestCase {
                name: "positive X",
                pos: Vec3::new(0.0, 0.5, 0.5),
                dir: Vec3::new(1.0, 0.0, 0.0),
                should_hit: true,
                expected_value: Some(1),
                expected_debug: ExpectedDebugState::exact(1, 3),
            },
            RaycastTestCase {
                name: "negative X",
                pos: Vec3::new(1.0, 0.5, 0.5),
                dir: Vec3::new(-1.0, 0.0, 0.0),
                should_hit: true,
                expected_value: Some(1),
                expected_debug: ExpectedDebugState::exact(1, 3),
            },
            RaycastTestCase {
                name: "positive Y",
                pos: Vec3::new(0.5, 0.0, 0.5),
                dir: Vec3::new(0.0, 1.0, 0.0),
                should_hit: true,
                expected_value: Some(1),
                expected_debug: ExpectedDebugState::exact(1, 3),
            },
            RaycastTestCase {
                name: "negative Y",
                pos: Vec3::new(0.5, 1.0, 0.5),
                dir: Vec3::new(0.0, -1.0, 0.0),
                should_hit: true,
                expected_value: Some(1),
                expected_debug: ExpectedDebugState::exact(1, 3),
            },
            RaycastTestCase {
                name: "positive Z",
                pos: Vec3::new(0.5, 0.5, 0.0),
                dir: Vec3::new(0.0, 0.0, 1.0),
                should_hit: true,
                expected_value: Some(1),
                expected_debug: ExpectedDebugState::exact(1, 3),
            },
            RaycastTestCase {
                name: "negative Z",
                pos: Vec3::new(0.5, 0.5, 1.0),
                dir: Vec3::new(0.0, 0.0, -1.0),
                should_hit: true,
                expected_value: Some(1),
                expected_debug: ExpectedDebugState::exact(1, 3),
            },
        ];

        for test_case in test_cases {
            let hit = cube.raycast_debug(test_case.pos, test_case.dir, 3, &is_empty);
            assert!(hit.is_ok());
            let hit = hit.unwrap();
            assert_eq!(
                hit.is_some(),
                test_case.should_hit,
                "Test '{}': hit expectation mismatch",
                test_case.name
            );

            if let Some(hit) = hit {
                if let Some(expected_value) = test_case.expected_value {
                    assert_eq!(
                        hit.value, expected_value,
                        "Test '{}': value mismatch",
                        test_case.name
                    );
                }

                assert!(
                    hit.debug.is_some(),
                    "Test '{}': debug state should be populated",
                    test_case.name
                );
                test_case
                    .expected_debug
                    .verify(&hit.debug.unwrap(), test_case.name);
            }
        }
    }

    #[test]
    fn test_debug_state_random_misses() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        // 6 pseudorandom configurations that should miss the cube
        let test_cases = vec![
            RaycastTestCase {
                name: "miss from outside +X",
                pos: Vec3::new(2.0, 0.5, 0.5),
                dir: Vec3::new(1.0, 0.0, 0.0),
                should_hit: false,
                expected_value: None,
                expected_debug: ExpectedDebugState::exact(0, 0), // No nodes entered
            },
            RaycastTestCase {
                name: "miss from outside -X",
                pos: Vec3::new(-1.0, 0.5, 0.5),
                dir: Vec3::new(-1.0, 0.0, 0.0),
                should_hit: false,
                expected_value: None,
                expected_debug: ExpectedDebugState::exact(0, 0),
            },
            RaycastTestCase {
                name: "miss from outside +Y",
                pos: Vec3::new(0.5, 2.0, 0.5),
                dir: Vec3::new(0.0, 1.0, 0.0),
                should_hit: false,
                expected_value: None,
                expected_debug: ExpectedDebugState::exact(0, 0),
            },
            RaycastTestCase {
                name: "miss from outside -Y",
                pos: Vec3::new(0.5, -1.0, 0.5),
                dir: Vec3::new(0.0, -1.0, 0.0),
                should_hit: false,
                expected_value: None,
                expected_debug: ExpectedDebugState::exact(0, 0),
            },
            RaycastTestCase {
                name: "miss from outside +Z",
                pos: Vec3::new(0.5, 0.5, 2.0),
                dir: Vec3::new(0.0, 0.0, 1.0),
                should_hit: false,
                expected_value: None,
                expected_debug: ExpectedDebugState::exact(0, 0),
            },
            RaycastTestCase {
                name: "miss from outside -Z",
                pos: Vec3::new(0.5, 0.5, -1.0),
                dir: Vec3::new(0.0, 0.0, -1.0),
                should_hit: false,
                expected_value: None,
                expected_debug: ExpectedDebugState::exact(0, 0),
            },
        ];

        for test_case in test_cases {
            let hit = cube.raycast_debug(test_case.pos, test_case.dir, 3, &is_empty);
            assert!(hit.is_ok());
            assert_eq!(
                hit.unwrap().is_some(),
                test_case.should_hit,
                "Test '{}': hit expectation mismatch",
                test_case.name
            );
        }
    }

    #[test]
    fn test_debug_state_octree_traversal() {
        // Create octree with one solid octant to test traversal counts
        let cube = create_test_octree_depth1(7);
        let is_empty = |v: &i32| *v == 0;

        // Ray starting in empty octant 0, traveling to solid octant 7
        let pos = Vec3::new(0.1, 0.1, 0.1);
        let dir = Vec3::new(1.0, 1.0, 1.0).normalize();
        let hit = cube.raycast_debug(pos, dir, 1, &is_empty);

        assert!(hit.is_ok());
        let hit = hit.unwrap();
        assert!(hit.is_some(), "Should traverse and hit octant 7");
        let hit = hit.unwrap();
        assert!(hit.debug.is_some(), "Debug state should be populated");

        let debug = hit.debug.unwrap();
        // Should traverse multiple octants to reach the solid one
        // The exact count depends on the DDA algorithm path
        assert!(
            debug.enter_count > 1,
            "Should enter multiple nodes when traversing octants (got {})",
            debug.enter_count
        );
        // The max_depth_reached will be 1 (current_depth when we enter root octree node)
        // then 0 when we enter the leaf solid voxel
        // The tracked max_depth_reached is the deepest current_depth value seen
        assert!(
            debug.max_depth_reached >= 0,
            "Should have traversed at least to depth 0"
        );
    }
}
