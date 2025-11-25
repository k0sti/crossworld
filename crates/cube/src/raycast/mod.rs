//! Raycasting through octrees
//!
//! This module provides efficient ray-octree intersection using a recursive DDA
//! (Digital Differential Analyzer) algorithm. The raycast system finds the first
//! solid voxel along a ray path by hierarchically traversing the octree structure.
//!
//! # Coordinate System
//!
//! The raycast uses an origin-centered coordinate system where the root cube
//! extends from -1.0 to 1.0 on each axis.
//!
//! # Algorithm
//!
//! The algorithm uses recursive traversal with DDA stepping for empty space skipping.
//! When a ray misses a child node, it calculates the distance to the next boundary
//! and steps to the adjacent octant.

use crate::{Axis, Cube, CubeCoord, IVec3Ext};
use glam::{ivec3, vec3, IVec3, Vec3};

/// Result of a raycast hit
#[derive(Debug, Clone)]
pub struct RaycastHit<T> {
    /// Coordinate of the hit voxel
    pub coord: CubeCoord,
    /// Voxel value at the hit position
    pub value: T,
    /// Axis of the surface normal at hit point (includes sign: PosX/NegX, etc)
    pub normal_axis: Axis,
    /// Exact hit position in local node space [-1, 1]
    pub hit_pos: Vec3,
    /// Debug information (optional)
    pub debug: Option<RaycastDebugState>,
}

impl<T> RaycastHit<T> {
    /// Get the normal vector from axis (including sign)
    pub fn normal(&self) -> Vec3 {
        // Use the Axis enum's built-in conversion
        self.normal_axis.as_vec3()
    }
}

pub mod error;
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

impl<T> Cube<T>
where
    T: Clone + PartialEq,
{
    /// Cast a ray through the octree and find the first non-empty voxel
    ///
    /// # Arguments
    /// * `ray_origin` - Starting position in local space [-1, 1]
    /// * `ray_dir` - Normalized ray direction
    /// * `max_depth` - Maximum depth to traverse
    /// * `is_empty` - Predicate to check if a voxel is empty
    pub fn raycast<F>(
        &self,
        ray_origin: Vec3,
        ray_dir: Vec3,
        max_depth: u32,
        is_empty: &F,
    ) -> Option<RaycastHit<T>>
    where
        F: Fn(&T) -> bool,
    {
        self.raycast_debug(ray_origin, ray_dir, max_depth, is_empty)
            .map(|hit| RaycastHit { debug: None, ..hit })
    }

    /// Cast a ray with debug tracking
    pub fn raycast_debug<F>(
        &self,
        ray_origin: Vec3,
        ray_dir: Vec3,
        max_depth: u32,
        is_empty: &F,
    ) -> Option<RaycastHit<T>>
    where
        F: Fn(&T) -> bool,
    {
        // Initial AABB check: Is the ray actually hitting the root node?
        // Root is assumed to be from -1.0 to 1.0
        let t_entry = intersect_aabb_entry(
            ray_origin,
            ray_dir,
            vec3(-1.0, -1.0, -1.0),
            vec3(1.0, 1.0, 1.0),
        );

        // If t_entry < 0.0, the ray might be inside or looking away.
        // For this recursive logic, we assume we start traversal at the entry point or current pos if inside.

        // Project ray to entry point if outside
        let start_pos = if t_entry > 0.0 {
            ray_origin + ray_dir * (t_entry + 0.00001) // Add epsilon to enter
        } else {
            // If inside, check if we are within bounds. If completely outside and looking away, return None.
            if ray_origin.abs().max_element() > 1.0 + 0.00001 {
                return None;
            }
            ray_origin
        };

        let mut debug = RaycastDebugState::new();
        let root_coord = CubeCoord::new(IVec3::ZERO, 0);

        self.recursive_raycast(
            start_pos,
            ray_dir,
            root_coord,
            max_depth,
            is_empty,
            Some(&mut debug),
        )
    }

    fn recursive_raycast<F>(
        &self,
        local_pos: Vec3,
        ray_dir: Vec3,
        current_coord: CubeCoord,
        max_depth: u32,
        is_empty: &F,
        mut debug: Option<&mut RaycastDebugState>,
    ) -> Option<RaycastHit<T>>
    where
        F: Fn(&T) -> bool,
    {
        if let Some(d) = debug.as_deref_mut() {
            d.record_enter(current_coord, current_coord.depth);
        }

        match self {
            Cube::Solid(value) => {
                // Leaf reached
                if !is_empty(value) {
                    let axis = get_entry_normal(local_pos, ray_dir);
                    Some(RaycastHit {
                        coord: current_coord,
                        value: value.clone(),
                        normal_axis: axis,
                        hit_pos: local_pos,
                        debug: debug.cloned(),
                    })
                } else {
                    None
                }
            }
            Cube::Cubes(children) => {
                if current_coord.depth >= max_depth {
                    return None;
                }

                // 1. Identify the first child octant the ray is in.
                let mut octant_idx = get_octant_index(local_pos);

                // We loop until the ray exits this parent Node.
                loop {
                    // Construct the CubeCoord for the specific child we are looking at
                    let child_offset = get_octant_offset_ivec(octant_idx);

                    // Note: User's code used `current_coord.coord * 2 + child_offset`.
                    // Our `CubeCoord` uses `pos`.
                    // Also, our `CubeCoord` logic in `neighbor_grid.rs` uses `(pos << 1) + offset` where offset is 0 or 1.
                    // The user's logic seems to use a centered coordinate system for indices too?
                    // "Coordinate in depth scale, centered at origin"
                    // Let's stick to our existing `CubeCoord` convention which is 0..2^depth.
                    // But wait, the user's code explicitly defines `CubeCoord` as "centered at origin".
                    // If I use existing `CubeCoord`, it expects 0..N.
                    // The user's `recursive_raycast` passes `next_coord`.
                    // If I want to maintain compatibility with existing `CubeCoord` (which is used elsewhere),
                    // I should probably map the traversal to the existing coordinate system or adapt `CubeCoord`.
                    // Existing `CubeCoord` is `pos: IVec3` (0 to 2^depth-1).
                    // User's `get_octant_offset_ivec` returns -1 or 1.
                    // Existing `CubeCoord::child` uses `(pos << 1) + offset` where offset is 0 or 1.

                    // Let's map octant index to 0..1 offset for `CubeCoord`
                    let child_coord_offset = IVec3::from_octant_index(octant_idx);
                    let next_coord = CubeCoord::new(
                        (current_coord.pos << 1) + child_coord_offset,
                        current_coord.depth + 1,
                    );

                    // Check if child exists (always true for Cube::Cubes array)
                    let child_node = &children[octant_idx];

                    // Pos2 calculation: Transform ray to child's local space.
                    // Local space is [-1, 1]. Child is a sub-cube.
                    // Formula: pos2 = (pos - child_center) * 2.0
                    let child_center = get_octant_center(octant_idx);
                    let pos2 = (local_pos - child_center) * 2.0;

                    // Recursion
                    let result = child_node.recursive_raycast(
                        pos2,
                        ray_dir,
                        next_coord,
                        max_depth,
                        is_empty,
                        debug.as_deref_mut(),
                    );

                    if result.is_some() {
                        return result;
                    }

                    // Miss Logic / Step Calculation
                    // We missed the child (or it was empty). We must advance to the neighbor.

                    // t = distance to plane (0) / dir
                    // We only care about positive t (moving forward)
                    let tx = if ray_dir.x != 0.0 {
                        -local_pos.x / ray_dir.x
                    } else {
                        f32::INFINITY
                    };
                    let ty = if ray_dir.y != 0.0 {
                        -local_pos.y / ray_dir.y
                    } else {
                        f32::INFINITY
                    };
                    let tz = if ray_dir.z != 0.0 {
                        -local_pos.z / ray_dir.z
                    } else {
                        f32::INFINITY
                    };

                    // Filter out negative t (behind us) by making them Infinity
                    let tx = if tx <= 0.0 { f32::INFINITY } else { tx };
                    let ty = if ty <= 0.0 { f32::INFINITY } else { ty };
                    let tz = if tz <= 0.0 { f32::INFINITY } else { tz };

                    // Find min t
                    let min_t = tx.min(ty).min(tz);

                    // If min_t is Infinity, we are stuck or direction is zero. Break.
                    if min_t == f32::INFINITY {
                        break;
                    }

                    // Perform the Step
                    // Update local position to the boundary intersection point
                    // (Add epsilon to ensure we cross the line)
                    let step_scale = min_t + 0.00001;
                    let next_pos = local_pos + ray_dir * step_scale;

                    // Determine which axis we stepped over to update the index
                    if tx <= ty && tx <= tz {
                        // Stepped X
                        if next_pos.x.abs() >= 1.0 {
                            return None;
                        } // Exited Parent
                        octant_idx ^= 1;
                    } else if ty <= tx && ty <= tz {
                        // Stepped Y
                        if next_pos.y.abs() >= 1.0 {
                            return None;
                        } // Exited Parent
                        octant_idx ^= 2;
                    } else {
                        // Stepped Z
                        if next_pos.z.abs() >= 1.0 {
                            return None;
                        } // Exited Parent
                        octant_idx ^= 4;
                    }

                    // Update local_pos for next iteration
                    // In the recursive call we used pos2, but here we update local_pos to the boundary
                    // and loop again. The next iteration will recalculate pos2 based on the new octant.
                    // NOTE: We must update `local_pos` to `next_pos` for the loop to progress correctly.
                    // However, modifying `local_pos` (argument) is fine since it's by value.
                    // BUT, `recursive_raycast` takes `local_pos` as argument.
                    // We need to update the variable used in the loop.
                    // Rust arguments are immutable by default, need to make it mutable or shadow it.
                    // But we are in a loop.
                    // Let's change the loop structure or use a mutable variable.

                    // We can't easily mutate `local_pos` because it's an argument.
                    // Let's use a mutable variable `current_pos` initialized to `local_pos`.
                    // Wait, I can't just change the argument in the loop header.
                    // I will refactor to use `current_pos`.

                    // Actually, the user's code had:
                    // `let _ = next_pos; // In a real loop we'd update local_pos = next_pos`
                    // and then `match recursive_raycast(next_pos, ...)`
                    // But here I am inside `recursive_raycast` (the method on Cube).
                    // I am implementing the loop *inside* `recursive_raycast`.
                    // So I should update `current_pos`.

                    // Refactoring loop:
                    // let mut current_pos = local_pos;
                    // loop { ... use current_pos ... current_pos = next_pos; }

                    // Wait, if I update `current_pos`, I need to be careful about accumulation of errors?
                    // The user's code mentions "updating float pos repeatedly causes drift".
                    // But for now I will follow the "update pos" sketch as requested.

                    // Let's restart the loop with `next_pos`.
                    // Since I can't easily restart the function call without recursion (which might blow stack if many steps),
                    // but here we only step max 3 times (visiting up to 4 octants) in a node?
                    // No, in a node we can visit up to 4 octants.
                    // So a loop is fine.

                    // I will use `current_pos` variable.
                    return self.recursive_raycast_loop(
                        local_pos,
                        ray_dir,
                        current_coord,
                        max_depth,
                        is_empty,
                        debug,
                    );
                }
                // If loop breaks without returning, no hit was found
                None
            }
            _ => None, // Other types not supported yet
        }
    }

    // Helper to handle the loop with mutable position
    fn recursive_raycast_loop<F>(
        &self,
        start_pos: Vec3,
        ray_dir: Vec3,
        parent_coord: CubeCoord,
        max_depth: u32,
        is_empty: &F,
        mut debug: Option<&mut RaycastDebugState>,
    ) -> Option<RaycastHit<T>>
    where
        F: Fn(&T) -> bool,
    {
        let mut current_pos = start_pos;
        let mut octant_idx = get_octant_index(current_pos);

        // We need to access children. This helper is only called for Cube::Cubes.
        let children = match self {
            Cube::Cubes(c) => c,
            _ => return None,
        };

        loop {
            // Construct the CubeCoord for the specific child we are looking at
            let child_coord_offset = IVec3::from_octant_index(octant_idx);
            let next_coord = CubeCoord::new(
                (parent_coord.pos << 1) + child_coord_offset,
                parent_coord.depth + 1,
            );

            let child_node = &children[octant_idx];
            let child_center = get_octant_center(octant_idx);
            let pos2 = (current_pos - child_center) * 2.0;

            let result = child_node.recursive_raycast(
                pos2,
                ray_dir,
                next_coord,
                max_depth,
                is_empty,
                debug.as_deref_mut(),
            );

            if result.is_some() {
                return result;
            }

            // Miss Logic
            let tx = if ray_dir.x != 0.0 {
                -current_pos.x / ray_dir.x
            } else {
                f32::INFINITY
            };
            let ty = if ray_dir.y != 0.0 {
                -current_pos.y / ray_dir.y
            } else {
                f32::INFINITY
            };
            let tz = if ray_dir.z != 0.0 {
                -current_pos.z / ray_dir.z
            } else {
                f32::INFINITY
            };

            let tx = if tx <= 0.0 { f32::INFINITY } else { tx };
            let ty = if ty <= 0.0 { f32::INFINITY } else { ty };
            let tz = if tz <= 0.0 { f32::INFINITY } else { tz };

            let min_t = tx.min(ty).min(tz);

            if min_t == f32::INFINITY {
                break;
            }

            let step_scale = min_t + 0.00001;
            let next_pos = current_pos + ray_dir * step_scale;

            if tx <= ty && tx <= tz {
                if next_pos.x.abs() >= 1.0 {
                    return None;
                }
                octant_idx ^= 1;
            } else if ty <= tx && ty <= tz {
                if next_pos.y.abs() >= 1.0 {
                    return None;
                }
                octant_idx ^= 2;
            } else {
                if next_pos.z.abs() >= 1.0 {
                    return None;
                }
                octant_idx ^= 4;
            }

            current_pos = next_pos;
        }
        None
    }
}

// --- Helpers ---

fn get_octant_index(pos: Vec3) -> usize {
    let mut idx = 0;
    if pos.x > 0.0 {
        idx |= 1;
    }
    if pos.y > 0.0 {
        idx |= 2;
    }
    if pos.z > 0.0 {
        idx |= 4;
    }
    idx
}

fn get_octant_center(idx: usize) -> Vec3 {
    let x = if (idx & 1) != 0 { 0.5 } else { -0.5 };
    let y = if (idx & 2) != 0 { 0.5 } else { -0.5 };
    let z = if (idx & 4) != 0 { 0.5 } else { -0.5 };
    vec3(x, y, z)
}

fn get_octant_offset_ivec(idx: usize) -> IVec3 {
    let x = if (idx & 1) != 0 { 1 } else { -1 };
    let y = if (idx & 2) != 0 { 1 } else { -1 };
    let z = if (idx & 4) != 0 { 1 } else { -1 };
    ivec3(x, y, z)
}

fn intersect_aabb_entry(origin: Vec3, dir: Vec3, box_min: Vec3, box_max: Vec3) -> f32 {
    let t1 = (box_min - origin) / dir;
    let t2 = (box_max - origin) / dir;
    let tmin = t1.min(t2).max_element();
    let tmax = t1.max(t2).min_element();

    if tmax >= tmin && tmax >= 0.0 {
        tmin
    } else {
        -1.0
    }
}

fn get_entry_normal(pos: Vec3, dir: Vec3) -> Axis {
    let x_dist = 1.0 - pos.x.abs();
    let y_dist = 1.0 - pos.y.abs();
    let z_dist = 1.0 - pos.z.abs();

    if x_dist < y_dist && x_dist < z_dist {
        if dir.x < 0.0 {
            Axis::PosX
        } else {
            Axis::NegX
        }
    } else if y_dist < z_dist {
        if dir.y < 0.0 {
            Axis::PosY
        } else {
            Axis::NegY
        }
    } else {
        if dir.z < 0.0 {
            Axis::PosZ
        } else {
            Axis::NegZ
        }
    }
}
