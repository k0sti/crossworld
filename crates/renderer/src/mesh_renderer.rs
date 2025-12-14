//! Simple GL mesh renderer for voxel cubes
//!
//! Renders triangle meshes generated from voxel octrees using standard
//! OpenGL vertex buffers and simple phong shading.

use cube::{generate_face_mesh, DefaultMeshBuilder};
use cube::Cube;
use glow::*;
use std::rc::Rc;

use crate::renderer::{CameraConfig, LIGHT_DIR, AMBIENT, DIFFUSE_STRENGTH};
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

impl MeshRenderer {
    pub fn new() -> Self {
        Self {
            program: None,
            meshes: Vec::new(),
        }
    }

    /// Initialize GL resources (must be called with active GL context)
    pub unsafe fn init_gl(&mut self, gl: &Context) -> Result<(), String> {
        unsafe {
            // Create shader program
            let program = create_program(gl, VERTEX_SHADER, FRAGMENT_SHADER)?;
            self.program = Some(program);

            Ok(())
        }
    }

    /// Upload a mesh from a voxel cube
    pub unsafe fn upload_mesh(
        &mut self,
        gl: &Context,
        cube: &Rc<Cube<u8>>,
        depth: u32,
    ) -> Result<usize, String> {
        unsafe {
            // Generate mesh from cube
            let mut builder = DefaultMeshBuilder::new();

            // Simple color mapper: voxel id to RGB
            let color_fn = |material_id: u8| -> [f32; 3] {
                // Use a simple HSV-based color mapping
                let hue = (material_id as f32 / 255.0) * 360.0;
                hsv_to_rgb(hue, 0.7, 0.9)
            };

            // No border materials for now
            let border_materials = [0, 0, 0, 0];

            generate_face_mesh(cube, &mut builder, color_fn, depth, border_materials, depth);

            if builder.indices.is_empty() {
                return Err("Generated mesh has no faces".to_string());
            }

            // Create VAO
            let vao = gl.create_vertex_array()
                .map_err(|e| format!("Failed to create VAO: {}", e))?;
            gl.bind_vertex_array(Some(vao));

            // Create and upload VBO
            let vbo = gl.create_buffer()
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
            let ebo = gl.create_buffer()
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

    /// Render a mesh at given position
    pub unsafe fn render_mesh(
        &self,
        gl: &Context,
        mesh_index: usize,
        position: glam::Vec3,
        rotation: glam::Quat,
        camera: &CameraConfig,
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

            // Calculate matrices
            let aspect = viewport_width as f32 / viewport_height as f32;
            let projection = glam::Mat4::perspective_rh(camera.vfov, aspect, 0.1, 1000.0);

            let view = glam::Mat4::from_rotation_translation(
                camera.rotation.conjugate(),
                -camera.position,
            );

            let model = glam::Mat4::from_rotation_translation(rotation, position);

            // Upload uniforms
            let mvp = projection * view * model;
            let mvp_loc = gl.get_uniform_location(program, "uMVP");
            gl.uniform_matrix_4_f32_slice(mvp_loc.as_ref(), false, mvp.as_ref());

            let model_loc = gl.get_uniform_location(program, "uModel");
            gl.uniform_matrix_4_f32_slice(model_loc.as_ref(), false, model.as_ref());

            let light_dir_loc = gl.get_uniform_location(program, "uLightDir");
            gl.uniform_3_f32(light_dir_loc.as_ref(), LIGHT_DIR.x, LIGHT_DIR.y, LIGHT_DIR.z);

            let ambient_loc = gl.get_uniform_location(program, "uAmbient");
            gl.uniform_1_f32(ambient_loc.as_ref(), AMBIENT);

            let diffuse_strength_loc = gl.get_uniform_location(program, "uDiffuseStrength");
            gl.uniform_1_f32(diffuse_strength_loc.as_ref(), DIFFUSE_STRENGTH);

            // Draw mesh
            gl.bind_vertex_array(Some(mesh.vao));
            gl.draw_elements(TRIANGLES, mesh.index_count, UNSIGNED_INT, 0);
            gl.bind_vertex_array(None);

            gl.disable(DEPTH_TEST);
        }
    }

    /// Cleanup GL resources
    pub unsafe fn destroy_gl(&mut self, gl: &Context) {
        unsafe {
            for mesh in &self.meshes {
                gl.delete_vertex_array(mesh.vao);
                gl.delete_buffer(mesh.vbo);
                gl.delete_buffer(mesh.ebo);
            }
            self.meshes.clear();

            if let Some(program) = self.program.take() {
                gl.delete_program(program);
            }
        }
    }
}

/// Convert HSV to RGB color
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = match h as i32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    [r + m, g + m, b + m]
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
    FragColor = vec4(lighting, 1.0);
}
"#;
