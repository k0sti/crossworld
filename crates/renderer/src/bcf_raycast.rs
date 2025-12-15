//! BCF raycast implementation matching cube::raycast API
//!
//! This module provides raycast functions for BCF (Binary Cube Format) data
//! that exactly match the signature and behavior of cube::raycast.
//!
//! Key differences from old bcf_cpu_tracer:
//! - No AABB tracking - works in normalized [-1,1]³ space
//! - Uses cube::Hit<u8> and cube::CubeCoord types
//! - Same signature as cube::raycast
//! - Iterative with stack (GPU-compatible) instead of recursive
//!
//! Algorithm mirrors crates/cube/src/core/raycast.rs exactly.

use cube::CubeCoord;
use cube::axis::Axis;
use cube::core::raycast::Hit;
use cube::io::bcf::{BcfNodeType, BcfReader};
use glam::{IVec3, Vec3};

// ============================================================================
// Helper Functions (matching cube::raycast)
// ============================================================================

#[inline]
fn sign(v: Vec3) -> Vec3 {
    Vec3::select(v.cmplt(Vec3::ZERO), Vec3::NEG_ONE, Vec3::ONE)
}

#[inline]
fn octant_to_index(o: IVec3) -> usize {
    (o.x + o.y * 2 + o.z * 4) as usize
}

/// Find axis with minimum time value, using dir_sign for Axis direction
#[inline]
fn min_time_axis(t: Vec3, dir_sign: Vec3) -> Axis {
    let i = if t.x <= t.y && t.x <= t.z {
        0
    } else if t.y <= t.z {
        1
    } else {
        2
    };
    Axis::from_index_sign(i, dir_sign[i] as i32)
}

/// Compute starting octant; at boundary (pos=0), use ray direction
#[inline]
fn compute_octant(pos: Vec3, dir_sign: Vec3) -> IVec3 {
    let positive = pos.cmpgt(Vec3::ZERO) | (pos.cmpeq(Vec3::ZERO) & dir_sign.cmpgt(Vec3::ZERO));
    Vec3::select(positive, Vec3::ONE, Vec3::ZERO).as_ivec3()
}

// ============================================================================
// Traversal State (stack-based for GPU compatibility)
// ============================================================================

const MAX_TRAVERSAL_DEPTH: usize = 16;

/// Traversal state for iterative BCF raycast
///
/// Unlike old implementation, this does NOT track AABB bounds.
/// Works in normalized [-1,1]³ space at each level.
#[derive(Debug, Clone, Copy)]
struct TraversalState {
    /// Offset in BCF data
    offset: usize,
    /// Ray origin in this node's [-1,1]³ local space
    local_origin: Vec3,
    /// Ray direction (same at all levels)
    ray_dir: Vec3,
    /// Entry normal from parent
    normal: Axis,
    /// Coordinate in octree space
    coord: CubeCoord,
}

// ============================================================================
// BCF Raycast (matching cube::raycast signature)
// ============================================================================

/// Raycast through BCF octree data
///
/// This function matches cube::raycast exactly:
/// - Same parameters: ray_origin, ray_dir (no Ray struct)
/// - Same return type: Option<Hit<u8>>
/// - Same algorithm: DDA traversal in [-1,1]³ space
///
/// # Parameters
/// - `bcf_data`: BCF binary data
/// - `ray_origin`: Ray origin in [-1,1]³ space
/// - `ray_dir`: Ray direction (normalized or not)
///
/// # Returns
/// - `Some(Hit<u8>)` if ray hits a non-empty voxel
/// - `None` if ray misses or exits cube
pub fn bcf_raycast(bcf_data: &[u8], ray_origin: Vec3, ray_dir: Vec3) -> Option<Hit<u8>> {
    if ray_dir == Vec3::ZERO {
        return None;
    }

    let reader = BcfReader::new(bcf_data);

    // Read header
    let header = reader.read_header().ok()?;

    // Find entry point if outside [-1,1]³ (same as cube::raycast)
    let dir_sign = sign(ray_dir);
    let ray_origin = if ray_origin.abs().max_element() > 1.0 {
        let t_entry = (-dir_sign - ray_origin) / ray_dir;
        let t_exit = (dir_sign - ray_origin) / ray_dir;

        let t_enter = t_entry.max_element();
        let t_leave = t_exit.min_element();

        if t_enter > t_leave || t_leave < 0.0 {
            return None;
        }

        ray_origin + ray_dir * t_enter.max(0.0)
    } else {
        ray_origin
    };

    // Check for axis-aligned ray (optimization path)
    let abs_dir = ray_dir.abs();
    let max_comp = abs_dir.max_element();
    let near_zero = abs_dir.cmple(Vec3::splat(max_comp * 1e-6));

    if near_zero.bitmask().count_ones() == 2 {
        // Axis-aligned: use specialized function
        let i = if abs_dir.x >= abs_dir.y && abs_dir.x >= abs_dir.z {
            0
        } else if abs_dir.y >= abs_dir.z {
            1
        } else {
            2
        };
        let axis = Axis::from_index_sign(i, dir_sign[i] as i32);
        return bcf_raycast_axis(bcf_data, ray_origin, axis);
    }

    // Calculate entry normal (same logic as cube::raycast)
    // This determines which face the ray entered from
    let entry_normal;

    // Check if we are on a boundary (within epsilon)
    let abs_origin = ray_origin.abs();
    let max_origin_comp = abs_origin.max_element();

    if (max_origin_comp - 1.0).abs() < 1e-5 {
        // On boundary: determine which face we are on
        if (abs_origin.x - 1.0).abs() < 1e-5 {
            entry_normal = Axis::from_index_sign(0, ray_origin.x.signum() as i32);
        } else if (abs_origin.y - 1.0).abs() < 1e-5 {
            entry_normal = Axis::from_index_sign(1, ray_origin.y.signum() as i32);
        } else {
            entry_normal = Axis::from_index_sign(2, ray_origin.z.signum() as i32);
        }
    } else {
        // Inside: pick axis most opposed to ray direction
        // This is a heuristic for "which face would we have entered from"
        let i = if abs_dir.x >= abs_dir.y && abs_dir.x >= abs_dir.z {
            0
        } else if abs_dir.y >= abs_dir.z {
            1
        } else {
            2
        };
        // Use negative direction sign (opposing face)
        entry_normal = Axis::from_index_sign(i, -dir_sign[i] as i32);
    }

    // General raycast with DDA traversal
    bcf_raycast_impl(
        &reader,
        header.root_offset,
        ray_origin,
        ray_dir,
        entry_normal,
    )
}

/// Axis-aligned raycast (optimized path)
///
/// Matches cube::raycast_axis signature and behavior.
pub fn bcf_raycast_axis(bcf_data: &[u8], ray_origin: Vec3, ray_axis: Axis) -> Option<Hit<u8>> {
    let reader = BcfReader::new(bcf_data);
    let header = reader.read_header().ok()?;

    let coord = CubeCoord {
        pos: IVec3::ZERO,
        depth: 0,
    };

    bcf_raycast_axis_impl(&reader, header.root_offset, ray_origin, ray_axis, coord)
}

// ============================================================================
// Implementation (iterative with stack)
// ============================================================================

/// Iterative BCF raycast implementation
///
/// Converts the recursive cube::raycast algorithm to iterative with explicit stack.
/// This is GPU-compatible (fixed-size stack, no recursion).
///
/// # Parameters
/// - `reader`: BCF data reader
/// - `root_offset`: Offset of root node in BCF data
/// - `ray_origin`: Ray origin (already adjusted to entry point)
/// - `ray_dir`: Ray direction
/// - `entry_normal`: Normal of the face the ray entered from
fn bcf_raycast_impl(
    reader: &BcfReader,
    root_offset: usize,
    ray_origin: Vec3,
    ray_dir: Vec3,
    entry_normal: Axis,
) -> Option<Hit<u8>> {
    let mut stack: [TraversalState; MAX_TRAVERSAL_DEPTH] = [TraversalState {
        offset: 0,
        local_origin: Vec3::ZERO,
        ray_dir: Vec3::ZERO,
        normal: Axis::PosX,
        coord: CubeCoord {
            pos: IVec3::ZERO,
            depth: 0,
        },
    }; MAX_TRAVERSAL_DEPTH];

    // Initialize with root node
    stack[0] = TraversalState {
        offset: root_offset,
        local_origin: ray_origin,
        ray_dir,
        normal: entry_normal,
        coord: CubeCoord {
            pos: IVec3::ZERO,
            depth: 0,
        },
    };
    let mut stack_ptr = 1;

    // Iterative traversal loop
    while stack_ptr > 0 {
        // Pop from stack
        stack_ptr -= 1;
        let state = stack[stack_ptr];

        // Read node type at current offset
        let node = match reader.read_node_at(state.offset) {
            Ok(n) => n,
            Err(_) => return None, // BCF read error
        };

        match node {
            BcfNodeType::InlineLeaf(value) | BcfNodeType::ExtendedLeaf(value) => {
                // Leaf node - check if non-empty
                if value != 0 {
                    return Some(Hit {
                        coord: state.coord,
                        value,
                        normal: state.normal,
                        pos: state.local_origin,
                    });
                }
                // Empty leaf - continue to next stack item
            }

            BcfNodeType::OctaLeaves(values) => {
                // Octa with 8 leaf values - traverse octants with DDA
                let dir_sign = sign(state.ray_dir);
                let mut octant = compute_octant(state.local_origin, dir_sign);
                let mut ray_origin = state.local_origin;
                let mut normal = state.normal;

                loop {
                    let oct_idx = octant_to_index(octant);
                    let value = values[oct_idx];

                    if value != 0 {
                        // Hit non-empty voxel
                        return Some(Hit {
                            coord: CubeCoord {
                                pos: state.coord.pos * 2 + octant,
                                depth: state.coord.depth + 1,
                            },
                            value,
                            normal,
                            pos: ray_origin,
                        });
                    }

                    // DDA step to next octant (same as cube::raycast)
                    let far_side = (ray_origin * dir_sign).cmpge(Vec3::ZERO);
                    let adjusted = Vec3::select(far_side, ray_origin - dir_sign, ray_origin);
                    let dist = adjusted.abs();
                    let time = dist / state.ray_dir.abs();

                    let exit_axis = min_time_axis(time, dir_sign);
                    let i = exit_axis.index();

                    // Advance ray
                    ray_origin += state.ray_dir * time[i];

                    // Step octant
                    octant = exit_axis.step(octant);

                    // Snap to boundary
                    let boundary = octant[i] as f32 - (exit_axis.sign() + 1) as f32 * 0.5;
                    ray_origin = exit_axis.set(ray_origin, boundary);

                    // Entry normal is opposite of exit direction
                    normal = exit_axis.flip();

                    // Exit parent node?
                    if octant[i] < 0 || octant[i] > 1 {
                        break; // Exit to next stack item
                    }
                }
            }

            BcfNodeType::OctaPointers { pointers, .. } => {
                // Octa with pointers to children - DDA traversal + collect children
                let dir_sign = sign(state.ray_dir);
                let mut octant = compute_octant(state.local_origin, dir_sign);
                let mut ray_origin = state.local_origin;
                let mut normal = state.normal;

                // Collect all non-empty children in DDA order
                let mut children_to_visit: [(usize, Vec3, Axis, CubeCoord); 8] = [(
                    0,
                    Vec3::ZERO,
                    Axis::PosX,
                    CubeCoord {
                        pos: IVec3::ZERO,
                        depth: 0,
                    },
                );
                    8];
                let mut children_count = 0;

                loop {
                    let oct_idx = octant_to_index(octant);
                    let child_offset = pointers[oct_idx];

                    if child_offset > 0 {
                        // Non-empty child - transform ray to child's [-1,1]³ space
                        let offset = octant.as_vec3() * 2.0 - 1.0;
                        let child_origin = ray_origin * 2.0 - offset;

                        let child_coord = CubeCoord {
                            pos: state.coord.pos * 2 + octant,
                            depth: state.coord.depth + 1,
                        };

                        // Record child for later processing
                        children_to_visit[children_count] =
                            (child_offset, child_origin, normal, child_coord);
                        children_count += 1;
                    }

                    // DDA step to next octant
                    let far_side = (ray_origin * dir_sign).cmpge(Vec3::ZERO);
                    let adjusted = Vec3::select(far_side, ray_origin - dir_sign, ray_origin);
                    let dist = adjusted.abs();
                    let time = dist / state.ray_dir.abs();

                    let exit_axis = min_time_axis(time, dir_sign);
                    let i = exit_axis.index();

                    // Advance ray
                    ray_origin += state.ray_dir * time[i];

                    // Step octant
                    octant = exit_axis.step(octant);

                    // Snap to boundary
                    let boundary = octant[i] as f32 - (exit_axis.sign() + 1) as f32 * 0.5;
                    ray_origin = exit_axis.set(ray_origin, boundary);

                    // Entry normal is opposite of exit direction
                    normal = exit_axis.flip();

                    // Exit parent node?
                    if octant[i] < 0 || octant[i] > 1 {
                        break; // Exit DDA loop
                    }
                }

                // Push all collected children to stack in REVERSE order
                // (so they pop in DDA order: closest first)
                for i in (0..children_count).rev() {
                    if stack_ptr < MAX_TRAVERSAL_DEPTH {
                        let (child_offset, child_origin, child_normal, child_coord) =
                            children_to_visit[i];
                        stack[stack_ptr] = TraversalState {
                            offset: child_offset,
                            local_origin: child_origin,
                            ray_dir: state.ray_dir,
                            normal: child_normal,
                            coord: child_coord,
                        };
                        stack_ptr += 1;
                    } else {
                        // Stack overflow - return error material
                        return Some(Hit {
                            coord: state.coord,
                            value: 7, // Error material 7
                            normal,
                            pos: state.local_origin,
                        });
                    }
                }
            }
        }
    }

    None // No hit found
}

/// Axis-aligned raycast implementation (optimized)
///
/// For axis-aligned rays, we can skip many calculations.
/// This matches cube::raycast_axis behavior.
fn bcf_raycast_axis_impl(
    reader: &BcfReader,
    offset: usize,
    ray_origin: Vec3,
    ray_axis: Axis,
    coord: CubeCoord,
) -> Option<Hit<u8>> {
    let node = reader.read_node_at(offset).ok()?;

    match node {
        BcfNodeType::InlineLeaf(value) | BcfNodeType::ExtendedLeaf(value) => {
            if value != 0 {
                Some(Hit {
                    coord,
                    value,
                    normal: ray_axis.flip(),
                    pos: ray_origin,
                })
            } else {
                None
            }
        }

        BcfNodeType::OctaLeaves(values) => {
            // Axis-aligned traversal through octants
            let i = ray_axis.index();
            let dir = ray_axis.sign();

            let mut octant = compute_octant(ray_origin, ray_axis.as_vec3());
            let mut pos = ray_origin;
            let mut normal = ray_axis.flip();

            loop {
                let oct_idx = octant_to_index(octant);
                let value = values[oct_idx];

                if value != 0 {
                    return Some(Hit {
                        coord: CubeCoord {
                            pos: coord.pos * 2 + octant,
                            depth: coord.depth + 1,
                        },
                        value,
                        normal,
                        pos,
                    });
                }

                // Step along axis
                let boundary = if dir > 0 { 1.0 } else { -1.0 };
                pos = ray_axis.set(pos, boundary);

                octant[i] += dir;
                normal = ray_axis.flip();

                if octant[i] < 0 || octant[i] > 1 {
                    break;
                }
            }

            None
        }

        BcfNodeType::OctaPointers { pointers, .. } => {
            // Axis-aligned traversal through children
            let i = ray_axis.index();
            let dir = ray_axis.sign();

            let mut octant = compute_octant(ray_origin, ray_axis.as_vec3());
            let mut pos = ray_origin;

            loop {
                let oct_idx = octant_to_index(octant);
                let child_offset = pointers[oct_idx];

                if child_offset > 0 {
                    // Transform to child space
                    let offset = octant.as_vec3() * 2.0 - 1.0;
                    let child_origin = pos * 2.0 - offset;

                    let child_coord = CubeCoord {
                        pos: coord.pos * 2 + octant,
                        depth: coord.depth + 1,
                    };

                    // Recurse into child
                    let hit = bcf_raycast_axis_impl(
                        reader,
                        child_offset,
                        child_origin,
                        ray_axis,
                        child_coord,
                    );
                    if hit.is_some() {
                        return hit;
                    }
                }

                // Step along axis
                let boundary = if dir > 0 { 1.0 } else { -1.0 };
                pos = ray_axis.set(pos, boundary);

                octant[i] += dir;

                if octant[i] < 0 || octant[i] > 1 {
                    break;
                }
            }

            None
        }
    }
}
