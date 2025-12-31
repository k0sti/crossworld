use app::App;
use glam::{Mat4, Vec3};
use glow::*;
use std::sync::Arc;
use std::time::Instant;
use winit::event::WindowEvent;

const VERTEX_SHADER: &str = r#"#version 330 core
layout(location = 0) in vec3 a_pos;
layout(location = 1) in vec3 a_color;

out vec3 v_color;

uniform mat4 u_mvp;

void main() {
    v_color = a_color;
    gl_Position = u_mvp * vec4(a_pos, 1.0);
}
"#;

const FRAGMENT_SHADER: &str = r#"#version 330 core
in vec3 v_color;
out vec4 frag_color;

void main() {
    frag_color = vec4(v_color, 1.0);
}
"#;

/// Cube vertices: position (xyz)
const CUBE_VERTICES: &[f32] = &[
    // Front face
    -0.5, -0.5,  0.5,
     0.5, -0.5,  0.5,
     0.5,  0.5,  0.5,
    -0.5,  0.5,  0.5,
    // Back face
    -0.5, -0.5, -0.5,
    -0.5,  0.5, -0.5,
     0.5,  0.5, -0.5,
     0.5, -0.5, -0.5,
    // Top face
    -0.5,  0.5, -0.5,
    -0.5,  0.5,  0.5,
     0.5,  0.5,  0.5,
     0.5,  0.5, -0.5,
    // Bottom face
    -0.5, -0.5, -0.5,
     0.5, -0.5, -0.5,
     0.5, -0.5,  0.5,
    -0.5, -0.5,  0.5,
    // Right face
     0.5, -0.5, -0.5,
     0.5,  0.5, -0.5,
     0.5,  0.5,  0.5,
     0.5, -0.5,  0.5,
    // Left face
    -0.5, -0.5, -0.5,
    -0.5, -0.5,  0.5,
    -0.5,  0.5,  0.5,
    -0.5,  0.5, -0.5,
];

/// Cube colors (one color per face)
const CUBE_COLORS: &[f32] = &[
    // Front face (red)
    1.0, 0.0, 0.0,
    1.0, 0.0, 0.0,
    1.0, 0.0, 0.0,
    1.0, 0.0, 0.0,
    // Back face (green)
    0.0, 1.0, 0.0,
    0.0, 1.0, 0.0, 
    0.0, 1.0, 0.0,
    0.0, 1.0, 0.0,
    // Top face (blue)
    0.0, 0.0, 1.0,
    0.0, 0.0, 1.0,
    0.0, 0.0, 1.0,
    0.0, 0.0, 1.0,
    // Bottom face (yellow)
    1.0, 1.0, 0.0,
    1.0, 1.0, 0.0,
    1.0, 1.0, 0.0,
    1.0, 1.0, 0.0,
    // Right face (magenta)
    1.0, 0.0, 1.0,
    1.0, 0.0, 1.0,
    1.0, 0.0, 1.0,
    1.0, 0.0, 1.0,
    // Left face (cyan)
    0.0, 1.0, 1.0,
    0.0, 1.0, 1.0,
    0.0, 1.0, 1.0,
    0.0, 1.0, 1.0,
];

/// Cube indices (two triangles per face)
const CUBE_INDICES: &[u32] = &[
    0,  1,  2,  2,  3,  0,   // Front
    4,  5,  6,  6,  7,  4,   // Back
    8,  9, 10, 10, 11,  8,   // Top
    12, 13, 14, 14, 15, 12,  // Bottom
    16, 17, 18, 18, 19, 16,  // Right
    20, 21, 22, 22, 23, 20,  // Left
];

pub struct RotatingCube {
    vao: Option<NativeVertexArray>,
    vbo_pos: Option<NativeBuffer>,
    vbo_color: Option<NativeBuffer>,
    ebo: Option<NativeBuffer>,
    shader_program: Option<NativeProgram>,
    rotation: f32,
    window_size: (u32, u32),
    gl: Option<Arc<Context>>,
    egui_ctx: Option<egui::Context>,
    egui_winit: Option<egui_winit::State>,
    egui_painter: Option<egui_glow::Painter>,
    start_time: std::time::Instant,
}

impl RotatingCube {
    pub fn new() -> Self {
        Self {
            vao: None,
            vbo_pos: None,
            vbo_color: None,
            ebo: None,
            shader_program: None,
            rotation: 0.0,
            window_size: (800, 600),
            gl: None,
            egui_ctx: None,
            egui_winit: None,
            egui_painter: None,
            start_time: Instant::now(),
        }
    }

    unsafe fn compile_shader(
        gl: &Context,
        shader_type: u32,
        source: &str,
    ) -> Result<NativeShader, String> {
        let shader = gl.create_shader(shader_type)?;
        gl.shader_source(shader, source);
        gl.compile_shader(shader);

        if !gl.get_shader_compile_status(shader) {
            let log = gl.get_shader_info_log(shader);
            return Err(format!("Shader compilation failed: {}", log));
        }

        Ok(shader)
    }
}

impl App for RotatingCube {
    unsafe fn init(&mut self, gl: Arc<Context>) {
        println!("[Game] Initializing rotating cube");

        // Store GL context
        self.gl = Some(Arc::clone(&gl));

        // Create and bind VAO
        let vao = gl.create_vertex_array().unwrap();
        gl.bind_vertex_array(Some(vao));

        // Create VBO for positions
        let vbo_pos = gl.create_buffer().unwrap();
        gl.bind_buffer(ARRAY_BUFFER, Some(vbo_pos));
        gl.buffer_data_u8_slice(
            ARRAY_BUFFER,
            bytemuck::cast_slice(CUBE_VERTICES),
            STATIC_DRAW,
        );
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(0, 3, FLOAT, false, 12, 0);

        // Create VBO for colors
        let vbo_color = gl.create_buffer().unwrap();
        gl.bind_buffer(ARRAY_BUFFER, Some(vbo_color));
        gl.buffer_data_u8_slice(
            ARRAY_BUFFER,
            bytemuck::cast_slice(CUBE_COLORS),
            STATIC_DRAW,
        );
        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_pointer_f32(1, 3, FLOAT, false, 12, 0);

        // Create EBO
        let ebo = gl.create_buffer().unwrap();
        gl.bind_buffer(ELEMENT_ARRAY_BUFFER, Some(ebo));
        gl.buffer_data_u8_slice(
            ELEMENT_ARRAY_BUFFER,
            bytemuck::cast_slice(CUBE_INDICES),
            STATIC_DRAW,
        );

        // Compile shaders
        let vertex_shader = Self::compile_shader(&gl, glow::VERTEX_SHADER, VERTEX_SHADER)
            .expect("Failed to compile vertex shader");
        let fragment_shader = Self::compile_shader(&gl, glow::FRAGMENT_SHADER, FRAGMENT_SHADER)
            .expect("Failed to compile fragment shader");

        // Link program
        let program = gl.create_program().unwrap();
        gl.attach_shader(program, vertex_shader);
        gl.attach_shader(program, fragment_shader);
        gl.link_program(program);

        if !gl.get_program_link_status(program) {
            let log = gl.get_program_info_log(program);
            panic!("Program linking failed: {}", log);
        }

        // Cleanup shaders (they're linked into the program now)
        gl.delete_shader(vertex_shader);
        gl.delete_shader(fragment_shader);

        // Enable depth testing
        gl.enable(DEPTH_TEST);

        // Store handles
        self.vao = Some(vao);
        self.vbo_pos = Some(vbo_pos);
        self.vbo_color = Some(vbo_color);
        self.ebo = Some(ebo);
        self.shader_program = Some(program);

        // Initialize egui
        let egui_ctx = egui::Context::default();
        let egui_painter = egui_glow::Painter::new(Arc::clone(&gl), "", None, false)
            .expect("Failed to create egui painter");

        self.egui_ctx = Some(egui_ctx);
        self.egui_painter = Some(egui_painter);

        println!("[Game] Rotating cube initialized successfully");
        println!("[Game] egui initialized - UI overlay enabled");
        println!("[Game] Hot-reload ready! Try changing rotation speed or colors.");
    }

    unsafe fn uninit(&mut self, gl: Arc<Context>) {
        println!("[Game] Cleaning up rotating cube");

        // Destroy egui
        if let Some(mut painter) = self.egui_painter.take() {
            painter.destroy();
        }
        self.egui_ctx = None;
        self.egui_winit = None;

        if let Some(vao) = self.vao.take() {
            gl.delete_vertex_array(vao);
        }
        if let Some(vbo) = self.vbo_pos.take() {
            gl.delete_buffer(vbo);
        }
        if let Some(vbo) = self.vbo_color.take() {
            gl.delete_buffer(vbo);
        }
        if let Some(ebo) = self.ebo.take() {
            gl.delete_buffer(ebo);
        }
        if let Some(program) = self.shader_program.take() {
            gl.delete_program(program);
        }

        self.gl = None;

        println!("[Game] Cleanup complete");
    }

    fn event(&mut self, event: &WindowEvent) {
        // Handle window resize
        if let WindowEvent::Resized(size) = event {
            self.window_size = (size.width, size.height);
        }
    }

    fn update(&mut self, delta_time: f32) {
        // Rotate at 45 degrees per second (change this value to test hot-reload!)
        self.rotation += 45.0_f32.to_radians() * delta_time;
    }

    unsafe fn render(&mut self, gl: Arc<Context>) {
        // Clear the screen
        gl.clear_color(0.1, 0.1, 0.1, 1.0);
        gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);

        if let (Some(vao), Some(program)) = (self.vao, self.shader_program) {
            // Use shader program
            gl.use_program(Some(program));

            // Create MVP matrix
            let model = Mat4::from_rotation_y(self.rotation) * Mat4::from_rotation_x(self.rotation * 0.5);
            let view = Mat4::look_at_rh(
                Vec3::new(0.0, 0.0, 3.0),  // Camera position
                Vec3::new(0.0, 0.0, 0.0),  // Look at center
                Vec3::new(0.0, 1.0, 0.0),  // Up vector
            );
            let aspect = self.window_size.0 as f32 / self.window_size.1 as f32;
            let projection = Mat4::perspective_rh(
                45.0_f32.to_radians(),
                aspect,
                0.1,
                100.0,
            );
            let mvp = projection * view * model;

            // Set uniform
            let u_mvp_loc = gl.get_uniform_location(program, "u_mvp");
            if let Some(loc) = u_mvp_loc {
                gl.uniform_matrix_4_f32_slice(Some(&loc), false, &mvp.to_cols_array());
            }

            // Draw cube
            gl.bind_vertex_array(Some(vao));
            gl.draw_elements(TRIANGLES, 36, UNSIGNED_INT, 0);

            // Unbind VAO before egui
            gl.bind_vertex_array(None);
            gl.use_program(None);
        }

        // Render egui UI on top of 3D scene
        if let Some(egui_ctx) = &self.egui_ctx {
            // Setup GL state for egui (2D overlay)
            gl.disable(DEPTH_TEST);
            gl.enable(BLEND);
            gl.blend_func(SRC_ALPHA, ONE_MINUS_SRC_ALPHA);
            gl.viewport(0, 0, self.window_size.0 as i32, self.window_size.1 as i32);

            let elapsed = self.start_time.elapsed().as_secs_f64();
            let raw_input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(self.window_size.0 as f32, self.window_size.1 as f32),
                )),
                time: Some(elapsed),
                predicted_dt: 1.0 / 60.0,
                ..Default::default()
            };

            let full_output = egui_ctx.run(raw_input, |ctx| {
                egui::Window::new("🎮 Hot Reload Demo")
                    .default_pos([10.0, 10.0])
                    .default_width(300.0)
                    .resizable(true)
                    .show(ctx, |ui| {
                        ui.heading("RotatingCube");
                        ui.colored_label(egui::Color32::GREEN, "✓ Hot-reloadable egui UI!");
                        ui.separator();
                        ui.label(format!("Rotation: {:.2}°", self.rotation.to_degrees()));
                        ui.label(format!("Window: {}x{}", self.window_size.0, self.window_size.1));
                        ui.separator();
                        ui.label("Edit crates/game/src/lib.rs to see hot-reload!");
                    });
            });

            // Paint egui
            if let Some(egui_painter) = &mut self.egui_painter {
                // Upload textures (fonts, images, etc.)
                for (id, image_delta) in &full_output.textures_delta.set {
                    egui_painter.set_texture(*id, image_delta);
                }

                let clipped_primitives = egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);

                egui_painter.paint_primitives(
                    [self.window_size.0, self.window_size.1],
                    full_output.pixels_per_point,
                    &clipped_primitives,
                );

                // Free old textures
                for id in &full_output.textures_delta.free {
                    egui_painter.free_texture(*id);
                }
            }

            // Restore GL state for 3D rendering
            gl.enable(DEPTH_TEST);
            gl.disable(BLEND);
        }
    }
}

/// Export the create_app function for dynamic loading
#[no_mangle]
pub extern "C" fn create_app() -> *mut dyn App {
    Box::into_raw(Box::new(RotatingCube::new()))
}
