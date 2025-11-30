//! CPU-based BCF octree raytracer with GPU-compatible operations
//!
//! This module implements octree ray traversal with BCF (Binary Cube Format) data directly.
//! All operations are designed to map 1:1 to GLSL fragment shader code for GPU translation.
//!
//! # Design Principles
//!
//! - **Iterative traversal**: No recursion (stack-based instead)
//! - **Explicit operations**: All math and logic explicit (maps to GLSL)
//! - **Zero smart pointers**: No Rc/Box (use offsets/indices instead)
//! - **GPU-compatible types**: Vec3, u8, usize, f32 (map to GLSL types)
//! - **Documented algorithm**: Each step documented for GLSL translation
//!
//! # GLSL Translation Guide
//!
//! ## Type Mappings
//! - `Vec3` → `vec3`
//! - `f32` → `float`
//! - `u8` → `uint` (GLSL has no u8, use uint)
//! - `usize` → `uint`
//! - `[T; N]` → `T[N]` (arrays)
//! - `Option<T>` → separate bool + T (e.g., `bool has_hit; BcfHit hit;`)
//!
//! ## Operation Mappings
//! - `if let Some(x) = opt` → `if (has_x) { ... }`
//! - `vec.len()` → `uint len = ...;` (pass length explicitly)
//! - `reader.read_u8(offset)` → `uint read_u8(uint offset) { return octree_data[offset]; }`
//! - Bit operations (<<, >>, &, |) map directly
//!
//! ## Limitations
//! - GLSL has no Vec/String/heap allocation
//! - GLSL has limited stack (use fixed-size arrays for traversal stack)
//! - GLSL loops must have compile-time bounds (use `for (int i = 0; i < MAX_DEPTH; i++)`)

use crate::renderer::*;
use crate::scenes::create_octa_cube;
use cube::Cube;
use cube::io::bcf::{BcfNodeType, BcfReader, serialize_bcf};
use glam::{IVec3, Vec3};
use image::{ImageBuffer, Rgb};
use std::rc::Rc;

/// Maximum octree depth for traversal (prevents infinite loops)
const MAX_TRAVERSAL_DEPTH: usize = 16;

/// Get error material color (materials 1-7 are reserved for errors)
///
/// Error material colors (matching GL shader octree_raycast.frag):
/// - 1: Hot pink (generic error)
/// - 2: Red-orange (bounds/pointer errors)
/// - 3: Orange (type validation errors)
/// - 4: Sky blue (stack/iteration errors)
/// - 5: Purple (octant errors)
/// - 6: Spring green (data truncation)
/// - 7: Yellow (unknown/other errors)
///
/// With animation enabled, returns animated checkered pattern.
/// Without animation, returns solid error color.
#[inline]
fn get_error_material_color(value: u8, pos: Vec3, time: f32, animate: bool) -> Vec3 {
    let base_color = match value {
        1 => Vec3::new(1.0, 0.0, 0.3),   // Hot pink - Generic error
        2 => Vec3::new(1.0, 0.2, 0.0),   // Red-orange - Bounds/pointer errors
        3 => Vec3::new(1.0, 0.6, 0.0),   // Orange - Type validation errors
        4 => Vec3::new(0.0, 0.8, 1.0),   // Sky blue - Stack/iteration errors
        5 => Vec3::new(0.6, 0.0, 1.0),   // Purple - Octant errors
        6 => Vec3::new(0.0, 1.0, 0.498), // Spring green - Data truncation
        7 => Vec3::new(1.0, 1.0, 0.0),   // Yellow - Unknown/other errors
        _ => Vec3::new(1.0, 0.0, 1.0),   // Magenta - Invalid error material
    };

    if !animate {
        return base_color;
    }

    // Apply animated checkered pattern (matching GL shader)
    // 8x8 checker based on position
    let checker_size = 8.0;
    let checker_x = ((pos.x * checker_size).floor() as i32) & 1;
    let checker_y = ((pos.y * checker_size).floor() as i32) & 1;
    let checker_z = ((pos.z * checker_size).floor() as i32) & 1;
    let checker = ((checker_x + checker_y + checker_z) & 1) == 1;

    // Brightness oscillation (2 second period)
    let brightness = 0.5 + 0.5 * (time * std::f32::consts::PI).sin();

    // Apply checker and brightness
    if checker {
        base_color * brightness
    } else {
        base_color * (1.0 - brightness * 0.5)
    }
}

// ============================================================================
// DDA Traversal Helper Functions
// (Translated from crates/cube/src/core/raycast.rs for BCF compatibility)
// ============================================================================

/// Compute sign of vector components (-1 or +1)
///
/// GLSL equivalent:
/// ```glsl
/// vec3 sign_vec(vec3 v) {
///     return vec3(v.x < 0.0 ? -1.0 : 1.0,
///                 v.y < 0.0 ? -1.0 : 1.0,
///                 v.z < 0.0 ? -1.0 : 1.0);
/// }
/// ```
#[inline]
fn sign(v: Vec3) -> Vec3 {
    Vec3::select(v.cmplt(Vec3::ZERO), Vec3::NEG_ONE, Vec3::ONE)
}

/// Convert 3D octant coordinates to 1D array index (0-7)
///
/// Encoding: x + y*2 + z*4
///
/// GLSL equivalent:
/// ```glsl
/// int octant_to_index(ivec3 o) {
///     return o.x + o.y * 2 + o.z * 4;
/// }
/// ```
#[inline]
fn octant_to_index(o: IVec3) -> usize {
    (o.x + o.y * 2 + o.z * 4) as usize
}

/// Find axis with minimum time value (which face ray exits through)
///
/// Returns (axis_index, axis_sign) where:
/// - axis_index: 0=X, 1=Y, 2=Z
/// - axis_sign: -1 or +1
///
/// GLSL equivalent:
/// ```glsl
/// void min_time_axis(vec3 t, vec3 dir_sign, out int axis_index, out int axis_sign) {
///     if (t.x <= t.y && t.x <= t.z) {
///         axis_index = 0;
///         axis_sign = int(dir_sign.x);
///     } else if (t.y <= t.z) {
///         axis_index = 1;
///         axis_sign = int(dir_sign.y);
///     } else {
///         axis_index = 2;
///         axis_sign = int(dir_sign.z);
///     }
/// }
/// ```
#[inline]
fn min_time_axis(t: Vec3, dir_sign: Vec3) -> (usize, i32) {
    let i = if t.x <= t.y && t.x <= t.z {
        0
    } else if t.y <= t.z {
        1
    } else {
        2
    };
    (i, dir_sign[i] as i32)
}

/// Compute starting octant from position and ray direction
///
/// At boundaries (pos=0), use ray direction to determine octant.
///
/// GLSL equivalent:
/// ```glsl
/// ivec3 compute_octant(vec3 pos, vec3 dir_sign) {
///     bvec3 positive = greaterThan(pos, vec3(0.0)) ||
///                      (equal(pos, vec3(0.0)) && greaterThan(dir_sign, vec3(0.0)));
///     return ivec3(positive);
/// }
/// ```
#[inline]
fn compute_octant(pos: Vec3, dir_sign: Vec3) -> IVec3 {
    let positive = pos.cmpgt(Vec3::ZERO) | (pos.cmpeq(Vec3::ZERO) & dir_sign.cmpgt(Vec3::ZERO));
    Vec3::select(positive, Vec3::ONE, Vec3::ZERO).as_ivec3()
}

/// Axis-aligned bounding box
///
/// GLSL equivalent:
/// ```glsl
/// struct AABB {
///     vec3 min;
///     vec3 max;
/// };
/// ```
#[derive(Debug, Clone, Copy)]
pub struct AABB {
    pub min: Vec3,
    pub max: Vec3,
}

impl AABB {
    /// Create AABB from min/max corners
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    /// Get center point
    #[inline]
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Get size (max - min)
    #[inline]
    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }

    /// Get half-size
    #[inline]
    pub fn half_size(&self) -> Vec3 {
        self.size() * 0.5
    }
}

/// Ray for ray tracing
///
/// GLSL equivalent:
/// ```glsl
/// struct Ray {
///     vec3 origin;
///     vec3 direction; // Must be normalized
/// };
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

/// Ray-cube intersection result
///
/// GLSL equivalent:
/// ```glsl
/// struct BcfHit {
///     uint value;      // Material index
///     vec3 normal;     // Surface normal
///     vec3 pos;        // Hit position
///     float distance;  // Distance from ray origin
/// };
/// ```
#[derive(Debug, Clone, Copy)]
pub struct BcfHit {
    pub value: u8,
    pub normal: Vec3,
    pub pos: Vec3,
    pub distance: f32,
}

/// Traversal stack entry
///
/// GLSL equivalent:
/// ```glsl
/// struct TraversalState {
///     uint offset;     // Offset in BCF data
///     vec3 aabb_min;   // Bounding box min
///     vec3 aabb_max;   // Bounding box max
///     uint depth;      // Current depth
/// };
/// ```
#[derive(Debug, Clone, Copy)]
struct TraversalState {
    offset: usize,
    bounds: AABB,
    depth: u8,
}

/// Ray-AABB intersection using slab method
///
/// Returns (t_near, t_far) if ray intersects box, None otherwise.
///
/// GLSL equivalent:
/// ```glsl
/// bool ray_aabb_intersect(vec3 ray_origin, vec3 ray_dir, vec3 aabb_min, vec3 aabb_max, out float t_near, out float t_far) {
///     vec3 inv_dir = 1.0 / ray_dir;
///     vec3 t_min = (aabb_min - ray_origin) * inv_dir;
///     vec3 t_max = (aabb_max - ray_origin) * inv_dir;
///
///     vec3 t1 = min(t_min, t_max);
///     vec3 t2 = max(t_min, t_max);
///
///     t_near = max(max(t1.x, t1.y), t1.z);
///     t_far = min(min(t2.x, t2.y), t2.z);
///
///     return t_far >= t_near && t_far >= 0.0;
/// }
/// ```
#[inline]
fn ray_aabb_intersect(ray: &Ray, aabb: &AABB) -> Option<(f32, f32)> {
    // Compute inverse ray direction (handle division by zero)
    let inv_dir = Vec3::new(
        if ray.direction.x.abs() < 1e-8 {
            1e8
        } else {
            1.0 / ray.direction.x
        },
        if ray.direction.y.abs() < 1e-8 {
            1e8
        } else {
            1.0 / ray.direction.y
        },
        if ray.direction.z.abs() < 1e-8 {
            1e8
        } else {
            1.0 / ray.direction.z
        },
    );

    // Compute intersection t values for each slab
    let t_min = (aabb.min - ray.origin) * inv_dir;
    let t_max = (aabb.max - ray.origin) * inv_dir;

    // Ensure t_min < t_max for each axis
    let t1 = t_min.min(t_max);
    let t2 = t_min.max(t_max);

    // Find overall intersection interval
    let t_near = t1.x.max(t1.y).max(t1.z);
    let t_far = t2.x.min(t2.y).min(t2.z);

    // Check if ray intersects box
    if t_far >= t_near && t_far >= 0.0 {
        Some((t_near.max(0.0), t_far))
    } else {
        None
    }
}

/// Compute child AABB for given octant
///
/// GLSL equivalent:
/// ```glsl
/// void compute_child_bounds(vec3 parent_min, vec3 parent_max, uint octant, out vec3 child_min, out vec3 child_max) {
///     vec3 center = (parent_min + parent_max) * 0.5;
///     uint x = (octant >> 2u) & 1u;
///     uint y = (octant >> 1u) & 1u;
///     uint z = octant & 1u;
///
///     child_min = vec3(
///         (x == 0u) ? parent_min.x : center.x,
///         (y == 0u) ? parent_min.y : center.y,
///         (z == 0u) ? parent_min.z : center.z
///     );
///
///     child_max = vec3(
///         (x == 0u) ? center.x : parent_max.x,
///         (y == 0u) ? center.y : parent_max.y,
///         (z == 0u) ? center.z : parent_max.z
///     );
/// }
/// ```
#[inline]
fn compute_child_bounds(parent: &AABB, octant: usize) -> AABB {
    let center = parent.center();
    let x = (octant >> 2) & 1;
    let y = (octant >> 1) & 1;
    let z = octant & 1;

    let min = Vec3::new(
        if x == 0 { parent.min.x } else { center.x },
        if y == 0 { parent.min.y } else { center.y },
        if z == 0 { parent.min.z } else { center.z },
    );

    let max = Vec3::new(
        if x == 0 { center.x } else { parent.max.x },
        if y == 0 { center.y } else { parent.max.y },
        if z == 0 { center.z } else { parent.max.z },
    );

    AABB::new(min, max)
}

/// Compute surface normal from ray entry face
///
/// Determines which face of the AABB the ray hit first.
///
/// GLSL equivalent:
/// ```glsl
/// vec3 compute_normal(vec3 ray_origin, vec3 ray_dir, vec3 aabb_min, vec3 aabb_max, float t_near) {
///     vec3 hit_point = ray_origin + ray_dir * t_near;
///     vec3 center = (aabb_min + aabb_max) * 0.5;
///     vec3 local_hit = hit_point - center;
///     vec3 half_size = (aabb_max - aabb_min) * 0.5;
///
///     // Find which axis hit_point is closest to (with small epsilon for floating point)
///     vec3 abs_local = abs(local_hit / half_size);
///     float max_val = max(max(abs_local.x, abs_local.y), abs_local.z);
///
///     vec3 normal = vec3(0.0);
///     if (abs(abs_local.x - max_val) < 0.001) normal.x = sign(local_hit.x);
///     else if (abs(abs_local.y - max_val) < 0.001) normal.y = sign(local_hit.y);
///     else normal.z = sign(local_hit.z);
///
///     return normalize(normal);
/// }
/// ```
#[inline]
fn compute_normal(ray: &Ray, aabb: &AABB, t_near: f32) -> Vec3 {
    let hit_point = ray.origin + ray.direction * t_near;
    let center = aabb.center();
    let local_hit = hit_point - center;
    let half_size = aabb.half_size();

    // Normalize to [-1, 1] box
    let abs_local = (local_hit / half_size).abs();

    // Find which axis is closest to the surface
    let max_val = abs_local.x.max(abs_local.y).max(abs_local.z);

    // Determine normal direction (handle floating point precision)
    let epsilon = 0.001;
    if (abs_local.x - max_val).abs() < epsilon {
        Vec3::new(local_hit.x.signum(), 0.0, 0.0)
    } else if (abs_local.y - max_val).abs() < epsilon {
        Vec3::new(0.0, local_hit.y.signum(), 0.0)
    } else {
        Vec3::new(0.0, 0.0, local_hit.z.signum())
    }
}

/// Trace ray through BCF octree (iterative traversal with DDA octant stepping)
///
/// This is the core raytracing algorithm that will map to GLSL.
///
/// # Algorithm Overview
///
/// The traversal uses a **stack-based approach** with **DDA (Digital Differential Analyzer) octant stepping**.
/// This matches the algorithm from `crates/cube/src/core/raycast.rs` but works directly with BCF binary data.
///
/// **For each BCF node:**
/// 1. Transform ray to local [-1, 1]³ space
/// 2. Compute starting octant from ray entry point
/// 3. **DDA Loop:** Step through octants along ray path:
///    - **OctaLeaves**: Check if current octant has solid voxel (value > 0), return if hit
///    - **OctaPointers**: Collect ALL non-empty children in DDA order, push to stack in reverse
///    - Compute exit axis (which face ray exits through)
///    - Advance ray to octant boundary
///    - Step to next octant
///    - Repeat until hit or exit parent node
///
/// **Key difference from recursive version:**
/// - Original algorithm (raycast.rs): Recursively calls child.raycast() and returns immediately on hit
/// - BCF version: Collects all non-empty children the ray passes through in DDA order,
///   then pushes them to stack in reverse order (so they're processed closest-first)
/// - This ensures all voxels along the ray path are checked, matching recursive behavior
///
/// # Error Materials
///
/// When errors occur during traversal, the function returns a hit with special error materials (1-7):
/// - Material 1 (red): BCF read error - invalid offset or corrupted data
/// - Material 7 (magenta): Stack overflow - traversal depth exceeded MAX_TRAVERSAL_DEPTH
///
/// These error materials help debug BCF data corruption or traversal issues.
///
/// GLSL equivalent will be a large function with fixed-size stack array.
/// ```glsl
/// bool trace_ray(vec3 ray_origin, vec3 ray_dir, out BcfHit hit) {
///     // Fixed-size traversal stack (no dynamic allocation in GLSL)
///     TraversalState stack[MAX_TRAVERSAL_DEPTH];
///     int stack_ptr = 0;
///
///     // Initialize with root node
///     stack[0] = TraversalState(...);
///     stack_ptr = 1;
///
///     while (stack_ptr > 0) {
///         stack_ptr--;
///         TraversalState state = stack[stack_ptr];
///
///         // Check ray-AABB intersection
///         float t_near, t_far;
///         if (!ray_aabb_intersect(ray_origin, ray_dir, state.aabb_min, state.aabb_max, t_near, t_far)) {
///             continue; // Miss, try next node from stack
///         }
///
///         // Read node type at offset
///         uint type_byte = read_u8(state.offset);
///         // ... decode node type and handle accordingly
///     }
///
///     return false; // No hit found
/// }
/// ```
fn trace_ray(bcf_data: &[u8], ray: &Ray, root_offset: usize) -> Option<BcfHit> {
    let reader = BcfReader::new(bcf_data);

    // Traversal stack (fixed size for GPU compatibility)
    let mut stack: [TraversalState; MAX_TRAVERSAL_DEPTH] = [TraversalState {
        offset: 0,
        bounds: AABB::new(Vec3::ZERO, Vec3::ZERO),
        depth: 0,
    }; MAX_TRAVERSAL_DEPTH];

    // Initialize with root node (bounds: [-1, 1]³)
    stack[0] = TraversalState {
        offset: root_offset,
        bounds: AABB::new(Vec3::splat(-1.0), Vec3::splat(1.0)),
        depth: 0,
    };
    let mut stack_ptr = 1;

    // Iterative traversal loop
    while stack_ptr > 0 {
        // Pop from stack
        stack_ptr -= 1;
        let state = stack[stack_ptr];

        // Check depth limit
        if state.depth as usize >= MAX_TRAVERSAL_DEPTH {
            continue;
        }

        // Check ray-AABB intersection
        let Some((t_near, _t_far)) = ray_aabb_intersect(ray, &state.bounds) else {
            continue; // Miss, try next node
        };

        // Read node type at current offset
        let node_type = match reader.read_node_at(state.offset) {
            Ok(node) => node,
            Err(_) => {
                // Error reading BCF node - return error material (red: material 1)
                let hit_pos = ray.origin + ray.direction * t_near;
                let normal = compute_normal(ray, &state.bounds, t_near);
                return Some(BcfHit {
                    value: 1, // Error material 1 (red)
                    normal,
                    pos: hit_pos,
                    distance: t_near,
                });
            }
        };

        // Handle different node types
        match node_type {
            BcfNodeType::InlineLeaf(value) | BcfNodeType::ExtendedLeaf(value) => {
                // Leaf node: check if non-zero (solid)
                if value > 0 {
                    // Hit!
                    let hit_pos = ray.origin + ray.direction * t_near;
                    let normal = compute_normal(ray, &state.bounds, t_near);

                    return Some(BcfHit {
                        value,
                        normal,
                        pos: hit_pos,
                        distance: t_near,
                    });
                }
                // value == 0 means empty, continue
            }
            BcfNodeType::OctaLeaves(values) => {
                // Octa with 8 leaf values - use DDA traversal through octants
                // Transform ray to local [-1, 1]³ space of this node
                let center = state.bounds.center();
                let half_size = state.bounds.half_size();
                let mut local_origin = (ray.origin + ray.direction * t_near - center) / half_size;

                let dir_sign = sign(ray.direction);
                let mut octant = compute_octant(local_origin, dir_sign);

                // DDA loop through octants
                loop {
                    let oct_idx = octant_to_index(octant);
                    let value = values[oct_idx];

                    if value > 0 {
                        // Hit solid voxel in this octant
                        let child_bounds = compute_child_bounds(&state.bounds, oct_idx);
                        let (child_t_near, _) =
                            ray_aabb_intersect(ray, &child_bounds).unwrap_or((t_near, t_near));
                        let hit_pos = ray.origin + ray.direction * child_t_near;
                        let normal = compute_normal(ray, &child_bounds, child_t_near);

                        return Some(BcfHit {
                            value,
                            normal,
                            pos: hit_pos,
                            distance: child_t_near,
                        });
                    }

                    // Compute exit distance for current octant
                    let far_side = (local_origin * dir_sign).cmpge(Vec3::ZERO);
                    let adjusted = Vec3::select(far_side, local_origin - dir_sign, local_origin);
                    let dist = adjusted.abs();
                    let time = dist / ray.direction.abs();

                    // Find exit axis
                    let (exit_idx, exit_sign) = min_time_axis(time, dir_sign);

                    // Advance ray to exit point
                    local_origin += ray.direction / half_size * time[exit_idx];

                    // Step to next octant
                    octant[exit_idx] += exit_sign;

                    // Snap to boundary
                    let boundary = octant[exit_idx] as f32 - (exit_sign + 1) as f32 * 0.5;
                    local_origin[exit_idx] = boundary;

                    // Check if exited parent node
                    if octant[exit_idx] < 0 || octant[exit_idx] > 1 {
                        break; // Exit octant loop, continue with next stack item
                    }
                }
            }
            BcfNodeType::OctaPointers { pointers, .. } => {
                // Octa with pointers to children - use DDA traversal through octants
                // Transform ray to local [-1, 1]³ space of this node
                let center = state.bounds.center();
                let half_size = state.bounds.half_size();
                let mut local_origin = (ray.origin + ray.direction * t_near - center) / half_size;

                let dir_sign = sign(ray.direction);
                let mut octant = compute_octant(local_origin, dir_sign);

                // Collect all non-empty children in DDA order
                // We need to check ALL octants the ray passes through, not just the first
                let mut children_to_visit = Vec::new();

                // DDA loop through octants to collect non-empty children
                loop {
                    let oct_idx = octant_to_index(octant);
                    let child_offset = pointers[oct_idx];

                    if child_offset > 0 {
                        // Non-empty child - record it with its bounds and entry distance
                        let child_bounds = compute_child_bounds(&state.bounds, oct_idx);
                        let (child_t_near, _) =
                            ray_aabb_intersect(ray, &child_bounds).unwrap_or((t_near, t_near));

                        children_to_visit.push((child_offset, child_bounds, child_t_near));
                    }

                    // Compute exit distance for current octant
                    let far_side = (local_origin * dir_sign).cmpge(Vec3::ZERO);
                    let adjusted = Vec3::select(far_side, local_origin - dir_sign, local_origin);
                    let dist = adjusted.abs();
                    let time = dist / ray.direction.abs();

                    // Find exit axis
                    let (exit_idx, exit_sign) = min_time_axis(time, dir_sign);

                    // Advance ray to exit point
                    local_origin += ray.direction / half_size * time[exit_idx];

                    // Step to next octant
                    octant[exit_idx] += exit_sign;

                    // Snap to boundary
                    let boundary = octant[exit_idx] as f32 - (exit_sign + 1) as f32 * 0.5;
                    local_origin[exit_idx] = boundary;

                    // Check if exited parent node
                    if octant[exit_idx] < 0 || octant[exit_idx] > 1 {
                        break; // Exit octant loop
                    }
                }

                // Push all collected children to stack in REVERSE order
                // (so they pop in DDA order: closest first)
                for (child_offset, child_bounds, _child_t_near) in children_to_visit.iter().rev() {
                    if stack_ptr < MAX_TRAVERSAL_DEPTH {
                        stack[stack_ptr] = TraversalState {
                            offset: *child_offset,
                            bounds: *child_bounds,
                            depth: state.depth + 1,
                        };
                        stack_ptr += 1;
                    } else {
                        // Stack overflow
                        let hit_pos = ray.origin + ray.direction * t_near;
                        let normal = compute_normal(ray, &state.bounds, t_near);
                        return Some(BcfHit {
                            value: 7, // Error material 7 (yellow - stack overflow)
                            normal,
                            pos: hit_pos,
                            distance: t_near,
                        });
                    }
                }
            }
        }
    }

    None // No hit found
}

/// CPU raytracer using BCF format
///
/// This tracer reads BCF binary data directly and performs iterative octree traversal.
/// All operations are designed to map 1:1 to GLSL for GPU translation.
pub struct BcfCpuTracer {
    bcf_data: Vec<u8>,
    root_offset: usize,
    image_buffer: Option<ImageBuffer<Rgb<u8>, Vec<u8>>>,
    disable_lighting: bool,
}

impl BcfCpuTracer {
    /// Create tracer from cube by serializing to BCF
    pub fn new_from_cube(cube: Rc<Cube<u8>>) -> Self {
        let bcf_data = serialize_bcf(&cube);

        // Parse header to get root offset
        let reader = BcfReader::new(&bcf_data);
        let header = reader.read_header().expect("Failed to read BCF header");

        Self {
            bcf_data,
            root_offset: header.root_offset,
            image_buffer: None,
            disable_lighting: false,
        }
    }

    /// Create tracer with default octa cube scene
    pub fn new() -> Self {
        let cube = create_octa_cube();
        Self::new_from_cube(cube)
    }

    /// Set whether to disable lighting (output pure material colors)
    pub fn set_disable_lighting(&mut self, disable: bool) {
        self.disable_lighting = disable;
    }

    /// Get the current lighting disable state
    pub fn is_lighting_disabled(&self) -> bool {
        self.disable_lighting
    }

    /// Get a reference to the image buffer
    pub fn image_buffer(&self) -> Option<&ImageBuffer<Rgb<u8>, Vec<u8>>> {
        self.image_buffer.as_ref()
    }

    /// Save the rendered image to a file
    pub fn save_image(&self, path: &str) -> Result<(), image::ImageError> {
        if let Some(buffer) = &self.image_buffer {
            buffer.save(path)?;
        }
        Ok(())
    }

    /// Render a single pixel with time-based camera
    fn render_pixel(&self, x: u32, y: u32, width: u32, height: u32, time: f32) -> Vec3 {
        // Normalized pixel coordinates (flip Y to match GL coordinate system)
        let uv = glam::Vec2::new(
            (x as f32 - 0.5 * width as f32) / height as f32,
            -((y as f32 - 0.5 * height as f32) / height as f32),
        );

        // Camera setup (same as existing CPU tracer)
        let camera_pos = Vec3::new(3.0 * (time * 0.3).cos(), 2.0, 3.0 * (time * 0.3).sin());
        let target = Vec3::ZERO;
        let up = Vec3::Y;

        // Create ray
        let ray_data = create_camera_ray(uv, camera_pos, target, up);
        let ray = Ray {
            origin: ray_data.origin,
            direction: ray_data.direction.normalize(),
        };

        self.render_ray(&ray, time)
    }

    /// Render a single pixel with explicit camera configuration
    fn render_pixel_with_camera(
        &self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        camera: &CameraConfig,
    ) -> Vec3 {
        // Normalized pixel coordinates (flip Y to match GL coordinate system)
        let uv = glam::Vec2::new(
            (x as f32 - 0.5 * width as f32) / height as f32,
            -((y as f32 - 0.5 * height as f32) / height as f32),
        );

        // Create ray from camera
        let ray_data = create_camera_ray(uv, camera.position, camera.target(), camera.up());
        let ray = Ray {
            origin: ray_data.origin,
            direction: ray_data.direction.normalize(),
        };

        // Use time=0.0 for static camera (no animation)
        self.render_ray(&ray, 0.0)
    }

    /// Render a ray and return the color
    fn render_ray(&self, ray: &Ray, time: f32) -> Vec3 {
        let mut color = BACKGROUND_COLOR;

        // Trace ray through BCF octree
        if let Some(hit) = trace_ray(&self.bcf_data, ray, self.root_offset) {
            // Check if this is an error material (1-7)
            let is_error_material = hit.value >= 1 && hit.value <= 7;

            let material_color = if is_error_material {
                // Error materials: use special error colors with animation
                get_error_material_color(hit.value, hit.pos, time, true)
            } else {
                // Normal materials: use material registry
                cube::material::get_material_color(hit.value as i32)
            };

            // Apply lighting or output pure color
            // Error materials skip lighting (always emissive)
            color = if self.disable_lighting || is_error_material {
                material_color
            } else {
                let hit_info = HitInfo {
                    hit: true,
                    t: hit.distance,
                    point: hit.pos,
                    normal: hit.normal,
                };
                calculate_lighting(&hit_info, material_color)
            };
        }

        // Gamma correction
        color.powf(1.0 / 2.2)
    }
}

impl Default for BcfCpuTracer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer for BcfCpuTracer {
    fn render(&mut self, width: u32, height: u32, time: f32) {
        // Create image buffer
        let buffer = ImageBuffer::from_fn(width, height, |x, y| {
            let color = self.render_pixel(x, y, width, height, time);

            // Convert to RGB8
            let r = (color.x.clamp(0.0, 1.0) * 255.0) as u8;
            let g = (color.y.clamp(0.0, 1.0) * 255.0) as u8;
            let b = (color.z.clamp(0.0, 1.0) * 255.0) as u8;

            Rgb([r, g, b])
        });

        self.image_buffer = Some(buffer);
    }

    fn render_with_camera(&mut self, width: u32, height: u32, camera: &CameraConfig) {
        // Create image buffer
        let buffer = ImageBuffer::from_fn(width, height, |x, y| {
            let color = self.render_pixel_with_camera(x, y, width, height, camera);

            // Convert to RGB8
            let r = (color.x.clamp(0.0, 1.0) * 255.0) as u8;
            let g = (color.y.clamp(0.0, 1.0) * 255.0) as u8;
            let b = (color.z.clamp(0.0, 1.0) * 255.0) as u8;

            Rgb([r, g, b])
        });

        self.image_buffer = Some(buffer);
    }

    fn name(&self) -> &str {
        "BCF CPU Tracer"
    }
}
