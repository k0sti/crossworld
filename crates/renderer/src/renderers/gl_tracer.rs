//! WebGL 2.0 octree raytracer using fragment shaders
//!
//! This tracer uses OpenGL ES 3.0 (WebGL 2.0) fragment shaders to render octree voxel data.
//! The octree is serialized to Binary Cube Format (BCF) and uploaded to GPU as a 1D-like 2D texture,
//! then traversed using hierarchical DDA in the fragment shader.
//!
//! # BCF Serialization Approach
//!
//! The GL renderer uses Binary Cube Format (BCF) to represent the octree on the GPU:
//! - **Compact representation**: BCF is 10-20x smaller than voxel grid sampling
//! - **Preserves structure**: Maintains exact octree hierarchy (no loss of detail)
//! - **GPU-friendly**: Simple byte buffer with bit operations for node parsing
//! - **Center-based coordinates**: Uses octree's native [-1,1]³ coordinate system
//!
//! # GPU Upload Strategy
//!
//! BCF data is uploaded as a 1D-like 2D texture (width=data_size, height=1):
//! - **Format**: R8UI (8-bit unsigned integer, single channel)
//! - **Sampling**: NEAREST filtering, CLAMP_TO_EDGE wrapping
//! - **Access**: `texelFetch(u_octree_data, ivec2(offset, 0), 0).r` in shader
//!
//! # Shader Traversal
//!
//! The fragment shader implements stack-based octree traversal:
//! 1. Parse BCF node type byte (inline leaf, extended leaf, octa-leaves, octa-pointers)
//! 2. Calculate child octant from ray-box intersection
//! 3. Follow pointer chain through BCF buffer
//! 4. Return material value when hitting solid leaf
//!
//! See `shaders/octree_raycast.frag` for full implementation details.

use crate::renderer::*;
use crate::shader_utils;
use cube::Cube;
use cube::io::bcf::serialize_bcf;
use glam::IVec3;
use glow::*;
use std::rc::Rc;

// Shader sources - include from files at compile time
const VERTEX_SHADER_SOURCE: &str = include_str!("../shaders/octree_raycast.vert");
const FRAGMENT_SHADER_SOURCE: &str = include_str!("../shaders/octree_raycast.frag");

/// WebGL 2.0 fragment shader raytracer with octree support
pub struct GlTracer {
    cube: Rc<Cube<u8>>,
    bounds: CubeBounds,
    // GL resources (Option for cases where GL context isn't available)
    gl_program: Option<GlTracerGl>,
    /// If true, disable lighting and output pure material colors
    disable_lighting: bool,
    /// If true, show error colors for debugging
    show_errors: bool,
}

/// GPU-specific OpenGL resources
pub struct GlTracerGl {
    program: Program,
    vao: VertexArray,
    octree_texture: Option<Texture>, // 2D texture for BCF data
    octree_data_size: u32,
    octree_texture_width: u32, // Width of the 2D texture (for coordinate conversion)
    // Uniform locations
    resolution_location: Option<UniformLocation>,
    time_location: Option<UniformLocation>,
    camera_pos_location: Option<UniformLocation>,
    camera_rot_location: Option<UniformLocation>,
    tan_half_vfov_location: Option<UniformLocation>,
    use_camera_location: Option<UniformLocation>,
    max_depth_location: Option<UniformLocation>,
    octree_data_location: Option<UniformLocation>,
    octree_data_size_location: Option<UniformLocation>,
    octree_texture_width_location: Option<UniformLocation>,
    octree_size_location: Option<UniformLocation>,
    material_palette_location: Option<UniformLocation>,
    material_palette_texture: Option<Texture>,
    disable_lighting_location: Option<UniformLocation>,
    show_errors_location: Option<UniformLocation>,
    near_location: Option<UniformLocation>,
    far_location: Option<UniformLocation>,
}

/// Raycast hit result for cube intersection
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct RaycastHit {
    pub hit: bool,
    pub t: f32,
    pub point: glam::Vec3,
    pub normal: glam::Vec3,
    pub voxel_pos: IVec3,
    pub voxel_value: i32,
}

impl Default for RaycastHit {
    fn default() -> Self {
        Self {
            hit: false,
            t: f32::MAX,
            point: glam::Vec3::ZERO,
            normal: glam::Vec3::ZERO,
            voxel_pos: IVec3::ZERO,
            voxel_value: 0,
        }
    }
}

impl From<HitInfo> for RaycastHit {
    fn from(hit_info: HitInfo) -> Self {
        Self {
            hit: hit_info.hit,
            t: hit_info.t,
            point: hit_info.point,
            normal: hit_info.normal,
            voxel_pos: IVec3::ZERO,
            voxel_value: 0,
        }
    }
}

impl RaycastHit {
    #[allow(dead_code)]
    pub fn with_voxel(mut self, pos: IVec3, value: i32) -> Self {
        self.voxel_pos = pos;
        self.voxel_value = value;
        self
    }
}

impl GlTracer {
    pub fn new(cube: Rc<Cube<u8>>) -> Self {
        Self {
            cube,
            bounds: CubeBounds::default(),
            gl_program: None,
            disable_lighting: false,
            show_errors: false,
        }
    }

    /// Initialize OpenGL resources for GPU raytracing
    ///
    /// # Safety
    /// Must be called with an active GL context. Caller must ensure the GL context
    /// remains valid for the lifetime of this object.
    pub unsafe fn init_gl(&mut self, gl: &Context) -> Result<(), String> {
        unsafe {
            let gl_program = GlTracerGl::new(gl, &self.cube)?;
            self.gl_program = Some(gl_program);
            Ok(())
        }
    }

    /// Get reference to the cube
    pub fn cube(&self) -> &Rc<Cube<u8>> {
        &self.cube
    }

    /// Set whether to disable lighting (output pure material colors)
    ///
    /// When disabled, renders pure material palette colors without any lighting calculations.
    /// Useful for debugging material system and color verification tests.
    pub fn set_disable_lighting(&mut self, disable: bool) {
        self.disable_lighting = disable;
    }

    /// Set whether to show error colors for debugging
    ///
    /// When enabled, displays different colors for different error types:
    /// - Bright red: Bounds exceeded
    /// - Dark red: Invalid pointer
    /// - Orange: Invalid type ID
    /// - Yellow: Truncated data
    /// - Blue: Stack overflow
    /// - Cyan: Iteration timeout
    /// - Magenta: Invalid octant
    /// - Purple: Pointer cycle
    pub fn set_show_errors(&mut self, show_errors: bool) {
        self.show_errors = show_errors;
    }

    /// Get the current lighting disable state
    pub fn is_lighting_disabled(&self) -> bool {
        self.disable_lighting
    }

    /// Raycast against the cube's bounding box (simple box intersection)
    /// Returns RaycastHit with intersection information
    pub fn raycast(&self, pos: glam::Vec3, dir: glam::Vec3) -> RaycastHit {
        let ray = Ray {
            origin: pos,
            direction: dir.normalize(),
        };

        let hit_info = intersect_box(ray, self.bounds.min, self.bounds.max);

        RaycastHit::from(hit_info)
    }

    /// Raycast against the octree structure (CPU-side)
    /// This uses the cube's octree traversal algorithm for accurate voxel intersection
    /// Returns Result<Option<RaycastHit>, RaycastError>
    pub fn raycast_octree(
        &self,
        pos: glam::Vec3,
        dir: glam::Vec3,
        _max_depth: u32,
    ) -> Result<Option<cube::Hit<u8>>, String> {
        Ok(cube::raycast(&self.cube, pos, dir, None))
    }

    /// Render to OpenGL context
    ///
    /// # Safety
    /// Must be called with an active GL context. GL resources must have been initialized via `init_gl()`.
    pub unsafe fn render_to_gl(&self, gl: &Context, width: i32, height: i32, time: f32) {
        unsafe {
            if let Some(gl_program) = &self.gl_program {
                gl_program.render_to_gl(gl, width, height, time, self.disable_lighting);
            }
        }
    }

    /// Render to OpenGL context with explicit camera
    ///
    /// # Safety
    /// Must be called with an active GL context. GL resources must have been initialized via `init_gl()`.
    pub unsafe fn render_to_gl_with_camera(
        &self,
        gl: &Context,
        width: i32,
        height: i32,
        camera: &Camera,
    ) {
        unsafe {
            if let Some(gl_program) = &self.gl_program {
                gl_program.render_to_gl_with_camera(
                    gl,
                    width,
                    height,
                    camera,
                    self.disable_lighting,
                    self.show_errors,
                );
            }
        }
    }

    /// Clean up GL resources
    ///
    /// # Safety
    /// Must be called with an active GL context. Should only be called once at shutdown.
    pub unsafe fn destroy_gl(&mut self, gl: &Context) {
        unsafe {
            if let Some(gl_program) = self.gl_program.take() {
                gl_program.destroy(gl);
            }
        }
    }
}

impl GlTracerGl {
    /// Create a new GL tracer with BCF serialization
    ///
    /// # Safety
    /// Must be called with an active GL context. GL context must remain valid for the lifetime of this object.
    pub unsafe fn new(gl: &Context, cube: &Cube<u8>) -> Result<Self, String> {
        unsafe {
            // Create shader program using shared utilities
            println!("[GL Tracer] Compiling vertex and fragment shaders...");
            let program =
                shader_utils::create_program(gl, VERTEX_SHADER_SOURCE, FRAGMENT_SHADER_SOURCE)?;
            println!("[GL Tracer] ✓ Shaders compiled and linked successfully!");

            // Create VAO (required for OpenGL core profile)
            let vao = gl
                .create_vertex_array()
                .map_err(|e| format!("Failed to create VAO: {}", e))?;

            // Serialize cube to BCF format
            // BCF (Binary Cube Format) is a compact binary representation of the octree:
            // - Header (12 bytes): magic, version, depth, root offset
            // - Nodes: Type byte + payload (inline leaf, extended leaf, octa-leaves, octa-pointers)
            // - Preserves exact octree structure with center-based coordinates
            println!("[GL Tracer] Serializing octree to BCF format...");
            let bcf_data = serialize_bcf(cube);
            println!("[GL Tracer] BCF data serialized: {} bytes", bcf_data.len());

            // Detect SSBO support (OpenGL 4.3+ or ES 3.1+)
            // For now, we'll use texture buffer as it's more widely supported
            let _use_ssbo = false; // TODO: Detect SSBO support properly

            // Create 2D texture for BCF data
            // We use a proper 2D texture because:
            // - WebGL 2.0 doesn't support TEXTURE_BUFFER
            // - 1D textures have width limits (typically 16384)
            // - For large data, we need to use width x height layout
            // - R8UI format provides direct byte access via texelFetch

            // Calculate texture dimensions to fit within GPU limits
            // Max texture size is typically 16384, we use 8192 to be safe
            const MAX_TEX_WIDTH: i32 = 8192;
            let data_len = bcf_data.len() as i32;
            let (tex_width, tex_height) = if data_len <= MAX_TEX_WIDTH {
                (data_len, 1)
            } else {
                // Calculate dimensions for 2D layout
                let height = (data_len + MAX_TEX_WIDTH - 1) / MAX_TEX_WIDTH;
                (MAX_TEX_WIDTH, height)
            };

            // Pad data to fill the texture completely
            let padded_size = (tex_width * tex_height) as usize;
            let mut padded_data = bcf_data.clone();
            padded_data.resize(padded_size, 0);

            println!(
                "[GL Tracer] Using {}x{} texture for {} bytes of octree data",
                tex_width, tex_height, data_len
            );

            let texture = gl
                .create_texture()
                .map_err(|e| format!("Failed to create texture: {}", e))?;

            gl.bind_texture(TEXTURE_2D, Some(texture));

            gl.tex_image_2d(
                TEXTURE_2D,
                0,                  // mip level
                R8UI as i32,        // internal format
                tex_width,          // width
                tex_height,         // height
                0,                  // border
                RED_INTEGER,        // format
                UNSIGNED_BYTE,      // type
                glow::PixelUnpackData::Slice(Some(&padded_data)), // data
            );

            // Set texture parameters
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MIN_FILTER, NEAREST as i32);
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MAG_FILTER, NEAREST as i32);
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_S, CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_T, CLAMP_TO_EDGE as i32);

            gl.bind_texture(TEXTURE_2D, None);

            println!("[GL Tracer] Octree buffer uploaded to GPU");

            // Get uniform locations
            let resolution_location = gl.get_uniform_location(program, "u_resolution");
            let time_location = gl.get_uniform_location(program, "u_time");
            let camera_pos_location = gl.get_uniform_location(program, "u_camera_pos");
            let camera_rot_location = gl.get_uniform_location(program, "u_camera_rot");
            let tan_half_vfov_location = gl.get_uniform_location(program, "u_tan_half_vfov");
            let use_camera_location = gl.get_uniform_location(program, "u_use_camera");
            let max_depth_location = gl.get_uniform_location(program, "u_max_depth");
            let octree_data_location = gl.get_uniform_location(program, "u_octree_data");
            let octree_data_size_location = gl.get_uniform_location(program, "u_octree_data_size");
            let octree_texture_width_location =
                gl.get_uniform_location(program, "u_octree_texture_width");
            let octree_size_location = gl.get_uniform_location(program, "u_octree_size");
            let material_palette_location = gl.get_uniform_location(program, "u_material_palette");
            let disable_lighting_location = gl.get_uniform_location(program, "u_disable_lighting");
            let show_errors_location = gl.get_uniform_location(program, "u_show_errors");
            let near_location = gl.get_uniform_location(program, "u_near");
            let far_location = gl.get_uniform_location(program, "u_far");

            // Debug: Print uniform locations
            println!("[GL Tracer] Uniform locations:");
            println!("  u_octree_data: {:?}", octree_data_location);
            println!("  u_octree_data_size: {:?}", octree_data_size_location);
            println!("  u_max_depth: {:?}", max_depth_location);
            println!("  u_octree_size: {:?}", octree_size_location);

            // Create and upload material palette texture
            println!("[GL Tracer] Creating material palette texture...");
            let material_palette_texture = Some(Self::create_material_palette_texture(gl)?);
            println!("[GL Tracer] Material palette texture created successfully!");

            Ok(Self {
                program,
                vao,
                octree_texture: Some(texture),
                octree_data_size: bcf_data.len() as u32,
                octree_texture_width: tex_width as u32,
                resolution_location,
                time_location,
                camera_pos_location,
                camera_rot_location,
                tan_half_vfov_location,
                use_camera_location,
                max_depth_location,
                octree_data_location,
                octree_data_size_location,
                octree_texture_width_location,
                octree_size_location,
                material_palette_location,
                material_palette_texture,
                disable_lighting_location,
                show_errors_location,
                near_location,
                far_location,
            })
        }
    }

    /// Create a 1D texture for the material palette
    unsafe fn create_material_palette_texture(gl: &Context) -> Result<Texture, String> {
        unsafe {
            let texture = gl
                .create_texture()
                .map_err(|e| format!("Failed to create material palette texture: {}", e))?;

            gl.bind_texture(TEXTURE_2D, Some(texture));

            // Create data buffer from MATERIAL_REGISTRY
            // 128 materials * 3 floats (RGB) = 384 floats
            let mut data = Vec::with_capacity(128 * 3);
            for material in cube::material::MATERIAL_REGISTRY.iter() {
                data.push(material.color.x);
                data.push(material.color.y);
                data.push(material.color.z);
            }

            gl.tex_image_2d(
                TEXTURE_2D,
                0,
                RGB32F as i32, // Use floating point texture for precision
                128,
                1,
                0,
                RGB,
                FLOAT,
                glow::PixelUnpackData::Slice(Some(bytemuck::cast_slice(&data))),
            );

            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MIN_FILTER, NEAREST as i32);
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MAG_FILTER, NEAREST as i32);
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_S, CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_T, CLAMP_TO_EDGE as i32);

            gl.bind_texture(TEXTURE_2D, None);

            Ok(texture)
        }
    }

    /// Near plane distance for depth buffer (matches mesh renderer)
    pub const NEAR_PLANE: f32 = 1.0;
    /// Far plane distance for depth buffer (matches mesh renderer)
    pub const FAR_PLANE: f32 = 50000.0;

    /// Render the octree to GL framebuffer
    ///
    /// # Safety
    /// Must be called with an active GL context. Shader program and textures must be initialized.
    pub unsafe fn render_to_gl(
        &self,
        gl: &Context,
        width: i32,
        height: i32,
        time: f32,
        disable_lighting: bool,
    ) {
        unsafe {
            // Set viewport
            gl.viewport(0, 0, width, height);

            // Enable depth test for proper depth buffer output
            gl.enable(DEPTH_TEST);
            gl.depth_func(LESS);
            gl.disable(BLEND);

            // Clear both color and depth buffers
            // Background color with gamma correction to match CPU tracer
            // BACKGROUND_COLOR is (0.4, 0.5, 0.6), gamma corrected: pow(x, 1/2.2)
            let bg_r = 0.4_f32.powf(1.0 / 2.2);
            let bg_g = 0.5_f32.powf(1.0 / 2.2);
            let bg_b = 0.6_f32.powf(1.0 / 2.2);
            gl.clear_color(bg_r, bg_g, bg_b, 1.0);
            gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);

            gl.use_program(Some(self.program));
            gl.bind_vertex_array(Some(self.vao));

            // Bind octree BCF texture (1D-like 2D texture) to texture unit 0
            if let Some(texture) = self.octree_texture {
                gl.active_texture(TEXTURE0);
                gl.bind_texture(TEXTURE_2D, Some(texture));
                if let Some(loc) = &self.octree_data_location {
                    gl.uniform_1_i32(Some(loc), 0); // Bind to texture unit 0
                }
            }

            // Set uniforms
            if let Some(loc) = &self.resolution_location {
                gl.uniform_2_f32(Some(loc), width as f32, height as f32);
            }
            if let Some(loc) = &self.time_location {
                gl.uniform_1_f32(Some(loc), time);
            }
            if let Some(loc) = &self.use_camera_location {
                gl.uniform_1_i32(Some(loc), 0);
            }
            if let Some(loc) = &self.max_depth_location {
                gl.uniform_1_i32(Some(loc), 3); // Max octree depth
            }
            if let Some(loc) = &self.octree_data_size_location {
                gl.uniform_1_u32(Some(loc), self.octree_data_size);
            }
            if let Some(loc) = &self.octree_texture_width_location {
                gl.uniform_1_u32(Some(loc), self.octree_texture_width);
            }
            if let Some(loc) = &self.octree_size_location {
                gl.uniform_1_i32(Some(loc), 8); // Octree bounds size
            }
            if let Some(loc) = &self.disable_lighting_location {
                gl.uniform_1_i32(Some(loc), if disable_lighting { 1 } else { 0 });
            }
            // Set near/far planes for depth calculation
            if let Some(loc) = &self.near_location {
                gl.uniform_1_f32(Some(loc), Self::NEAR_PLANE);
            }
            if let Some(loc) = &self.far_location {
                gl.uniform_1_f32(Some(loc), Self::FAR_PLANE);
            }

            // Bind material palette texture to unit 1
            if let Some(texture) = self.material_palette_texture {
                gl.active_texture(TEXTURE1);
                gl.bind_texture(TEXTURE_2D, Some(texture));
                if let Some(loc) = &self.material_palette_location {
                    gl.uniform_1_i32(Some(loc), 1);
                }
            }

            // Draw fullscreen triangle
            gl.draw_arrays(TRIANGLES, 0, 3);

            // Unbind
            gl.bind_texture(TEXTURE_2D, None);
        }
    }

    /// Render the octree to GL framebuffer with explicit camera
    ///
    /// # Safety
    /// Must be called with an active GL context. Shader program and textures must be initialized.
    pub unsafe fn render_to_gl_with_camera(
        &self,
        gl: &Context,
        width: i32,
        height: i32,
        camera: &Camera,
        disable_lighting: bool,
        show_errors: bool,
    ) {
        unsafe {
            // Set viewport
            gl.viewport(0, 0, width, height);

            // Enable depth test for proper depth buffer output
            gl.enable(DEPTH_TEST);
            gl.depth_func(LESS);
            gl.disable(BLEND);

            // Clear both color and depth buffers
            // Background color with gamma correction to match CPU tracer
            // BACKGROUND_COLOR is (0.4, 0.5, 0.6), gamma corrected: pow(x, 1/2.2)
            let bg_r = 0.4_f32.powf(1.0 / 2.2);
            let bg_g = 0.5_f32.powf(1.0 / 2.2);
            let bg_b = 0.6_f32.powf(1.0 / 2.2);
            gl.clear_color(bg_r, bg_g, bg_b, 1.0);
            gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);

            gl.use_program(Some(self.program));
            gl.bind_vertex_array(Some(self.vao));

            // Bind octree BCF texture (1D-like 2D texture) to texture unit 0
            if let Some(texture) = self.octree_texture {
                gl.active_texture(TEXTURE0);
                gl.bind_texture(TEXTURE_2D, Some(texture));
                if let Some(loc) = &self.octree_data_location {
                    gl.uniform_1_i32(Some(loc), 0); // Bind to texture unit 0
                }
            }

            // Set uniforms
            if let Some(loc) = &self.resolution_location {
                gl.uniform_2_f32(Some(loc), width as f32, height as f32);
            }
            if let Some(loc) = &self.camera_pos_location {
                gl.uniform_3_f32(
                    Some(loc),
                    camera.position.x,
                    camera.position.y,
                    camera.position.z,
                );
            }
            if let Some(loc) = &self.camera_rot_location {
                gl.uniform_4_f32(
                    Some(loc),
                    camera.rotation.x,
                    camera.rotation.y,
                    camera.rotation.z,
                    camera.rotation.w,
                );
            }
            if let Some(loc) = &self.tan_half_vfov_location {
                gl.uniform_1_f32(Some(loc), (camera.vfov * 0.5).tan());
            }
            if let Some(loc) = &self.use_camera_location {
                gl.uniform_1_i32(Some(loc), 1);
            }
            if let Some(loc) = &self.max_depth_location {
                gl.uniform_1_i32(Some(loc), 3); // Max octree depth
            }
            if let Some(loc) = &self.octree_data_size_location {
                gl.uniform_1_u32(Some(loc), self.octree_data_size);
            }
            if let Some(loc) = &self.octree_texture_width_location {
                gl.uniform_1_u32(Some(loc), self.octree_texture_width);
            }
            if let Some(loc) = &self.octree_size_location {
                gl.uniform_1_i32(Some(loc), 8); // Octree bounds size
            }
            if let Some(loc) = &self.disable_lighting_location {
                gl.uniform_1_i32(Some(loc), if disable_lighting { 1 } else { 0 });
            }
            if let Some(loc) = &self.show_errors_location {
                gl.uniform_1_i32(Some(loc), if show_errors { 1 } else { 0 });
            }
            // Set near/far planes for depth calculation
            if let Some(loc) = &self.near_location {
                gl.uniform_1_f32(Some(loc), Self::NEAR_PLANE);
            }
            if let Some(loc) = &self.far_location {
                gl.uniform_1_f32(Some(loc), Self::FAR_PLANE);
            }

            // Bind material palette texture to unit 1
            if let Some(texture) = self.material_palette_texture {
                gl.active_texture(TEXTURE1);
                gl.bind_texture(TEXTURE_2D, Some(texture));
                if let Some(loc) = &self.material_palette_location {
                    gl.uniform_1_i32(Some(loc), 1);
                }
            }

            // Draw fullscreen triangle
            gl.draw_arrays(TRIANGLES, 0, 3);

            // Unbind
            gl.bind_texture(TEXTURE_2D, None);
        }
    }

    /// Clean up all GL resources
    ///
    /// # Safety
    /// Must be called with an active GL context. Should only be called once at shutdown.
    pub unsafe fn destroy(self, gl: &Context) {
        unsafe {
            gl.delete_program(self.program);
            gl.delete_vertex_array(self.vao);
            if let Some(texture) = self.octree_texture {
                gl.delete_texture(texture);
            }
            if let Some(texture) = self.material_palette_texture {
                gl.delete_texture(texture);
            }
        }
    }
}

impl Renderer for GlTracer {
    fn render(&mut self, _width: u32, _height: u32, _time: f32) {
        panic!("GlTracer requires GL context. Use render_to_framebuffer() instead.");
    }

    fn render_with_camera(&mut self, _width: u32, _height: u32, _camera: &Camera) {
        panic!("GlTracer requires GL context. Use render_to_framebuffer() instead.");
    }

    fn name(&self) -> &str {
        "GL Tracer"
    }

    fn supports_gl(&self) -> bool {
        true
    }

    fn init_gl(&mut self, gl: &Context) -> Result<(), String> {
        unsafe { GlTracer::init_gl(self, gl) }
    }

    fn destroy_gl(&mut self, gl: &Context) {
        unsafe {
            if let Some(gl_program) = self.gl_program.take() {
                gl_program.destroy(gl);
            }
        }
    }

    fn render_to_framebuffer(
        &mut self,
        gl: &Context,
        width: u32,
        height: u32,
        camera: Option<&Camera>,
        time: Option<f32>,
    ) -> Result<(), String> {
        unsafe {
            if let Some(camera) = camera {
                self.render_to_gl_with_camera(gl, width as i32, height as i32, camera);
            } else if let Some(t) = time {
                self.render_to_gl(gl, width as i32, height as i32, t);
            } else {
                return Err("Must provide either camera or time parameter".to_string());
            }
        }
        Ok(())
    }

    fn save_framebuffer_to_file(
        &self,
        gl: &Context,
        width: u32,
        height: u32,
        path: &str,
    ) -> Result<(), String> {
        // Read pixels from framebuffer
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        unsafe {
            gl.read_pixels(
                0,
                0,
                width as i32,
                height as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut pixels)),
            );
        }

        // Convert RGBA to RGB
        let rgb_pixels: Vec<u8> = pixels
            .chunks(4)
            .flat_map(|rgba| [rgba[0], rgba[1], rgba[2]])
            .collect();

        // Flip Y-axis (GL origin is bottom-left, image origin is top-left)
        let mut flipped = vec![0u8; rgb_pixels.len()];
        for y in 0..height {
            let src_row =
                &rgb_pixels[(y * width * 3) as usize..((y + 1) * width * 3) as usize];
            let dst_y = height - 1 - y;
            let dst_row =
                &mut flipped[(dst_y * width * 3) as usize..((dst_y + 1) * width * 3) as usize];
            dst_row.copy_from_slice(src_row);
        }

        // Save to file
        image::save_buffer(
            path,
            &flipped,
            width,
            height,
            image::ColorType::Rgb8,
        )
        .map_err(|e| e.to_string())
    }
}
