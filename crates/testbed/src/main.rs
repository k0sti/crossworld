//! Physics Component Testbed
//!
//! Compares physics behavior between:
//! - Left: Simple flat ground using cuboid collider
//! - Right: Flat ground constructed from Cube objects using terrain collider
//!
//! Features:
//! - Dropdown to select different physics setups
//! - Compact physics state display above each viewport
//! - Single large ground cube with smaller falling cube

use crossworld_physics::{
    create_box_collider,
    rapier3d::prelude::{ColliderBuilder, RigidBodyBuilder},
    CubeObject, PhysicsWorld, VoxelColliderBuilder,
};
use cube::Cube;
use glam::{Quat, Vec3};
use glow::*;
use glutin::config::ConfigTemplateBuilder;
use glutin::context::{ContextApi, ContextAttributesBuilder, Version};
use glutin::display::GetGlDisplay;
use glutin::prelude::*;
use glutin::surface::{SurfaceAttributesBuilder, WindowSurface};
use glutin_winit::DisplayBuilder;
use raw_window_handle::HasWindowHandle;
use renderer::{Camera, MeshRenderer, OrbitController, OrbitControllerConfig};
use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

#[cfg(target_os = "linux")]
use winit::platform::x11::EventLoopBuilderExtX11;

/// Available physics setup configurations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PhysicsSetup {
    SingleCube,
}

impl PhysicsSetup {
    fn name(&self) -> &'static str {
        match self {
            PhysicsSetup::SingleCube => "Single Falling Cube",
        }
    }

    fn all() -> &'static [PhysicsSetup] {
        &[PhysicsSetup::SingleCube]
    }
}

/// Physics state for display
#[derive(Default, Clone)]
struct PhysicsState {
    falling_position: Vec3,
    falling_velocity: Vec3,
    is_on_ground: bool,
}

impl PhysicsState {
    fn format_compact(&self) -> String {
        let ground_status = if self.is_on_ground { "HIT" } else { "AIR" };
        format!(
            "Y: {:.2} | Vel: {:.2} | {}",
            self.falling_position.y, self.falling_velocity.y, ground_status
        )
    }
}

/// Physics testbed state for one side (left = cuboid, right = terrain)
struct PhysicsScene {
    world: PhysicsWorld,
    falling_object: CubeObject,
    ground_mesh_index: Option<usize>,
    falling_mesh_index: Option<usize>,
    #[allow(dead_code)]
    ground_collider_type: &'static str,
    state: PhysicsState,
}

/// Main application state
struct App {
    window: Option<Window>,
    gl_context: Option<glutin::context::PossiblyCurrentContext>,
    gl_surface: Option<glutin::surface::Surface<WindowSurface>>,
    gl: Option<Arc<Context>>,

    // Egui integration
    egui_ctx: Option<egui::Context>,
    egui_state: Option<egui_winit::State>,
    painter: Option<egui_glow::Painter>,

    // Rendering
    mesh_renderer: Option<MeshRenderer>,
    camera: Camera,
    orbit_controller: OrbitController,

    // Physics scenes
    left_scene: Option<PhysicsScene>,  // Cuboid ground
    right_scene: Option<PhysicsScene>, // Terrain collider ground

    // Cubes for rendering
    ground_cube: Option<Rc<Cube<u8>>>,
    falling_cube: Option<Rc<Cube<u8>>>,

    // UI state
    current_setup: PhysicsSetup,
    reset_requested: bool,

    // Timing and debug
    start_time: Instant,
    frame_count: u64,
    debug_frames: Option<u64>,
    physics_dt: f32,
    last_physics_update: Instant,
}

impl Default for App {
    fn default() -> Self {
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
            window: None,
            gl_context: None,
            gl_surface: None,
            gl: None,
            egui_ctx: None,
            egui_state: None,
            painter: None,
            mesh_renderer: None,
            // Camera positioned to see ground top and falling cube
            camera: Camera::look_at(camera_position, camera_target, Vec3::Y),
            orbit_controller: OrbitController::new(camera_target, orbit_config),
            left_scene: None,
            right_scene: None,
            ground_cube: None,
            falling_cube: None,
            current_setup: PhysicsSetup::SingleCube,
            reset_requested: false,
            start_time: Instant::now(),
            frame_count: 0,
            debug_frames: None,
            physics_dt: 1.0 / 60.0,
            last_physics_update: Instant::now(),
        }
    }
}

impl App {
    fn with_debug_frames(mut self, frames: Option<u64>) -> Self {
        self.debug_frames = frames;
        self
    }

    /// Create physics scene with simple cuboid ground collider
    fn create_cuboid_ground_scene(&self) -> PhysicsScene {
        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

        // Create ground as a cube matching the terrain collider
        // 0.5x0.5x0.5 cube (half-extents 0.25), positioned so top is at Y=0
        // Center at Y=-0.25 so top is at Y=0
        let ground_collider = ColliderBuilder::cuboid(0.25, 0.25, 0.25)
            .translation([0.0, -0.25, 0.0].into())
            .build();
        world.add_static_collider(ground_collider);

        // Create small falling cube at Y=8 (smaller than ground)
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
        // Use a small world_size (0.5) to create a thin slab
        // The 0.5x0.5x0.5 cube spans [-0.25, +0.25], so top is at Y=+0.25
        // Position the fixed body at Y=-0.25 to put top at Y=0
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
    fn reset_scenes(&mut self) {
        if let Some(ground_cube) = self.ground_cube.clone() {
            self.left_scene = Some(self.create_cuboid_ground_scene());
            self.right_scene = Some(self.create_terrain_ground_scene(&ground_cube));

            // Re-upload meshes
            if let Some(gl) = self.gl.clone() {
                unsafe {
                    self.upload_meshes(&gl);
                }
            }
        }
    }

    /// Upload meshes for rendering
    unsafe fn upload_meshes(&mut self, gl: &Context) {
        let Some(mesh_renderer) = &mut self.mesh_renderer else {
            return;
        };

        // Create ground cube - a solid cube (green-ish color)
        let ground_cube = Rc::new(Cube::Solid(156u8));
        self.ground_cube = Some(ground_cube.clone());

        // Create falling cube (red color, smaller)
        let falling_cube = Rc::new(Cube::Solid(224u8));
        self.falling_cube = Some(falling_cube.clone());

        // Upload ground mesh (for both scenes)
        // SAFETY: GL context is valid and we're in the render thread
        match unsafe { mesh_renderer.upload_mesh(gl, &ground_cube, 1) } {
            Ok(idx) => {
                if let Some(scene) = &mut self.left_scene {
                    scene.ground_mesh_index = Some(idx);
                }
            }
            Err(e) => eprintln!("Failed to upload ground mesh: {}", e),
        }

        // Upload second ground mesh for right scene
        // SAFETY: GL context is valid and we're in the render thread
        match unsafe { mesh_renderer.upload_mesh(gl, &ground_cube, 1) } {
            Ok(idx) => {
                if let Some(scene) = &mut self.right_scene {
                    scene.ground_mesh_index = Some(idx);
                }
            }
            Err(e) => eprintln!("Failed to upload ground mesh: {}", e),
        }

        // Upload falling cube mesh (for left scene)
        // SAFETY: GL context is valid and we're in the render thread
        match unsafe { mesh_renderer.upload_mesh(gl, &falling_cube, 1) } {
            Ok(idx) => {
                if let Some(scene) = &mut self.left_scene {
                    scene.falling_mesh_index = Some(idx);
                }
            }
            Err(e) => eprintln!("Failed to upload falling mesh: {}", e),
        }

        // Upload falling cube mesh (for right scene)
        // SAFETY: GL context is valid and we're in the render thread
        match unsafe { mesh_renderer.upload_mesh(gl, &falling_cube, 1) } {
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
                // Consider on ground if Y velocity is near zero and position is low
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

}

/// Render a scene to the given viewport
///
/// # Safety
/// Must be called with a valid GL context on the current thread.
unsafe fn render_scene(
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

        // Create camera for this scene with offset position, maintaining same orientation
        let offset_position = camera.position + Vec3::new(x_offset, 0.0, 0.0);
        let offset_target = camera_target + Vec3::new(x_offset, 0.0, 0.0);
        let scene_camera = Camera::look_at(offset_position, offset_target, Vec3::Y);

        // Render ground cube (0.5x0.5x0.5 cube with top at Y=0)
        // The mesh is a unit cube [0,1]³, scaled to 0.5 units.
        // After centering, spans [-0.25, 0.25]. Positioned at Y=-0.25 so top is at Y=0.
        if let Some(ground_idx) = scene.ground_mesh_index {
            let ground_pos = Vec3::new(x_offset, -0.25, 0.0);
            mesh_renderer.render_mesh_with_options(
                gl,
                ground_idx,
                ground_pos,
                Quat::IDENTITY,
                0.5,       // 0.5 unit cube
                Vec3::ONE, // full cube, centered
                false,
                &scene_camera,
                viewport_width,
                viewport_height,
            );
        }

        // Render falling cube (small cube)
        if let Some(falling_idx) = scene.falling_mesh_index {
            let falling_pos =
                scene.falling_object.position(&scene.world) + Vec3::new(x_offset, 0.0, 0.0);
            let falling_rot = scene.falling_object.rotation(&scene.world);
            mesh_renderer.render_mesh_with_scale(
                gl,
                falling_idx,
                falling_pos,
                falling_rot,
                0.8, // Small cube (0.8 units)
                &scene_camera,
                viewport_width,
                viewport_height,
            );
        }

        gl.disable(SCISSOR_TEST);
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        println!("[Testbed] Initializing window and GL context");

        let window_attributes = Window::default_attributes()
            .with_title("Physics Testbed - Left: Cuboid | Right: Terrain Collider")
            .with_inner_size(winit::dpi::LogicalSize::new(1200, 700));

        let template = ConfigTemplateBuilder::new()
            .with_alpha_size(8)
            .with_transparency(false);

        let display_builder = DisplayBuilder::new().with_window_attributes(Some(window_attributes));

        let (window, gl_config) = display_builder
            .build(event_loop, template, |configs| {
                configs
                    .reduce(|accum, config| {
                        if config.num_samples() > accum.num_samples() {
                            config
                        } else {
                            accum
                        }
                    })
                    .unwrap()
            })
            .unwrap();

        let window = window.unwrap();
        let window_handle = window.window_handle().ok().map(|h| h.as_raw());
        let gl_display = gl_config.display();

        let context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(Some(Version::new(4, 3))))
            .build(window_handle);

        let gl_context = unsafe {
            gl_display
                .create_context(&gl_config, &context_attributes)
                .unwrap()
        };

        let size = window.inner_size();
        let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            window_handle.unwrap(),
            NonZeroU32::new(size.width).unwrap(),
            NonZeroU32::new(size.height).unwrap(),
        );

        let gl_surface = unsafe {
            gl_display
                .create_window_surface(&gl_config, &attrs)
                .unwrap()
        };

        let gl_context = gl_context.make_current(&gl_surface).unwrap();

        let gl = Arc::new(unsafe {
            Context::from_loader_function_cstr(|s| gl_display.get_proc_address(s))
        });

        println!("[Testbed] OpenGL context created successfully");

        // Initialize egui
        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            None,
            None,
            None,
        );
        let painter = egui_glow::Painter::new(gl.clone(), "", None, false).unwrap();

        // Initialize mesh renderer
        let mut mesh_renderer = MeshRenderer::new();
        unsafe {
            if let Err(e) = mesh_renderer.init_gl(&gl) {
                eprintln!("[Testbed] Failed to initialize mesh renderer: {}", e);
            }
        }

        self.window = Some(window);
        self.gl_context = Some(gl_context);
        self.gl_surface = Some(gl_surface);
        self.gl = Some(gl.clone());
        self.egui_ctx = Some(egui_ctx);
        self.egui_state = Some(egui_state);
        self.painter = Some(painter);
        self.mesh_renderer = Some(mesh_renderer);

        // Create ground cube for terrain collider
        // A solid cube that will be scaled to form the ground
        let ground_cube = Rc::new(Cube::Solid(156u8));

        // Create physics scenes
        self.left_scene = Some(self.create_cuboid_ground_scene());
        self.right_scene = Some(self.create_terrain_ground_scene(&ground_cube));

        // Upload meshes
        unsafe {
            self.upload_meshes(&gl);
        }

        self.start_time = Instant::now();
        self.last_physics_update = Instant::now();

        println!("[Testbed] Physics scenes initialized");
        println!("  Left:  Simple cuboid ground collider");
        println!("  Right: Terrain collider from Cube objects");
        if let Some(frames) = self.debug_frames {
            println!("  Debug mode: running {} frames", frames);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // Let egui handle events first
        if let Some(egui_state) = &mut self.egui_state {
            if let Some(window) = &self.window {
                let _ = egui_state.on_window_event(window, &event);
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                println!("[Testbed] Close requested");
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let (Some(gl_surface), Some(gl_context)) =
                    (self.gl_surface.as_ref(), self.gl_context.as_ref())
                {
                    gl_surface.resize(
                        gl_context,
                        NonZeroU32::new(size.width).unwrap(),
                        NonZeroU32::new(size.height).unwrap(),
                    );
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                // Check debug frame limit
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
                                println!(
                                    "The terrain collider is producing different results than the cuboid collider."
                                );
                            } else {
                                println!(
                                    "\nPhysics behavior is consistent between both collider types."
                                );
                            }
                        }

                        event_loop.exit();
                        return;
                    }
                }

                // Step physics
                self.step_physics();

                // Print debug info
                self.print_debug_info();

                // Render
                if let (
                    Some(window),
                    Some(gl),
                    Some(mesh_renderer),
                    Some(gl_context),
                    Some(gl_surface),
                    Some(egui_ctx),
                    Some(egui_state),
                    Some(painter),
                ) = (
                    self.window.as_ref(),
                    self.gl.as_ref(),
                    self.mesh_renderer.as_ref(),
                    self.gl_context.as_ref(),
                    self.gl_surface.as_ref(),
                    self.egui_ctx.as_ref(),
                    self.egui_state.as_mut(),
                    self.painter.as_mut(),
                ) {
                    let size = window.inner_size();
                    let width = size.width as i32;
                    let height = size.height as i32;

                    // Reserve space for top bar
                    let top_bar_height = 40;
                    let label_height = 25;
                    let render_height = height - top_bar_height - label_height;
                    let half_width = width / 2;

                    // Clear the entire window first
                    unsafe {
                        gl.viewport(0, 0, width, height);
                        gl.clear_color(0.1, 0.1, 0.1, 1.0);
                        gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);
                    }

                    // Get camera target for consistent rendering
                    let camera_target = self.orbit_controller.target;

                    // Render left scene (cuboid ground)
                    if let Some(scene) = &self.left_scene {
                        unsafe {
                            render_scene(
                                gl,
                                scene,
                                mesh_renderer,
                                &self.camera,
                                camera_target,
                                0,
                                0,
                                half_width,
                                render_height,
                                -6.0, // X offset for left scene camera target
                            );
                        }
                    }

                    // Render right scene (terrain ground)
                    if let Some(scene) = &self.right_scene {
                        unsafe {
                            render_scene(
                                gl,
                                scene,
                                mesh_renderer,
                                &self.camera,
                                camera_target,
                                half_width,
                                0,
                                half_width,
                                render_height,
                                6.0, // X offset for right scene camera target
                            );
                        }
                    }

                    // Draw divider line between viewports
                    unsafe {
                        gl.viewport(0, 0, width, height);
                        gl.scissor(half_width - 1, 0, 2, render_height);
                        gl.enable(SCISSOR_TEST);
                        gl.clear_color(0.4, 0.4, 0.4, 1.0);
                        gl.clear(COLOR_BUFFER_BIT);
                        gl.disable(SCISSOR_TEST);
                    }

                    // Prepare egui state for rendering
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
                    let current_setup = self.current_setup;
                    let frame_count = self.frame_count;

                    // Run egui
                    let raw_input = egui_state.take_egui_input(window);
                    let full_output = egui_ctx.run(raw_input, |ctx| {
                        // Top bar with dropdown
                        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
                            ui.horizontal(|ui| {
                                ui.heading("Physics Testbed");
                                ui.separator();

                                ui.label("Setup:");
                                egui::ComboBox::from_id_salt("physics_setup")
                                    .selected_text(current_setup.name())
                                    .show_ui(ui, |ui| {
                                        for setup in PhysicsSetup::all() {
                                            if ui
                                                .selectable_label(current_setup == *setup, setup.name())
                                                .clicked()
                                            {
                                                // Note: we can't modify self.current_setup inside the closure
                                            }
                                        }
                                    });

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
                            .show(ctx, |ui| {
                                ui.set_height(label_height as f32);
                                ui.horizontal(|ui| {
                                    let available_width = ui.available_width();
                                    let half = available_width / 2.0;

                                    // Left scene state
                                    ui.allocate_ui_with_layout(
                                        egui::vec2(half - 5.0, label_height as f32),
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
                                        egui::vec2(half - 5.0, label_height as f32),
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

                        // Central panel for camera control (covers viewport area)
                        egui::CentralPanel::default()
                            .frame(egui::Frame::none())
                            .show(ctx, |ui| {
                                // Capture mouse input for orbit camera control
                                let rect = ui.available_rect_before_wrap();
                                let response = ui.allocate_rect(rect, egui::Sense::drag());

                                // Handle camera orbit and zoom
                                self.orbit_controller.handle_response(&response, &mut self.camera);

                                // Show hint at bottom of viewport area
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
                    });

                    egui_state.handle_platform_output(window, full_output.platform_output);

                    // Paint egui
                    let clipped_primitives =
                        egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
                    let size_in_pixels = [size.width, size.height];
                    painter.paint_and_update_textures(
                        size_in_pixels,
                        full_output.pixels_per_point,
                        &clipped_primitives,
                        &full_output.textures_delta,
                    );

                    gl_surface.swap_buffers(gl_context).unwrap();
                }

                // Handle reset after the borrow ends
                if self.reset_requested {
                    self.reset_requested = false;
                    self.reset_scenes();
                }

                self.frame_count += 1;

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if let (Some(gl), Some(mesh_renderer)) = (&self.gl, &mut self.mesh_renderer) {
            unsafe {
                mesh_renderer.destroy_gl(gl);
            }
        }
        if let Some(mut painter) = self.painter.take() {
            painter.destroy();
        }
    }
}

fn main() {
    println!("=== Physics Component Testbed ===");
    println!("Comparing cuboid vs terrain collider physics behavior\n");

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let mut debug_frames: Option<u64> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--debug" => {
                if i + 1 < args.len() {
                    match args[i + 1].parse::<u64>() {
                        Ok(n) => {
                            debug_frames = Some(n);
                            println!("Debug mode: running {} frames\n", n);
                        }
                        Err(_) => {
                            eprintln!("Error: --debug requires a number of frames");
                            return;
                        }
                    }
                    i += 1;
                } else {
                    eprintln!("Error: --debug requires a number of frames");
                    return;
                }
            }
            "--help" | "-h" => {
                println!("Usage: testbed [OPTIONS]");
                println!();
                println!("Options:");
                println!("  --debug N    Run only N frames with debug output");
                println!("  --help       Show this help message");
                return;
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                eprintln!("Use --help for usage information");
                return;
            }
        }
        i += 1;
    }

    #[cfg(target_os = "linux")]
    let event_loop = {
        let mut builder = EventLoop::builder();
        builder.with_x11();
        builder.build().expect("Failed to create event loop")
    };

    #[cfg(not(target_os = "linux"))]
    let event_loop = EventLoop::new().expect("Failed to create event loop");

    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default().with_debug_frames(debug_frames);

    event_loop.run_app(&mut app).expect("Event loop error");
}
