//! Compute shader octree raytracer
//!
//! This tracer uses OpenGL compute shaders (GL 4.3+) for high-performance parallel raytracing.
//! Compute shaders allow for more flexible GPU programming compared to fragment shaders.
//!
//! The implementation uses BCF (Binary Cube Format) for octree representation on the GPU,
//! matching the GL tracer's approach for consistency.

use crate::renderer::*;
use crate::shader_utils::create_compute_program;
use cube::Cube;
use cube::io::bcf::serialize_bcf;
use glow::*;
use std::rc::Rc;

const COMPUTE_SHADER_SOURCE: &str = include_str!("../shaders/basic_raycast.comp");

// Simple vertex shader for fullscreen quad
const BLIT_VERTEX_SHADER: &str = r#"#version 300 es
precision highp float;

out vec2 vUv;

void main() {
    // Generate fullscreen triangle
    float x = float((gl_VertexID & 1) << 2) - 1.0;
    float y = float((gl_VertexID & 2) << 1) - 1.0;
    vUv = vec2((x + 1.0) * 0.5, (y + 1.0) * 0.5);
    gl_Position = vec4(x, y, 0.0, 1.0);
}
"#;

// Simple fragment shader to sample and display the compute shader output
const BLIT_FRAGMENT_SHADER: &str = r#"#version 300 es
precision highp float;

in vec2 vUv;
out vec4 fragColor;

uniform sampler2D uTexture;

void main() {
    // Sample the texture written by compute shader
    fragColor = texture(uTexture, vUv);
}
"#;

/// Compute shader raytracer with full BCF octree traversal
pub struct ComputeTracer {
    cube: Rc<Cube<u8>>,
    #[allow(dead_code)]
    bounds: CubeBounds,
    gl_state: Option<ComputeTracerGl>,
}

/// OpenGL resources for compute shader raytracing
struct ComputeTracerGl {
    compute_program: Program,
    blit_program: Program,
    blit_vao: VertexArray,
    output_texture: Texture,
    depth_texture: Texture,
    texture_width: i32,
    texture_height: i32,
    // BCF octree data SSBO (binding = 0)
    octree_ssbo: Buffer,
    octree_data_size: u32,
    // Material palette SSBO (binding = 1)
    material_palette_ssbo: Buffer,
    // Compute shader uniform locations
    uniform_resolution: Option<UniformLocation>,
    uniform_time: Option<UniformLocation>,
    uniform_camera_pos: Option<UniformLocation>,
    uniform_camera_rot: Option<UniformLocation>,
    uniform_use_camera: Option<UniformLocation>,
    uniform_octree_data_size: Option<UniformLocation>,
    uniform_near: Option<UniformLocation>,
    uniform_far: Option<UniformLocation>,
    // Blit shader uniform locations
    blit_uniform_texture: Option<UniformLocation>,
}

impl ComputeTracer {
    pub fn new(cube: Rc<Cube<u8>>) -> Self {
        Self {
            cube,
            bounds: CubeBounds::default(),
            gl_state: None,
        }
    }

    /// Initialize OpenGL compute shader resources
    ///
    /// # Safety
    /// Must be called with an active GL context that supports compute shaders (GL 4.3+).
    /// GL context must remain valid for the lifetime of this object.
    pub unsafe fn init_gl(&mut self, gl: &Context) -> Result<(), String> {
        unsafe {
            println!("[GPU Tracer] Initializing...");

            // Compile compute shader program
            let compute_program = create_compute_program(gl, COMPUTE_SHADER_SOURCE)?;
            println!("[GPU Tracer] ✓ Compute shader compiled successfully");

            // Get uniform locations for compute shader
            let uniform_resolution = gl.get_uniform_location(compute_program, "uResolution");
            let uniform_time = gl.get_uniform_location(compute_program, "uTime");
            let uniform_camera_pos = gl.get_uniform_location(compute_program, "uCameraPos");
            let uniform_camera_rot = gl.get_uniform_location(compute_program, "uCameraRot");
            let uniform_use_camera = gl.get_uniform_location(compute_program, "uUseCamera");
            let uniform_octree_data_size =
                gl.get_uniform_location(compute_program, "uOctreeDataSize");
            let uniform_near = gl.get_uniform_location(compute_program, "uNear");
            let uniform_far = gl.get_uniform_location(compute_program, "uFar");

            println!("[GPU Tracer] Uniform locations:");
            println!("  uResolution: {:?}", uniform_resolution);
            println!("  uTime: {:?}", uniform_time);
            println!("  uCameraPos: {:?}", uniform_camera_pos);
            println!("  uCameraRot: {:?}", uniform_camera_rot);
            println!("  uUseCamera: {:?}", uniform_use_camera);
            println!("  uOctreeDataSize: {:?}", uniform_octree_data_size);

            // Compile blit shader program (for displaying compute shader output)
            let blit_program =
                crate::shader_utils::create_program(gl, BLIT_VERTEX_SHADER, BLIT_FRAGMENT_SHADER)?;
            println!("[GPU Tracer] ✓ Blit shader compiled successfully");

            let blit_uniform_texture = gl.get_uniform_location(blit_program, "uTexture");
            println!("[GPU Tracer] Blit uniform location:");
            println!("  uTexture: {:?}", blit_uniform_texture);

            // Create VAO for fullscreen quad (using vertex shader that generates positions)
            let blit_vao = gl
                .create_vertex_array()
                .map_err(|e| format!("Failed to create VAO: {}", e))?;

            // Create output texture (will be resized on first render)
            let output_texture = gl
                .create_texture()
                .map_err(|e| format!("Failed to create output texture: {}", e))?;

            // Create depth texture (R32F format for high precision)
            let depth_texture = gl
                .create_texture()
                .map_err(|e| format!("Failed to create depth texture: {}", e))?;

            // Serialize cube to BCF format
            println!("[GPU Tracer] Serializing octree to BCF format...");
            let bcf_data = serialize_bcf(&self.cube);
            println!("[GPU Tracer] BCF data serialized: {} bytes", bcf_data.len());

            // Pad BCF data to multiple of 4 bytes for uint32 packing
            let mut padded_bcf = bcf_data.clone();
            while !padded_bcf.len().is_multiple_of(4) {
                padded_bcf.push(0);
            }

            // Create octree SSBO (binding = 0)
            let octree_ssbo = gl
                .create_buffer()
                .map_err(|e| format!("Failed to create octree SSBO: {}", e))?;

            gl.bind_buffer(SHADER_STORAGE_BUFFER, Some(octree_ssbo));
            gl.buffer_data_u8_slice(SHADER_STORAGE_BUFFER, &padded_bcf, STATIC_DRAW);
            gl.bind_buffer(SHADER_STORAGE_BUFFER, None);
            println!(
                "[GPU Tracer] ✓ BCF octree data uploaded to SSBO ({} bytes)",
                padded_bcf.len()
            );

            // Create material palette SSBO (binding = 1)
            let material_palette_ssbo = Self::create_material_palette_ssbo(gl)?;
            println!("[GPU Tracer] ✓ Material palette SSBO created");

            self.gl_state = Some(ComputeTracerGl {
                compute_program,
                blit_program,
                blit_vao,
                output_texture,
                depth_texture,
                texture_width: 0,
                texture_height: 0,
                octree_ssbo,
                octree_data_size: bcf_data.len() as u32,
                material_palette_ssbo,
                uniform_resolution,
                uniform_time,
                uniform_camera_pos,
                uniform_camera_rot,
                uniform_use_camera,
                uniform_octree_data_size,
                uniform_near,
                uniform_far,
                blit_uniform_texture,
            });

            println!("[GPU Tracer] ✓ Initialization complete");
            Ok(())
        }
    }

    /// Create an SSBO for the material palette (128 RGBA entries, vec4 for alignment)
    unsafe fn create_material_palette_ssbo(gl: &Context) -> Result<Buffer, String> {
        unsafe {
            let buffer = gl
                .create_buffer()
                .map_err(|e| format!("Failed to create material palette SSBO: {}", e))?;

            // Create data buffer from MATERIAL_REGISTRY (vec4 for proper alignment)
            let mut data = Vec::with_capacity(128 * 4);
            for material in cube::material::MATERIAL_REGISTRY.iter() {
                data.push(material.color.x);
                data.push(material.color.y);
                data.push(material.color.z);
                data.push(1.0f32); // padding for vec4 alignment
            }

            gl.bind_buffer(SHADER_STORAGE_BUFFER, Some(buffer));
            gl.buffer_data_u8_slice(
                SHADER_STORAGE_BUFFER,
                bytemuck::cast_slice(&data),
                STATIC_DRAW,
            );
            gl.bind_buffer(SHADER_STORAGE_BUFFER, None);

            Ok(buffer)
        }
    }

    /// Get reference to the cube
    pub fn cube(&self) -> &Rc<Cube<u8>> {
        &self.cube
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

    /// Render to OpenGL context using compute shader with time-based orbit camera
    ///
    /// # Safety
    /// Must be called with an active GL context. GL resources must have been initialized via `init_gl()`.
    pub unsafe fn render_to_gl(&mut self, gl: &Context, width: i32, height: i32, time: f32) {
        let Some(gl_state) = &mut self.gl_state else {
            return;
        };

        unsafe {
            // Recreate texture with immutable storage if size changed
            Self::ensure_output_texture(gl, gl_state, width, height);

            // Use compute shader program
            gl.use_program(Some(gl_state.compute_program));

            // Bind output texture as image (unit 0)
            gl.bind_image_texture(
                0,
                Some(gl_state.output_texture),
                0,
                false,
                0,
                WRITE_ONLY,
                RGBA8,
            );

            // Bind depth texture as image (unit 1)
            gl.bind_image_texture(
                1,
                Some(gl_state.depth_texture),
                0,
                false,
                0,
                WRITE_ONLY,
                R32F,
            );

            // Bind octree data SSBO (binding = 0)
            gl.bind_buffer_base(SHADER_STORAGE_BUFFER, 0, Some(gl_state.octree_ssbo));

            // Bind material palette SSBO (binding = 1)
            gl.bind_buffer_base(
                SHADER_STORAGE_BUFFER,
                1,
                Some(gl_state.material_palette_ssbo),
            );

            // Set uniforms
            if let Some(loc) = &gl_state.uniform_resolution {
                gl.uniform_2_f32(Some(loc), width as f32, height as f32);
            }
            if let Some(loc) = &gl_state.uniform_time {
                gl.uniform_1_f32(Some(loc), time);
            }
            if let Some(loc) = &gl_state.uniform_use_camera {
                gl.uniform_1_i32(Some(loc), 0); // Use time-based orbit camera
            }
            if let Some(loc) = &gl_state.uniform_octree_data_size {
                gl.uniform_1_u32(Some(loc), gl_state.octree_data_size);
            }
            // Set near/far planes for depth calculation (matches mesh renderer)
            if let Some(loc) = &gl_state.uniform_near {
                gl.uniform_1_f32(Some(loc), 1.0);
            }
            if let Some(loc) = &gl_state.uniform_far {
                gl.uniform_1_f32(Some(loc), 50000.0);
            }

            // Dispatch compute shader (8x8 local work group size)
            let work_groups_x = (width + 7) / 8;
            let work_groups_y = (height + 7) / 8;
            gl.dispatch_compute(work_groups_x as u32, work_groups_y as u32, 1);

            // Wait for compute shader to finish
            gl.memory_barrier(SHADER_IMAGE_ACCESS_BARRIER_BIT);

            // Blit output texture to screen
            let output_texture = gl_state.output_texture;
            Self::blit_texture_to_screen_static(gl, gl_state, output_texture, width, height);
        }
    }

    /// Render to OpenGL context with explicit camera using compute shader
    ///
    /// # Safety
    /// Must be called with an active GL context. GL resources must have been initialized via `init_gl()`.
    pub unsafe fn render_to_gl_with_camera(
        &mut self,
        gl: &Context,
        width: i32,
        height: i32,
        camera: &Camera,
    ) {
        let Some(gl_state) = &mut self.gl_state else {
            return;
        };

        unsafe {
            // Recreate texture with immutable storage if size changed
            Self::ensure_output_texture(gl, gl_state, width, height);

            // Use compute shader program
            gl.use_program(Some(gl_state.compute_program));

            // Bind output texture as image (unit 0)
            gl.bind_image_texture(
                0,
                Some(gl_state.output_texture),
                0,
                false,
                0,
                WRITE_ONLY,
                RGBA8,
            );

            // Bind depth texture as image (unit 1)
            gl.bind_image_texture(
                1,
                Some(gl_state.depth_texture),
                0,
                false,
                0,
                WRITE_ONLY,
                R32F,
            );

            // Bind octree data SSBO (binding = 0)
            gl.bind_buffer_base(SHADER_STORAGE_BUFFER, 0, Some(gl_state.octree_ssbo));

            // Bind material palette SSBO (binding = 1)
            gl.bind_buffer_base(
                SHADER_STORAGE_BUFFER,
                1,
                Some(gl_state.material_palette_ssbo),
            );

            // Set uniforms
            if let Some(loc) = &gl_state.uniform_resolution {
                gl.uniform_2_f32(Some(loc), width as f32, height as f32);
            }
            if let Some(loc) = &gl_state.uniform_camera_pos {
                gl.uniform_3_f32(
                    Some(loc),
                    camera.position.x,
                    camera.position.y,
                    camera.position.z,
                );
            }
            if let Some(loc) = &gl_state.uniform_camera_rot {
                gl.uniform_4_f32(
                    Some(loc),
                    camera.rotation.x,
                    camera.rotation.y,
                    camera.rotation.z,
                    camera.rotation.w,
                );
            }
            if let Some(loc) = &gl_state.uniform_use_camera {
                gl.uniform_1_i32(Some(loc), 1); // Use explicit camera
            }
            if let Some(loc) = &gl_state.uniform_octree_data_size {
                gl.uniform_1_u32(Some(loc), gl_state.octree_data_size);
            }
            // Set near/far planes for depth calculation (matches mesh renderer)
            if let Some(loc) = &gl_state.uniform_near {
                gl.uniform_1_f32(Some(loc), 1.0);
            }
            if let Some(loc) = &gl_state.uniform_far {
                gl.uniform_1_f32(Some(loc), 50000.0);
            }

            // Dispatch compute shader (8x8 local work group size)
            let work_groups_x = (width + 7) / 8;
            let work_groups_y = (height + 7) / 8;
            gl.dispatch_compute(work_groups_x as u32, work_groups_y as u32, 1);

            // Wait for compute shader to finish
            gl.memory_barrier(SHADER_IMAGE_ACCESS_BARRIER_BIT);

            // Blit output texture to screen
            let output_texture = gl_state.output_texture;
            Self::blit_texture_to_screen_static(gl, gl_state, output_texture, width, height);
        }
    }

    /// Ensure output texture and depth texture exist and have correct size
    unsafe fn ensure_output_texture(
        gl: &Context,
        gl_state: &mut ComputeTracerGl,
        width: i32,
        height: i32,
    ) {
        if gl_state.texture_width != width || gl_state.texture_height != height {
            unsafe {
                // Delete old textures
                gl.delete_texture(gl_state.output_texture);
                gl.delete_texture(gl_state.depth_texture);

                // Create new output texture
                let output_texture = gl
                    .create_texture()
                    .map_err(|e| format!("Failed to recreate texture: {}", e))
                    .unwrap();

                gl.bind_texture(TEXTURE_2D, Some(output_texture));

                // Use tex_storage_2d for immutable storage (required for glBindImageTexture)
                gl.tex_storage_2d(TEXTURE_2D, 1, RGBA8, width, height);

                // Set texture parameters
                gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MIN_FILTER, NEAREST as i32);
                gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MAG_FILTER, NEAREST as i32);
                gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_S, CLAMP_TO_EDGE as i32);
                gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_T, CLAMP_TO_EDGE as i32);

                gl.bind_texture(TEXTURE_2D, None);

                // Create new depth texture (R32F format for high precision depth)
                let depth_texture = gl
                    .create_texture()
                    .map_err(|e| format!("Failed to recreate depth texture: {}", e))
                    .unwrap();

                gl.bind_texture(TEXTURE_2D, Some(depth_texture));

                // Use R32F for depth texture
                gl.tex_storage_2d(TEXTURE_2D, 1, R32F, width, height);

                // Set texture parameters
                gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MIN_FILTER, NEAREST as i32);
                gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MAG_FILTER, NEAREST as i32);
                gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_S, CLAMP_TO_EDGE as i32);
                gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_T, CLAMP_TO_EDGE as i32);

                gl.bind_texture(TEXTURE_2D, None);

                gl_state.output_texture = output_texture;
                gl_state.depth_texture = depth_texture;
                gl_state.texture_width = width;
                gl_state.texture_height = height;
            }
        }
    }

    /// Helper: Blit texture to screen using a fullscreen quad (static version)
    unsafe fn blit_texture_to_screen_static(
        gl: &Context,
        gl_state: &ComputeTracerGl,
        texture: Texture,
        width: i32,
        height: i32,
    ) {
        unsafe {
            // Use blit shader program
            gl.use_program(Some(gl_state.blit_program));

            // Bind output texture
            gl.active_texture(TEXTURE0);
            gl.bind_texture(TEXTURE_2D, Some(texture));

            // Set texture uniform
            if let Some(loc) = &gl_state.blit_uniform_texture {
                gl.uniform_1_i32(Some(loc), 0);
            }

            // Set viewport
            gl.viewport(0, 0, width, height);

            // Draw fullscreen triangle (vertex shader generates positions)
            gl.bind_vertex_array(Some(gl_state.blit_vao));
            gl.draw_arrays(TRIANGLES, 0, 3);
            gl.bind_vertex_array(None);

            // Clean up
            gl.bind_texture(TEXTURE_2D, None);
            gl.use_program(None);
        }
    }

    /// Clean up GL resources
    ///
    /// # Safety
    /// Must be called with an active GL context. Should only be called once at shutdown.
    pub unsafe fn destroy_gl(&mut self, gl: &Context) {
        if let Some(gl_state) = self.gl_state.take() {
            unsafe {
                gl.delete_program(gl_state.compute_program);
                gl.delete_program(gl_state.blit_program);
                gl.delete_vertex_array(gl_state.blit_vao);
                gl.delete_texture(gl_state.output_texture);
                gl.delete_buffer(gl_state.octree_ssbo);
                gl.delete_buffer(gl_state.material_palette_ssbo);
            }
        }
    }
}

impl Renderer for ComputeTracer {
    fn render(&mut self, _width: u32, _height: u32, _time: f32) {
        panic!("ComputeTracer requires GL context. Use render_to_framebuffer() instead.");
    }

    fn render_with_camera(&mut self, _width: u32, _height: u32, _camera: &Camera) {
        panic!("ComputeTracer requires GL context. Use render_to_framebuffer() instead.");
    }

    fn name(&self) -> &str {
        "Compute Tracer"
    }

    fn supports_gl(&self) -> bool {
        true
    }

    fn init_gl(&mut self, gl: &Context) -> Result<(), String> {
        unsafe { ComputeTracer::init_gl(self, gl) }
    }

    fn destroy_gl(&mut self, gl: &Context) {
        unsafe {
            if let Some(gl_state) = self.gl_state.take() {
                gl.delete_program(gl_state.compute_program);
                gl.delete_program(gl_state.blit_program);
                gl.delete_vertex_array(gl_state.blit_vao);
                gl.delete_texture(gl_state.output_texture);
                gl.delete_buffer(gl_state.octree_ssbo);
                gl.delete_buffer(gl_state.material_palette_ssbo);
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

        // Flip Y-axis
        let mut flipped = vec![0u8; rgb_pixels.len()];
        for y in 0..height {
            let src_row = &rgb_pixels[(y * width * 3) as usize..((y + 1) * width * 3) as usize];
            let dst_y = height - 1 - y;
            let dst_row =
                &mut flipped[(dst_y * width * 3) as usize..((dst_y + 1) * width * 3) as usize];
            dst_row.copy_from_slice(src_row);
        }

        // Save to file
        image::save_buffer(path, &flipped, width, height, image::ColorType::Rgb8)
            .map_err(|e| e.to_string())
    }
}
