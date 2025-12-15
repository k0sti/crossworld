//! Simple GL mesh renderer for voxel cubes
//!
//! Renders triangle meshes generated from voxel octrees using standard
//! OpenGL vertex buffers and simple phong shading.

use cube::Cube;
use cube::material::get_material_color;
use cube::{DefaultMeshBuilder, generate_face_mesh};
use glow::*;
use std::rc::Rc;

use crate::renderer::{AMBIENT, CameraConfig, DIFFUSE_STRENGTH, Entity, LIGHT_DIR};
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
            self.render_mesh_with_depth(
                gl,
                mesh_index,
                position,
                rotation,
                camera,
                viewport_width,
                viewport_height,
                1, // Default depth
            )
        }
    }

    /// Render a mesh at given position with specified depth for scaling
    pub unsafe fn render_mesh_with_depth(
        &self,
        gl: &Context,
        mesh_index: usize,
        position: glam::Vec3,
        rotation: glam::Quat,
        camera: &CameraConfig,
        viewport_width: i32,
        viewport_height: i32,
        _depth: u32,
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

            // Calculate scale to transform mesh coordinates to [-1, 1] space
            // Mesh vertices are in [0, 1] space (visit_faces outputs normalized positions)
            // Transform: position_world = mesh_pos * 2 - 1
            // This maps [0, 1] -> [-1, 1]
            let mesh_scale = 2.0;
            let mesh_offset = glam::Vec3::splat(-1.0);

            // Model matrix: translate to position, rotate, then scale mesh from [0,1] to [-1,1]
            // Order of operations (right to left):
            // 1. Scale by 2.0: [0,1] -> [0,2]
            // 2. Translate by -1.0: [0,2] -> [-1,1]
            // 3. Apply rotation (if any)
            // 4. Translate to world position
            let model = glam::Mat4::from_translation(position)
                * glam::Mat4::from_quat(rotation)
                * glam::Mat4::from_translation(mesh_offset)
                * glam::Mat4::from_scale(glam::Vec3::splat(mesh_scale));

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

    /// Render a mesh for a given entity
    pub unsafe fn render_entity(
        &self,
        gl: &Context,
        mesh_index: usize,
        entity: &dyn Entity,
        camera: &CameraConfig,
        viewport_width: i32,
        viewport_height: i32,
    ) {
        unsafe {
            self.render_mesh(
                gl,
                mesh_index,
                entity.position(),
                entity.rotation(),
                camera,
                viewport_width,
                viewport_height,
            )
        }
    }

    /// Render a mesh for a given entity with specified depth
    pub unsafe fn render_entity_with_depth(
        &self,
        gl: &Context,
        mesh_index: usize,
        entity: &dyn Entity,
        camera: &CameraConfig,
        viewport_width: i32,
        viewport_height: i32,
        depth: u32,
    ) {
        unsafe {
            self.render_mesh_with_depth(
                gl,
                mesh_index,
                entity.position(),
                entity.rotation(),
                camera,
                viewport_width,
                viewport_height,
                depth,
            )
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
