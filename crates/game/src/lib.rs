mod config;

use app::{
    App, Camera, CursorMode, FirstPersonController, FirstPersonControllerConfig, FrameContext,
    InputState,
};
use config::GameConfig;
use glam::{Quat, Vec3};
use glow::*;
use renderer::{MeshRenderer, SkyboxRenderer};
use std::path::PathBuf;
use winit::keyboard::KeyCode;

/// Check if debug mode is enabled via environment variable
fn is_debug_mode() -> bool {
    std::env::var("GAME_DEBUG")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false)
}

pub struct VoxelGame {
    mesh_renderer: MeshRenderer,
    skybox_renderer: SkyboxRenderer,
    world_mesh_index: Option<usize>,
    world: Option<crossworld_world::NativeWorldCube>,
    config: GameConfig,
    spawn_position: Vec3,
    camera: Camera,
    fps_controller: FirstPersonController,
}

impl Default for VoxelGame {
    fn default() -> Self {
        Self {
            mesh_renderer: MeshRenderer::new(),
            skybox_renderer: SkyboxRenderer::new(),
            world_mesh_index: None,
            world: None,
            config: GameConfig::default(),
            spawn_position: Vec3::new(0.0, 2.0, 5.0),
            camera: Camera::from_pitch_yaw(Vec3::new(0.0, 2.0, 5.0), 0.0, 0.0),
            fps_controller: FirstPersonController::new(FirstPersonControllerConfig::default()),
        }
    }
}

impl VoxelGame {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load world configuration from Lua file
    fn load_config(&mut self) {
        let mut config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        config_path.push("config");
        config_path.push("world.lua");

        match GameConfig::from_file(&config_path) {
            Ok(config) => {
                println!("[Game] Loaded config from: {}", config_path.display());
                println!(
                    "[Game] World config: macro_depth={}, micro_depth={}, border_depth={}, seed={}",
                    config.world.macro_depth,
                    config.world.micro_depth,
                    config.world.border_depth,
                    config.world.seed
                );
                println!("[Game] Map layout: {} rows", config.map.layout.len());
                self.config = config;
            }
            Err(e) => {
                eprintln!("[Game] Failed to load config: {}", e);
                eprintln!("[Game] Using default configuration");
                self.config = GameConfig::default();
            }
        }
    }

    /// Initialize the voxel world from config
    fn init_world(&mut self) {
        let debug = is_debug_mode();

        if debug {
            println!("[Game] Initializing world in DEBUG mode");
            println!(
                "[Game] World config: macro_depth={}, micro_depth={}, border_depth={}, seed={}",
                self.config.world.macro_depth,
                self.config.world.micro_depth,
                self.config.world.border_depth,
                self.config.world.seed
            );
        }

        let mut world = crossworld_world::NativeWorldCube::new(
            self.config.world.macro_depth,
            self.config.world.micro_depth,
            self.config.world.border_depth,
            self.config.world.seed,
        );

        if debug {
            println!("[Game] World cube created, applying 2D map...");
        }

        // Apply 2D map to world
        if let Some(spawn) = self.config.apply_map_to_world(&mut world, debug) {
            self.spawn_position = spawn;
            self.camera.position = spawn + Vec3::new(0.0, 1.6, 0.0); // Eye height
            println!("[Game] Spawn position set to: {:?}", self.spawn_position);
        }

        // Debug: Log cube model structure
        if debug {
            let csm = cube::serialize_csm(world.root());
            let csm_preview = if csm.len() > 500 {
                format!("{}... (truncated, {} total chars)", &csm[..500], csm.len())
            } else {
                csm.clone()
            };
            println!("[Game] Resulting cube model (CSM):");
            println!("{}", csm_preview);
        }

        self.world = Some(world);
        println!("[Game] World initialized successfully");
    }

    /// Update world mesh from current world state
    fn update_world_mesh(&mut self, gl: &Context) {
        if let Some(world) = &self.world {
            use std::rc::Rc;

            // Get the cube root and wrap in Rc for upload_mesh
            let cube_rc = Rc::new(world.root().clone());

            unsafe {
                let depth = self.config.world.macro_depth
                    + self.config.world.micro_depth
                    + self.config.world.border_depth;
                match self.mesh_renderer.upload_mesh(gl, &cube_rc, depth) {
                    Ok(mesh_index) => {
                        self.world_mesh_index = Some(mesh_index);
                        println!("[Game] World mesh uploaded successfully");
                    }
                    Err(e) => {
                        eprintln!("[Game] Failed to upload world mesh: {}", e);
                    }
                }
            }
        }
    }
}

impl App for VoxelGame {
    fn init(&mut self, ctx: &FrameContext) {
        println!("[Game] Initializing voxel game with world generation");

        // Load configuration
        self.load_config();

        unsafe {
            self.skybox_renderer
                .init_gl(ctx.gl)
                .expect("Failed to initialize SkyboxRenderer");

            self.mesh_renderer
                .init_gl(ctx.gl)
                .expect("Failed to initialize MeshRenderer");

            ctx.gl.enable(DEPTH_TEST);
        }

        // Initialize world and generate mesh
        self.init_world();
        self.update_world_mesh(ctx.gl);

        println!("[Game] Voxel game initialized successfully");
        println!("[Game] Controls: WASD to move, Mouse to look, Right-click to capture mouse");
    }

    fn shutdown(&mut self, ctx: &FrameContext) {
        println!("[Game] Cleaning up voxel game");

        unsafe {
            self.skybox_renderer.destroy_gl(ctx.gl);
            self.mesh_renderer.destroy_gl(ctx.gl);
        }

        self.world_mesh_index = None;
        self.world = None;

        println!("[Game] Cleanup complete");
    }

    fn on_event(&mut self, event: &winit::event::WindowEvent) -> bool {
        use winit::event::{ElementState, MouseButton, WindowEvent};

        if let WindowEvent::MouseInput { state, button, .. } = event {
            if *button == MouseButton::Right && *state == ElementState::Pressed {
                self.fps_controller.toggle_mouse_capture();
                return true;
            }
        }
        false
    }

    fn update(&mut self, ctx: &FrameContext, input: &InputState) {
        // Handle controller camera movement
        if let Some(gamepad) = &input.gamepad {
            if gamepad.right_stick.length() > 0.01 {
                let sensitivity = 3.0 * ctx.delta_time;
                let delta_x = -gamepad.right_stick.x * sensitivity;
                let delta_y = gamepad.right_stick.y * sensitivity;
                self.fps_controller
                    .apply_rotation(&mut self.camera, delta_x, delta_y);
            }
        }

        // Handle raw mouse movement for FPS camera
        if input.raw_mouse_delta.x.abs() > 0.001 || input.raw_mouse_delta.y.abs() > 0.001 {
            self.fps_controller.handle_mouse_move(
                &mut self.camera,
                input.raw_mouse_delta.x,
                input.raw_mouse_delta.y,
            );
        }

        // Handle keyboard input for FPS camera movement
        let forward = input.is_key_pressed(KeyCode::KeyW);
        let backward = input.is_key_pressed(KeyCode::KeyS);
        let left = input.is_key_pressed(KeyCode::KeyA);
        let right = input.is_key_pressed(KeyCode::KeyD);
        let up = input.is_key_pressed(KeyCode::Space);
        let down =
            input.is_key_pressed(KeyCode::ShiftLeft) || input.is_key_pressed(KeyCode::ShiftRight);

        // Get controller movement input
        let (controller_x, controller_y_vertical, controller_z) = input
            .gamepad
            .as_ref()
            .map(|g| {
                let vertical = g.right_trigger - g.left_trigger;
                (g.left_stick.x, vertical, g.left_stick.y)
            })
            .unwrap_or((0.0, 0.0, 0.0));

        // Combine keyboard and controller input
        let mut total_velocity = self.fps_controller.calculate_velocity(
            &self.camera,
            forward,
            backward,
            left,
            right,
            up,
            down,
        );

        // Add controller movement
        if controller_x.abs() > 0.01
            || controller_z.abs() > 0.01
            || controller_y_vertical.abs() > 0.01
        {
            let fwd = self.camera.forward();
            let fwd_xz = Vec3::new(fwd.x, 0.0, fwd.z).normalize_or_zero();
            let rgt = self.camera.right();
            let rgt_xz = Vec3::new(rgt.x, 0.0, rgt.z).normalize_or_zero();

            let controller_move_dir = rgt_xz * controller_x + fwd_xz * controller_z;
            let controller_vel = Vec3::new(
                controller_move_dir.x * self.fps_controller.config.move_speed,
                controller_y_vertical * self.fps_controller.config.move_speed,
                controller_move_dir.z * self.fps_controller.config.move_speed,
            );
            total_velocity += controller_vel;
        }

        self.camera.position += total_velocity * ctx.delta_time;
    }

    fn render(&mut self, ctx: &FrameContext) {
        unsafe {
            ctx.gl.clear_color(0.1, 0.1, 0.1, 1.0);
            ctx.gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);

            self.skybox_renderer
                .render(ctx.gl, &self.camera, ctx.size.0 as i32, ctx.size.1 as i32);

            if let Some(mesh_index) = self.world_mesh_index {
                self.mesh_renderer.render_mesh(
                    ctx.gl,
                    mesh_index,
                    Vec3::ZERO,
                    Quat::IDENTITY,
                    &self.camera,
                    ctx.size.0 as i32,
                    ctx.size.1 as i32,
                );
            }
        }
    }

    fn ui(&mut self, ctx: &FrameContext, egui_ctx: &egui::Context) {
        let camera_pos = self.camera.position;
        let mouse_captured = self.fps_controller.mouse_captured;

        egui::Window::new("Voxel Game")
            .fixed_pos([10.0, 10.0])
            .default_width(320.0)
            .resizable(false)
            .movable(false)
            .title_bar(false)
            .frame(
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgba_premultiplied(0, 0, 0, 180))
                    .inner_margin(egui::Margin::same(10)),
            )
            .show(egui_ctx, |ui| {
                ui.heading("Voxel World Demo");
                ui.separator();

                ui.label(format!("Window: {}x{}", ctx.size.0, ctx.size.1));

                ui.separator();
                ui.heading("Camera");
                ui.label(format!(
                    "Pos: ({:.1}, {:.1}, {:.1})",
                    camera_pos.x, camera_pos.y, camera_pos.z
                ));
                if mouse_captured {
                    ui.colored_label(egui::Color32::GREEN, "Mouse captured");
                } else {
                    ui.colored_label(egui::Color32::GRAY, "Right-click to capture");
                }

                ui.separator();
                ui.heading("World Config");
                ui.label(format!("Macro depth: {}", self.config.world.macro_depth));
                ui.label(format!("Micro depth: {}", self.config.world.micro_depth));
                ui.label(format!("Seed: {}", self.config.world.seed));

                ui.separator();
                ui.heading("Controls");
                ui.label("WASD - Move");
                ui.label("Space/Shift - Up/Down");
                ui.label("Mouse - Look around");
                ui.label("Right-click - Toggle mouse");
            });
    }

    fn cursor_mode(&self) -> CursorMode {
        if self.fps_controller.mouse_captured {
            CursorMode::Grabbed
        } else {
            CursorMode::Normal
        }
    }
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn create_app() -> *mut dyn App {
    Box::into_raw(Box::new(VoxelGame::new()))
}
