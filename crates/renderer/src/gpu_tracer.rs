//! Compute shader octree raytracer
//!
//! This tracer uses OpenGL compute shaders (GL 4.3+) for high-performance parallel raytracing.
//! Compute shaders allow for more flexible GPU programming compared to fragment shaders.
//!
//! **Phase 1**: Basic ray-cube bounding box intersection (current implementation)
//! **Future**: Full octree traversal in compute shader

use crate::renderer::*;
use crate::shader_utils::{create_compute_program, create_program};
use cube::Cube;
use glow::*;
use std::rc::Rc;

const COMPUTE_SHADER_SOURCE: &str = include_str!("shaders/basic_raycast.comp");

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

/// Compute shader raytracer
pub struct GpuTracer {
    cube: Rc<Cube<i32>>,
    #[allow(dead_code)]
    bounds: CubeBounds,
    gl_state: Option<GpuTracerGl>,
}

/// OpenGL resources for compute shader raytracing
struct GpuTracerGl {
    compute_program: Program,
    blit_program: Program,
    blit_vao: VertexArray,
    output_texture: Texture,
    texture_width: i32,
    texture_height: i32,
    uniform_resolution: Option<UniformLocation>,
    uniform_time: Option<UniformLocation>,
    uniform_camera_pos: Option<UniformLocation>,
    uniform_camera_rot: Option<UniformLocation>,
    uniform_use_camera: Option<UniformLocation>,
    blit_uniform_texture: Option<UniformLocation>,
}

impl GpuTracer {
    pub fn new(cube: Rc<Cube<i32>>) -> Self {
        Self {
            cube,
            bounds: CubeBounds::default(),
            gl_state: None,
        }
    }

    /// Initialize OpenGL compute shader resources
    /// Must be called with an active GL context that supports compute shaders (GL 4.3+)
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

            println!("[GPU Tracer] Uniform locations:");
            println!("  uResolution: {:?}", uniform_resolution);
            println!("  uTime: {:?}", uniform_time);
            println!("  uCameraPos: {:?}", uniform_camera_pos);
            println!("  uCameraRot: {:?}", uniform_camera_rot);
            println!("  uUseCamera: {:?}", uniform_use_camera);

            // Compile blit shader program (for displaying compute shader output)
            let blit_program = create_program(gl, BLIT_VERTEX_SHADER, BLIT_FRAGMENT_SHADER)?;
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

            self.gl_state = Some(GpuTracerGl {
                compute_program,
                blit_program,
                blit_vao,
                output_texture,
                texture_width: 0,
                texture_height: 0,
                uniform_resolution,
                uniform_time,
                uniform_camera_pos,
                uniform_camera_rot,
                uniform_use_camera,
                blit_uniform_texture,
            });

            Ok(())
        }
    }

    /// Get reference to the cube
    pub fn cube(&self) -> &Rc<Cube<i32>> {
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
    ) -> Result<Option<cube::Hit<i32>>, String> {
        Ok(cube::raycast(&self.cube, pos, dir, None))
    }

    /// Render to OpenGL context using compute shader with time-based orbit camera
    pub unsafe fn render_to_gl(&mut self, gl: &Context, width: i32, height: i32, time: f32) {
        let Some(gl_state) = &mut self.gl_state else {
            return;
        };

        unsafe {
            // Recreate texture with immutable storage if size changed
            if gl_state.texture_width != width || gl_state.texture_height != height {
                // Delete old texture
                gl.delete_texture(gl_state.output_texture);

                // Create new texture
                let output_texture = gl
                    .create_texture()
                    .map_err(|e| format!("Failed to recreate texture: {}", e))
                    .unwrap();

                gl.bind_texture(TEXTURE_2D, Some(output_texture));

                // Use tex_storage_2d for immutable storage (required for glBindImageTexture)
                gl.tex_storage_2d(
                    TEXTURE_2D, 1, // 1 mipmap level
                    RGBA8, width, height,
                );

                // Set texture parameters
                gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MIN_FILTER, NEAREST as i32);
                gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MAG_FILTER, NEAREST as i32);
                gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_S, CLAMP_TO_EDGE as i32);
                gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_T, CLAMP_TO_EDGE as i32);

                // Unbind texture
                gl.bind_texture(TEXTURE_2D, None);

                // Update stored texture and size
                gl_state.output_texture = output_texture;
                gl_state.texture_width = width;
                gl_state.texture_height = height;
            }

            // Use compute shader program
            gl.use_program(Some(gl_state.compute_program));

            // Bind output texture as image
            gl.bind_image_texture(0, gl_state.output_texture, 0, false, 0, WRITE_ONLY, RGBA8);

            // Set uniforms
            if let Some(loc) = gl_state.uniform_resolution {
                gl.uniform_2_f32(Some(&loc), width as f32, height as f32);
            }
            if let Some(loc) = gl_state.uniform_time {
                gl.uniform_1_f32(Some(&loc), time);
            }
            if let Some(loc) = gl_state.uniform_use_camera {
                gl.uniform_1_i32(Some(&loc), 0); // Use time-based orbit camera
            }

            // Dispatch compute shader (8x8 local work group size)
            let work_groups_x = (width + 7) / 8;
            let work_groups_y = (height + 7) / 8;
            gl.dispatch_compute(work_groups_x as u32, work_groups_y as u32, 1);

            // Wait for compute shader to finish
            gl.memory_barrier(SHADER_IMAGE_ACCESS_BARRIER_BIT);

            // Blit output texture to screen (simple fullscreen quad)
            let output_texture = gl_state.output_texture; // Copy texture handle to avoid borrow conflict
            self.blit_texture_to_screen(gl, output_texture, width, height);
        }
    }

    /// Render to OpenGL context with explicit camera using compute shader
    pub unsafe fn render_to_gl_with_camera(
        &mut self,
        gl: &Context,
        width: i32,
        height: i32,
        camera: &CameraConfig,
    ) {
        let Some(gl_state) = &mut self.gl_state else {
            return;
        };

        unsafe {
            // Recreate texture with immutable storage if size changed
            if gl_state.texture_width != width || gl_state.texture_height != height {
                // Delete old texture
                gl.delete_texture(gl_state.output_texture);

                // Create new texture
                let output_texture = gl
                    .create_texture()
                    .map_err(|e| format!("Failed to recreate texture: {}", e))
                    .unwrap();

                gl.bind_texture(TEXTURE_2D, Some(output_texture));

                // Use tex_storage_2d for immutable storage (required for glBindImageTexture)
                gl.tex_storage_2d(
                    TEXTURE_2D, 1, // 1 mipmap level
                    RGBA8, width, height,
                );

                // Set texture parameters
                gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MIN_FILTER, NEAREST as i32);
                gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MAG_FILTER, NEAREST as i32);
                gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_S, CLAMP_TO_EDGE as i32);
                gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_T, CLAMP_TO_EDGE as i32);

                // Unbind texture
                gl.bind_texture(TEXTURE_2D, None);

                // Update stored texture and size
                gl_state.output_texture = output_texture;
                gl_state.texture_width = width;
                gl_state.texture_height = height;
            }

            // Use compute shader program
            gl.use_program(Some(gl_state.compute_program));

            // Bind output texture as image
            gl.bind_image_texture(0, gl_state.output_texture, 0, false, 0, WRITE_ONLY, RGBA8);

            // Set uniforms
            if let Some(loc) = gl_state.uniform_resolution {
                gl.uniform_2_f32(Some(&loc), width as f32, height as f32);
            }
            if let Some(loc) = gl_state.uniform_camera_pos {
                gl.uniform_3_f32(
                    Some(&loc),
                    camera.position.x,
                    camera.position.y,
                    camera.position.z,
                );
            }
            if let Some(loc) = gl_state.uniform_camera_rot {
                gl.uniform_4_f32(
                    Some(&loc),
                    camera.rotation.x,
                    camera.rotation.y,
                    camera.rotation.z,
                    camera.rotation.w,
                );
            }
            if let Some(loc) = gl_state.uniform_use_camera {
                gl.uniform_1_i32(Some(&loc), 1); // Use explicit camera
            }

            // Dispatch compute shader (8x8 local work group size)
            let work_groups_x = (width + 7) / 8;
            let work_groups_y = (height + 7) / 8;
            gl.dispatch_compute(work_groups_x as u32, work_groups_y as u32, 1);

            // Wait for compute shader to finish
            gl.memory_barrier(SHADER_IMAGE_ACCESS_BARRIER_BIT);

            // Blit output texture to screen
            let output_texture = gl_state.output_texture; // Copy texture handle to avoid borrow conflict
            self.blit_texture_to_screen(gl, output_texture, width, height);
        }
    }

    /// Helper: Blit texture to screen using a fullscreen quad
    unsafe fn blit_texture_to_screen(
        &self,
        gl: &Context,
        texture: Texture,
        width: i32,
        height: i32,
    ) {
        let Some(gl_state) = &self.gl_state else {
            return;
        };

        unsafe {
            // Use blit shader program
            gl.use_program(Some(gl_state.blit_program));

            // Bind output texture
            gl.active_texture(TEXTURE0);
            gl.bind_texture(TEXTURE_2D, Some(texture));

            // Set texture uniform
            if let Some(loc) = gl_state.blit_uniform_texture {
                gl.uniform_1_i32(Some(&loc), 0);
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
    pub unsafe fn destroy_gl(&mut self, gl: &Context) {
        if let Some(gl_state) = self.gl_state.take() {
            unsafe {
                gl.delete_program(gl_state.compute_program);
                gl.delete_program(gl_state.blit_program);
                gl.delete_vertex_array(gl_state.blit_vao);
                gl.delete_texture(gl_state.output_texture);
            }
        }
    }
}

impl Renderer for GpuTracer {
    fn render(&mut self, _width: u32, _height: u32, _time: f32) {
        // Note: Compute shader rendering requires GL context
        // Use render_to_gl() method instead
    }

    fn render_with_camera(&mut self, _width: u32, _height: u32, _camera: &CameraConfig) {
        // Note: Compute shader rendering requires GL context
        // Use render_to_gl_with_camera() method instead
    }

    fn name(&self) -> &str {
        "GpuTracer (Compute Shader - Phase 1: Ray-Cube Intersection)"
    }
}
