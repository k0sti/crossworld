use app::{App, CursorMode, FrameContext, InputState};
// Hot-reload trigger: 1767968902506
use glam::{Quat, Vec3};
use glow::*;
use renderer::{Camera, MeshRenderer, SkyboxRenderer};
use std::time::Instant;
use winit::keyboard::KeyCode;

/// Camera control state
pub struct CameraController {
    pub move_speed: f32,
    pub sensitivity: f32,
    pub mouse_captured: bool,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            move_speed: 5.0,
            sensitivity: 0.003,
            mouse_captured: false,
        }
    }
}

impl CameraController {
    pub fn new() -> Self {
        Self::default()
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

    #[allow(clippy::too_many_arguments)]
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
    #[allow(dead_code)]
    start_time: Instant,
    last_reload_trigger: Option<Instant>,
    // Camera and controller
    camera: Camera,
    camera_controller: CameraController,
}

impl Default for RotatingCube {
    fn default() -> Self {
        Self {
            mesh_renderer: MeshRenderer::new(),
            skybox_renderer: SkyboxRenderer::new(),
            mesh_index: None,
            rotation: 0.0,
            start_time: Instant::now(),
            last_reload_trigger: None,
            camera: Camera::from_pitch_yaw(Vec3::new(0.0, 2.0, 5.0), 0.0, 0.0),
            camera_controller: CameraController::new(),
        }
    }
}

impl RotatingCube {
    pub fn new() -> Self {
        Self::default()
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
                    println!("[Game] Triggered hot-reload by modifying source file (timestamp: {})", timestamp);
                }
                Err(e) => {
                    eprintln!("[Game] Failed to write source file: {}", e);
                }
            }
        } else {
            eprintln!("[Game] Failed to read source file");
        }
    }
}

impl App for RotatingCube {
    fn init(&mut self, ctx: &FrameContext) {
        println!("[Game] Initializing rotating cube with MeshRenderer");

        unsafe {
            // Initialize skybox renderer
            self.skybox_renderer
                .init_gl(ctx.gl)
                .expect("Failed to initialize SkyboxRenderer");

            // Initialize mesh renderer
            self.mesh_renderer
                .init_gl(ctx.gl)
                .expect("Failed to initialize MeshRenderer");

            // Create a simple colored cube scene
            let cube = renderer::create_octa_cube();

            // Upload mesh to GPU (depth 1 since octa_cube is a simple 2x2x2 octree)
            let mesh_index = self
                .mesh_renderer
                .upload_mesh(ctx.gl, &cube, 1)
                .expect("Failed to upload cube mesh");
            self.mesh_index = Some(mesh_index);

            // Enable depth testing for 3D rendering
            ctx.gl.enable(DEPTH_TEST);
        }

        println!("[Game] Rotating cube initialized successfully");
        println!("[Game] Using MeshRenderer with proper lighting and materials");
        println!("[Game] Hot-reload ready! Try changing rotation speed.");
    }

    fn shutdown(&mut self, ctx: &FrameContext) {
        println!("[Game] Cleaning up rotating cube");

        unsafe {
            // Cleanup skybox renderer
            self.skybox_renderer.destroy_gl(ctx.gl);

            // Cleanup mesh renderer
            self.mesh_renderer.destroy_gl(ctx.gl);
        }
        self.mesh_index = None;

        println!("[Game] Cleanup complete");
    }

    fn on_event(&mut self, event: &winit::event::WindowEvent) -> bool {
        use winit::event::{ElementState, MouseButton, WindowEvent};

        // Right-click to toggle mouse capture for FPS camera
        if let WindowEvent::MouseInput { state, button, .. } = event {
            if *button == MouseButton::Right && *state == ElementState::Pressed {
                self.camera_controller.mouse_captured = !self.camera_controller.mouse_captured;
                return true;
            }
        }
        false
    }

    fn update(&mut self, ctx: &FrameContext, input: &InputState) {
        // Rotate at 45 degrees per second (change this value to test hot-reload!)
        self.rotation += 45.0_f32.to_radians() * ctx.delta_time;

        // Handle controller camera movement
        if let Some(gamepad) = &input.gamepad {
            if gamepad.right_stick.length() > 0.01 {
                // Apply controller look
                let sensitivity = 3.0 * ctx.delta_time;
                let delta_x = -gamepad.right_stick.x * sensitivity;
                let delta_y = gamepad.right_stick.y * sensitivity;
                self.camera_controller.apply_camera_rotation(&mut self.camera, delta_x, delta_y);
            }
        }

        // Handle raw mouse movement for FPS camera
        if input.raw_mouse_delta.x.abs() > 0.001 || input.raw_mouse_delta.y.abs() > 0.001 {
            self.camera_controller.handle_mouse_move(
                &mut self.camera,
                input.raw_mouse_delta.x,
                input.raw_mouse_delta.y
            );
        }

        // Handle keyboard input for FPS camera movement
        let forward = input.is_key_pressed(KeyCode::KeyW);
        let backward = input.is_key_pressed(KeyCode::KeyS);
        let left = input.is_key_pressed(KeyCode::KeyA);
        let right = input.is_key_pressed(KeyCode::KeyD);
        let up = input.is_key_pressed(KeyCode::Space);
        let down = input.is_key_pressed(KeyCode::ShiftLeft) || input.is_key_pressed(KeyCode::ShiftRight);

        // Get controller movement input
        let (controller_x, controller_y_vertical, controller_z) = input.gamepad
            .as_ref()
            .map(|g| {
                let vertical = g.right_trigger - g.left_trigger;
                (g.left_stick.x, vertical, g.left_stick.y)
            })
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

        self.camera.position += total_velocity * ctx.delta_time;
    }

    fn render(&mut self, ctx: &FrameContext) {
        unsafe {
            // Clear the screen
            ctx.gl.clear_color(0.1, 0.1, 0.1, 1.0);
            ctx.gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);

            // Render skybox first (background)
            self.skybox_renderer.render(
                ctx.gl,
                &self.camera,
                ctx.size.0 as i32,
                ctx.size.1 as i32,
            );

            // Render the cube mesh on top of skybox
            if let Some(mesh_index) = self.mesh_index {
                // Create rotation quaternion (rotate around Y and X axes)
                let rotation = Quat::from_rotation_y(self.rotation)
                    * Quat::from_rotation_x(self.rotation * 0.5);

                // Render mesh at origin with rotation
                self.mesh_renderer.render_mesh(
                    ctx.gl,
                    mesh_index,
                    Vec3::ZERO,    // Position at origin
                    rotation,      // Apply rotation
                    &self.camera,
                    ctx.size.0 as i32,
                    ctx.size.1 as i32,
                );
            }
        }
    }

    fn ui(&mut self, ctx: &FrameContext, egui_ctx: &egui::Context) {
        let rotation_degrees = self.rotation.to_degrees();
        let camera_pos = self.camera.position;
        let mouse_captured = self.camera_controller.mouse_captured;
        let last_reload_trigger = self.last_reload_trigger;

        // Get gamepad state for UI display (we need to store this temporarily)
        let mut should_trigger_reload = false;

        egui::Window::new("Hot Reload Demo")
            .fixed_pos([10.0, 10.0])
            .default_width(320.0)
            .resizable(false)
            .movable(false)
            .title_bar(false)
            .frame(egui::Frame::NONE
                .fill(egui::Color32::from_rgba_premultiplied(0, 0, 0, 180))
                .inner_margin(egui::Margin::same(10)))
            .show(egui_ctx, |ui| {
                ui.heading("First Person Demo");
                ui.colored_label(egui::Color32::GREEN, "With Skybox!");
                ui.separator();

                ui.label(format!("Cube rotation: {:.2}", rotation_degrees));
                ui.label(format!("Window: {}x{}", ctx.size.0, ctx.size.1));

                ui.separator();
                ui.heading("Camera");
                ui.label(format!("Pos: ({:.1}, {:.1}, {:.1})", camera_pos.x, camera_pos.y, camera_pos.z));
                if mouse_captured {
                    ui.colored_label(egui::Color32::GREEN, "Mouse captured");
                } else {
                    ui.colored_label(egui::Color32::GRAY, "Right-click to capture");
                }

                ui.separator();
                ui.heading("Controls");
                ui.label("WASD - Move");
                ui.label("Space/Shift - Up/Down");
                ui.label("Mouse - Look around");
                ui.label("Right-click - Toggle mouse");

                ui.separator();
                ui.heading("Hot-Reload");

                if ui.button("Trigger Reload").clicked() {
                    should_trigger_reload = true;
                }

                if let Some(trigger_time) = last_reload_trigger {
                    let elapsed_ms = trigger_time.elapsed().as_millis();
                    ui.label(format!("Last: {}ms ago", elapsed_ms));
                }
            });

        if should_trigger_reload {
            self.trigger_reload();
        }
    }

    fn cursor_mode(&self) -> CursorMode {
        if self.camera_controller.mouse_captured {
            CursorMode::Grabbed
        } else {
            CursorMode::Normal
        }
    }
}

/// Export the create_app function for dynamic loading
///
/// Note: This uses `dyn App` which isn't strictly FFI-safe, but this is only used
/// for hot-reload between Rust code, not for interop with other languages.
#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn create_app() -> *mut dyn App {
    Box::into_raw(Box::new(RotatingCube::new()))
}
