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
use glam::Vec3;
use image::{ImageBuffer, Rgb};
use std::rc::Rc;

/// Maximum octree depth for traversal (prevents infinite loops)
const MAX_TRAVERSAL_DEPTH: usize = 16;

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

/// Select octant (0-7) based on position relative to center
///
/// Octant encoding: x*4 + y*2 + z
/// - x: 0 = left (negative), 1 = right (positive)
/// - y: 0 = bottom, 1 = top
/// - z: 0 = back, 1 = front
///
/// GLSL equivalent:
/// ```glsl
/// uint select_octant(vec3 pos, vec3 center) {
///     uint x = (pos.x >= center.x) ? 1u : 0u;
///     uint y = (pos.y >= center.y) ? 1u : 0u;
///     uint z = (pos.z >= center.z) ? 1u : 0u;
///     return (x << 2u) | (y << 1u) | z;
/// }
/// ```
#[inline]
fn select_octant(pos: Vec3, center: Vec3) -> usize {
    let x = if pos.x >= center.x { 1 } else { 0 };
    let y = if pos.y >= center.y { 1 } else { 0 };
    let z = if pos.z >= center.z { 1 } else { 0 };
    (x << 2) | (y << 1) | z
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

/// Trace ray through BCF octree (iterative traversal)
///
/// This is the core raytracing algorithm that will map to GLSL.
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
            Err(_) => continue, // Error reading node, skip
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
                // Octa with 8 leaf values
                // Determine which octant the ray enters first
                let entry_point = ray.origin + ray.direction * (t_near + 1e-6);
                let octant = select_octant(entry_point, state.bounds.center());

                let value = values[octant];
                if value > 0 {
                    // Hit in selected octant
                    let child_bounds = compute_child_bounds(&state.bounds, octant);
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

                // TODO: For complete traversal, we should check all 8 octants in order
                // For now, we only check the entry octant (good enough for simple cases)
            }
            BcfNodeType::OctaPointers { pointers, .. } => {
                // Octa with pointers to children
                // Determine which octant the ray enters first
                let entry_point = ray.origin + ray.direction * (t_near + 1e-6);
                let octant = select_octant(entry_point, state.bounds.center());

                let child_offset = pointers[octant];
                if child_offset > 0 {
                    // Push child to stack for traversal
                    let child_bounds = compute_child_bounds(&state.bounds, octant);

                    if stack_ptr < MAX_TRAVERSAL_DEPTH {
                        stack[stack_ptr] = TraversalState {
                            offset: child_offset,
                            bounds: child_bounds,
                            depth: state.depth + 1,
                        };
                        stack_ptr += 1;
                    }
                }

                // TODO: For complete traversal, we should check all 8 octants in order
                // For now, we only check the entry octant (good enough for simple cases)
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

        self.render_ray(&ray)
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

        self.render_ray(&ray)
    }

    /// Render a ray and return the color
    fn render_ray(&self, ray: &Ray) -> Vec3 {
        let mut color = BACKGROUND_COLOR;

        // Trace ray through BCF octree
        if let Some(hit) = trace_ray(&self.bcf_data, ray, self.root_offset) {
            // Get material color
            let material_color = cube::material::get_material_color(hit.value as i32);

            // Apply lighting or output pure color
            color = if self.disable_lighting {
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
