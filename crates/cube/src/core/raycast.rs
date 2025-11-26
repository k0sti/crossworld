use crate::axis::Axis;
use crate::core::cube::Cube;
use crate::CubeCoord;
use glam::{IVec3, Vec3};

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
// Hit and Debug types
// ============================================================================

#[derive(Clone, Debug)]
pub struct Hit<T> {
    pub coord: CubeCoord,
    pub value: T,
    pub normal: Axis,
    pub pos: Vec3,
}

#[derive(Default)]
pub struct RaycastDebugState {
    pub entry_count: u32,
    pub max_entries: u32,
    pub path: Vec<CubeCoord>,
}

// ============================================================================
// Raycast implementation
// ============================================================================

impl<T: Copy + Default + PartialEq> Cube<T> {
    /// Raycast through octree
    pub fn raycast(
        &self,
        mut ray_origin: Vec3,
        ray_dir: Vec3,
        mut normal: Axis,
        coord: CubeCoord,
        mut debug: Option<&mut RaycastDebugState>,
    ) -> Option<Hit<T>> {
        if let Some(ref mut d) = debug {
            d.entry_count += 1;
            if d.max_entries > 0 && d.entry_count > d.max_entries {
                return None;
            }
        }

        match self {
            Cube::Solid(value) => {
                if let Some(d) = debug {
                    d.path.push(coord);
                }
                if *value != T::default() {
                    Some(Hit {
                        coord,
                        value: *value,
                        normal,
                        pos: ray_origin,
                    })
                } else {
                    None
                }
            }

            Cube::Cubes(children) => {
                let dir_sign = sign(ray_dir);
                let mut octant = compute_octant(ray_origin, dir_sign);

                loop {
                    let child = &children[octant_to_index(octant)];

                    // Transform to child's [-1,1]³ space
                    let offset = octant.as_vec3() * 2.0 - 1.0;
                    let child_origin = ray_origin * 2.0 - offset;

                    let child_coord = CubeCoord {
                        pos: coord.pos * 2 + octant,
                        depth: coord.depth + 1,
                    };

                    let hit = child.raycast(
                        child_origin,
                        ray_dir,
                        normal,
                        child_coord,
                        debug.as_deref_mut(),
                    );
                    if hit.is_some() {
                        return hit;
                    }

                    // Unified exit distance calculation
                    let far_side = (ray_origin * dir_sign).cmpge(Vec3::ZERO);
                    let adjusted = Vec3::select(far_side, ray_origin - dir_sign, ray_origin);
                    let dist = adjusted.abs();
                    let time = dist / ray_dir.abs();

                    // Find exit axis and step
                    let exit_axis = min_time_axis(time, dir_sign);
                    let i = exit_axis.index();

                    // Advance ray
                    ray_origin += ray_dir * time[i];

                    // Step octant
                    octant = exit_axis.step(octant);

                    // Snap to boundary: -1, 0, or 1
                    // boundary = new_octant - (sign + 1) / 2
                    let boundary = octant[i] as f32 - (exit_axis.sign() + 1) as f32 * 0.5;
                    ray_origin = exit_axis.set(ray_origin, boundary);

                    // Entry normal is opposite of exit direction
                    normal = exit_axis.flip();

                    // Exit parent cube?
                    if octant[i] < 0 || octant[i] > 1 {
                        return None;
                    }
                }
            }

            // Handle other variants as solid
            _ => {
                if let Some(d) = debug {
                    d.path.push(coord);
                }
                None
            }
        }
    }

    /// Optimized raycast for axis-aligned rays
    pub fn raycast_axis(
        &self,
        ray_origin: Vec3,
        ray_axis: Axis,
        coord: CubeCoord,
        mut debug: Option<&mut RaycastDebugState>,
    ) -> Option<Hit<T>> {
        if let Some(ref mut d) = debug {
            d.entry_count += 1;
            if d.max_entries > 0 && d.entry_count > d.max_entries {
                return None;
            }
        }

        match self {
            Cube::Solid(value) => {
                if let Some(d) = debug {
                    d.path.push(coord);
                }
                if *value != T::default() {
                    Some(Hit {
                        coord,
                        value: *value,
                        normal: ray_axis.flip(),
                        pos: ray_origin,
                    })
                } else {
                    None
                }
            }

            Cube::Cubes(children) => {
                let dir_sign = ray_axis.to_vec3();
                let mut octant = compute_octant(ray_origin, dir_sign);
                let i = ray_axis.index();

                // Loop until we exit the cube
                loop {
                    let child = &children[octant_to_index(octant)];

                    let offset = octant.as_vec3() * 2.0 - 1.0;
                    let child_origin = ray_origin * 2.0 - offset;

                    let child_coord = CubeCoord {
                        pos: coord.pos * 2 + octant,
                        depth: coord.depth + 1,
                    };

                    let hit = child.raycast_axis(
                        child_origin,
                        ray_axis,
                        child_coord,
                        debug.as_deref_mut(),
                    );
                    if hit.is_some() {
                        return hit;
                    }

                    // Step to next octant
                    octant = ray_axis.step(octant);

                    // Check if we exited
                    if octant[i] < 0 || octant[i] > 1 {
                        return None;
                    }
                }
            }

            // Handle other variants as solid
            _ => {
                if let Some(d) = debug {
                    d.path.push(coord);
                }
                None
            }
        }
    }
}

// ============================================================================
// Entry point
// ============================================================================

/// Raycast through octree from origin in direction
pub fn raycast<T: Copy + Default + PartialEq>(
    root: &Cube<T>,
    ray_origin: Vec3,
    ray_dir: Vec3,
    debug: Option<&mut RaycastDebugState>,
) -> Option<Hit<T>> {
    if ray_dir == Vec3::ZERO {
        return None;
    }

    let dir_sign = sign(ray_dir);

    // Find entry point if outside [-1,1]³
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

    // Check for axis-aligned ray
    let abs_dir = ray_dir.abs();
    let max_comp = abs_dir.max_element();
    let near_zero = abs_dir.cmple(Vec3::splat(max_comp * 1e-6));

    if near_zero.bitmask().count_ones() == 2 {
        // Axis-aligned: find the non-zero axis
        let i = if abs_dir.x >= abs_dir.y && abs_dir.x >= abs_dir.z {
            0
        } else if abs_dir.y >= abs_dir.z {
            1
        } else {
            2
        };
        let axis = Axis::from_index_sign(i, dir_sign[i] as i32);

        root.raycast_axis(
            ray_origin,
            axis,
            CubeCoord {
                pos: IVec3::ZERO,
                depth: 0,
            },
            debug,
        )
    } else {
        // Default entry normal
        let entry_axis;

        // Check if we are on a boundary (within epsilon)
        let abs_origin = ray_origin.abs();
        let max_comp = abs_origin.max_element();

        if (max_comp - 1.0).abs() < 1e-5 {
            // On boundary: determine which face we are on
            if (abs_origin.x - 1.0).abs() < 1e-5 {
                entry_axis = Axis::from_index_sign(0, ray_origin.x.signum() as i32);
            } else if (abs_origin.y - 1.0).abs() < 1e-5 {
                entry_axis = Axis::from_index_sign(1, ray_origin.y.signum() as i32);
            } else {
                entry_axis = Axis::from_index_sign(2, ray_origin.z.signum() as i32);
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
            entry_axis = Axis::from_index_sign(i, -dir_sign[i] as i32);
        }

        root.raycast(
            ray_origin,
            ray_dir,
            entry_axis,
            CubeCoord {
                pos: IVec3::ZERO,
                depth: 0,
            },
            debug,
        )
    }
}
