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
    /// Wireframe box mesh for bounding box rendering
    wireframe_box: Option<GlMesh>,
    /// 2D shader program for screen-space rendering
    program_2d: Option<NativeProgram>,
    /// Axis arrows mesh for 3D gizmo
    axis_arrows: Option<GlMesh>,
}

impl Default for MeshRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Depth test mode for two-pass wireframe rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireframeDepthMode {
    /// Render behind geometry (depth test GREATER) - for occluded/inside parts
    Behind,
    /// Render in front of geometry (depth test LESS) - for visible parts
    InFront,
    /// Render always (no depth test) - original behavior
    Always,
}

impl MeshRenderer {
    pub fn new() -> Self {
        Self {
            program: None,
            meshes: Vec::new(),
            wireframe_box: None,
            program_2d: None,
            axis_arrows: None,
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

            // Create 2D shader program for screen-space rendering
            let program_2d = create_program(gl, VERTEX_SHADER_2D, FRAGMENT_SHADER_2D)?;
            self.program_2d = Some(program_2d);

            // Create wireframe box mesh (unit cube from 0 to 1)
            self.wireframe_box = Some(self.create_wireframe_box(gl)?);

            // Create axis arrows mesh for 3D gizmo
            self.axis_arrows = Some(self.create_axis_arrows(gl)?);

            Ok(())
        }
    }

    /// Create a wireframe box mesh (12 edges of a unit cube)
    unsafe fn create_wireframe_box(&self, gl: &Context) -> Result<GlMesh, String> {
        unsafe {
            // 8 vertices of a unit cube [0, 1]³
            // Each vertex: position (3) + normal (3) + color (3) = 9 floats
            let white = [1.0f32, 1.0, 1.0];
            let normal = [0.0f32, 1.0, 0.0]; // Dummy normal for wireframe

            #[rustfmt::skip]
            let vertices: Vec<f32> = vec![
                // Position          Normal       Color
                0.0, 0.0, 0.0,  normal[0], normal[1], normal[2],  white[0], white[1], white[2], // 0: front-bottom-left
                1.0, 0.0, 0.0,  normal[0], normal[1], normal[2],  white[0], white[1], white[2], // 1: front-bottom-right
                1.0, 1.0, 0.0,  normal[0], normal[1], normal[2],  white[0], white[1], white[2], // 2: front-top-right
                0.0, 1.0, 0.0,  normal[0], normal[1], normal[2],  white[0], white[1], white[2], // 3: front-top-left
                0.0, 0.0, 1.0,  normal[0], normal[1], normal[2],  white[0], white[1], white[2], // 4: back-bottom-left
                1.0, 0.0, 1.0,  normal[0], normal[1], normal[2],  white[0], white[1], white[2], // 5: back-bottom-right
                1.0, 1.0, 1.0,  normal[0], normal[1], normal[2],  white[0], white[1], white[2], // 6: back-top-right
                0.0, 1.0, 1.0,  normal[0], normal[1], normal[2],  white[0], white[1], white[2], // 7: back-top-left
            ];

            // 12 edges as line pairs (24 indices)
            #[rustfmt::skip]
            let indices: Vec<u32> = vec![
                // Front face edges
                0, 1,  1, 2,  2, 3,  3, 0,
                // Back face edges
                4, 5,  5, 6,  6, 7,  7, 4,
                // Connecting edges
                0, 4,  1, 5,  2, 6,  3, 7,
            ];

            let vao = gl
                .create_vertex_array()
                .map_err(|e| format!("Failed to create VAO: {}", e))?;
            gl.bind_vertex_array(Some(vao));

            let vbo = gl
                .create_buffer()
                .map_err(|e| format!("Failed to create VBO: {}", e))?;
            gl.bind_buffer(ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(ARRAY_BUFFER, bytemuck::cast_slice(&vertices), STATIC_DRAW);

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

            let ebo = gl
                .create_buffer()
                .map_err(|e| format!("Failed to create EBO: {}", e))?;
            gl.bind_buffer(ELEMENT_ARRAY_BUFFER, Some(ebo));
            gl.buffer_data_u8_slice(
                ELEMENT_ARRAY_BUFFER,
                bytemuck::cast_slice(&indices),
                STATIC_DRAW,
            );

            gl.bind_vertex_array(None);

            Ok(GlMesh {
                vao,
                vbo,
                ebo,
                index_count: indices.len() as i32,
            })
        }
    }

    /// Create axis arrows mesh for 3D gizmo (X=red, Y=green, Z=blue)
    unsafe fn create_axis_arrows(&self, gl: &Context) -> Result<GlMesh, String> {
        unsafe {
            // Each axis arrow: origin to tip line + small arrowhead lines
            // Arrow length is 1.0 units
            let arrow_len = 1.0f32;
            let head_len = 0.15f32;
            let head_spread = 0.05f32;
            let normal = [0.0f32, 1.0, 0.0]; // Dummy normal

            // Colors for each axis
            let red = [1.0f32, 0.2, 0.2];
            let green = [0.2f32, 1.0, 0.2];
            let blue = [0.2f32, 0.2, 1.0];

            // Build vertices: position (3) + normal (3) + color (3) = 9 floats per vertex
            #[rustfmt::skip]
            let vertices: Vec<f32> = vec![
                // X axis (red) - main line
                0.0, 0.0, 0.0,  normal[0], normal[1], normal[2],  red[0], red[1], red[2], // 0
                arrow_len, 0.0, 0.0,  normal[0], normal[1], normal[2],  red[0], red[1], red[2], // 1
                // X axis arrowhead
                arrow_len - head_len, head_spread, 0.0,  normal[0], normal[1], normal[2],  red[0], red[1], red[2], // 2
                arrow_len - head_len, -head_spread, 0.0,  normal[0], normal[1], normal[2],  red[0], red[1], red[2], // 3
                arrow_len - head_len, 0.0, head_spread,  normal[0], normal[1], normal[2],  red[0], red[1], red[2], // 4
                arrow_len - head_len, 0.0, -head_spread,  normal[0], normal[1], normal[2],  red[0], red[1], red[2], // 5

                // Y axis (green) - main line
                0.0, 0.0, 0.0,  normal[0], normal[1], normal[2],  green[0], green[1], green[2], // 6
                0.0, arrow_len, 0.0,  normal[0], normal[1], normal[2],  green[0], green[1], green[2], // 7
                // Y axis arrowhead
                head_spread, arrow_len - head_len, 0.0,  normal[0], normal[1], normal[2],  green[0], green[1], green[2], // 8
                -head_spread, arrow_len - head_len, 0.0,  normal[0], normal[1], normal[2],  green[0], green[1], green[2], // 9
                0.0, arrow_len - head_len, head_spread,  normal[0], normal[1], normal[2],  green[0], green[1], green[2], // 10
                0.0, arrow_len - head_len, -head_spread,  normal[0], normal[1], normal[2],  green[0], green[1], green[2], // 11

                // Z axis (blue) - main line
                0.0, 0.0, 0.0,  normal[0], normal[1], normal[2],  blue[0], blue[1], blue[2], // 12
                0.0, 0.0, arrow_len,  normal[0], normal[1], normal[2],  blue[0], blue[1], blue[2], // 13
                // Z axis arrowhead
                head_spread, 0.0, arrow_len - head_len,  normal[0], normal[1], normal[2],  blue[0], blue[1], blue[2], // 14
                -head_spread, 0.0, arrow_len - head_len,  normal[0], normal[1], normal[2],  blue[0], blue[1], blue[2], // 15
                0.0, head_spread, arrow_len - head_len,  normal[0], normal[1], normal[2],  blue[0], blue[1], blue[2], // 16
                0.0, -head_spread, arrow_len - head_len,  normal[0], normal[1], normal[2],  blue[0], blue[1], blue[2], // 17
            ];

            // Line indices for each axis (main line + 4 arrowhead lines)
            #[rustfmt::skip]
            let indices: Vec<u32> = vec![
                // X axis
                0, 1,   // main line
                1, 2,  1, 3,  1, 4,  1, 5,  // arrowhead
                // Y axis
                6, 7,   // main line
                7, 8,  7, 9,  7, 10,  7, 11,  // arrowhead
                // Z axis
                12, 13,  // main line
                13, 14,  13, 15,  13, 16,  13, 17,  // arrowhead
            ];

            let vao = gl
                .create_vertex_array()
                .map_err(|e| format!("Failed to create VAO: {}", e))?;
            gl.bind_vertex_array(Some(vao));

            let vbo = gl
                .create_buffer()
                .map_err(|e| format!("Failed to create VBO: {}", e))?;
            gl.bind_buffer(ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(ARRAY_BUFFER, bytemuck::cast_slice(&vertices), STATIC_DRAW);

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

            let ebo = gl
                .create_buffer()
                .map_err(|e| format!("Failed to create EBO: {}", e))?;
            gl.bind_buffer(ELEMENT_ARRAY_BUFFER, Some(ebo));
            gl.buffer_data_u8_slice(
                ELEMENT_ARRAY_BUFFER,
                bytemuck::cast_slice(&indices),
                STATIC_DRAW,
            );

            gl.bind_vertex_array(None);

            Ok(GlMesh {
                vao,
                vbo,
                ebo,
                index_count: indices.len() as i32,
            })
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

    /// Render a mesh at given position with specified uniform scale
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
            self.render_mesh_with_options(
                gl,
                mesh_index,
                position,
                rotation,
                glam::Vec3::splat(scale), // Convert uniform scale to Vec3
                glam::Vec3::ONE,          // Default normalized_size (full octree)
                false,                    // wireframe
                camera,
                viewport_width,
                viewport_height,
            )
        }
    }

    /// Render a mesh at given position with specified uniform scale and normalized size (for centering)
    ///
    /// # Safety
    ///
    /// Must be called with an active GL context on the current thread.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn render_mesh_with_normalized_size(
        &self,
        gl: &Context,
        mesh_index: usize,
        position: glam::Vec3,
        rotation: glam::Quat,
        scale: f32,
        normalized_size: glam::Vec3,
        camera: &Camera,
        viewport_width: i32,
        viewport_height: i32,
    ) {
        unsafe {
            self.render_mesh_with_options(
                gl,
                mesh_index,
                position,
                rotation,
                glam::Vec3::splat(scale), // Convert uniform scale to Vec3
                normalized_size,
                false, // wireframe
                camera,
                viewport_width,
                viewport_height,
            )
        }
    }

    /// Render a mesh at given position with specified scale and wireframe option
    ///
    /// # Arguments
    /// * `scale` - Non-uniform scale to apply to the mesh (Vec3 for X/Y/Z independent scaling)
    /// * `normalized_size` - Model size as fraction of octree (for centering offset)
    ///
    /// # Safety
    ///
    /// Must be called with an active GL context on the current thread.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn render_mesh_with_options(
        &self,
        gl: &Context,
        mesh_index: usize,
        position: glam::Vec3,
        rotation: glam::Quat,
        scale: glam::Vec3,
        normalized_size: glam::Vec3,
        wireframe: bool,
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

            // Enable backface culling for proper rendering (disabled for wireframe)
            // Voxel faces should be counter-clockwise when viewed from outside
            if wireframe {
                gl.disable(CULL_FACE);
                gl.disable(DEPTH_TEST);
                gl.polygon_mode(FRONT_AND_BACK, LINE);
            } else {
                gl.enable(CULL_FACE);
                gl.cull_face(BACK);
                gl.front_face(CCW);
            }

            // Calculate matrices
            let aspect = viewport_width as f32 / viewport_height as f32;
            let projection = glam::Mat4::perspective_rh(camera.vfov, aspect, 1.0, 50000.0);

            // View matrix construction to match raytracer's camera EXACTLY
            // The raytracer rotates basis vectors: forward = rotate(-Z), up = rotate(+Y)
            // Use look_at_rh with a target point in the forward direction
            let forward = camera.rotation * glam::Vec3::NEG_Z;
            let up = camera.rotation * glam::Vec3::Y;
            let target = camera.position + forward;
            let view = glam::Mat4::look_at_rh(camera.position, target, up);

            // Transform mesh to match CubeBox position within mesh:
            // 1. The mesh uses [0, normalized_size] space
            // 2. To render mesh centered at 'position':
            // - Scale unit [0,1] mesh by normalized_size to get actual mesh bounds
            // - Apply -normalized_size/2 offset to center at origin
            // - Apply world scale (non-uniform supported)
            // - Apply rotation and translation
            //
            // Order of operations (right to left):
            // 1. Scale by normalized_size (mesh occupies [0, normalized_size])
            // 2. Offset to center: [0, normalized_size] -> [-normalized_size/2, normalized_size/2]
            // 3. Scale by world scale (non-uniform)
            // 4. Apply rotation
            // 5. Translate to world position
            let model = glam::Mat4::from_translation(position)
                * glam::Mat4::from_quat(rotation)
                * glam::Mat4::from_scale(scale)
                * glam::Mat4::from_translation(normalized_size * -0.5)
                * glam::Mat4::from_scale(normalized_size);

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
            if wireframe {
                gl.polygon_mode(FRONT_AND_BACK, FILL);
            }
        }
    }

    /// Render a wireframe bounding box around an object (uniform scale, full octree)
    ///
    /// This renders a wireframe around the full octree cube. For CubeBox models
    /// where the model only occupies part of the octree, use `render_cubebox_wireframe`.
    ///
    /// # Safety
    ///
    /// Must be called with an active GL context on the current thread.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn render_wireframe_box(
        &self,
        gl: &Context,
        position: glam::Vec3,
        rotation: glam::Quat,
        scale: f32,
        camera: &Camera,
        viewport_width: i32,
        viewport_height: i32,
    ) {
        unsafe {
            // Full octree: normalized_size = 1.0 on all axes
            self.render_cubebox_wireframe(
                gl,
                position,
                rotation,
                glam::Vec3::ONE,
                scale,
                camera,
                viewport_width,
                viewport_height,
            );
        }
    }

    /// Render a wireframe bounding box for a CubeBox model
    ///
    /// This method renders a wireframe around the actual model bounds within the octree,
    /// not the full power-of-2 octree cube.
    ///
    /// # Arguments
    /// * `normalized_size` - Model size as fraction of octree (model_size / octree_size)
    /// * `scale` - Uniform world scale applied to the mesh
    ///
    /// # Safety
    ///
    /// Must be called with an active GL context on the current thread.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn render_cubebox_wireframe(
        &self,
        gl: &Context,
        position: glam::Vec3,
        rotation: glam::Quat,
        normalized_size: glam::Vec3,
        scale: f32,
        camera: &Camera,
        viewport_width: i32,
        viewport_height: i32,
    ) {
        unsafe {
            self.render_cubebox_wireframe_colored(
                gl,
                position,
                rotation,
                normalized_size,
                scale,
                [1.0, 1.0, 1.0], // Default white color
                camera,
                viewport_width,
                viewport_height,
            );
        }
    }

    /// Render a wireframe bounding box with a specific color
    ///
    /// # Arguments
    /// * `normalized_size` - Model size as fraction of octree (model_size / octree_size)
    /// * `scale` - Uniform world scale applied to the mesh
    /// * `color` - RGB color for the wireframe [0.0-1.0]
    ///
    /// # Safety
    ///
    /// Must be called with an active GL context on the current thread.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn render_cubebox_wireframe_colored(
        &self,
        gl: &Context,
        position: glam::Vec3,
        rotation: glam::Quat,
        normalized_size: glam::Vec3,
        scale: f32,
        color: [f32; 3],
        camera: &Camera,
        viewport_width: i32,
        viewport_height: i32,
    ) {
        // Delegate to RGBA version with alpha = 1.0
        unsafe {
            self.render_cubebox_wireframe_colored_alpha(
                gl,
                position,
                rotation,
                normalized_size,
                scale,
                [color[0], color[1], color[2], 1.0],
                camera,
                viewport_width,
                viewport_height,
            )
        }
    }

    /// Render a colored wireframe box with alpha transparency support
    ///
    /// # Safety
    ///
    /// Must be called with an active GL context on the current thread.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn render_cubebox_wireframe_colored_alpha(
        &self,
        gl: &Context,
        position: glam::Vec3,
        rotation: glam::Quat,
        normalized_size: glam::Vec3,
        scale: f32,
        color: [f32; 4],
        camera: &Camera,
        viewport_width: i32,
        viewport_height: i32,
    ) {
        unsafe {
            let Some(program) = self.program else { return };
            let Some(ref wireframe_mesh) = self.wireframe_box else {
                return;
            };

            gl.use_program(Some(program));
            gl.disable(DEPTH_TEST); // Render without depth test (always visible)
            gl.depth_mask(false); // Don't write to depth buffer
            gl.disable(CULL_FACE);
            gl.line_width(1.0); // Thin wireframe lines

            // Enable blending for alpha transparency
            if color[3] < 1.0 {
                gl.enable(BLEND);
                gl.blend_func(SRC_ALPHA, ONE_MINUS_SRC_ALPHA);
            }

            // Calculate matrices
            let aspect = viewport_width as f32 / viewport_height as f32;
            let projection = glam::Mat4::perspective_rh(camera.vfov, aspect, 1.0, 50000.0);

            let forward = camera.rotation * glam::Vec3::NEG_Z;
            let up = camera.rotation * glam::Vec3::Y;
            let target = camera.position + forward;
            let view = glam::Mat4::look_at_rh(camera.position, target, up);

            // Transform wireframe to match CubeBox position within mesh:
            // 1. The mesh uses [0, normalized_size] space with -normalized_size/2 offset
            // 2. To render wireframe around the model:
            // - Scale unit [0,1] wireframe to normalized_size
            // - Apply same -normalized_size/2 offset as mesh
            // - Apply world scale and transforms
            let model = glam::Mat4::from_translation(position)
                * glam::Mat4::from_quat(rotation)
                * glam::Mat4::from_scale(glam::Vec3::splat(scale))
                * glam::Mat4::from_translation(normalized_size * -0.5)
                * glam::Mat4::from_scale(normalized_size);

            // Upload uniforms
            let mvp = projection * view * model;
            let mvp_loc = gl.get_uniform_location(program, "uMVP");
            gl.uniform_matrix_4_f32_slice(mvp_loc.as_ref(), false, mvp.as_ref());

            let model_loc = gl.get_uniform_location(program, "uModel");
            gl.uniform_matrix_4_f32_slice(model_loc.as_ref(), false, model.as_ref());

            // Full brightness for wireframe (no lighting)
            let ambient_loc = gl.get_uniform_location(program, "uAmbient");
            gl.uniform_1_f32(ambient_loc.as_ref(), 1.0);

            let diffuse_strength_loc = gl.get_uniform_location(program, "uDiffuseStrength");
            gl.uniform_1_f32(diffuse_strength_loc.as_ref(), 0.0);

            // Set wireframe color via vertex color override uniform (now RGBA)
            let color_override_loc = gl.get_uniform_location(program, "uColorOverride");
            gl.uniform_4_f32(
                color_override_loc.as_ref(),
                color[0],
                color[1],
                color[2],
                color[3],
            );

            let use_color_override_loc = gl.get_uniform_location(program, "uUseColorOverride");
            gl.uniform_1_i32(use_color_override_loc.as_ref(), 1);

            // Draw wireframe box using LINES
            gl.bind_vertex_array(Some(wireframe_mesh.vao));
            gl.draw_elements(LINES, wireframe_mesh.index_count, UNSIGNED_INT, 0);
            gl.bind_vertex_array(None);

            // Restore GL state
            gl.uniform_1_i32(use_color_override_loc.as_ref(), 0);
            gl.depth_mask(true); // Restore depth writing
            if color[3] < 1.0 {
                gl.disable(BLEND);
            }
        }
    }

    /// Render a colored wireframe box with depth testing support
    ///
    /// This allows two-pass rendering where lines behind geometry are rendered
    /// thinner than lines in front. Use `WireframeDepthMode::Behind` for the first
    /// pass (thinner, occluded) and `WireframeDepthMode::InFront` for the second
    /// pass (thicker, visible).
    ///
    /// # Safety
    ///
    /// Must be called with an active GL context on the current thread.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn render_cubebox_wireframe_depth(
        &self,
        gl: &Context,
        position: glam::Vec3,
        rotation: glam::Quat,
        normalized_size: glam::Vec3,
        scale: f32,
        color: [f32; 4],
        depth_mode: WireframeDepthMode,
        camera: &Camera,
        viewport_width: i32,
        viewport_height: i32,
    ) {
        unsafe {
            let Some(program) = self.program else { return };
            let Some(ref wireframe_mesh) = self.wireframe_box else {
                return;
            };

            gl.use_program(Some(program));
            gl.disable(CULL_FACE);
            gl.line_width(1.0); // Thin wireframe lines

            // Configure depth test based on mode
            match depth_mode {
                WireframeDepthMode::Behind => {
                    gl.enable(DEPTH_TEST);
                    gl.depth_func(GREATER); // Only render behind other geometry
                    gl.depth_mask(false); // Don't write to depth buffer
                }
                WireframeDepthMode::InFront => {
                    gl.enable(DEPTH_TEST);
                    gl.depth_func(LEQUAL); // Render in front of (or at same depth as) other geometry
                    gl.depth_mask(false); // Don't write to depth buffer
                }
                WireframeDepthMode::Always => {
                    gl.disable(DEPTH_TEST);
                    gl.depth_mask(false);
                }
            }

            // Enable blending for alpha transparency
            if color[3] < 1.0 {
                gl.enable(BLEND);
                gl.blend_func(SRC_ALPHA, ONE_MINUS_SRC_ALPHA);
            }

            // Calculate matrices
            let aspect = viewport_width as f32 / viewport_height as f32;
            let projection = glam::Mat4::perspective_rh(camera.vfov, aspect, 1.0, 50000.0);

            let forward = camera.rotation * glam::Vec3::NEG_Z;
            let up = camera.rotation * glam::Vec3::Y;
            let target = camera.position + forward;
            let view = glam::Mat4::look_at_rh(camera.position, target, up);

            let model = glam::Mat4::from_translation(position)
                * glam::Mat4::from_quat(rotation)
                * glam::Mat4::from_scale(glam::Vec3::splat(scale))
                * glam::Mat4::from_translation(normalized_size * -0.5)
                * glam::Mat4::from_scale(normalized_size);

            // Upload uniforms
            let mvp = projection * view * model;
            let mvp_loc = gl.get_uniform_location(program, "uMVP");
            gl.uniform_matrix_4_f32_slice(mvp_loc.as_ref(), false, mvp.as_ref());

            let model_loc = gl.get_uniform_location(program, "uModel");
            gl.uniform_matrix_4_f32_slice(model_loc.as_ref(), false, model.as_ref());

            // Full brightness for wireframe (no lighting)
            let ambient_loc = gl.get_uniform_location(program, "uAmbient");
            gl.uniform_1_f32(ambient_loc.as_ref(), 1.0);

            let diffuse_strength_loc = gl.get_uniform_location(program, "uDiffuseStrength");
            gl.uniform_1_f32(diffuse_strength_loc.as_ref(), 0.0);

            // Set wireframe color via vertex color override uniform (RGBA)
            let color_override_loc = gl.get_uniform_location(program, "uColorOverride");
            gl.uniform_4_f32(
                color_override_loc.as_ref(),
                color[0],
                color[1],
                color[2],
                color[3],
            );

            let use_color_override_loc = gl.get_uniform_location(program, "uUseColorOverride");
            gl.uniform_1_i32(use_color_override_loc.as_ref(), 1);

            // Draw wireframe box using LINES
            gl.bind_vertex_array(Some(wireframe_mesh.vao));
            gl.draw_elements(LINES, wireframe_mesh.index_count, UNSIGNED_INT, 0);
            gl.bind_vertex_array(None);

            // Restore GL state
            gl.uniform_1_i32(use_color_override_loc.as_ref(), 0);
            gl.depth_mask(true);
            gl.depth_func(LESS); // Restore default depth function
            if color[3] < 1.0 {
                gl.disable(BLEND);
            }
        }
    }

    /// Render a wireframe AABB (Axis-Aligned Bounding Box) with a specific color
    ///
    /// Unlike `render_cubebox_wireframe`, this renders an AABB directly from min/max
    /// corners in world space, without any model transformation.
    ///
    /// # Arguments
    /// * `aabb_min` - Minimum corner of the AABB in world space
    /// * `aabb_max` - Maximum corner of the AABB in world space
    /// * `color` - RGB color for the wireframe [0.0-1.0]
    ///
    /// # Safety
    ///
    /// Must be called with an active GL context on the current thread.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn render_aabb_wireframe(
        &self,
        gl: &Context,
        aabb_min: glam::Vec3,
        aabb_max: glam::Vec3,
        color: [f32; 3],
        camera: &Camera,
        viewport_width: i32,
        viewport_height: i32,
    ) {
        unsafe {
            let Some(program) = self.program else { return };
            let Some(ref wireframe_mesh) = self.wireframe_box else {
                return;
            };

            gl.use_program(Some(program));
            gl.disable(DEPTH_TEST); // Always on top
            gl.disable(CULL_FACE);

            // Calculate matrices
            let aspect = viewport_width as f32 / viewport_height as f32;
            let projection = glam::Mat4::perspective_rh(camera.vfov, aspect, 1.0, 50000.0);

            let forward = camera.rotation * glam::Vec3::NEG_Z;
            let up = camera.rotation * glam::Vec3::Y;
            let target = camera.position + forward;
            let view = glam::Mat4::look_at_rh(camera.position, target, up);

            // Transform unit [0,1] wireframe to AABB bounds:
            // Size = max - min
            // Position = min (corner-based, not center-based)
            let size = aabb_max - aabb_min;
            let model = glam::Mat4::from_translation(aabb_min) * glam::Mat4::from_scale(size);

            // Upload uniforms
            let mvp = projection * view * model;
            let mvp_loc = gl.get_uniform_location(program, "uMVP");
            gl.uniform_matrix_4_f32_slice(mvp_loc.as_ref(), false, mvp.as_ref());

            let model_loc = gl.get_uniform_location(program, "uModel");
            gl.uniform_matrix_4_f32_slice(model_loc.as_ref(), false, model.as_ref());

            // Full brightness for wireframe (no lighting)
            let ambient_loc = gl.get_uniform_location(program, "uAmbient");
            gl.uniform_1_f32(ambient_loc.as_ref(), 1.0);

            let diffuse_strength_loc = gl.get_uniform_location(program, "uDiffuseStrength");
            gl.uniform_1_f32(diffuse_strength_loc.as_ref(), 0.0);

            // Set wireframe color via vertex color override uniform (RGBA with alpha = 1.0)
            let color_override_loc = gl.get_uniform_location(program, "uColorOverride");
            gl.uniform_4_f32(
                color_override_loc.as_ref(),
                color[0],
                color[1],
                color[2],
                1.0,
            );

            let use_color_override_loc = gl.get_uniform_location(program, "uUseColorOverride");
            gl.uniform_1_i32(use_color_override_loc.as_ref(), 1);

            // Draw wireframe box using LINES
            gl.bind_vertex_array(Some(wireframe_mesh.vao));
            gl.draw_elements(LINES, wireframe_mesh.index_count, UNSIGNED_INT, 0);
            gl.bind_vertex_array(None);

            // Disable color override
            gl.uniform_1_i32(use_color_override_loc.as_ref(), 0);
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

    /// Render a 2D crosshair gizmo at screen coordinates
    ///
    /// Draws a small crosshair (+) at the given screen position.
    ///
    /// # Arguments
    /// * `screen_pos` - Position in window coordinates (pixels, origin top-left)
    /// * `size` - Size of the crosshair in pixels
    /// * `color` - RGB color for the crosshair [0.0-1.0]
    /// * `viewport_width` - Window width in pixels
    /// * `viewport_height` - Window height in pixels
    ///
    /// # Safety
    ///
    /// Must be called with an active GL context on the current thread.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn render_2d_crosshair(
        &self,
        gl: &Context,
        screen_pos: glam::Vec2,
        size: f32,
        color: [f32; 3],
        viewport_width: i32,
        viewport_height: i32,
    ) {
        unsafe {
            let Some(program) = self.program_2d else {
                return;
            };

            gl.use_program(Some(program));
            gl.disable(DEPTH_TEST);
            gl.disable(CULL_FACE);

            // Convert screen coordinates to NDC (-1 to 1)
            // Screen origin is top-left, NDC origin is center
            let x_ndc = (screen_pos.x / viewport_width as f32) * 2.0 - 1.0;
            let y_ndc = 1.0 - (screen_pos.y / viewport_height as f32) * 2.0;
            let half_w = size / viewport_width as f32;
            let half_h = size / viewport_height as f32;

            // Set uniforms
            let color_loc = gl.get_uniform_location(program, "uColor");
            gl.uniform_3_f32(color_loc.as_ref(), color[0], color[1], color[2]);

            // Create crosshair vertices (horizontal and vertical lines)
            #[rustfmt::skip]
            let vertices: [f32; 8] = [
                // Horizontal line
                x_ndc - half_w, y_ndc,
                x_ndc + half_w, y_ndc,
                // Vertical line
                x_ndc, y_ndc - half_h,
                x_ndc, y_ndc + half_h,
            ];

            // Create temporary VAO/VBO for dynamic 2D geometry
            let vao = gl.create_vertex_array().unwrap();
            let vbo = gl.create_buffer().unwrap();

            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(ARRAY_BUFFER, bytemuck::cast_slice(&vertices), DYNAMIC_DRAW);

            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, FLOAT, false, 0, 0);

            gl.draw_arrays(LINES, 0, 4);

            gl.bind_vertex_array(None);
            gl.delete_buffer(vbo);
            gl.delete_vertex_array(vao);
        }
    }

    /// Render a 2D pointer gizmo at screen coordinates
    ///
    /// Draws a circle or ring at the given screen position.
    ///
    /// # Arguments
    /// * `screen_pos` - Position in window coordinates (pixels, origin top-left)
    /// * `radius` - Radius of the circle in pixels
    /// * `color` - RGB color for the circle [0.0-1.0]
    /// * `viewport_width` - Window width in pixels
    /// * `viewport_height` - Window height in pixels
    ///
    /// # Safety
    ///
    /// Must be called with an active GL context on the current thread.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn render_2d_circle(
        &self,
        gl: &Context,
        screen_pos: glam::Vec2,
        radius: f32,
        color: [f32; 3],
        viewport_width: i32,
        viewport_height: i32,
    ) {
        unsafe {
            let Some(program) = self.program_2d else {
                return;
            };

            gl.use_program(Some(program));
            gl.disable(DEPTH_TEST);
            gl.disable(CULL_FACE);

            // Convert screen coordinates to NDC
            let x_ndc = (screen_pos.x / viewport_width as f32) * 2.0 - 1.0;
            let y_ndc = 1.0 - (screen_pos.y / viewport_height as f32) * 2.0;
            let r_x = radius / viewport_width as f32;
            let r_y = radius / viewport_height as f32;

            // Set uniforms
            let color_loc = gl.get_uniform_location(program, "uColor");
            gl.uniform_3_f32(color_loc.as_ref(), color[0], color[1], color[2]);

            // Create circle vertices (line loop with 16 segments)
            const SEGMENTS: usize = 16;
            let mut vertices: Vec<f32> = Vec::with_capacity(SEGMENTS * 2);
            for i in 0..SEGMENTS {
                let angle = (i as f32 / SEGMENTS as f32) * std::f32::consts::TAU;
                vertices.push(x_ndc + angle.cos() * r_x);
                vertices.push(y_ndc + angle.sin() * r_y);
            }

            // Create temporary VAO/VBO for dynamic 2D geometry
            let vao = gl.create_vertex_array().unwrap();
            let vbo = gl.create_buffer().unwrap();

            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(ARRAY_BUFFER, bytemuck::cast_slice(&vertices), DYNAMIC_DRAW);

            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, FLOAT, false, 0, 0);

            gl.draw_arrays(LINE_LOOP, 0, SEGMENTS as i32);

            gl.bind_vertex_array(None);
            gl.delete_buffer(vbo);
            gl.delete_vertex_array(vao);
        }
    }

    /// Render 3D axis arrows gizmo at world position
    ///
    /// Draws colored axis arrows (X=red, Y=green, Z=blue) at the given world position.
    ///
    /// # Arguments
    /// * `position` - World position for the gizmo origin
    /// * `scale` - Scale factor for the arrows
    /// * `camera` - Camera for view/projection matrices
    /// * `viewport_width` - Viewport width in pixels
    /// * `viewport_height` - Viewport height in pixels
    ///
    /// # Safety
    ///
    /// Must be called with an active GL context on the current thread.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn render_3d_axis_arrows(
        &self,
        gl: &Context,
        position: glam::Vec3,
        scale: f32,
        camera: &Camera,
        viewport_width: i32,
        viewport_height: i32,
    ) {
        unsafe {
            let Some(program) = self.program else { return };
            let Some(ref axis_mesh) = self.axis_arrows else {
                return;
            };

            gl.use_program(Some(program));
            gl.disable(DEPTH_TEST); // Always on top
            gl.disable(CULL_FACE);

            // Calculate matrices
            let aspect = viewport_width as f32 / viewport_height as f32;
            let projection = glam::Mat4::perspective_rh(camera.vfov, aspect, 1.0, 50000.0);

            let forward = camera.rotation * glam::Vec3::NEG_Z;
            let up = camera.rotation * glam::Vec3::Y;
            let target = camera.position + forward;
            let view = glam::Mat4::look_at_rh(camera.position, target, up);

            // Simple translation and scale model matrix
            let model = glam::Mat4::from_translation(position)
                * glam::Mat4::from_scale(glam::Vec3::splat(scale));

            // Upload uniforms
            let mvp = projection * view * model;
            let mvp_loc = gl.get_uniform_location(program, "uMVP");
            gl.uniform_matrix_4_f32_slice(mvp_loc.as_ref(), false, mvp.as_ref());

            let model_loc = gl.get_uniform_location(program, "uModel");
            gl.uniform_matrix_4_f32_slice(model_loc.as_ref(), false, model.as_ref());

            // Full brightness for gizmo (no lighting)
            let ambient_loc = gl.get_uniform_location(program, "uAmbient");
            gl.uniform_1_f32(ambient_loc.as_ref(), 1.0);

            let diffuse_strength_loc = gl.get_uniform_location(program, "uDiffuseStrength");
            gl.uniform_1_f32(diffuse_strength_loc.as_ref(), 0.0);

            // Don't use color override - use vertex colors
            let use_color_override_loc = gl.get_uniform_location(program, "uUseColorOverride");
            gl.uniform_1_i32(use_color_override_loc.as_ref(), 0);

            // Draw axis arrows using LINES
            gl.bind_vertex_array(Some(axis_mesh.vao));
            gl.draw_elements(LINES, axis_mesh.index_count, UNSIGNED_INT, 0);
            gl.bind_vertex_array(None);
        }
    }

    /// Save the current framebuffer to an image file
    ///
    /// # Arguments
    /// * `gl` - OpenGL context
    /// * `width` - Framebuffer width
    /// * `height` - Framebuffer height
    /// * `path` - Output file path (PNG format recommended)
    ///
    /// # Safety
    ///
    /// Must be called with an active GL context on the current thread.
    pub fn save_framebuffer_to_file(
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

        // Flip Y-axis (OpenGL has origin at bottom-left)
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

            if let Some(program) = self.program_2d.take() {
                gl.delete_program(program);
            }

            if let Some(ref mesh) = self.axis_arrows.take() {
                gl.delete_vertex_array(mesh.vao);
                gl.delete_buffer(mesh.vbo);
                gl.delete_buffer(mesh.ebo);
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
uniform vec4 uColorOverride;
uniform int uUseColorOverride;

out vec4 FragColor;

void main() {
    // Use color override if set (for colored wireframes)
    vec3 baseColor = uUseColorOverride == 1 ? uColorOverride.rgb : vColor;
    float alpha = uUseColorOverride == 1 ? uColorOverride.a : 1.0;

    vec3 normal = normalize(vNormal);
    float diffuse = max(dot(normal, uLightDir), 0.0);
    vec3 lighting = baseColor * (uAmbient + diffuse * uDiffuseStrength);

    // Gamma correction to match CPU tracer output
    vec3 gammaCorrected = pow(lighting, vec3(1.0 / 2.2));

    FragColor = vec4(gammaCorrected, alpha);
}
"#;

/// Simple 2D vertex shader for screen-space rendering
const VERTEX_SHADER_2D: &str = r#"#version 300 es
precision highp float;

layout(location = 0) in vec2 aPosition;

void main() {
    gl_Position = vec4(aPosition, 0.0, 1.0);
}
"#;

/// Simple 2D fragment shader with uniform color
const FRAGMENT_SHADER_2D: &str = r#"#version 300 es
precision highp float;

uniform vec3 uColor;

out vec4 FragColor;

void main() {
    FragColor = vec4(uColor, 1.0);
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
