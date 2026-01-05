use app::{App, ControllerBackend, create_controller_backend};
// Hot-reload trigger: 1767203552400
use glam::{Quat, Vec3};
use glow::*;
use renderer::{Camera, MeshRenderer, SkyboxRenderer};
use std::sync::Arc;
use std::time::Instant;
use winit::event::WindowEvent;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::CursorGrabMode;

/// Camera control state
pub struct CameraController {
    pub move_speed: f32,
    pub sensitivity: f32,
    pub mouse_captured: bool,
}

impl CameraController {
    pub fn new() -> Self {
        Self {
            move_speed: 5.0,
            sensitivity: 0.003,
            mouse_captured: false,
        }
    }

    pub fn handle_mouse_move(&mut self, camera: &mut Camera, delta_x: f32, delta_y: f32) {
        if !self.mouse_captured {
            return;
        }

        // Convert mouse delta to pitch/yaw adjustments
        let yaw_delta = -delta_x * self.sensitivity;
        let pitch_delta = -delta_y * self.sensitivity;

        // Update pitch and yaw with clamping
        camera.yaw += yaw_delta;
        camera.pitch += pitch_delta;

        // Clamp pitch to prevent gimbal lock
        const MAX_PITCH: f32 = 89.0 * std::f32::consts::PI / 180.0;
        camera.pitch = camera.pitch.clamp(-MAX_PITCH, MAX_PITCH);

        // Update rotation from pitch and yaw
        camera.update_from_pitch_yaw();
    }

    pub fn apply_camera_rotation(&mut self, camera: &mut Camera, delta_x: f32, delta_y: f32) {
        camera.yaw += delta_x;
        camera.pitch += delta_y;

        // Clamp pitch to prevent gimbal lock
        const MAX_PITCH: f32 = 89.0 * std::f32::consts::PI / 180.0;
        camera.pitch = camera.pitch.clamp(-MAX_PITCH, MAX_PITCH);

        camera.update_from_pitch_yaw();
    }

    pub fn calculate_velocity(
        &self,
        camera: &Camera,
        forward: bool,
        backward: bool,
        left: bool,
        right: bool,
        up: bool,
        down: bool,
    ) -> Vec3 {
        let mut velocity = Vec3::ZERO;

        // Get forward and right vectors in XZ plane (for movement)
        let fwd = camera.forward();
        let fwd_xz = Vec3::new(fwd.x, 0.0, fwd.z).normalize_or_zero();
        let rgt = camera.right();
        let rgt_xz = Vec3::new(rgt.x, 0.0, rgt.z).normalize_or_zero();

        if forward {
            velocity += fwd_xz;
        }
        if backward {
            velocity -= fwd_xz;
        }
        if left {
            velocity -= rgt_xz;
        }
        if right {
            velocity += rgt_xz;
        }
        if up {
            velocity.y += 1.0;
        }
        if down {
            velocity.y -= 1.0;
        }

        if velocity.length_squared() > 0.0 {
            velocity = velocity.normalize() * self.move_speed;
        }

        velocity
    }
}

pub struct RotatingCube {
    mesh_renderer: MeshRenderer,
    skybox_renderer: SkyboxRenderer,
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
    // Camera and controller
    camera: Camera,
    camera_controller: CameraController,
    // Keyboard input state
    keys_pressed: std::collections::HashSet<KeyCode>,
    // Raw mouse delta accumulator (from device events)
    raw_mouse_delta: (f64, f64),
    // Controller backend
    controller_backend: Option<Box<dyn ControllerBackend>>,
}

impl RotatingCube {
    pub fn new() -> Self {
        // Initialize controller backend
        let controller_backend = create_controller_backend();

        Self {
            mesh_renderer: MeshRenderer::new(),
            skybox_renderer: SkyboxRenderer::new(),
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
            camera: Camera::from_pitch_yaw(Vec3::new(0.0, 2.0, 5.0), 0.0, 0.0),
            camera_controller: CameraController::new(),
            keys_pressed: std::collections::HashSet::new(),
            raw_mouse_delta: (0.0, 0.0),
            controller_backend,
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

        // Initialize skybox renderer
        self.skybox_renderer
            .init_gl(&gl)
            .expect("Failed to initialize SkyboxRenderer");

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

        // Cleanup skybox renderer
        self.skybox_renderer.destroy_gl(&gl);

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
                // Store pointer position for egui only
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

                // Right-click to toggle mouse capture for FPS camera
                if *button == MouseButton::Right && pressed {
                    self.camera_controller.mouse_captured = !self.camera_controller.mouse_captured;
                }
            }
            WindowEvent::KeyboardInput { event: key_event, .. } => {
                if let PhysicalKey::Code(keycode) = key_event.physical_key {
                    match key_event.state {
                        ElementState::Pressed => {
                            self.keys_pressed.insert(keycode);
                        }
                        ElementState::Released => {
                            self.keys_pressed.remove(&keycode);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn update(&mut self, delta_time: f32) {
        // Rotate at 45 degrees per second (change this value to test hot-reload!)
        self.rotation += 45.0_f32.to_radians() * delta_time;

        // Poll controller backend for gamepad input
        if let Some(backend) = &mut self.controller_backend {
            backend.poll();
        }

        // Get controller input from the first connected controller
        let controller = self.controller_backend.as_mut().and_then(|b| b.get_first_controller());

        // Handle controller camera movement
        if let Some(ctrl) = controller {
            let (controller_delta_x, controller_delta_y) = ctrl.get_camera_delta(delta_time);
            // Always apply controller input if it exists (no threshold)
            if ctrl.gamepad.right_stick.length() > 0.01 {
                // Controller delta already includes sensitivity, apply rotation
                self.camera_controller.apply_camera_rotation(&mut self.camera, -controller_delta_x, controller_delta_y);
            }
        }

        // Handle raw mouse movement for FPS camera (from device events)
        if self.raw_mouse_delta.0.abs() > 0.001 || self.raw_mouse_delta.1.abs() > 0.001 {
            self.camera_controller.handle_mouse_move(
                &mut self.camera,
                self.raw_mouse_delta.0 as f32,
                self.raw_mouse_delta.1 as f32
            );
            self.raw_mouse_delta = (0.0, 0.0);
        }

        // Handle keyboard input for FPS camera movement
        let forward = self.keys_pressed.contains(&KeyCode::KeyW);
        let backward = self.keys_pressed.contains(&KeyCode::KeyS);
        let left = self.keys_pressed.contains(&KeyCode::KeyA);
        let right = self.keys_pressed.contains(&KeyCode::KeyD);
        let up = self.keys_pressed.contains(&KeyCode::Space);
        let down = self.keys_pressed.contains(&KeyCode::ShiftLeft)
                || self.keys_pressed.contains(&KeyCode::ShiftRight);

        // Get controller movement input (left stick for movement, right stick for camera, triggers for vertical)
        let controller = self.controller_backend.as_mut().and_then(|b| b.get_first_controller());
        let (controller_x, controller_y_vertical, controller_z) = controller
            .map(|c| c.get_movement_input())
            .unwrap_or((0.0, 0.0, 0.0));

        // Combine keyboard and controller input
        let mut total_velocity = self.camera_controller.calculate_velocity(&self.camera, forward, backward, left, right, up, down);

        // Add controller movement
        if controller_x.abs() > 0.01 || controller_z.abs() > 0.01 || controller_y_vertical.abs() > 0.01 {
            // Get camera direction vectors in XZ plane
            let fwd = self.camera.forward();
            let fwd_xz = Vec3::new(fwd.x, 0.0, fwd.z).normalize_or_zero();
            let rgt = self.camera.right();
            let rgt_xz = Vec3::new(rgt.x, 0.0, rgt.z).normalize_or_zero();

            let controller_move_dir = rgt_xz * controller_x + fwd_xz * controller_z;
            let controller_vel = Vec3::new(
                controller_move_dir.x * self.camera_controller.move_speed,
                controller_y_vertical * self.camera_controller.move_speed,
                controller_move_dir.z * self.camera_controller.move_speed,
            );
            total_velocity += controller_vel;
        }

        self.camera.position += total_velocity * delta_time;
    }

    unsafe fn render(&mut self, gl: Arc<Context>) {
        // Clear the screen
        gl.clear_color(0.1, 0.1, 0.1, 1.0);
        gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);

        // Render skybox first (background)
        self.skybox_renderer.render(
            &gl,
            &self.camera,
            self.window_size.0 as i32,
            self.window_size.1 as i32,
        );

        // Render the cube mesh on top of skybox
        if let Some(mesh_index) = self.mesh_index {
            // Create rotation quaternion (rotate around Y and X axes)
            let rotation = Quat::from_rotation_y(self.rotation)
                * Quat::from_rotation_x(self.rotation * 0.5);

            // Render mesh at origin with rotation
            self.mesh_renderer.render_mesh(
                &gl,
                mesh_index,
                Vec3::ZERO,    // Position at origin
                rotation,      // Apply rotation
                &self.camera,
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
            let camera_pos = self.camera.position;
            let mouse_captured = self.camera_controller.mouse_captured;

            // Get controller state for UI display
            let controller = self.controller_backend.as_mut().and_then(|b| b.get_first_controller());
            let (right_stick, left_stick, left_trigger, right_trigger, has_controller_input, gamepad_connected) =
                controller.map(|c| {
                    (c.gamepad.right_stick, c.gamepad.left_stick, c.gamepad.left_trigger, c.gamepad.right_trigger, c.has_input(), c.gamepad.connected)
                }).unwrap_or_default();

            let full_output = egui_ctx.run(raw_input, |ui_ctx| {
                egui::Window::new("🎮 Hot Reload Demo")
                    .fixed_pos([10.0, 10.0])
                    .default_width(320.0)
                    .resizable(false)
                    .movable(false)
                    .title_bar(false)
                    .frame(egui::Frame::none()
                        .fill(egui::Color32::from_rgba_premultiplied(0, 0, 0, 180))
                        .inner_margin(egui::Margin::same(10.0)))
                    .show(ui_ctx, |ui| {
                        ui.heading("First Person Demo");
                        ui.colored_label(egui::Color32::GREEN, "✓ With Skybox!");
                        ui.separator();

                        ui.label(format!("Cube rotation: {:.2}°", rotation_degrees));
                        ui.label(format!("Window: {}x{}", window_size.0, window_size.1));

                        ui.separator();
                        ui.heading("📷 Camera");
                        ui.label(format!("Pos: ({:.1}, {:.1}, {:.1})", camera_pos.x, camera_pos.y, camera_pos.z));
                        if mouse_captured {
                            ui.colored_label(egui::Color32::GREEN, "🖱️  Mouse captured");
                        } else {
                            ui.colored_label(egui::Color32::GRAY, "🖱️  Right-click to capture");
                        }

                        ui.separator();
                        ui.heading("⌨️  Controls");
                        ui.label("WASD - Move");
                        ui.label("Space/Shift - Up/Down");
                        ui.label("Mouse - Look around");
                        ui.label("Right-click - Toggle mouse");

                        ui.separator();
                        ui.heading("🎮 Gamepad");
                        if gamepad_connected {
                            ui.colored_label(egui::Color32::GREEN, "✓ Connected");
                        } else {
                            ui.colored_label(egui::Color32::GRAY, "○ Not connected");
                        }

                        if has_controller_input {
                            if right_stick.length() > 0.01 {
                                ui.label(format!("👁  Look: ({:.2}, {:.2})", right_stick.x, right_stick.y));
                            }
                            if left_stick.length() > 0.01 {
                                ui.label(format!("🚶 Move: ({:.2}, {:.2})", left_stick.x, left_stick.y));
                            }
                            if left_trigger > 0.01 || right_trigger > 0.01 {
                                ui.label(format!("⬇️⬆️  Triggers: {:.2} / {:.2}", left_trigger, right_trigger));
                            }
                        }

                        ui.separator();
                        ui.heading("🔥 Hot-Reload");

                        if ui.button("🔄 Trigger Reload").clicked() {
                            should_trigger_reload = true;
                        }

                        if let Some(trigger_time) = last_reload_trigger {
                            let elapsed_ms = trigger_time.elapsed().as_millis();
                            ui.label(format!("⏱️  Last: {}ms ago", elapsed_ms));
                        }
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

    fn cursor_state(&self) -> Option<(CursorGrabMode, bool)> {
        if self.camera_controller.mouse_captured {
            // When mouse is captured: grab cursor and hide it
            Some((CursorGrabMode::Locked, false))
        } else {
            // When not captured: free cursor and show it
            Some((CursorGrabMode::None, true))
        }
    }

    fn mouse_motion(&mut self, delta: (f64, f64)) {
        // Only accumulate raw mouse motion when mouse is captured
        if self.camera_controller.mouse_captured {
            self.raw_mouse_delta.0 += delta.0;
            self.raw_mouse_delta.1 += delta.1;
        }
    }
}

/// Export the create_app function for dynamic loading
#[no_mangle]
pub extern "C" fn create_app() -> *mut dyn App {
    Box::into_raw(Box::new(RotatingCube::new()))
}
