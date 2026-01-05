//! Physics Component Testbed
//!
//! Compares physics behavior between:
//! - Left: Simple flat ground using cuboid collider
//! - Right: Flat ground constructed from Cube objects using terrain collider

use app::{App, FrameContext, InputState};
use crossworld_physics::{
    create_box_collider,
    rapier3d::prelude::{ColliderBuilder, RigidBodyBuilder},
    CubeObject, PhysicsWorld, VoxelColliderBuilder,
};
use cube::Cube;
use glam::{Quat, Vec2, Vec3};
use glow::{Context, HasContext, COLOR_BUFFER_BIT, DEPTH_BUFFER_BIT, SCISSOR_TEST};
use renderer::{Camera, MeshRenderer, OrbitController, OrbitControllerConfig};
use std::rc::Rc;
use std::time::Instant;

/// Physics state for display
#[derive(Default, Clone)]
pub struct PhysicsState {
    pub falling_position: Vec3,
    pub falling_velocity: Vec3,
    pub is_on_ground: bool,
}

impl PhysicsState {
    pub fn format_compact(&self) -> String {
        let ground_status = if self.is_on_ground { "HIT" } else { "AIR" };
        format!(
            "Y: {:.2} | Vel: {:.2} | {}",
            self.falling_position.y, self.falling_velocity.y, ground_status
        )
    }
}

/// Physics testbed state for one side (left = cuboid, right = terrain)
pub struct PhysicsScene {
    pub world: PhysicsWorld,
    pub falling_object: CubeObject,
    pub ground_mesh_index: Option<usize>,
    pub falling_mesh_index: Option<usize>,
    #[allow(dead_code)]
    pub ground_collider_type: &'static str,
    pub state: PhysicsState,
}

/// Physics testbed application
pub struct PhysicsTestbed {
    // Rendering
    mesh_renderer: MeshRenderer,
    camera: Camera,
    orbit_controller: OrbitController,

    // Physics scenes
    left_scene: Option<PhysicsScene>,  // Cuboid ground
    right_scene: Option<PhysicsScene>, // Terrain collider ground

    // Cubes for rendering
    ground_cube: Option<Rc<Cube<u8>>>,
    #[allow(dead_code)]
    falling_cube: Option<Rc<Cube<u8>>>,

    // UI state
    reset_requested: bool,

    // Timing
    frame_count: u64,
    debug_frames: Option<u64>,
    physics_dt: f32,
    last_physics_update: Instant,
}

impl Default for PhysicsTestbed {
    fn default() -> Self {
        Self::new()
    }
}

impl PhysicsTestbed {
    pub fn new() -> Self {
        // Camera target is the top of the ground cube (Y=0)
        let camera_target = Vec3::new(0.0, 0.0, 0.0);
        // Camera positioned above and to the side, looking at ground top
        let camera_position = Vec3::new(10.0, 8.0, 10.0);

        // Configure orbit controller with appropriate limits
        let orbit_config = OrbitControllerConfig {
            mouse_sensitivity: 0.005,
            zoom_sensitivity: 0.5,
            min_distance: 2.0,
            max_distance: 30.0,
        };

        Self {
            mesh_renderer: MeshRenderer::new(),
            camera: Camera::look_at(camera_position, camera_target, Vec3::Y),
            orbit_controller: OrbitController::new(camera_target, orbit_config),
            left_scene: None,
            right_scene: None,
            ground_cube: None,
            falling_cube: None,
            reset_requested: false,
            frame_count: 0,
            debug_frames: None,
            physics_dt: 1.0 / 60.0,
            last_physics_update: Instant::now(),
        }
    }

    pub fn with_debug_frames(mut self, frames: Option<u64>) -> Self {
        self.debug_frames = frames;
        self
    }

    /// Create physics scene with simple cuboid ground collider
    fn create_cuboid_ground_scene(&self) -> PhysicsScene {
        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

        // Create ground as a cube matching the terrain collider
        let ground_collider = ColliderBuilder::cuboid(0.25, 0.25, 0.25)
            .translation([0.0, -0.25, 0.0].into())
            .build();
        world.add_static_collider(ground_collider);

        // Create small falling cube at Y=8
        let mut falling_object = CubeObject::new_dynamic(&mut world, Vec3::new(0.0, 8.0, 0.0), 1.0);
        let falling_collider = create_box_collider(Vec3::new(0.4, 0.4, 0.4));
        falling_object.attach_collider(&mut world, falling_collider);

        PhysicsScene {
            world,
            falling_object,
            ground_mesh_index: None,
            falling_mesh_index: None,
            ground_collider_type: "Cuboid",
            state: PhysicsState::default(),
        }
    }

    /// Create physics scene with terrain collider from Cube objects
    fn create_terrain_ground_scene(&self, ground_cube: &Rc<Cube<u8>>) -> PhysicsScene {
        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

        // Create ground using VoxelColliderBuilder from a solid cube
        let terrain_collider = VoxelColliderBuilder::from_cube_scaled(ground_cube, 0, 0.5);
        let terrain_body = RigidBodyBuilder::fixed()
            .translation([0.0, -0.25, 0.0].into())
            .build();
        let terrain_body_handle = world.add_rigid_body(terrain_body);
        world.add_collider(terrain_collider, terrain_body_handle);

        // Create small falling cube at Y=8
        let mut falling_object = CubeObject::new_dynamic(&mut world, Vec3::new(0.0, 8.0, 0.0), 1.0);
        let falling_collider = create_box_collider(Vec3::new(0.4, 0.4, 0.4));
        falling_object.attach_collider(&mut world, falling_collider);

        PhysicsScene {
            world,
            falling_object,
            ground_mesh_index: None,
            falling_mesh_index: None,
            ground_collider_type: "Terrain (Cube)",
            state: PhysicsState::default(),
        }
    }

    /// Reset physics scenes to initial state
    ///
    /// # Safety
    /// The GL context must be current when this is called.
    unsafe fn reset_scenes(&mut self, gl: &Context) {
        if let Some(ground_cube) = self.ground_cube.clone() {
            self.left_scene = Some(self.create_cuboid_ground_scene());
            self.right_scene = Some(self.create_terrain_ground_scene(&ground_cube));

            // Re-upload meshes
            self.upload_meshes(gl);
        }
    }

    /// Upload meshes for rendering
    ///
    /// # Safety
    /// The GL context must be current when this is called.
    unsafe fn upload_meshes(&mut self, gl: &Context) {
        // Create ground cube - a solid cube (green-ish color)
        let ground_cube = Rc::new(Cube::Solid(156u8));
        self.ground_cube = Some(ground_cube.clone());

        // Create falling cube (red color, smaller)
        let falling_cube = Rc::new(Cube::Solid(224u8));
        self.falling_cube = Some(falling_cube.clone());

        // Upload ground mesh (for both scenes)
        match self.mesh_renderer.upload_mesh(gl, &ground_cube, 1) {
            Ok(idx) => {
                if let Some(scene) = &mut self.left_scene {
                    scene.ground_mesh_index = Some(idx);
                }
            }
            Err(e) => eprintln!("Failed to upload ground mesh: {}", e),
        }

        // Upload second ground mesh for right scene
        match self.mesh_renderer.upload_mesh(gl, &ground_cube, 1) {
            Ok(idx) => {
                if let Some(scene) = &mut self.right_scene {
                    scene.ground_mesh_index = Some(idx);
                }
            }
            Err(e) => eprintln!("Failed to upload ground mesh: {}", e),
        }

        // Upload falling cube mesh (for left scene)
        match self.mesh_renderer.upload_mesh(gl, &falling_cube, 1) {
            Ok(idx) => {
                if let Some(scene) = &mut self.left_scene {
                    scene.falling_mesh_index = Some(idx);
                }
            }
            Err(e) => eprintln!("Failed to upload falling mesh: {}", e),
        }

        // Upload falling cube mesh (for right scene)
        match self.mesh_renderer.upload_mesh(gl, &falling_cube, 1) {
            Ok(idx) => {
                if let Some(scene) = &mut self.right_scene {
                    scene.falling_mesh_index = Some(idx);
                }
            }
            Err(e) => eprintln!("Failed to upload falling mesh: {}", e),
        }
    }

    /// Step physics for both scenes
    fn step_physics(&mut self) {
        let now = Instant::now();
        let elapsed = (now - self.last_physics_update).as_secs_f32();

        // Fixed timestep physics
        if elapsed >= self.physics_dt {
            self.last_physics_update = now;

            if let Some(scene) = &mut self.left_scene {
                scene.world.step(self.physics_dt);

                // Update physics state
                let pos = scene.falling_object.position(&scene.world);
                let vel = scene.falling_object.velocity(&scene.world);
                let is_on_ground = vel.y.abs() < 0.1 && pos.y < 1.0;

                scene.state = PhysicsState {
                    falling_position: pos,
                    falling_velocity: vel,
                    is_on_ground,
                };
            }
            if let Some(scene) = &mut self.right_scene {
                scene.world.step(self.physics_dt);

                // Update physics state
                let pos = scene.falling_object.position(&scene.world);
                let vel = scene.falling_object.velocity(&scene.world);
                let is_on_ground = vel.y.abs() < 0.1 && pos.y < 1.0;

                scene.state = PhysicsState {
                    falling_position: pos,
                    falling_velocity: vel,
                    is_on_ground,
                };
            }
        }
    }

    /// Print debug information about physics state
    fn print_debug_info(&self) {
        if self.debug_frames.is_none() {
            return;
        }

        let left_pos = self
            .left_scene
            .as_ref()
            .map(|s| s.falling_object.position(&s.world))
            .unwrap_or(Vec3::ZERO);

        let right_pos = self
            .right_scene
            .as_ref()
            .map(|s| s.falling_object.position(&s.world))
            .unwrap_or(Vec3::ZERO);

        let left_vel = self
            .left_scene
            .as_ref()
            .map(|s| s.falling_object.velocity(&s.world))
            .unwrap_or(Vec3::ZERO);

        let right_vel = self
            .right_scene
            .as_ref()
            .map(|s| s.falling_object.velocity(&s.world))
            .unwrap_or(Vec3::ZERO);

        let pos_diff = left_pos - right_pos;
        let vel_diff = left_vel - right_vel;

        println!(
            "Frame {}: Left(Cuboid) pos={:.4},{:.4},{:.4} vel={:.4},{:.4},{:.4} | Right(Terrain) pos={:.4},{:.4},{:.4} vel={:.4},{:.4},{:.4}",
            self.frame_count,
            left_pos.x, left_pos.y, left_pos.z,
            left_vel.x, left_vel.y, left_vel.z,
            right_pos.x, right_pos.y, right_pos.z,
            right_vel.x, right_vel.y, right_vel.z,
        );

        if pos_diff.length() > 0.001 || vel_diff.length() > 0.001 {
            println!(
                "  DIFF: pos_delta={:.6},{:.6},{:.6} vel_delta={:.6},{:.6},{:.6}",
                pos_diff.x, pos_diff.y, pos_diff.z, vel_diff.x, vel_diff.y, vel_diff.z,
            );
        }
    }

    /// Render a scene to the given viewport
    fn render_scene(
        gl: &Context,
        scene: &PhysicsScene,
        mesh_renderer: &MeshRenderer,
        camera: &Camera,
        camera_target: Vec3,
        viewport_x: i32,
        viewport_y: i32,
        viewport_width: i32,
        viewport_height: i32,
        x_offset: f32,
    ) {
        unsafe {
            // Set viewport and scissor
            gl.viewport(viewport_x, viewport_y, viewport_width, viewport_height);
            gl.scissor(viewport_x, viewport_y, viewport_width, viewport_height);
            gl.enable(SCISSOR_TEST);

            // Clear this viewport with dark background
            gl.clear_color(0.12, 0.12, 0.18, 1.0);
            gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);
        }

        // Create camera for this scene with offset position
        let offset_position = camera.position + Vec3::new(x_offset, 0.0, 0.0);
        let offset_target = camera_target + Vec3::new(x_offset, 0.0, 0.0);
        let scene_camera = Camera::look_at(offset_position, offset_target, Vec3::Y);

        // Render ground cube
        if let Some(ground_idx) = scene.ground_mesh_index {
            let ground_pos = Vec3::new(x_offset, -0.25, 0.0);
            unsafe {
                mesh_renderer.render_mesh_with_options(
                    gl,
                    ground_idx,
                    ground_pos,
                    Quat::IDENTITY,
                    0.5,
                    Vec3::ONE,
                    false,
                    &scene_camera,
                    viewport_width,
                    viewport_height,
                );
            }
        }

        // Render falling cube
        if let Some(falling_idx) = scene.falling_mesh_index {
            let falling_pos =
                scene.falling_object.position(&scene.world) + Vec3::new(x_offset, 0.0, 0.0);
            let falling_rot = scene.falling_object.rotation(&scene.world);
            unsafe {
                mesh_renderer.render_mesh_with_scale(
                    gl,
                    falling_idx,
                    falling_pos,
                    falling_rot,
                    0.8,
                    &scene_camera,
                    viewport_width,
                    viewport_height,
                );
            }
        }

        unsafe {
            gl.disable(SCISSOR_TEST);
        }
    }

    fn check_debug_exit(&mut self) -> bool {
        if let Some(max_frames) = self.debug_frames {
            if self.frame_count >= max_frames {
                println!("\n[Testbed] Reached {} frames, exiting", max_frames);

                // Print final comparison
                if let (Some(left), Some(right)) = (&self.left_scene, &self.right_scene) {
                    let left_pos = left.falling_object.position(&left.world);
                    let right_pos = right.falling_object.position(&right.world);
                    let diff = left_pos - right_pos;

                    println!("\n=== Final Comparison ===");
                    println!("Left (Cuboid):   Y = {:.6}", left_pos.y);
                    println!("Right (Terrain): Y = {:.6}", right_pos.y);
                    println!("Difference:      Y = {:.6}", diff.y);

                    if diff.length() > 0.01 {
                        println!("\nWARNING: Significant physics difference detected!");
                    } else {
                        println!("\nPhysics behavior is consistent between both collider types.");
                    }
                }

                return true;
            }
        }
        false
    }

    /// Handle orbit camera input from egui response
    fn handle_orbit_input(&mut self, input: &InputState) {
        // Handle camera orbit with left mouse drag
        if input.mouse_buttons.left && input.mouse_delta.length() > 0.0 {
            let delta = input.mouse_delta * self.orbit_controller.config.mouse_sensitivity;
            self.orbit_controller.rotate(delta.x, delta.y, &mut self.camera);
        }

        // Handle zoom with scroll
        if input.scroll_delta.y.abs() > 0.0 {
            self.orbit_controller.zoom(input.scroll_delta.y * self.orbit_controller.config.zoom_sensitivity, &mut self.camera);
        }
    }
}

impl App for PhysicsTestbed {
    fn init(&mut self, ctx: &FrameContext) {
        println!("[Testbed] Initializing physics testbed");

        // Initialize mesh renderer
        if let Err(e) = unsafe { self.mesh_renderer.init_gl(ctx.gl) } {
            eprintln!("[Testbed] Failed to initialize mesh renderer: {}", e);
        }

        // Create ground cube for terrain collider
        let ground_cube = Rc::new(Cube::Solid(156u8));

        // Create physics scenes
        self.left_scene = Some(self.create_cuboid_ground_scene());
        self.right_scene = Some(self.create_terrain_ground_scene(&ground_cube));

        // Upload meshes
        unsafe { self.upload_meshes(ctx.gl) };

        self.last_physics_update = Instant::now();

        println!("[Testbed] Physics scenes initialized");
        println!("  Left:  Simple cuboid ground collider");
        println!("  Right: Terrain collider from Cube objects");
        if let Some(frames) = self.debug_frames {
            println!("  Debug mode: running {} frames", frames);
        }
    }

    fn shutdown(&mut self, ctx: &FrameContext) {
        println!("[Testbed] Cleaning up");
        unsafe { self.mesh_renderer.destroy_gl(ctx.gl) };
    }

    fn update(&mut self, ctx: &FrameContext, input: &InputState) {
        // Check debug frame limit
        if self.check_debug_exit() {
            return;
        }

        // Handle orbit camera
        self.handle_orbit_input(input);

        // Step physics
        self.step_physics();

        // Print debug info
        self.print_debug_info();

        // Handle reset after the frame
        if self.reset_requested {
            self.reset_requested = false;
            unsafe { self.reset_scenes(ctx.gl) };
        }

        self.frame_count += 1;
    }

    fn render(&mut self, ctx: &FrameContext) {
        let width = ctx.size.0 as i32;
        let height = ctx.size.1 as i32;

        // Reserve space for top bar
        let top_bar_height = 40;
        let label_height = 25;
        let render_height = height - top_bar_height - label_height;
        let half_width = width / 2;

        // Clear the entire window first
        unsafe {
            ctx.gl.viewport(0, 0, width, height);
            ctx.gl.clear_color(0.1, 0.1, 0.1, 1.0);
            ctx.gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);
        }

        // Get camera target for consistent rendering
        let camera_target = self.orbit_controller.target;

        // Render left scene (cuboid ground)
        if let Some(scene) = &self.left_scene {
            Self::render_scene(
                ctx.gl,
                scene,
                &self.mesh_renderer,
                &self.camera,
                camera_target,
                0,
                0,
                half_width,
                render_height,
                -6.0,
            );
        }

        // Render right scene (terrain ground)
        if let Some(scene) = &self.right_scene {
            Self::render_scene(
                ctx.gl,
                scene,
                &self.mesh_renderer,
                &self.camera,
                camera_target,
                half_width,
                0,
                half_width,
                render_height,
                6.0,
            );
        }

        // Draw divider line between viewports
        unsafe {
            ctx.gl.viewport(0, 0, width, height);
            ctx.gl.scissor(half_width - 1, 0, 2, render_height);
            ctx.gl.enable(SCISSOR_TEST);
            ctx.gl.clear_color(0.4, 0.4, 0.4, 1.0);
            ctx.gl.clear(COLOR_BUFFER_BIT);
            ctx.gl.disable(SCISSOR_TEST);
        }
    }

    fn ui(&mut self, ctx: &FrameContext, egui_ctx: &egui::Context) {
        let left_state_text = self
            .left_scene
            .as_ref()
            .map(|s| format!("Cuboid: {}", s.state.format_compact()))
            .unwrap_or_else(|| "Cuboid: N/A".to_string());
        let right_state_text = self
            .right_scene
            .as_ref()
            .map(|s| format!("Terrain: {}", s.state.format_compact()))
            .unwrap_or_else(|| "Terrain: N/A".to_string());
        let frame_count = self.frame_count;
        let label_height_f = 25.0;

        // Top bar with dropdown
        egui::TopBottomPanel::top("top_panel").show(egui_ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Physics Testbed");
                ui.separator();
                ui.label("Setup: Single Falling Cube");
                ui.separator();

                if ui.button("Reset").clicked() {
                    self.reset_requested = true;
                }

                ui.separator();
                ui.label(format!("Frame: {}", frame_count));
            });
        });

        // Physics state labels above viewports
        egui::TopBottomPanel::bottom("state_panel")
            .frame(egui::Frame::none().fill(egui::Color32::from_gray(30)))
            .show(egui_ctx, |ui| {
                ui.set_height(label_height_f);
                ui.horizontal(|ui| {
                    let available_width = ui.available_width();
                    let half = available_width / 2.0;

                    // Left scene state
                    ui.allocate_ui_with_layout(
                        egui::vec2(half - 5.0, label_height_f),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.label(
                                egui::RichText::new(&left_state_text)
                                    .monospace()
                                    .color(egui::Color32::LIGHT_GREEN),
                            );
                        },
                    );

                    ui.separator();

                    // Right scene state
                    ui.allocate_ui_with_layout(
                        egui::vec2(half - 5.0, label_height_f),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.label(
                                egui::RichText::new(&right_state_text)
                                    .monospace()
                                    .color(egui::Color32::LIGHT_BLUE),
                            );
                        },
                    );
                });
            });

        // Central panel for camera control hint
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(egui_ctx, |ui| {
                let rect = ui.available_rect_before_wrap();

                // Show hint at bottom
                let hint_rect = egui::Rect::from_min_size(
                    egui::pos2(rect.min.x + 5.0, rect.max.y - 20.0),
                    egui::vec2(300.0, 20.0),
                );
                ui.painter().text(
                    hint_rect.min,
                    egui::Align2::LEFT_TOP,
                    "(Drag: orbit, Scroll: zoom)",
                    egui::FontId::proportional(12.0),
                    egui::Color32::from_gray(120),
                );
            });
    }
}

/// Export the create_app function for dynamic loading (optional)
///
/// Note: This uses `dyn App` which isn't strictly FFI-safe, but this is only used
/// for hot-reload between Rust code, not for interop with other languages.
#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn create_app() -> *mut dyn App {
    Box::into_raw(Box::new(PhysicsTestbed::new()))
}
