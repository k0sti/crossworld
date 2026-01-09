//! Skybox renderer with logarithmic gradients
//!
//! Renders a skybox with configurable gradient colors for sky (top) and ground (bottom).
//! Uses logarithmic interpolation to compress colors near the horizon.

use glow::*;
use glam::Mat4;

use crate::camera::Camera;
use crate::shader_utils::create_program;

/// Skybox renderer with gradient support
pub struct SkyboxRenderer {
    program: Option<NativeProgram>,
    vao: Option<NativeVertexArray>,
    vbo: Option<NativeBuffer>,

    // Gradient colors
    sky_colors: Vec<[f32; 3]>,
    ground_colors: Vec<[f32; 3]>,
}

impl Default for SkyboxRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl SkyboxRenderer {
    pub fn new() -> Self {
        Self {
            program: None,
            vao: None,
            vbo: None,
            // Default blue sky gradient (light blue to darker blue)
            sky_colors: vec![
                [0.53, 0.81, 0.92], // Light sky blue at horizon
                [0.2, 0.4, 0.8],    // Deeper blue at zenith
            ],
            // Default gray ground gradient (light gray to dark gray)
            ground_colors: vec![
                [0.7, 0.7, 0.7],    // Light gray at horizon
                [0.3, 0.3, 0.3],    // Dark gray at nadir
            ],
        }
    }

    /// Set custom sky gradient colors (horizon to zenith)
    pub fn set_sky_colors(&mut self, colors: Vec<[f32; 3]>) {
        assert!(!colors.is_empty(), "Sky colors array cannot be empty");
        self.sky_colors = colors;
    }

    /// Set custom ground gradient colors (horizon to nadir)
    pub fn set_ground_colors(&mut self, colors: Vec<[f32; 3]>) {
        assert!(!colors.is_empty(), "Ground colors array cannot be empty");
        self.ground_colors = colors;
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

            // Create fullscreen quad (two triangles covering screen)
            // Positions in NDC space (-1 to 1)
            #[rustfmt::skip]
            let vertices: [f32; 12] = [
                // Triangle 1
                -1.0, -1.0,  // bottom-left
                 1.0, -1.0,  // bottom-right
                 1.0,  1.0,  // top-right
                // Triangle 2
                -1.0, -1.0,  // bottom-left
                 1.0,  1.0,  // top-right
                -1.0,  1.0,  // top-left
            ];

            let vao = gl
                .create_vertex_array()
                .map_err(|e| format!("Failed to create VAO: {}", e))?;
            gl.bind_vertex_array(Some(vao));

            let vbo = gl
                .create_buffer()
                .map_err(|e| format!("Failed to create VBO: {}", e))?;
            gl.bind_buffer(ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(
                ARRAY_BUFFER,
                bytemuck::cast_slice(&vertices),
                STATIC_DRAW,
            );

            // Position attribute (location 0)
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, FLOAT, false, 8, 0);

            gl.bind_vertex_array(None);

            self.vao = Some(vao);
            self.vbo = Some(vbo);

            Ok(())
        }
    }

    /// Render the skybox
    ///
    /// # Safety
    ///
    /// Must be called with an active GL context on the current thread.
    pub unsafe fn render(
        &self,
        gl: &Context,
        camera: &Camera,
        width: i32,
        height: i32,
    ) {
        unsafe {
            let program = match self.program {
                Some(p) => p,
                None => return,
            };

            let vao = match self.vao {
                Some(v) => v,
                None => return,
            };

            // Disable depth testing (skybox is always in background)
            gl.disable(DEPTH_TEST);
            gl.depth_mask(false);

            gl.use_program(Some(program));
            gl.bind_vertex_array(Some(vao));

            // Calculate inverse projection-view matrix for ray direction
            let aspect = width as f32 / height as f32;
            let proj = Mat4::perspective_rh_gl(camera.vfov, aspect, 0.1, 1000.0);

            // Build view matrix manually from camera transform
            let view = Mat4::from_rotation_translation(camera.rotation, camera.position).inverse();
            let inv_proj_view = (proj * view).inverse();

            // Upload uniforms
            let inv_proj_view_loc = gl.get_uniform_location(program, "u_invProjView");
            gl.uniform_matrix_4_f32_slice(inv_proj_view_loc.as_ref(), false, &inv_proj_view.to_cols_array());

            let camera_pos_loc = gl.get_uniform_location(program, "u_cameraPos");
            gl.uniform_3_f32(camera_pos_loc.as_ref(), camera.position.x, camera.position.y, camera.position.z);

            // Upload sky gradient colors
            let sky_count_loc = gl.get_uniform_location(program, "u_skyColorCount");
            gl.uniform_1_i32(sky_count_loc.as_ref(), self.sky_colors.len() as i32);

            for (i, color) in self.sky_colors.iter().enumerate() {
                if i >= 8 { break; } // Shader supports max 8 colors
                let uniform_name = format!("u_skyColors[{}]", i);
                let color_loc = gl.get_uniform_location(program, &uniform_name);
                gl.uniform_3_f32(color_loc.as_ref(), color[0], color[1], color[2]);
            }

            // Upload ground gradient colors
            let ground_count_loc = gl.get_uniform_location(program, "u_groundColorCount");
            gl.uniform_1_i32(ground_count_loc.as_ref(), self.ground_colors.len() as i32);

            for (i, color) in self.ground_colors.iter().enumerate() {
                if i >= 8 { break; } // Shader supports max 8 colors
                let uniform_name = format!("u_groundColors[{}]", i);
                let color_loc = gl.get_uniform_location(program, &uniform_name);
                gl.uniform_3_f32(color_loc.as_ref(), color[0], color[1], color[2]);
            }

            // Draw fullscreen quad
            gl.draw_arrays(TRIANGLES, 0, 6);

            gl.bind_vertex_array(None);
            gl.use_program(None);

            // Re-enable depth testing
            gl.depth_mask(true);
            gl.enable(DEPTH_TEST);
        }
    }

    /// Clean up GL resources
    ///
    /// # Safety
    ///
    /// Must be called with an active GL context on the current thread.
    pub unsafe fn destroy_gl(&mut self, gl: &Context) {
        unsafe {
            if let Some(vao) = self.vao.take() {
                gl.delete_vertex_array(vao);
            }
            if let Some(vbo) = self.vbo.take() {
                gl.delete_buffer(vbo);
            }
            if let Some(program) = self.program.take() {
                gl.delete_program(program);
            }
        }
    }
}

impl Drop for SkyboxRenderer {
    fn drop(&mut self) {
        if self.program.is_some() || self.vao.is_some() || self.vbo.is_some() {
            eprintln!("WARNING: SkyboxRenderer dropped without calling destroy_gl()");
        }
    }
}

const VERTEX_SHADER: &str = r#"#version 330 core

layout(location = 0) in vec2 a_position;

out vec2 v_screenPos;

void main() {
    v_screenPos = a_position;
    gl_Position = vec4(a_position, 0.0, 1.0);
}
"#;

const FRAGMENT_SHADER: &str = r#"#version 330 core

in vec2 v_screenPos;
out vec4 FragColor;

uniform mat4 u_invProjView;
uniform vec3 u_cameraPos;

uniform int u_skyColorCount;
uniform vec3 u_skyColors[8];
uniform int u_groundColorCount;
uniform vec3 u_groundColors[8];

// Logarithmic interpolation between gradient colors
vec3 logarithmicGradient(float t, vec3 colors[8], int count) {
    if (count == 0) return vec3(0.0);
    if (count == 1) return colors[0];

    // Apply logarithmic curve (compresses colors near 0, expands near 1)
    // Using log in base 2 for smooth falloff
    float logT = log2(1.0 + t * 15.0) / log2(16.0); // Maps [0,1] -> [0,1] with log curve

    // Find which segment we're in
    float segmentFloat = logT * float(count - 1);
    int segment = int(floor(segmentFloat));
    segment = clamp(segment, 0, count - 2);

    // Interpolate within segment
    float segmentT = fract(segmentFloat);

    return mix(colors[segment], colors[segment + 1], segmentT);
}

void main() {
    // Convert screen position to clip space
    vec4 clipPos = vec4(v_screenPos, 0.0, 1.0);

    // Transform to world space
    vec4 worldPos = u_invProjView * clipPos;
    worldPos /= worldPos.w;

    // Calculate view direction
    vec3 viewDir = normalize(worldPos.xyz - u_cameraPos);

    // Use Y component to determine sky vs ground
    float y = viewDir.y;

    vec3 color;
    if (y >= 0.0) {
        // Sky (above horizon)
        // Map y from [0, 1] (horizon to zenith)
        float t = y;
        color = logarithmicGradient(t, u_skyColors, u_skyColorCount);
    } else {
        // Ground (below horizon)
        // Map y from [0, -1] (horizon to nadir) to [0, 1]
        float t = -y;
        color = logarithmicGradient(t, u_groundColors, u_groundColorCount);
    }

    FragColor = vec4(color, 1.0);
}
"#;
