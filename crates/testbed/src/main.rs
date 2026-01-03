//! Physics Component Testbed
//!
//! Compares physics behavior between:
//! - Left: Simple flat ground using cuboid collider
//! - Right: Flat ground constructed from Cube objects using terrain collider
//!
//! Both sides have a falling CubeObject to demonstrate any physics differences.

use crossworld_physics::{create_box_collider, CubeObject, PhysicsWorld, VoxelColliderBuilder};
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
use renderer::{Camera, MeshRenderer};
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

/// Physics testbed state for one side (left = cuboid, right = terrain)
struct PhysicsScene {
    world: PhysicsWorld,
    falling_object: CubeObject,
    ground_mesh_index: Option<usize>,
    falling_mesh_index: Option<usize>,
    #[allow(dead_code)]
    ground_collider_type: &'static str,
}

/// Main application state
struct App {
    window: Option<Window>,
    gl_context: Option<glutin::context::PossiblyCurrentContext>,
    gl_surface: Option<glutin::surface::Surface<WindowSurface>>,
    gl: Option<Arc<Context>>,

    // Rendering
    mesh_renderer: Option<MeshRenderer>,
    camera: Camera,

    // Physics scenes
    left_scene: Option<PhysicsScene>,  // Cuboid ground
    right_scene: Option<PhysicsScene>, // Terrain collider ground

    // Cubes for rendering
    ground_cube: Option<Rc<Cube<u8>>>,
    falling_cube: Option<Rc<Cube<u8>>>,

    // Timing and debug
    start_time: Instant,
    frame_count: u64,
    debug_frames: Option<u64>,
    physics_dt: f32,
    last_physics_update: Instant,
}

impl Default for App {
    fn default() -> Self {
        Self {
            window: None,
            gl_context: None,
            gl_surface: None,
            gl: None,
            mesh_renderer: None,
            camera: Camera::look_at(
                Vec3::new(5.0, 8.0, 10.0),
                Vec3::new(0.0, 2.0, 0.0),
                Vec3::Y,
            ),
            left_scene: None,
            right_scene: None,
            ground_cube: None,
            falling_cube: None,
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

        // Create ground plane as a flat cuboid
        // Ground is at Y=0, thin (0.1 thick), spanning 10x10 units
        let ground_collider = create_box_collider(Vec3::new(5.0, 0.1, 5.0));
        world.add_static_collider(ground_collider);

        // Create falling cube at Y=5
        let mut falling_object = CubeObject::new_dynamic(&mut world, Vec3::new(0.0, 5.0, 0.0), 1.0);
        let falling_collider = create_box_collider(Vec3::new(0.5, 0.5, 0.5));
        falling_object.attach_collider(&mut world, falling_collider);

        PhysicsScene {
            world,
            falling_object,
            ground_mesh_index: None,
            falling_mesh_index: None,
            ground_collider_type: "Cuboid",
        }
    }

    /// Create physics scene with terrain collider from Cube objects
    fn create_terrain_ground_scene(&self, ground_cube: &Rc<Cube<u8>>) -> PhysicsScene {
        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

        // Create ground using VoxelColliderBuilder from a solid cube
        // The terrain collider generates triangle meshes from voxel faces
        // Scale: 10 units world size, centered at origin
        let terrain_collider = VoxelColliderBuilder::from_cube_scaled(ground_cube, 0, 10.0);
        world.add_static_collider(terrain_collider);

        // Create falling cube at Y=5
        let mut falling_object = CubeObject::new_dynamic(&mut world, Vec3::new(0.0, 5.0, 0.0), 1.0);
        let falling_collider = create_box_collider(Vec3::new(0.5, 0.5, 0.5));
        falling_object.attach_collider(&mut world, falling_collider);

        PhysicsScene {
            world,
            falling_object,
            ground_mesh_index: None,
            falling_mesh_index: None,
            ground_collider_type: "Terrain (Cube)",
        }
    }

    /// Upload meshes for rendering
    unsafe fn upload_meshes(&mut self, gl: &Context) {
        let Some(mesh_renderer) = &mut self.mesh_renderer else {
            return;
        };

        // Create ground cube - a flat slab
        let ground_cube = Rc::new(Cube::Solid(156u8)); // Green color
        self.ground_cube = Some(ground_cube.clone());

        // Create falling cube
        let falling_cube = Rc::new(Cube::Solid(224u8)); // Red color
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
            }
            if let Some(scene) = &mut self.right_scene {
                scene.world.step(self.physics_dt);
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
    ///
    /// # Safety
    /// Must be called with a valid GL context on the current thread.
    unsafe fn render_scene(
        &self,
        gl: &Context,
        scene: &PhysicsScene,
        mesh_renderer: &MeshRenderer,
        viewport_x: i32,
        viewport_y: i32,
        viewport_width: i32,
        viewport_height: i32,
        x_offset: f32,
    ) {
        // SAFETY: Caller guarantees valid GL context
        unsafe {
            gl.viewport(viewport_x, viewport_y, viewport_width, viewport_height);
            gl.scissor(viewport_x, viewport_y, viewport_width, viewport_height);
            gl.enable(SCISSOR_TEST);

            // Clear this viewport
            let bg_r = 0.15_f32;
            let bg_g = 0.15_f32;
            let bg_b = 0.2_f32;
            gl.clear_color(bg_r, bg_g, bg_b, 1.0);
            gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);
        }

        // Create camera for this scene with offset position
        let offset_position = self.camera.position + Vec3::new(x_offset, 0.0, 0.0);
        let camera = Camera::look_at(offset_position, Vec3::new(x_offset, 2.0, 0.0), Vec3::Y);

        // Render ground (flat slab at Y=0, scale to 10x0.2x10)
        if let Some(ground_idx) = scene.ground_mesh_index {
            // Ground position: centered at origin, but we need to offset Y
            // The mesh is a unit cube [0,1], centered at 0.5
            // After scaling by (10, 0.2, 10) and offsetting, ground top should be at Y=0
            let ground_pos = Vec3::new(x_offset, -0.1, 0.0);
            // SAFETY: Caller guarantees valid GL context
            unsafe {
                mesh_renderer.render_mesh_with_options(
                    gl,
                    ground_idx,
                    ground_pos,
                    Quat::IDENTITY,
                    10.0, // scale
                    Vec3::new(1.0, 0.02, 1.0), // normalized size (flat slab)
                    false,
                    &camera,
                    viewport_width,
                    viewport_height,
                );
            }
        }

        // Render falling cube
        if let Some(falling_idx) = scene.falling_mesh_index {
            let falling_pos = scene.falling_object.position(&scene.world) + Vec3::new(x_offset, 0.0, 0.0);
            let falling_rot = scene.falling_object.rotation(&scene.world);
            // SAFETY: Caller guarantees valid GL context
            unsafe {
                mesh_renderer.render_mesh_with_scale(
                    gl,
                    falling_idx,
                    falling_pos,
                    falling_rot,
                    1.0,
                    &camera,
                    viewport_width,
                    viewport_height,
                );
            }
        }

        // SAFETY: Caller guarantees valid GL context
        unsafe {
            gl.disable(SCISSOR_TEST);
        }
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
            .with_inner_size(winit::dpi::LogicalSize::new(1200, 600));

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
                                println!("The terrain collider is producing different results than the cuboid collider.");
                            } else {
                                println!("\nPhysics behavior is consistent between both collider types.");
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
                ) = (
                    self.window.as_ref(),
                    self.gl.as_ref(),
                    self.mesh_renderer.as_ref(),
                    self.gl_context.as_ref(),
                    self.gl_surface.as_ref(),
                ) {
                    let size = window.inner_size();
                    let half_width = size.width as i32 / 2;
                    let height = size.height as i32;

                    unsafe {
                        gl.viewport(0, 0, size.width as i32, height);
                        gl.clear_color(0.1, 0.1, 0.1, 1.0);
                        gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);

                        // Render left scene (cuboid ground)
                        if let Some(scene) = &self.left_scene {
                            self.render_scene(
                                gl,
                                scene,
                                mesh_renderer,
                                0,
                                0,
                                half_width,
                                height,
                                -6.0, // X offset for left scene camera target
                            );
                        }

                        // Render right scene (terrain ground)
                        if let Some(scene) = &self.right_scene {
                            self.render_scene(
                                gl,
                                scene,
                                mesh_renderer,
                                half_width,
                                0,
                                half_width,
                                height,
                                6.0, // X offset for right scene camera target
                            );
                        }

                        // Draw divider line
                        gl.viewport(0, 0, size.width as i32, height);
                        gl.scissor(half_width - 1, 0, 2, height);
                        gl.enable(SCISSOR_TEST);
                        gl.clear_color(0.5, 0.5, 0.5, 1.0);
                        gl.clear(COLOR_BUFFER_BIT);
                        gl.disable(SCISSOR_TEST);
                    }

                    gl_surface.swap_buffers(gl_context).unwrap();
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
