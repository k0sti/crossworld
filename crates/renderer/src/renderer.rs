//! Renderer trait and helper types for voxel rendering

use glam::Vec3;

// Re-export for use within this module and for backward compatibility
pub use crate::camera::Camera;
pub use crate::lighting::{AMBIENT, BACKGROUND_COLOR, DIFFUSE_STRENGTH, LIGHT_DIR};

// Re-export Object trait from physics crate
pub use crossworld_physics::Object;

/// Common renderer trait for cube raytracers
///
/// This trait provides a unified interface for all renderer implementations,
/// supporting both software renderers (CPU-based) and GL-based renderers.
///
/// # Capability Queries
///
/// Use `supports_gl()` and `supports_image_output()` to check renderer capabilities.
///
/// # Software Renderers
///
/// Software renderers (CpuTracer, BcfTracer) render to an internal image buffer:
/// - Call `render()` or `render_with_camera()` to render a frame
/// - Access the image buffer with `image_buffer()`
/// - Save to file with `save_to_file()`
///
/// # GL-Based Renderers
///
/// GL renderers (GlTracer, ComputeTracer, MeshRenderer) require an OpenGL context:
/// - Call `init_gl(&Context)` to initialize GL resources
/// - Call `render_to_framebuffer(&Context, ...)` to render to the bound framebuffer
/// - Call `save_framebuffer_to_file(&Context, ...)` to save framebuffer to file
/// - Call `destroy_gl(&Context)` to clean up GL resources
pub trait Renderer {
    /// Render a single frame at the given time
    ///
    /// For software renderers, this renders to an internal image buffer.
    /// For GL renderers, this may panic - use `render_to_framebuffer()` instead.
    fn render(&mut self, width: u32, height: u32, time: f32);

    /// Render with explicit camera configuration
    ///
    /// For software renderers, this renders to an internal image buffer.
    /// For GL renderers, this may panic - use `render_to_framebuffer()` instead.
    fn render_with_camera(&mut self, width: u32, height: u32, camera: &Camera);

    /// Get the name of the renderer
    fn name(&self) -> &str;

    // Capability queries

    /// Returns true if this renderer supports GL rendering
    ///
    /// If true, the renderer requires an OpenGL context and implements
    /// `init_gl()`, `destroy_gl()`, and `render_to_framebuffer()`.
    fn supports_gl(&self) -> bool {
        false
    }

    /// Returns true if this renderer outputs to an image buffer
    ///
    /// If true, the renderer provides an image buffer accessible via `image_buffer()`.
    fn supports_image_output(&self) -> bool {
        false
    }

    // GL lifecycle methods

    /// Initialize GL resources (shaders, buffers, textures)
    ///
    /// Must be called with an active GL context before rendering.
    /// Returns an error if initialization fails or if the renderer doesn't support GL.
    ///
    /// # Safety
    ///
    /// Requires a valid GL context to be current on the calling thread.
    fn init_gl(&mut self, gl: &glow::Context) -> Result<(), String> {
        let _ = gl;
        Err(format!("{} does not support GL rendering", self.name()))
    }

    /// Clean up GL resources (delete shaders, buffers, textures)
    ///
    /// Should be called before the GL context is destroyed.
    /// Safe to call multiple times (idempotent).
    ///
    /// # Safety
    ///
    /// Requires a valid GL context to be current on the calling thread.
    fn destroy_gl(&mut self, gl: &glow::Context) {
        let _ = gl;
        // No-op for renderers without GL support
    }

    /// Render to the currently bound framebuffer
    ///
    /// Renders using the provided camera configuration or time parameter.
    /// The framebuffer must be bound before calling this method.
    ///
    /// # Arguments
    ///
    /// * `gl` - OpenGL context
    /// * `width` - Framebuffer width in pixels
    /// * `height` - Framebuffer height in pixels
    /// * `camera` - Optional camera configuration (overrides time-based camera)
    /// * `time` - Optional time parameter for animated camera (ignored if camera is provided)
    ///
    /// # Safety
    ///
    /// Requires a valid GL context to be current on the calling thread.
    fn render_to_framebuffer(
        &mut self,
        gl: &glow::Context,
        width: u32,
        height: u32,
        camera: Option<&Camera>,
        time: Option<f32>,
    ) -> Result<(), String> {
        let _ = (gl, width, height, camera, time);
        Err(format!(
            "{} does not support framebuffer rendering",
            self.name()
        ))
    }

    // Image buffer access

    /// Get the internal image buffer (for software renderers)
    ///
    /// Returns `Some` if the renderer outputs to an image buffer, `None` otherwise.
    /// Use `supports_image_output()` to check availability.
    fn image_buffer(&self) -> Option<&image::ImageBuffer<image::Rgb<u8>, Vec<u8>>> {
        None
    }

    // File output

    /// Save rendered image to file (for software renderers)
    ///
    /// Saves the internal image buffer to the specified file path.
    /// Supports common image formats (PNG, JPEG, etc.) based on file extension.
    ///
    /// Returns an error if the renderer doesn't have an image buffer or if saving fails.
    fn save_to_file(&self, path: &str) -> Result<(), String> {
        if let Some(buffer) = self.image_buffer() {
            buffer.save(path).map_err(|e| e.to_string())
        } else {
            Err(format!("{} does not have an image buffer", self.name()))
        }
    }

    /// Save framebuffer to file (for GL renderers)
    ///
    /// Reads pixels from the currently bound framebuffer and saves to the specified file path.
    /// Handles color space conversion and coordinate system flipping (GL origin is bottom-left,
    /// image origin is top-left).
    ///
    /// # Arguments
    ///
    /// * `gl` - OpenGL context
    /// * `width` - Framebuffer width in pixels
    /// * `height` - Framebuffer height in pixels
    /// * `path` - Output file path (format determined by extension)
    ///
    /// # Safety
    ///
    /// Requires a valid GL context to be current on the calling thread.
    fn save_framebuffer_to_file(
        &self,
        gl: &glow::Context,
        width: u32,
        height: u32,
        path: &str,
    ) -> Result<(), String> {
        let _ = (gl, width, height, path);
        Err(format!(
            "{} does not support framebuffer readback",
            self.name()
        ))
    }
}

/// Cube bounds for raytracing
#[derive(Debug, Clone, Copy)]
pub struct CubeBounds {
    pub min: glam::Vec3,
    pub max: glam::Vec3,
}

impl Default for CubeBounds {
    fn default() -> Self {
        Self {
            min: glam::Vec3::new(-1.0, -1.0, -1.0),
            max: glam::Vec3::new(1.0, 1.0, 1.0),
        }
    }
}

/// Ray for raytracing
#[derive(Debug, Clone, Copy)]
pub struct Ray {
    pub origin: glam::Vec3,
    pub direction: glam::Vec3,
}

/// Hit information from ray-cube intersection
#[derive(Debug, Clone, Copy)]
pub struct HitInfo {
    pub hit: bool,
    pub t: f32,
    pub point: glam::Vec3,
    pub normal: glam::Vec3,
}

impl Default for HitInfo {
    fn default() -> Self {
        Self {
            hit: false,
            t: f32::MAX,
            point: glam::Vec3::ZERO,
            normal: glam::Vec3::ZERO,
        }
    }
}

/// Ray-box intersection algorithm
pub fn intersect_box(ray: Ray, box_min: glam::Vec3, box_max: glam::Vec3) -> HitInfo {
    let mut hit_info = HitInfo::default();

    let inv_dir = 1.0 / ray.direction;
    let t_min = (box_min - ray.origin) * inv_dir;
    let t_max = (box_max - ray.origin) * inv_dir;

    let t1 = t_min.min(t_max);
    let t2 = t_min.max(t_max);

    let t_near = t1.x.max(t1.y).max(t1.z);
    let t_far = t2.x.min(t2.y).min(t2.z);

    if t_near > t_far || t_far < 0.0 {
        return hit_info;
    }

    hit_info.hit = true;
    hit_info.t = if t_near > 0.0 { t_near } else { t_far };
    hit_info.point = ray.origin + ray.direction * hit_info.t;

    // Calculate normal
    let center = (box_min + box_max) * 0.5;
    let local_point = hit_info.point - center;
    let size = (box_max - box_min) * 0.5;
    let d = (local_point / size).abs();

    let max_component = d.x.max(d.y).max(d.z);
    if (max_component - d.x).abs() < 0.0001 {
        hit_info.normal = glam::Vec3::new(local_point.x.signum(), 0.0, 0.0);
    } else if (max_component - d.y).abs() < 0.0001 {
        hit_info.normal = glam::Vec3::new(0.0, local_point.y.signum(), 0.0);
    } else {
        hit_info.normal = glam::Vec3::new(0.0, 0.0, local_point.z.signum());
    }

    hit_info
}

/// Create a camera ray for a given pixel coordinate
pub fn create_camera_ray(
    uv: glam::Vec2,
    camera_pos: glam::Vec3,
    target: glam::Vec3,
    up: glam::Vec3,
) -> Ray {
    let forward = (target - camera_pos).normalize();
    let right = forward.cross(up).normalize();
    let cam_up = right.cross(forward);

    Ray {
        origin: camera_pos,
        direction: (forward + uv.x * right + uv.y * cam_up).normalize(),
    }
}

/// Calculate lighting for a hit point with material color
///
/// Applies standardized lighting model: `materialColor * (AMBIENT + diffuse * DIFFUSE_STRENGTH)`
///
/// # Arguments
///
/// * `hit` - Hit information including position and normal
/// * `material_color` - Base material color from palette
///
/// # Returns
///
/// Final lit color (before gamma correction)
pub fn calculate_lighting(hit: &HitInfo, material_color: Vec3) -> Vec3 {
    // Diffuse lighting using Lambert's cosine law
    let diffuse = hit.normal.dot(LIGHT_DIR).max(0.0);

    // Combine lighting: material color * (ambient + diffuse)
    material_color * (AMBIENT + diffuse * DIFFUSE_STRENGTH)
}

/// Calculate unlit material color (for debug mode)
///
/// Returns the pure material color without any lighting applied.
///
/// # Arguments
///
/// * `material_color` - Base material color from palette
///
/// # Returns
///
/// Unmodified material color
pub fn calculate_lighting_unlit(material_color: Vec3) -> Vec3 {
    material_color
}
