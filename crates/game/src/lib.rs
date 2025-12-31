use app::App;
// Hot-reload trigger: 1767200762409
use glam::{Quat, Vec3};
use glow::*;
use renderer::{Camera, MeshRenderer};
use std::sync::Arc;
use std::time::Instant;
use winit::event::WindowEvent;

pub struct RotatingCube {
    mesh_renderer: MeshRenderer,
    mesh_index: Option<usize>,
    rotation: f32,
    window_size: (u32, u32),
    gl: Option<Arc<Context>>,
    egui_ctx: Option<egui::Context>,
    egui_painter: Option<egui_glow::Painter>,
    start_time: Instant,
    last_reload_trigger: Option<Instant>,
    // Input state for egui
    pointer_pos: Option<egui::Pos2>,
    mouse_button_events: Vec<(egui::PointerButton, bool)>, // (button, pressed)
}

impl RotatingCube {
    pub fn new() -> Self {
        Self {
            mesh_renderer: MeshRenderer::new(),
            mesh_index: None,
            rotation: 0.0,
            window_size: (800, 600),
            gl: None,
            egui_ctx: None,
            egui_painter: None,
            start_time: Instant::now(),
            last_reload_trigger: None,
            pointer_pos: None,
            mouse_button_events: Vec::new(),
        }
    }

    /// Trigger a hot-reload by modifying the source file
    fn trigger_reload(&mut self) {
        use std::fs;
        use std::path::PathBuf;
        use std::time::SystemTime;

        let mut src_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        src_path.push("src");
        src_path.push("lib.rs");

        if let Ok(mut content) = fs::read_to_string(&src_path) {
            // Find or create the dummy comment line
            let marker = "// Hot-reload trigger:";

            // Use SystemTime to get a real timestamp
            let timestamp = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis();

            if let Some(pos) = content.find(marker) {
                // Update existing marker with new timestamp
                let line_end = content[pos..].find('\n').map(|i| pos + i).unwrap_or(content.len());
                let new_line = format!("{} {}", marker, timestamp);
                content.replace_range(pos..line_end, &new_line);
            } else {
                // Add marker at the top after the first line
                if let Some(first_newline) = content.find('\n') {
                    content.insert_str(first_newline + 1, &format!("{} {}\n", marker, timestamp));
                }
            }

            match fs::write(&src_path, content) {
                Ok(_) => {
                    self.last_reload_trigger = Some(Instant::now());
                    println!("[Game] 🔄 Triggered hot-reload by modifying source file (timestamp: {})", timestamp);
                }
                Err(e) => {
                    eprintln!("[Game] ❌ Failed to write source file: {}", e);
                }
            }
        } else {
            eprintln!("[Game] ❌ Failed to read source file");
        }
    }
}

impl App for RotatingCube {
    unsafe fn init(&mut self, gl: Arc<Context>) {
        println!("[Game] Initializing rotating cube with MeshRenderer");

        // Store GL context
        self.gl = Some(Arc::clone(&gl));

        // Initialize mesh renderer
        self.mesh_renderer
            .init_gl(&gl)
            .expect("Failed to initialize MeshRenderer");

        // Create a simple colored cube scene
        let cube = renderer::create_octa_cube();

        // Upload mesh to GPU (depth 1 since octa_cube is a simple 2x2x2 octree)
        let mesh_index = self
            .mesh_renderer
            .upload_mesh(&gl, &cube, 1)
            .expect("Failed to upload cube mesh");
        self.mesh_index = Some(mesh_index);

        // Enable depth testing for 3D rendering
        gl.enable(DEPTH_TEST);

        // Initialize egui
        let egui_ctx = egui::Context::default();
        let egui_painter = egui_glow::Painter::new(Arc::clone(&gl), "", None, false)
            .expect("Failed to create egui painter");

        self.egui_ctx = Some(egui_ctx);
        self.egui_painter = Some(egui_painter);

        println!("[Game] Rotating cube initialized successfully");
        println!("[Game] Using MeshRenderer with proper lighting and materials");
        println!("[Game] Hot-reload ready! Try changing rotation speed.");
    }

    unsafe fn uninit(&mut self, gl: Arc<Context>) {
        println!("[Game] Cleaning up rotating cube");

        // Destroy egui
        if let Some(mut painter) = self.egui_painter.take() {
            painter.destroy();
        }
        self.egui_ctx = None;

        // Cleanup mesh renderer
        self.mesh_renderer.destroy_gl(&gl);
        self.mesh_index = None;

        self.gl = None;

        println!("[Game] Cleanup complete");
    }

    fn event(&mut self, event: &WindowEvent) {
        use winit::event::{ElementState, MouseButton};

        match event {
            WindowEvent::Resized(size) => {
                self.window_size = (size.width, size.height);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.pointer_pos = Some(egui::Pos2::new(position.x as f32, position.y as f32));
            }
            WindowEvent::CursorLeft { .. } => {
                self.pointer_pos = None;
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let egui_button = match button {
                    MouseButton::Left => egui::PointerButton::Primary,
                    MouseButton::Right => egui::PointerButton::Secondary,
                    MouseButton::Middle => egui::PointerButton::Middle,
                    _ => return,
                };

                let pressed = *state == ElementState::Pressed;
                self.mouse_button_events.push((egui_button, pressed));
            }
            _ => {}
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

        // Render the cube mesh
        if let Some(mesh_index) = self.mesh_index {
            // Setup camera looking at the cube
            let mut camera = Camera::default();
            camera.position = Vec3::new(0.0, 0.0, 3.0);
            camera.set_look_at(Vec3::ZERO);

            // Create rotation quaternion (rotate around Y and X axes)
            let rotation = Quat::from_rotation_y(self.rotation)
                * Quat::from_rotation_x(self.rotation * 0.5);

            // Render mesh at origin with rotation
            self.mesh_renderer.render_mesh(
                &gl,
                mesh_index,
                Vec3::ZERO,    // Position at origin
                rotation,      // Apply rotation
                &camera,
                self.window_size.0 as i32,
                self.window_size.1 as i32,
            );
        }

        // Render egui UI on top of 3D scene
        let mut should_trigger_reload = false;

        if let Some(egui_ctx) = &self.egui_ctx {
            // Setup GL state for egui (2D overlay)
            gl.disable(DEPTH_TEST);
            gl.enable(BLEND);
            gl.blend_func(SRC_ALPHA, ONE_MINUS_SRC_ALPHA);
            gl.viewport(0, 0, self.window_size.0 as i32, self.window_size.1 as i32);

            let elapsed = self.start_time.elapsed().as_secs_f64();

            // Build raw input with mouse events
            let mut raw_input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(self.window_size.0 as f32, self.window_size.1 as f32),
                )),
                time: Some(elapsed),
                predicted_dt: 1.0 / 60.0,
                ..Default::default()
            };

            // Add pointer position
            if let Some(pos) = self.pointer_pos {
                raw_input.events.push(egui::Event::PointerMoved(pos));
            }

            // Add mouse button events (both press and release)
            for (button, pressed) in self.mouse_button_events.drain(..) {
                if let Some(pos) = self.pointer_pos {
                    raw_input.events.push(egui::Event::PointerButton {
                        pos,
                        button,
                        pressed,
                        modifiers: egui::Modifiers::default(),
                    });
                }
            }

            let rotation_degrees = self.rotation.to_degrees();
            let window_size = self.window_size;
            let last_reload_trigger = self.last_reload_trigger;

            let full_output = egui_ctx.run(raw_input, |ui_ctx| {
                egui::Window::new("🎮 Hot Reload Demo")
                    .default_pos([10.0, 10.0])
                    .default_width(320.0)
                    .resizable(true)
                    .show(ui_ctx, |ui| {
                        ui.heading("Rotating Cube");
                        ui.colored_label(egui::Color32::GREEN, "✓ Using MeshRenderer!");
                        ui.separator();

                        ui.label(format!("Rotation: {:.2}°", rotation_degrees));
                        ui.label(format!("Window: {}x{}", window_size.0, window_size.1));

                        ui.separator();
                        ui.label("Colored voxel cube with proper lighting");

                        ui.separator();
                        ui.heading("🔥 Hot-Reload Benchmark");

                        if ui.button("🔄 Trigger Reload").clicked() {
                            should_trigger_reload = true;
                        }

                        if let Some(trigger_time) = last_reload_trigger {
                            let elapsed_ms = trigger_time.elapsed().as_millis();
                            ui.label(format!("⏱️  Last trigger: {}ms ago", elapsed_ms));
                        } else {
                            ui.label("Click button to test reload speed");
                        }

                        ui.separator();
                        ui.label("Edit crates/game/src/lib.rs to test!");
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

        // Trigger reload after all borrows are dropped
        if should_trigger_reload {
            self.trigger_reload();
        }
    }
}

/// Export the create_app function for dynamic loading
#[no_mangle]
pub extern "C" fn create_app() -> *mut dyn App {
    Box::into_raw(Box::new(RotatingCube::new()))
}
