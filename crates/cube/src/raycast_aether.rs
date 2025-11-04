use crate::{Cube, CubeCoord, IVec3Ext};
use glam::{IVec3, Vec3};

/// Result of a raycast hit
#[derive(Debug, Clone)]
pub struct RaycastHit {
    /// Coordinate of the hit voxel in octree space
    pub coord: CubeCoord,
    /// Hit position in world space (normalized [0,1] cube space)
    pub position: Vec3,
    /// Surface normal at hit point
    pub normal: Vec3,
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
    /// `Some(RaycastHit)` if a non-empty voxel is hit, `None` otherwise
    pub fn raycast<F>(
        &self,
        pos: Vec3,
        dir: Vec3,
        max_depth: u32,
        is_empty: &F,
    ) -> Option<RaycastHit>
    where
        F: Fn(&T) -> bool,
    {
        self.raycast_recursive(pos, dir, max_depth, IVec3::ZERO, max_depth, is_empty)
    }

    /// Recursive raycast implementation
    fn raycast_recursive<F>(
        &self,
        pos: Vec3,
        dir: Vec3,
        _max_depth: u32,
        octree_pos: IVec3,
        current_depth: u32,
        is_empty: &F,
    ) -> Option<RaycastHit>
    where
        F: Fn(&T) -> bool,
    {
        // Check if position is in valid range [0, 1]
        if !pos.cmpge(Vec3::ZERO).all() || !pos.cmple(Vec3::ONE).all() {
            return None;
        }

        // Check if this is a leaf node
        match self {
            Cube::Solid(value) => {
                // Hit a solid voxel - check if it's non-empty
                if !is_empty(value) {
                    // Calculate surface normal from entry point
                    let normal = calculate_entry_normal(pos, dir);
                    return Some(RaycastHit {
                        coord: CubeCoord::new(octree_pos, current_depth),
                        position: pos,
                        normal,
                    });
                }
                // Empty voxel - no hit
                None
            }
            Cube::Cubes(children) if current_depth > 0 => {
                // Continue traversing octree
                let sign = dir.signum();
                let sign_int = IVec3::new(
                    if dir.x >= 0.0 { 1 } else { -1 },
                    if dir.y >= 0.0 { 1 } else { -1 },
                    if dir.z >= 0.0 { 1 } else { -1 },
                );
                let sign10 = IVec3::new(
                    if dir.x >= 0.0 { 0 } else { 1 },
                    if dir.y >= 0.0 { 0 } else { 1 },
                    if dir.z >= 0.0 { 0 } else { 1 },
                );

                // Calculate which octant we're in
                let pos2 = pos * 2.0;
                let mut bit = (pos2 * Vec3::new(sign.x, sign.y, sign.z))
                    .floor()
                    .as_ivec3();
                bit = bit * sign_int + sign10;

                // Check if bit is in valid range [0, 1]
                if bit.x < 0 || bit.x > 1 || bit.y < 0 || bit.y > 1 || bit.z < 0 || bit.z > 1 {
                    return None;
                }

                // Calculate octant index
                let index = bit.to_octant_index();

                // Try casting into child octant
                let child_pos = (pos2 - bit.as_vec3()) / 2.0;
                let child_octree_pos = (octree_pos << 1) + bit;

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

                // Miss in this octant - step to next octant boundary
                let next_integer = next_integer_boundary(pos2, sign);
                let diff = next_integer - pos2;

                // Avoid division by zero
                if diff.x.abs() < 1e-8 && diff.y.abs() < 1e-8 && diff.z.abs() < 1e-8 {
                    return None;
                }

                let inv_time = dir / diff;
                let max_inv = inv_time.x.max(inv_time.y).max(inv_time.z);

                if max_inv.abs() < 1e-8 {
                    return None;
                }

                let step = diff * (inv_time / max_inv);
                let next_pos = (pos2 + step) / 2.0;

                // Clamp to valid range
                let next_pos_clamped = next_pos.clamp(Vec3::ZERO, Vec3::ONE);

                // Continue raycast from new position
                self.raycast_recursive(
                    next_pos_clamped,
                    dir,
                    _max_depth,
                    octree_pos,
                    current_depth,
                    is_empty,
                )
            }
            _ => {
                // At max depth or non-subdivided structure
                // Treat as solid
                if let Cube::Solid(value) = self {
                    if !is_empty(value) {
                        let normal = calculate_entry_normal(pos, dir);
                        return Some(RaycastHit {
                            coord: CubeCoord::new(octree_pos, current_depth),
                            position: pos,
                            normal,
                        });
                    }
                }
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

/// Calculate surface normal from entry point
/// The normal points towards the direction the ray came from
fn calculate_entry_normal(pos: Vec3, _dir: Vec3) -> Vec3 {
    // Find which face we entered from by checking which coordinate is closest to 0 or 1
    let dist_to_min = pos;
    let dist_to_max = Vec3::ONE - pos;

    let min_dist = dist_to_min.min_element();
    let max_dist = dist_to_max.min_element();

    if min_dist < max_dist {
        // Entered from min face
        if dist_to_min.x == min_dist {
            Vec3::new(-1.0, 0.0, 0.0)
        } else if dist_to_min.y == min_dist {
            Vec3::new(0.0, -1.0, 0.0)
        } else {
            Vec3::new(0.0, 0.0, -1.0)
        }
    } else {
        // Entered from max face
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
}
