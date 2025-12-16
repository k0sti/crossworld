//! Simple GL mesh renderer for voxel cubes
//!
//! Renders triangle meshes generated from voxel octrees using standard
//! OpenGL vertex buffers and simple phong shading.

use cube::Cube;
use cube::material::get_material_color;
use cube::{DefaultMeshBuilder, generate_face_mesh};
use glow::*;
use std::rc::Rc;

use crate::renderer::{AMBIENT, Camera, DIFFUSE_STRENGTH, LIGHT_DIR, Object};
use crate::shader_utils::create_program;

/// Compiled mesh ready for GL rendering
pub struct GlMesh {
    vao: NativeVertexArray,
    vbo: NativeBuffer,
    ebo: NativeBuffer,
    index_count: i32,
}

/// Simple mesh renderer for voxel cubes
pub struct MeshRenderer {
    program: Option<NativeProgram>,
    meshes: Vec<GlMesh>,
}

impl Default for MeshRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl MeshRenderer {
    pub fn new() -> Self {
        Self {
            program: None,
            meshes: Vec::new(),
        }
    }

    /// Initialize GL resources
    ///
    /// # Safety
    ///
    /// Must be called with an active GL context on the current thread.
    pub unsafe fn init_gl(&mut self, gl: &Context) -> Result<(), String> {
        unsafe {
            // Create shader program
            let program = create_program(gl, VERTEX_SHADER, FRAGMENT_SHADER)?;
            self.program = Some(program);

            Ok(())
        }
    }

    /// Upload a mesh from a voxel cube
    ///
    /// # Safety
    ///
    /// Must be called with an active GL context on the current thread.
    pub unsafe fn upload_mesh(
        &mut self,
        gl: &Context,
        cube: &Rc<Cube<u8>>,
        depth: u32,
    ) -> Result<usize, String> {
        unsafe {
            // Generate mesh from cube
            let mut builder = DefaultMeshBuilder::new();

            // Use the same material palette as the raytracers
            let color_fn = |material_id: u8| -> [f32; 3] {
                let color = get_material_color(material_id as i32);
                [color.x, color.y, color.z]
            };

            // No border materials for now
            let border_materials = [0, 0, 0, 0];

            generate_face_mesh(cube, &mut builder, color_fn, border_materials, depth);

            if builder.indices.is_empty() {
                return Err("Generated mesh has no faces".to_string());
            }

            // Create VAO
            let vao = gl
                .create_vertex_array()
                .map_err(|e| format!("Failed to create VAO: {}", e))?;
            gl.bind_vertex_array(Some(vao));

            // Create and upload VBO
            let vbo = gl
                .create_buffer()
                .map_err(|e| format!("Failed to create VBO: {}", e))?;
            gl.bind_buffer(ARRAY_BUFFER, Some(vbo));

            // Interleave vertex data: [position(3), normal(3), color(3)]
            let mut vertex_data = Vec::new();
            let vertex_count = builder.vertices.len() / 3;
            for i in 0..vertex_count {
                // Position
                vertex_data.push(builder.vertices[i * 3]);
                vertex_data.push(builder.vertices[i * 3 + 1]);
                vertex_data.push(builder.vertices[i * 3 + 2]);
                // Normal
                vertex_data.push(builder.normals[i * 3]);
                vertex_data.push(builder.normals[i * 3 + 1]);
                vertex_data.push(builder.normals[i * 3 + 2]);
                // Color
                vertex_data.push(builder.colors[i * 3]);
                vertex_data.push(builder.colors[i * 3 + 1]);
                vertex_data.push(builder.colors[i * 3 + 2]);
            }

            gl.buffer_data_u8_slice(
                ARRAY_BUFFER,
                bytemuck::cast_slice(&vertex_data),
                STATIC_DRAW,
            );

            // Position attribute (location 0)
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(
                0,
                3,
                FLOAT,
                false,
                9 * std::mem::size_of::<f32>() as i32,
                0,
            );

            // Normal attribute (location 1)
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(
                1,
                3,
                FLOAT,
                false,
                9 * std::mem::size_of::<f32>() as i32,
                3 * std::mem::size_of::<f32>() as i32,
            );

            // Color attribute (location 2)
            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(
                2,
                3,
                FLOAT,
                false,
                9 * std::mem::size_of::<f32>() as i32,
                6 * std::mem::size_of::<f32>() as i32,
            );

            // Create and upload EBO
            let ebo = gl
                .create_buffer()
                .map_err(|e| format!("Failed to create EBO: {}", e))?;
            gl.bind_buffer(ELEMENT_ARRAY_BUFFER, Some(ebo));
            gl.buffer_data_u8_slice(
                ELEMENT_ARRAY_BUFFER,
                bytemuck::cast_slice(&builder.indices),
                STATIC_DRAW,
            );

            gl.bind_vertex_array(None);

            let mesh = GlMesh {
                vao,
                vbo,
                ebo,
                index_count: builder.indices.len() as i32,
            };

            self.meshes.push(mesh);
            Ok(self.meshes.len() - 1)
        }
    }

    /// Render a mesh at given position with default scale of 1.0
    ///
    /// # Safety
    ///
    /// Must be called with an active GL context on the current thread.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn render_mesh(
        &self,
        gl: &Context,
        mesh_index: usize,
        position: glam::Vec3,
        rotation: glam::Quat,
        camera: &Camera,
        viewport_width: i32,
        viewport_height: i32,
    ) {
        unsafe {
            self.render_mesh_with_scale(
                gl,
                mesh_index,
                position,
                rotation,
                1.0, // Default scale
                camera,
                viewport_width,
                viewport_height,
            )
        }
    }

    /// Render a mesh at given position with specified scale
    ///
    /// # Safety
    ///
    /// Must be called with an active GL context on the current thread.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn render_mesh_with_scale(
        &self,
        gl: &Context,
        mesh_index: usize,
        position: glam::Vec3,
        rotation: glam::Quat,
        scale: f32,
        camera: &Camera,
        viewport_width: i32,
        viewport_height: i32,
    ) {
        unsafe {
            let Some(program) = self.program else { return };
            if mesh_index >= self.meshes.len() {
                return;
            }

            let mesh = &self.meshes[mesh_index];

            gl.use_program(Some(program));
            gl.enable(DEPTH_TEST);
            gl.depth_func(LESS);

            // Enable backface culling for proper rendering
            // Voxel faces should be counter-clockwise when viewed from outside
            gl.enable(CULL_FACE);
            gl.cull_face(BACK);
            gl.front_face(CCW);

            // Calculate matrices
            let aspect = viewport_width as f32 / viewport_height as f32;
            let projection = glam::Mat4::perspective_rh(camera.vfov, aspect, 0.1, 1000.0);

            // View matrix construction to match raytracer's camera EXACTLY
            // The raytracer rotates basis vectors: forward = rotate(-Z), up = rotate(+Y)
            // Use look_at_rh with a target point in the forward direction
            let forward = camera.rotation * glam::Vec3::NEG_Z;
            let up = camera.rotation * glam::Vec3::Y;
            let target = camera.position + forward;
            let view = glam::Mat4::look_at_rh(camera.position, target, up);

            // Mesh vertices are in [0, 1] space (visit_faces outputs normalized positions)
            // Transform to [-0.5, 0.5] centered, then scale by the object size
            // mesh_offset centers the mesh at origin
            let mesh_offset = glam::Vec3::splat(-0.5);

            // Model matrix: translate to position, rotate, then scale mesh
            // Order of operations (right to left):
            // 1. Offset by -0.5: [0,1] -> [-0.5, 0.5] (centered)
            // 2. Scale by object scale
            // 3. Apply rotation (if any)
            // 4. Translate to world position
            let model = glam::Mat4::from_translation(position)
                * glam::Mat4::from_quat(rotation)
                * glam::Mat4::from_scale(glam::Vec3::splat(scale))
                * glam::Mat4::from_translation(mesh_offset);

            // Upload uniforms
            let mvp = projection * view * model;
            let mvp_loc = gl.get_uniform_location(program, "uMVP");
            gl.uniform_matrix_4_f32_slice(mvp_loc.as_ref(), false, mvp.as_ref());

            let model_loc = gl.get_uniform_location(program, "uModel");
            gl.uniform_matrix_4_f32_slice(model_loc.as_ref(), false, model.as_ref());

            let light_dir_loc = gl.get_uniform_location(program, "uLightDir");
            gl.uniform_3_f32(
                light_dir_loc.as_ref(),
                LIGHT_DIR.x,
                LIGHT_DIR.y,
                LIGHT_DIR.z,
            );

            let ambient_loc = gl.get_uniform_location(program, "uAmbient");
            gl.uniform_1_f32(ambient_loc.as_ref(), AMBIENT);

            let diffuse_strength_loc = gl.get_uniform_location(program, "uDiffuseStrength");
            gl.uniform_1_f32(diffuse_strength_loc.as_ref(), DIFFUSE_STRENGTH);

            // Draw mesh
            gl.bind_vertex_array(Some(mesh.vao));
            gl.draw_elements(TRIANGLES, mesh.index_count, UNSIGNED_INT, 0);
            gl.bind_vertex_array(None);

            // Restore GL state
            gl.disable(DEPTH_TEST);
            gl.disable(CULL_FACE);
        }
    }

    /// Render a mesh for a given object
    ///
    /// # Safety
    ///
    /// Must be called with an active GL context on the current thread.
    pub unsafe fn render_object(
        &self,
        gl: &Context,
        mesh_index: usize,
        object: &dyn Object,
        camera: &Camera,
        viewport_width: i32,
        viewport_height: i32,
    ) {
        unsafe {
            self.render_mesh(
                gl,
                mesh_index,
                object.position(),
                object.rotation(),
                camera,
                viewport_width,
                viewport_height,
            )
        }
    }

    /// Render a mesh for a given object with specified scale
    ///
    /// # Safety
    ///
    /// Must be called with an active GL context on the current thread.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn render_object_with_scale(
        &self,
        gl: &Context,
        mesh_index: usize,
        object: &dyn Object,
        scale: f32,
        camera: &Camera,
        viewport_width: i32,
        viewport_height: i32,
    ) {
        unsafe {
            self.render_mesh_with_scale(
                gl,
                mesh_index,
                object.position(),
                object.rotation(),
                scale,
                camera,
                viewport_width,
                viewport_height,
            )
        }
    }

    /// Clear all uploaded meshes (keeps shader program)
    ///
    /// # Safety
    ///
    /// Must be called with an active GL context on the current thread.
    pub unsafe fn clear_meshes(&mut self, gl: &Context) {
        unsafe {
            for mesh in &self.meshes {
                gl.delete_vertex_array(mesh.vao);
                gl.delete_buffer(mesh.vbo);
                gl.delete_buffer(mesh.ebo);
            }
            self.meshes.clear();
        }
    }

    /// Cleanup GL resources
    ///
    /// # Safety
    ///
    /// Must be called with an active GL context on the current thread.
    pub unsafe fn destroy_gl(&mut self, gl: &Context) {
        unsafe {
            self.clear_meshes(gl);

            if let Some(program) = self.program.take() {
                gl.delete_program(program);
            }
        }
    }
}

const VERTEX_SHADER: &str = r#"#version 300 es
precision highp float;

layout(location = 0) in vec3 aPosition;
layout(location = 1) in vec3 aNormal;
layout(location = 2) in vec3 aColor;

uniform mat4 uMVP;
uniform mat4 uModel;

out vec3 vNormal;
out vec3 vColor;

void main() {
    gl_Position = uMVP * vec4(aPosition, 1.0);

    // Transform normal to world space
    vNormal = mat3(uModel) * aNormal;
    vColor = aColor;
}
"#;

const FRAGMENT_SHADER: &str = r#"#version 300 es
precision highp float;

in vec3 vNormal;
in vec3 vColor;

uniform vec3 uLightDir;
uniform float uAmbient;
uniform float uDiffuseStrength;

out vec4 FragColor;

void main() {
    vec3 normal = normalize(vNormal);
    float diffuse = max(dot(normal, uLightDir), 0.0);
    vec3 lighting = vColor * (uAmbient + diffuse * uDiffuseStrength);

    // Gamma correction to match CPU tracer output
    vec3 gammaCorrected = pow(lighting, vec3(1.0 / 2.2));

    FragColor = vec4(gammaCorrected, 1.0);
}
"#;

impl crate::renderer::Renderer for MeshRenderer {
    fn render(&mut self, _width: u32, _height: u32, _time: f32) {
        panic!("MeshRenderer requires GL context. Use render_to_framebuffer() instead.");
    }

    fn render_with_camera(&mut self, _width: u32, _height: u32, _camera: &crate::renderer::Camera) {
        panic!("MeshRenderer requires GL context. Use render_to_framebuffer() instead.");
    }

    fn name(&self) -> &str {
        "Mesh Renderer"
    }

    fn supports_gl(&self) -> bool {
        true
    }

    fn init_gl(&mut self, gl: &Context) -> Result<(), String> {
        unsafe { MeshRenderer::init_gl(self, gl) }
    }

    fn destroy_gl(&mut self, gl: &Context) {
        unsafe { MeshRenderer::destroy_gl(self, gl) }
    }

    fn render_to_framebuffer(
        &mut self,
        _gl: &Context,
        _width: u32,
        _height: u32,
        _camera: Option<&crate::renderer::Camera>,
        _time: Option<f32>,
    ) -> Result<(), String> {
        Err("MeshRenderer requires explicit mesh index, position, and rotation. Use render_mesh() or render_mesh_with_scale() instead.".to_string())
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
                glow::PixelPackData::Slice(&mut pixels),
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
            let dst_row = &mut flipped[(dst_y * width * 3) as usize..((dst_y + 1) * width * 3) as usize];
            dst_row.copy_from_slice(src_row);
        }

        // Save to file
        image::save_buffer(path, &flipped, width, height, image::ColorType::Rgb8)
            .map_err(|e| e.to_string())
    }
}
