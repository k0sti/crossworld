//! WebGL 2.0 octree raytracer using fragment shaders
//!
//! This tracer uses OpenGL ES 3.0 (WebGL 2.0) fragment shaders to render octree voxel data.
//! The octree is serialized to a 3D texture and traversed on the GPU for real-time rendering.

use crate::renderer::*;
use crate::shader_utils;
use cube::Cube;
use glam::IVec3;
use glow::*;
use std::rc::Rc;

// Shader sources - include from files at compile time
const VERTEX_SHADER_SOURCE: &str = include_str!("shaders/octree_raycast.vert");
const FRAGMENT_SHADER_SOURCE: &str = include_str!("shaders/octree_raycast.frag");

/// WebGL 2.0 fragment shader raytracer with octree support
pub struct GlCubeTracer {
    cube: Rc<Cube<i32>>,
    bounds: CubeBounds,
    // GL resources (Option for cases where GL context isn't available)
    gl_program: Option<GlTracerGl>,
}

/// GPU-specific OpenGL resources
pub struct GlTracerGl {
    program: Program,
    vao: VertexArray,
    octree_texture: Option<Texture>,
    // Uniform locations
    resolution_location: Option<UniformLocation>,
    time_location: Option<UniformLocation>,
    camera_pos_location: Option<UniformLocation>,
    camera_rot_location: Option<UniformLocation>,
    use_camera_location: Option<UniformLocation>,
    max_depth_location: Option<UniformLocation>,
    octree_texture_location: Option<UniformLocation>,
    octree_size_location: Option<UniformLocation>,
    material_palette_location: Option<UniformLocation>,
    material_palette_texture: Option<Texture>,
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
    pub fn new(cube: Rc<Cube<i32>>) -> Self {
        Self {
            cube,
            bounds: CubeBounds::default(),
            gl_program: None,
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
    pub fn cube(&self) -> &Rc<Cube<i32>> {
        &self.cube
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
        max_depth: u32,
    ) -> Result<Option<cube::RaycastHit<i32>>, cube::RaycastError> {
        let is_empty = |v: &i32| *v == 0;
        self.cube.raycast_debug(pos, dir, max_depth, &is_empty)
    }

    /// Render to OpenGL context
    pub unsafe fn render_to_gl(&self, gl: &Context, width: i32, height: i32, time: f32) {
        unsafe {
            if let Some(gl_program) = &self.gl_program {
                gl_program.render_to_gl(gl, width, height, time);
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
                gl_program.render_to_gl_with_camera(gl, width, height, camera);
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
    pub unsafe fn new(gl: &Context, cube: &Cube<i32>) -> Result<Self, String> {
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

            // Get uniform locations
            let resolution_location = gl.get_uniform_location(program, "u_resolution");
            let time_location = gl.get_uniform_location(program, "u_time");
            let camera_pos_location = gl.get_uniform_location(program, "u_camera_pos");
            let camera_rot_location = gl.get_uniform_location(program, "u_camera_rot");
            let use_camera_location = gl.get_uniform_location(program, "u_use_camera");
            let max_depth_location = gl.get_uniform_location(program, "u_max_depth");
            let octree_texture_location = gl.get_uniform_location(program, "u_octree_texture");
            let octree_size_location = gl.get_uniform_location(program, "u_octree_size");

            // Debug: Print uniform locations
            println!("[GL Tracer] Uniform locations:");
            println!("  u_octree_texture: {:?}", octree_texture_location);
            println!("  u_max_depth: {:?}", max_depth_location);
            println!("  u_octree_size: {:?}", octree_size_location);

            // Create and upload octree texture
            println!("[GL Tracer] Creating octree texture...");
            let octree_texture = Some(Self::create_octree_texture(gl, cube)?);
            println!("[GL Tracer] Octree texture created successfully!");
            let material_palette_location = gl.get_uniform_location(program, "u_material_palette");

            // Create and upload material palette texture
            println!("[GL Tracer] Creating material palette texture...");
            let material_palette_texture = Some(Self::create_material_palette_texture(gl)?);
            println!("[GL Tracer] Material palette texture created successfully!");

            Ok(Self {
                program,
                vao,
                octree_texture,
                resolution_location,
                time_location,
                camera_pos_location,
                camera_rot_location,
                use_camera_location,
                max_depth_location,
                octree_texture_location,
                octree_size_location,
                material_palette_location,
                material_palette_texture,
            })
        }
    }

    /// Create a 3D texture from the octree data
    /// For simplicity, we'll serialize the octree to a 3D grid
    unsafe fn create_octree_texture(gl: &Context, cube: &Cube<i32>) -> Result<Texture, String> {
        unsafe {
            // For now, create a simple 8x8x8 voxel grid (depth 3)
            // This will be expanded to support arbitrary octree depths
            const SIZE: usize = 8;
            // Use RGBA format for better compatibility (4 bytes per voxel)
            let mut voxel_data = vec![0u8; SIZE * SIZE * SIZE * 4];

            // Serialize octree to voxel grid
            // Sample each voxel position
            for z in 0..SIZE {
                for y in 0..SIZE {
                    for x in 0..SIZE {
                        // Convert to normalized [0,1] coordinates
                        let pos = glam::Vec3::new(
                            (x as f32 + 0.5) / SIZE as f32,
                            (y as f32 + 0.5) / SIZE as f32,
                            (z as f32 + 0.5) / SIZE as f32,
                        );

                        // Sample cube at this position
                        let value = sample_cube_at_position(cube, pos, 3);
                        let idx = (x + y * SIZE + z * SIZE * SIZE) * 4;
                        // Encode material value in R channel (0-255)
                        // Fragment shader will decode this to get material index
                        voxel_data[idx] = value.clamp(0, 255) as u8; // R: material index
                        voxel_data[idx + 1] = 0; // G: unused
                        voxel_data[idx + 2] = 0; // B: unused
                        voxel_data[idx + 3] = 255; // A: unused
                    }
                }
            }

            // Create 3D texture
            let texture = gl
                .create_texture()
                .map_err(|e| format!("Failed to create texture: {}", e))?;

            gl.bind_texture(TEXTURE_3D, Some(texture));

            // Upload texture data
            // Use RGBA8 format for better OpenGL ES compatibility
            gl.tex_image_3d(
                TEXTURE_3D,
                0,                 // mip level
                RGBA8 as i32,      // internal format
                SIZE as i32,       // width
                SIZE as i32,       // height
                SIZE as i32,       // depth
                0,                 // border
                RGBA,              // format
                UNSIGNED_BYTE,     // type
                Some(&voxel_data), // data
            );

            // Set texture parameters
            gl.tex_parameter_i32(TEXTURE_3D, TEXTURE_MIN_FILTER, NEAREST as i32);
            gl.tex_parameter_i32(TEXTURE_3D, TEXTURE_MAG_FILTER, NEAREST as i32);
            gl.tex_parameter_i32(TEXTURE_3D, TEXTURE_WRAP_S, CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(TEXTURE_3D, TEXTURE_WRAP_T, CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(TEXTURE_3D, TEXTURE_WRAP_R, CLAMP_TO_EDGE as i32);

            gl.bind_texture(TEXTURE_3D, None);

            // Log texture creation
            println!(
                "[GL Tracer] 3D texture uploaded: {}x{}x{} = {} voxels",
                SIZE,
                SIZE,
                SIZE,
                SIZE * SIZE * SIZE
            );
            // Count solid voxels (check R channel, which is every 4th byte)
            let solid_count = voxel_data.iter().step_by(4).filter(|&&v| v != 0).count();
            println!(
                "[GL Tracer] Solid voxels: {} ({:.1}%)",
                solid_count,
                (solid_count as f32 / (SIZE * SIZE * SIZE) as f32) * 100.0
            );

            Ok(texture)
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

    pub unsafe fn render_to_gl(&self, gl: &Context, width: i32, height: i32, time: f32) {
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

            // Bind octree texture
            if let Some(texture) = self.octree_texture {
                gl.active_texture(TEXTURE0);
                gl.bind_texture(TEXTURE_3D, Some(texture));
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
                gl.uniform_1_i32(Some(loc), 0); // Depth 0 for flat 8x8x8 grid
            }
            if let Some(loc) = &self.octree_texture_location {
                gl.uniform_1_i32(Some(loc), 0); // Texture unit 0
            }
            if let Some(loc) = &self.octree_size_location {
                gl.uniform_1_i32(Some(loc), 8); // 8x8x8 grid
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
            gl.bind_texture(TEXTURE_3D, None);
        }
    }

    pub unsafe fn render_to_gl_with_camera(
        &self,
        gl: &Context,
        width: i32,
        height: i32,
        camera: &CameraConfig,
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

            // Bind octree texture
            if let Some(texture) = self.octree_texture {
                gl.active_texture(TEXTURE0);
                gl.bind_texture(TEXTURE_3D, Some(texture));
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
                gl.uniform_1_i32(Some(loc), 0); // Depth 0 for flat 8x8x8 grid
            }
            if let Some(loc) = &self.octree_texture_location {
                gl.uniform_1_i32(Some(loc), 0);
            }
            if let Some(loc) = &self.octree_size_location {
                gl.uniform_1_i32(Some(loc), 8);
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
            gl.bind_texture(TEXTURE_3D, None);
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

/// Sample cube at a given normalized position
///
/// Converts the octree structure to a voxel value at a specific position
/// by using the cube's raycast functionality. This is used to serialize
/// the octree data into a 3D texture that can be uploaded to the GPU.
///
/// # Arguments
///
/// * `cube` - The octree cube to sample
/// * `pos` - Normalized [0,1]³ position to sample
/// * `max_depth` - Maximum octree depth to traverse
///
/// # Returns
///
/// The voxel value (i32) at the given position, or 0 if empty
fn sample_cube_at_position(cube: &Cube<i32>, pos: glam::Vec3, max_depth: u32) -> i32 {
    use glam::IVec3;

    match cube {
        Cube::Solid(value) => *value,
        _ => {
            // Convert normalized [0,1] coordinates to octree integer coordinates
            // For max_depth=3, grid is 2^3 = 8, so coordinates are in [0,8)
            let grid_size = 1 << max_depth; // 2^max_depth
            let octree_pos = IVec3::new(
                (pos.x * grid_size as f32).floor() as i32,
                (pos.y * grid_size as f32).floor() as i32,
                (pos.z * grid_size as f32).floor() as i32,
            );

            // Clamp to valid range [0, grid_size)
            let octree_pos = octree_pos.clamp(IVec3::ZERO, IVec3::splat(grid_size - 1));

            // Sample the cube at this position at the given depth
            // depth determines resolution: depth=3 means 8x8x8 grid (2^3)
            cube.get_id(max_depth, octree_pos)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_octa_cube() {
        // Create octa cube: 6 solid, 2 empty
        // Octant indexing: index = x*4 + y*2 + z
        let children: [Rc<Cube<i32>>; 8] = [
            Rc::new(Cube::Solid(1)), // 0: (0,0,0)
            Rc::new(Cube::Solid(1)), // 1: (0,0,1)
            Rc::new(Cube::Solid(1)), // 2: (0,1,0)
            Rc::new(Cube::Solid(0)), // 3: (0,1,1) - EMPTY
            Rc::new(Cube::Solid(1)), // 4: (1,0,0)
            Rc::new(Cube::Solid(1)), // 5: (1,0,1)
            Rc::new(Cube::Solid(1)), // 6: (1,1,0)
            Rc::new(Cube::Solid(0)), // 7: (1,1,1) - EMPTY
        ];
        let cube = Cube::Cubes(Box::new(children));

        // Test sampling solid voxels
        // Octant 0 (0,0,0) -> grid positions (x: 0-3, y: 0-3, z: 0-3)
        let pos_octant_0 = glam::Vec3::new(0.125, 0.125, 0.125);
        assert_eq!(sample_cube_at_position(&cube, pos_octant_0, 3), 1);

        // Test sampling empty voxels
        // Octant 3 (0,1,1) -> grid positions (x: 0-3, y: 4-7, z: 4-7)
        let pos_octant_3 = glam::Vec3::new(0.1875, 0.6875, 0.6875);
        assert_eq!(sample_cube_at_position(&cube, pos_octant_3, 3), 0);

        // Octant 7 (1,1,1) -> grid positions (x: 4-7, y: 4-7, z: 4-7)
        let pos_octant_7 = glam::Vec3::new(0.6875, 0.6875, 0.6875);
        assert_eq!(sample_cube_at_position(&cube, pos_octant_7, 3), 0);

        // Test another solid octant
        // Octant 4 (1,0,0) -> grid positions (x: 4-7, y: 0-3, z: 0-3)
        let pos_octant_4 = glam::Vec3::new(0.625, 0.125, 0.125);
        assert_eq!(sample_cube_at_position(&cube, pos_octant_4, 3), 1);
    }
}
