//! Raycasting through octrees
//!
//! This module provides efficient ray-octree intersection using a recursive DDA
//! (Digital Differential Analyzer) algorithm. The raycast system finds the first
//! solid voxel along a ray path by hierarchically traversing the octree structure.
//!
//! # Algorithm Overview
//!
//! The raycast uses recursive octree traversal with DDA stepping to efficiently
//! skip empty space:
//!
//! 1. **Octant Selection**: Determine which child octant contains the ray position
//! 2. **Recursive Descent**: Transform ray to child coordinate space and recurse
//! 3. **DDA Stepping**: On miss, calculate next octant boundary and step to it
//! 4. **Early Termination**: Stop immediately upon hitting first solid voxel
//!
//! # Coordinate Systems
//!
//! - **Normalized [0,1]³ Space**: Octree traversal uses normalized coordinates
//!   where (0,0,0) is the cube minimum and (1,1,1) is the cube maximum
//! - **Octree Coordinates**: Encoded as Morton code path through octree hierarchy
//!
//! # Octant Indexing
//!
//! Octants are indexed 0-7 using bit encoding: `(x_bit << 2) | (y_bit << 1) | z_bit`
//!
//! ```text
//! Octant 0 (000): (-x, -y, -z)   Octant 4 (100): (+x, -y, -z)
//! Octant 1 (001): (-x, -y, +z)   Octant 5 (101): (+x, -y, +z)
//! Octant 2 (010): (-x, +y, -z)   Octant 6 (110): (+x, +y, -z)
//! Octant 3 (011): (-x, +y, +z)   Octant 7 (111): (+x, +y, +z)
//! ```
//!
//! # Performance
//!
//! - **Early Termination**: Exits on first solid hit, no full tree traversal
//! - **Empty Space Skipping**: DDA stepping efficiently jumps over empty octants
//! - **Depth Limiting**: `max_depth` parameter prevents excessive recursion
//!
//! # Example
//!
//! ```rust
//! use cube::{Cube, raycast::RaycastHit};
//! use glam::Vec3;
//!
//! let cube = Cube::Solid(1i32);
//! let is_empty = |v: &i32| *v == 0;
//!
//! // Cast ray from bottom going up
//! let pos = Vec3::new(0.5, 0.5, 0.0);
//! let dir = Vec3::new(0.0, 0.0, 1.0);
//! let hit: Option<RaycastHit<i32>> = cube.raycast(pos, dir, 3, &is_empty);
//!
//! if let Some(hit) = hit {
//!     println!("Hit at {:?}", hit.position);
//!     println!("Normal: {:?}", hit.normal);
//!     println!("Voxel value: {:?}", hit.value);
//! }
//! ```
//!
//! # References
//!
//! - Design document: `docs/raycast.md`
//! - "An Efficient Parametric Algorithm for Octree Traversal" - Revelles et al.
//! - "A Fast Voxel Traversal Algorithm for Ray Tracing" - Amanatides & Woo

use crate::{Cube, CubeCoord, IVec3Ext};
use glam::{IVec3, Vec3};

/// Result of a raycast hit
///
/// Generic over the voxel type `T`, allowing different voxel data types
/// (e.g., `i32`, custom materials, colors, etc.)
#[derive(Debug, Clone)]
pub struct RaycastHit<T> {
    /// Coordinate of the hit voxel in octree space
    pub coord: CubeCoord,
    /// Hit position in world space (normalized [0,1] cube space)
    pub position: Vec3,
    /// Surface normal at hit point
    pub normal: Vec3,
    /// Voxel value at the hit position
    pub value: T,
}

impl<T> Cube<T>
where
    T: Clone + PartialEq,
{
    /// Cast a ray through the octree and find the first non-empty voxel
    ///
    /// # Arguments
    /// * `pos` - Starting position in normalized [0, 1] cube space
    /// * `dir` - Ray direction (should be normalized)
    /// * `max_depth` - Maximum octree depth to traverse
    /// * `is_empty` - Function to test if a voxel value is considered empty
    ///
    /// # Returns
    /// `Some(RaycastHit<T>)` if a non-empty voxel is hit, `None` otherwise
    pub fn raycast<F>(
        &self,
        pos: Vec3,
        dir: Vec3,
        max_depth: u32,
        is_empty: &F,
    ) -> Option<RaycastHit<T>>
    where
        F: Fn(&T) -> bool,
    {
        self.raycast_recursive(pos, dir, max_depth, IVec3::ZERO, max_depth, is_empty)
    }

    /// Recursive raycast implementation following the design in docs/raycast.md
    fn raycast_recursive<F>(
        &self,
        pos: Vec3,
        dir: Vec3,
        _max_depth: u32,
        octree_pos: IVec3,
        current_depth: u32,
        is_empty: &F,
    ) -> Option<RaycastHit<T>>
    where
        F: Fn(&T) -> bool,
    {
        // Validate position is in [0, 1]³
        if !pos.cmpge(Vec3::ZERO).all() || !pos.cmple(Vec3::ONE).all() {
            return None;
        }

        // Check cube type
        match self {
            Cube::Solid(value) => {
                // Non-empty voxel - return hit
                if !is_empty(value) {
                    let normal = calculate_entry_normal(pos, dir);
                    Some(RaycastHit {
                        coord: CubeCoord::new(octree_pos, current_depth),
                        position: pos,
                        normal,
                        value: value.clone(),
                    })
                } else {
                    // Empty voxel
                    None
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
                if let Some(hit) = children[index].raycast_recursive(
                    child_pos,
                    dir,
                    _max_depth,
                    child_octree_pos,
                    current_depth - 1,
                    is_empty,
                ) {
                    return Some(hit);
                }

                // Miss in this octant - step to next boundary
                let next_pos = calculate_next_position(pos2, dir, sign);

                // Check if next position is still within valid bounds or if we've made no progress
                // If it's outside [0,1]³ or identical to current position, we've exited/stuck
                if !next_pos.cmpge(Vec3::ZERO).all() || !next_pos.cmple(Vec3::ONE).all() {
                    return None;
                }

                // Prevent infinite recursion: if next_pos is same as pos, we haven't moved
                const EPSILON: f32 = 1e-10;
                if (next_pos - pos).length() < EPSILON {
                    return None;
                }

                // Continue raycasting from new position
                self.raycast_recursive(
                    next_pos,
                    dir,
                    _max_depth,
                    octree_pos,
                    current_depth,
                    is_empty,
                )
            }
            _ => {
                // Max depth or unsupported structure
                None
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
fn calculate_entry_normal(pos: Vec3, _dir: Vec3) -> Vec3 {
    let dist_to_min = pos;
    let dist_to_max = Vec3::ONE - pos;

    let min_dist = dist_to_min.min_element();
    let max_dist = dist_to_max.min_element();

    if min_dist < max_dist {
        // Entered from min face (0, 0, 0)
        if dist_to_min.x == min_dist {
            Vec3::new(-1.0, 0.0, 0.0)
        } else if dist_to_min.y == min_dist {
            Vec3::new(0.0, -1.0, 0.0)
        } else {
            Vec3::new(0.0, 0.0, -1.0)
        }
    } else {
        // Entered from max face (1, 1, 1)
        if dist_to_max.x == max_dist {
            Vec3::new(1.0, 0.0, 0.0)
        } else if dist_to_max.y == max_dist {
            Vec3::new(0.0, 1.0, 0.0)
        } else {
            Vec3::new(0.0, 0.0, 1.0)
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
        assert_eq!(hit.normal, Vec3::new(0.0, 0.0, -1.0));
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
        assert!(hit.is_none());
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
        assert_eq!(hit.normal, Vec3::new(0.0, 0.0, -1.0));
    }

    #[test]
    fn test_raycast_from_side() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        // Cast ray from left side (x=0) going right (x+)
        let pos = Vec3::new(0.0, 0.5, 0.5);
        let dir = Vec3::new(1.0, 0.0, 0.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty);

        assert!(hit.is_some());
        let hit = hit.unwrap();

        // Check normal (entering from -X face)
        assert_eq!(hit.normal, Vec3::new(-1.0, 0.0, 0.0));
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

        assert!(hit.is_some());
        let hit = hit.unwrap();

        // Check normal (entering from +Y face)
        assert_eq!(hit.normal, Vec3::new(0.0, 1.0, 0.0));
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

        // Should miss - outside valid [0,1] range
        assert!(hit.is_none());
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
        assert_eq!(hit.normal, Vec3::new(0.0, 0.0, -1.0));
    }

    #[test]
    fn test_raycast_diagonal() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        // Cast diagonal ray from corner
        let pos = Vec3::new(0.0, 0.0, 0.0);
        let dir = Vec3::new(1.0, 1.0, 1.0).normalize();
        let hit = cube.raycast(pos, dir, 3, &is_empty);

        assert!(hit.is_some());
        let hit = hit.unwrap();

        // Check that we hit at the corner
        assert_eq!(hit.position, Vec3::new(0.0, 0.0, 0.0));

        // Normal should be one of the three face normals (depending on implementation)
        let normal_is_valid = hit.normal == Vec3::new(-1.0, 0.0, 0.0)
            || hit.normal == Vec3::new(0.0, -1.0, 0.0)
            || hit.normal == Vec3::new(0.0, 0.0, -1.0);
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

        assert!(hit.is_some(), "Ray through center should hit");
        let hit = hit.unwrap();
        assert_eq!(hit.normal, Vec3::new(0.0, 0.0, -1.0));
    }

    #[test]
    fn test_basic_ray_from_outside_bounds() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        // Ray starting outside [0,1] range
        let pos = Vec3::new(-0.5, 0.5, 0.5);
        let dir = Vec3::new(1.0, 0.0, 0.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty);

        assert!(hit.is_none(), "Ray from outside should miss");
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

        assert!(hit.is_some(), "Should hit octant 0");
    }

    #[test]
    fn test_octant_7_top_right_front() {
        // Octant 7 is (x=1, y=1, z=1) in child space
        let cube = create_test_octree_depth1(7);
        let is_empty = |v: &i32| *v == 0;

        let pos = Vec3::new(0.9, 0.9, 0.0);
        let dir = Vec3::new(0.0, 0.0, 1.0);
        let hit = cube.raycast(pos, dir, 1, &is_empty);

        assert!(hit.is_some(), "Should hit octant 7");
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
        let hit = cube.raycast(pos, dir, 3, &is_empty).unwrap();

        assert_eq!(hit.normal, Vec3::new(-1.0, 0.0, 0.0), "Normal from -X face");
    }

    #[test]
    fn test_normal_from_max_x_face() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        let pos = Vec3::new(1.0, 0.5, 0.5);
        let dir = Vec3::new(-1.0, 0.0, 0.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty).unwrap();

        assert_eq!(hit.normal, Vec3::new(1.0, 0.0, 0.0), "Normal from +X face");
    }

    #[test]
    fn test_normal_from_min_y_face() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        let pos = Vec3::new(0.5, 0.0, 0.5);
        let dir = Vec3::new(0.0, 1.0, 0.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty).unwrap();

        assert_eq!(hit.normal, Vec3::new(0.0, -1.0, 0.0), "Normal from -Y face");
    }

    #[test]
    fn test_normal_from_max_y_face() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        let pos = Vec3::new(0.5, 1.0, 0.5);
        let dir = Vec3::new(0.0, -1.0, 0.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty).unwrap();

        assert_eq!(hit.normal, Vec3::new(0.0, 1.0, 0.0), "Normal from +Y face");
    }

    #[test]
    fn test_normal_from_min_z_face() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        let pos = Vec3::new(0.5, 0.5, 0.0);
        let dir = Vec3::new(0.0, 0.0, 1.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty).unwrap();

        assert_eq!(hit.normal, Vec3::new(0.0, 0.0, -1.0), "Normal from -Z face");
    }

    #[test]
    fn test_normal_from_max_z_face() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        let pos = Vec3::new(0.5, 0.5, 1.0);
        let dir = Vec3::new(0.0, 0.0, -1.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty).unwrap();

        assert_eq!(hit.normal, Vec3::new(0.0, 0.0, 1.0), "Normal from +Z face");
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

        assert!(hit.is_some(), "Axis-aligned +X ray should hit");
        assert_eq!(hit.unwrap().normal, Vec3::new(-1.0, 0.0, 0.0));
    }

    #[test]
    fn test_axis_aligned_negative_x() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        let pos = Vec3::new(1.0, 0.5, 0.5);
        let dir = Vec3::new(-1.0, 0.0, 0.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty);

        assert!(hit.is_some(), "Axis-aligned -X ray should hit");
        assert_eq!(hit.unwrap().normal, Vec3::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn test_axis_aligned_positive_y() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        let pos = Vec3::new(0.5, 0.0, 0.5);
        let dir = Vec3::new(0.0, 1.0, 0.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty);

        assert!(hit.is_some(), "Axis-aligned +Y ray should hit");
        assert_eq!(hit.unwrap().normal, Vec3::new(0.0, -1.0, 0.0));
    }

    #[test]
    fn test_axis_aligned_negative_y() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        let pos = Vec3::new(0.5, 1.0, 0.5);
        let dir = Vec3::new(0.0, -1.0, 0.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty);

        assert!(hit.is_some(), "Axis-aligned -Y ray should hit");
        assert_eq!(hit.unwrap().normal, Vec3::new(0.0, 1.0, 0.0));
    }

    #[test]
    fn test_axis_aligned_positive_z() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        let pos = Vec3::new(0.5, 0.5, 0.0);
        let dir = Vec3::new(0.0, 0.0, 1.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty);

        assert!(hit.is_some(), "Axis-aligned +Z ray should hit");
        assert_eq!(hit.unwrap().normal, Vec3::new(0.0, 0.0, -1.0));
    }

    #[test]
    fn test_axis_aligned_negative_z() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        let pos = Vec3::new(0.5, 0.5, 1.0);
        let dir = Vec3::new(0.0, 0.0, -1.0);
        let hit = cube.raycast(pos, dir, 3, &is_empty);

        assert!(hit.is_some(), "Axis-aligned -Z ray should hit");
        assert_eq!(hit.unwrap().normal, Vec3::new(0.0, 0.0, 1.0));
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
        assert!(
            hit_with_depth2.is_some(),
            "Should hit with sufficient depth"
        );

        // With max_depth=1, cannot traverse into depth 2, misses solid
        // The implementation treats subdivided nodes at max depth as non-solid boundaries
        let hit_with_depth1 = cube.raycast(pos, dir, 1, &is_empty);
        // This is expected to miss because we can't traverse deep enough
        // (The _ pattern in match treats non-Solid Cubes at depth 0 as None)
        assert!(
            hit_with_depth1.is_none(),
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

        assert!(hit.is_some(), "Should traverse and hit octant 7");
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

        assert!(hit.is_some(), "Should traverse to depth 3");
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
        assert!(hit.is_some(), "Ray on boundary should hit");
    }

    #[test]
    fn test_ray_at_corner() {
        let cube = Cube::Solid(1i32);
        let is_empty = |v: &i32| *v == 0;

        // Ray at exact corner
        let pos = Vec3::new(1.0, 1.0, 1.0);
        let dir = Vec3::new(-1.0, -1.0, -1.0).normalize();
        let hit = cube.raycast(pos, dir, 3, &is_empty);

        assert!(hit.is_some(), "Ray at corner should hit");
    }
}
