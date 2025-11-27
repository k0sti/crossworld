//! WebGL 2.0 octree raytracer using fragment shaders
//!
//! This tracer uses OpenGL ES 3.0 (WebGL 2.0) fragment shaders to render octree voxel data.
//! The octree is serialized to Binary Cube Format (BCF) and uploaded to GPU as a buffer,
//! then traversed using hierarchical DDA in the fragment shader.

use crate::renderer::*;
use crate::shader_utils;
use cube::Cube;
use cube::io::bcf::serialize_bcf;
use glam::IVec3;
use glow::*;
use std::rc::Rc;

// Shader sources - include from files at compile time
const VERTEX_SHADER_SOURCE: &str = include_str!("shaders/octree_raycast.vert");
const FRAGMENT_SHADER_SOURCE: &str = include_str!("shaders/octree_raycast.frag");

/// WebGL 2.0 fragment shader raytracer with octree support
pub struct GlCubeTracer {
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
    octree_texture: Option<Texture>, // 1D-like 2D texture for BCF data
    octree_data_size: u32,
    // Uniform locations
    resolution_location: Option<UniformLocation>,
    time_location: Option<UniformLocation>,
    camera_pos_location: Option<UniformLocation>,
    camera_rot_location: Option<UniformLocation>,
    use_camera_location: Option<UniformLocation>,
    max_depth_location: Option<UniformLocation>,
    octree_data_location: Option<UniformLocation>,
    octree_data_size_location: Option<UniformLocation>,
    octree_size_location: Option<UniformLocation>,
    material_palette_location: Option<UniformLocation>,
    material_palette_texture: Option<Texture>,
    disable_lighting_location: Option<UniformLocation>,
    show_errors_location: Option<UniformLocation>,
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

impl GlCubeTracer {
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
    /// Must be called with an active GL context
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
    pub unsafe fn render_to_gl(&self, gl: &Context, width: i32, height: i32, time: f32) {
        unsafe {
            if let Some(gl_program) = &self.gl_program {
                gl_program.render_to_gl(gl, width, height, time, self.disable_lighting);
            }
        }
    }

    /// Render to OpenGL context with explicit camera
    pub unsafe fn render_to_gl_with_camera(
        &self,
        gl: &Context,
        width: i32,
        height: i32,
        camera: &CameraConfig,
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
    pub unsafe fn destroy_gl(&mut self, gl: &Context) {
        unsafe {
            if let Some(gl_program) = self.gl_program.take() {
                gl_program.destroy(gl);
            }
        }
    }
}

impl GlTracerGl {
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
            println!("[GL Tracer] Serializing octree to BCF format...");
            let bcf_data = serialize_bcf(cube);
            println!("[GL Tracer] BCF data serialized: {} bytes", bcf_data.len());

            // Detect SSBO support (OpenGL 4.3+ or ES 3.1+)
            // For now, we'll use texture buffer as it's more widely supported
            let _use_ssbo = false; // TODO: Detect SSBO support properly

            println!("[GL Tracer] Using texture buffer for octree data");

            // Create 1D texture for BCF data (more compatible than texture buffer)
            let texture = gl
                .create_texture()
                .map_err(|e| format!("Failed to create texture: {}", e))?;

            gl.bind_texture(TEXTURE_2D, Some(texture));

            // Upload as 1D-like 2D texture (width x 1 height)
            let width = bcf_data.len() as i32;
            gl.tex_image_2d(
                TEXTURE_2D,
                0,               // mip level
                R8UI as i32,     // internal format
                width,           // width
                1,               // height (1 for 1D-like texture)
                0,               // border
                RED_INTEGER,     // format
                UNSIGNED_BYTE,   // type
                Some(&bcf_data), // data
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
            let use_camera_location = gl.get_uniform_location(program, "u_use_camera");
            let max_depth_location = gl.get_uniform_location(program, "u_max_depth");
            let octree_data_location = gl.get_uniform_location(program, "u_octree_data");
            let octree_data_size_location = gl.get_uniform_location(program, "u_octree_data_size");
            let octree_size_location = gl.get_uniform_location(program, "u_octree_size");
            let material_palette_location = gl.get_uniform_location(program, "u_material_palette");
            let disable_lighting_location = gl.get_uniform_location(program, "u_disable_lighting");
            let show_errors_location = gl.get_uniform_location(program, "u_show_errors");

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
                resolution_location,
                time_location,
                camera_pos_location,
                camera_rot_location,
                use_camera_location,
                max_depth_location,
                octree_data_location,
                octree_data_size_location,
                octree_size_location,
                material_palette_location,
                material_palette_texture,
                disable_lighting_location,
                show_errors_location,
            })
        }
    }

    /// Create a 3D texture from the octree data
    /// For simplicity, we'll serialize the octree to a 3D grid

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
                Some(bytemuck::cast_slice(&data)),
            );

            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MIN_FILTER, NEAREST as i32);
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MAG_FILTER, NEAREST as i32);
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_S, CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_T, CLAMP_TO_EDGE as i32);

            gl.bind_texture(TEXTURE_2D, None);

            Ok(texture)
        }
    }

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

            // Disable depth test and blending (we're rendering a fullscreen quad)
            gl.disable(DEPTH_TEST);
            gl.disable(BLEND);

            // Clear to background color (matches BACKGROUND_COLOR in Rust)
            gl.clear_color(0.4, 0.5, 0.6, 1.0);
            gl.clear(COLOR_BUFFER_BIT);

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
            if let Some(loc) = &self.octree_size_location {
                gl.uniform_1_i32(Some(loc), 8); // Octree bounds size
            }
            if let Some(loc) = &self.disable_lighting_location {
                gl.uniform_1_i32(Some(loc), if disable_lighting { 1 } else { 0 });
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

    pub unsafe fn render_to_gl_with_camera(
        &self,
        gl: &Context,
        width: i32,
        height: i32,
        camera: &CameraConfig,
        disable_lighting: bool,
        show_errors: bool,
    ) {
        unsafe {
            // Set viewport
            gl.viewport(0, 0, width, height);

            // Disable depth test and blending (we're rendering a fullscreen quad)
            gl.disable(DEPTH_TEST);
            gl.disable(BLEND);

            // Clear to background color (matches BACKGROUND_COLOR in Rust)
            gl.clear_color(0.4, 0.5, 0.6, 1.0);
            gl.clear(COLOR_BUFFER_BIT);

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
            if let Some(loc) = &self.use_camera_location {
                gl.uniform_1_i32(Some(loc), 1);
            }
            if let Some(loc) = &self.max_depth_location {
                gl.uniform_1_i32(Some(loc), 3); // Max octree depth
            }
            if let Some(loc) = &self.octree_data_size_location {
                gl.uniform_1_u32(Some(loc), self.octree_data_size);
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

impl Renderer for GlCubeTracer {
    fn render(&mut self, _width: u32, _height: u32, _time: f32) {
        // Note: GL rendering is handled by render_to_gl in the app loop
        // This is here to satisfy the trait, but actual rendering needs GL context
    }

    fn render_with_camera(&mut self, _width: u32, _height: u32, _camera: &CameraConfig) {
        // Note: GL rendering is handled by render_to_gl_with_camera in the app loop
        // This is here to satisfy the trait, but actual rendering needs GL context
    }

    fn name(&self) -> &str {
        "GlCubeTracer (WebGL 2.0 Fragment Shader)"
    }
}
