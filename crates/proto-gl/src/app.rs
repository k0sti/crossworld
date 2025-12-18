use glow::*;
use glutin::config::ConfigTemplateBuilder;
use glutin::context::{ContextApi, ContextAttributesBuilder, Version};
use glutin::display::GetGlDisplay;
use glutin::prelude::*;
use glutin::surface::{SurfaceAttributesBuilder, WindowSurface};
use glutin_winit::DisplayBuilder;
use raw_window_handle::HasWindowHandle;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowId};

use cube::Cube;
use crossworld_physics::{rapier3d::prelude::*, PhysicsWorld, VoxelColliderBuilder};
use renderer::{CameraConfig, GlTracer, MeshRenderer};

use crate::camera::{CameraMode, FirstPersonCamera, OrbitCamera};
use crate::config::{load_config, ProtoGlConfig};
use crate::models::{load_vox_models, CubeObject};
use crate::physics::{spawn_cube_objects, CameraObject};
use crate::ui::{render_debug_panel, UiState};
use crate::world::generate_world;

/// Keyboard input state for FPS controls
#[derive(Default, Clone, Copy)]
pub struct KeyboardState {
    pub w: bool,
    pub a: bool,
    pub s: bool,
    pub d: bool,
    pub f: bool,  // Up
    pub v: bool,  // Down
    pub space: bool,  // Jump (for future use)
}

pub struct ProtoGlApp {
    // Window and GL state
    window: Option<Window>,
    gl_context: Option<glutin::context::PossiblyCurrentContext>,
    gl_surface: Option<glutin::surface::Surface<WindowSurface>>,
    gl: Option<Arc<Context>>,

    // egui state
    egui_ctx: Option<egui::Context>,
    egui_state: Option<egui_winit::State>,
    painter: Option<egui_glow::Painter>,

    // Rendering state
    gl_tracer: Option<GlTracer>,
    mesh_renderer: Option<MeshRenderer>,
    object_mesh_indices: Vec<usize>,

    // Camera state
    camera_mode: CameraMode,
    orbit_camera: OrbitCamera,
    fps_camera: FirstPersonCamera,
    camera_object: Option<CameraObject>,

    // Input state
    keyboard_state: KeyboardState,
    /// Last mouse position for FPS camera delta calculation
    last_mouse_pos: Option<(f64, f64)>,

    // World state
    world_cube: Option<Cube<u8>>,
    world_depth: u32,

    // Physics state
    physics_world: Option<PhysicsWorld>,
    physics_accumulator: f32,
    objects: Vec<CubeObject>,

    // Timing
    last_frame: Instant,
    frame_time: f32,
    fps: f32,

    // Config
    config: ProtoGlConfig,

    // Debug toggles
    render_world: bool,
    render_objects: bool,
    show_debug_info: bool,

    // Debug mode (single frame)
    debug_mode: bool,
    frame_count: u32,

    // Frame capture mode
    capture_frame_path: Option<String>,
}

impl Default for ProtoGlApp {
    fn default() -> Self {
        Self::new(false, None)
    }
}

impl ProtoGlApp {
    pub fn new(debug_mode: bool, capture_frame_path: Option<String>) -> Self {
        // Load config from file or use defaults
        let config = load_config().unwrap_or_else(|e| {
            eprintln!("Warning: Failed to load config.toml: {}", e);
            eprintln!("Using default configuration");
            ProtoGlConfig::default()
        });

        let orbit_camera = OrbitCamera::new(config.rendering.camera_distance);

        // Initialize FPS camera with config
        let spawn_pos = glam::Vec3::from(config.fps.spawn_position);
        let mut fps_camera = FirstPersonCamera::new(spawn_pos);
        fps_camera.move_speed = config.fps.move_speed;
        fps_camera.sensitivity = config.fps.mouse_sensitivity;

        if debug_mode {
            println!("[DEBUG] Running in debug mode - will exit after single frame");
        }
        if let Some(ref path) = capture_frame_path {
            println!("[CAPTURE] Will save frame to: {}", path);
        }

        Self {
            window: None,
            gl_context: None,
            gl_surface: None,
            gl: None,
            egui_ctx: None,
            egui_state: None,
            painter: None,
            gl_tracer: None,
            mesh_renderer: None,
            object_mesh_indices: Vec::new(),
            camera_mode: CameraMode::Orbit,
            orbit_camera,
            fps_camera,
            camera_object: None,
            keyboard_state: KeyboardState::default(),
            last_mouse_pos: None,
            world_cube: None,
            world_depth: 0,
            physics_world: None,
            physics_accumulator: 0.0,
            objects: Vec::new(),
            last_frame: Instant::now(),
            frame_time: 0.0,
            fps: 60.0,
            config,
            render_world: true,
            render_objects: true,
            show_debug_info: true,
            debug_mode,
            frame_count: 0,
            capture_frame_path,
        }
    }
}

impl ApplicationHandler for ProtoGlApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attributes = Window::default_attributes()
            .with_title("Proto-GL Physics Viewer")
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.config.rendering.viewport_width,
                self.config.rendering.viewport_height,
            ));

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
            .with_context_api(ContextApi::Gles(Some(Version::new(3, 0))))
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

        // Generate world
        let (world_cube, world_depth) = generate_world(&self.config.world);

        // Initialize GL tracer with world cube
        let world_cube_rc = Rc::new(world_cube.clone());
        let mut gl_tracer = GlTracer::new(world_cube_rc.clone());

        // Initialize GL resources
        unsafe {
            if let Err(e) = gl_tracer.init_gl(&gl) {
                eprintln!("Failed to initialize GL tracer: {}", e);
                return;
            }
        }

        // Enable debug visualization
        gl_tracer.set_show_errors(true);
        gl_tracer.set_disable_lighting(true);

        // Debug: print world cube info
        println!("[DEBUG] World cube type: {:?}", match &world_cube {
            Cube::Solid(v) => format!("Solid({})", v),
            Cube::Cubes(_) => "Cubes(...)".to_string(),
            Cube::Quad { .. } => "Quad(...)".to_string(),
            Cube::Layers { .. } => "Layers(...)".to_string(),
        });

        // Initialize mesh renderer
        let mut mesh_renderer = MeshRenderer::new();
        unsafe {
            if let Err(e) = mesh_renderer.init_gl(&gl) {
                eprintln!("Failed to initialize mesh renderer: {}", e);
                return;
            }
        }

        // Initialize physics world
        let gravity = glam::Vec3::new(0.0, self.config.physics.gravity, 0.0);
        let mut physics_world = PhysicsWorld::new(gravity);

        // Create world collider (static terrain)
        let world_collider = VoxelColliderBuilder::from_cube(&world_cube_rc, world_depth);
        let world_body = RigidBodyBuilder::fixed().build();
        let world_body_handle = physics_world.add_rigid_body(world_body);
        physics_world.add_collider(world_collider, world_body_handle);

        // Load models and spawn dynamic cubes
        let models = load_vox_models(&self.config.spawning.models_path);
        let objects = spawn_cube_objects(&self.config.spawning, &models, &mut physics_world);

        // Create camera physics object for first-person mode
        let spawn_pos = glam::Vec3::from(self.config.fps.spawn_position);
        let camera_object = CameraObject::new(
            &mut physics_world,
            spawn_pos,
            self.config.fps.eye_height,
            self.config.fps.collision_radius,
        );

        // Upload meshes for each spawned object
        let mut object_mesh_indices = Vec::new();
        for obj in &objects {
            unsafe {
                match mesh_renderer.upload_mesh(&gl, &obj.cube, obj.depth) {
                    Ok(mesh_idx) => {
                        object_mesh_indices.push(mesh_idx);
                        println!("  Uploaded mesh for {} (index: {})", obj.model_name, mesh_idx);
                    }
                    Err(e) => {
                        eprintln!("  Warning: Failed to upload mesh for {}: {}", obj.model_name, e);
                        object_mesh_indices.push(0); // Use fallback
                    }
                }
            }
        }

        println!("Proto-GL Physics Viewer initialized!");
        println!("  World depth: {}", world_depth);
        println!("  Camera distance: {:.1}", self.orbit_camera.distance);
        println!("  Gravity: {:.2}", self.config.physics.gravity);
        println!("  Physics timestep: {:.4}", self.config.physics.timestep);
        println!("  Spawned objects: {}", objects.len());
        println!("  Uploaded meshes: {}", object_mesh_indices.len());
        println!("  FPS camera spawn: {:?}", self.config.fps.spawn_position);
        println!("Controls:");
        println!("  Tab: Enter/exit First-Person mode");
        println!("  Orbit: Right-click drag to rotate, scroll to zoom");
        println!("  FPS: WASD to move, F/V for up/down, Tab/Esc to exit");

        self.window = Some(window);
        self.gl_context = Some(gl_context);
        self.gl_surface = Some(gl_surface);
        self.gl = Some(gl);
        self.egui_ctx = Some(egui_ctx);
        self.egui_state = Some(egui_state);
        self.painter = Some(painter);
        self.gl_tracer = Some(gl_tracer);
        self.mesh_renderer = Some(mesh_renderer);
        self.object_mesh_indices = object_mesh_indices;
        self.world_cube = Some(world_cube);
        self.world_depth = world_depth;
        self.physics_world = Some(physics_world);
        self.camera_object = Some(camera_object);
        self.objects = objects;
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(egui_state) = &mut self.egui_state {
            let _ = egui_state.on_window_event(self.window.as_ref().unwrap(), &event);
        }

        match event {
            WindowEvent::CloseRequested => {
                self.cleanup();
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
            WindowEvent::KeyboardInput { event, .. } => {
                let pressed = event.state == ElementState::Pressed;

                match event.physical_key {
                    // Tab or Escape to toggle/exit camera mode
                    PhysicalKey::Code(KeyCode::Tab) | PhysicalKey::Code(KeyCode::Escape) if pressed => {
                        self.toggle_camera_mode();
                    }
                    // WASD movement
                    PhysicalKey::Code(KeyCode::KeyW) => self.keyboard_state.w = pressed,
                    PhysicalKey::Code(KeyCode::KeyA) => self.keyboard_state.a = pressed,
                    PhysicalKey::Code(KeyCode::KeyS) => self.keyboard_state.s = pressed,
                    PhysicalKey::Code(KeyCode::KeyD) => self.keyboard_state.d = pressed,
                    // F/V for up/down
                    PhysicalKey::Code(KeyCode::KeyF) => self.keyboard_state.f = pressed,
                    PhysicalKey::Code(KeyCode::KeyV) => self.keyboard_state.v = pressed,
                    // Space for jump
                    PhysicalKey::Code(KeyCode::Space) => self.keyboard_state.space = pressed,
                    _ => {}
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                // Orbit camera: right-click drag to rotate
                if self.camera_mode == CameraMode::Orbit && button == MouseButton::Right {
                    self.orbit_camera.dragging = state == ElementState::Pressed;
                    if !self.orbit_camera.dragging {
                        self.orbit_camera.last_mouse_pos = None;
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                match self.camera_mode {
                    CameraMode::Orbit => {
                        if self.orbit_camera.dragging {
                            if let Some((last_x, last_y)) = self.orbit_camera.last_mouse_pos {
                                let delta_x = position.x as f32 - last_x;
                                let delta_y = position.y as f32 - last_y;
                                self.orbit_camera.handle_mouse_drag(delta_x, delta_y);
                            }
                            self.orbit_camera.last_mouse_pos = Some((position.x as f32, position.y as f32));

                            if let Some(window) = &self.window {
                                window.request_redraw();
                            }
                        }
                    }
                    CameraMode::FirstPerson => {
                        if self.fps_camera.mouse_captured {
                            // Calculate delta from last frame's position
                            if let Some((last_x, last_y)) = self.last_mouse_pos {
                                let delta_x = position.x - last_x;
                                let delta_y = position.y - last_y;

                                // Only process if there's actual movement
                                if delta_x.abs() > 0.1 || delta_y.abs() > 0.1 {
                                    self.fps_camera.handle_mouse_move(delta_x as f32, delta_y as f32);
                                }
                            }
                            // Always update last position for next frame's delta calculation
                            self.last_mouse_pos = Some((position.x, position.y));
                        }
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if self.camera_mode == CameraMode::Orbit {
                    let scroll_delta = match delta {
                        MouseScrollDelta::LineDelta(_x, y) => y * 2.0,
                        MouseScrollDelta::PixelDelta(pos) => pos.y as f32 * 0.01,
                    };
                    self.orbit_camera.handle_scroll(scroll_delta);

                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.render(event_loop);
            }
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        // Handle raw mouse motion for locked cursor mode (used on Wayland)
        if let DeviceEvent::MouseMotion { delta } = event {
            if self.camera_mode == CameraMode::FirstPerson && self.fps_camera.mouse_captured {
                let (delta_x, delta_y) = delta;
                self.fps_camera.handle_mouse_move(delta_x as f32, delta_y as f32);
            }
        }
    }
}

impl ProtoGlApp {
    /// Toggle between Orbit and FirstPerson camera modes
    fn toggle_camera_mode(&mut self) {
        self.camera_mode = match self.camera_mode {
            CameraMode::Orbit => {
                println!("Switched to First-Person mode (Tab/Esc to exit)");
                // Sync FPS camera position with physics object
                if let (Some(physics_world), Some(camera_obj)) = (&self.physics_world, &self.camera_object) {
                    self.fps_camera.set_position(camera_obj.position(physics_world));
                }
                // Capture mouse immediately
                self.capture_mouse();
                CameraMode::FirstPerson
            }
            CameraMode::FirstPerson => {
                println!("Switched to Orbit mode");
                self.release_mouse();
                CameraMode::Orbit
            }
        };
    }

    /// Capture mouse for FPS look-around
    fn capture_mouse(&mut self) {
        if let Some(window) = &self.window {
            // Try Locked mode first (provides raw motion on Wayland), fall back to Confined
            let grab_result = window.set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined));

            if let Err(e) = grab_result {
                println!("Warning: Failed to grab cursor: {:?}", e);
            }

            window.set_cursor_visible(false);
            self.fps_camera.mouse_captured = true;
            // Reset last position so first move doesn't cause a jump
            self.last_mouse_pos = None;

            println!("Mouse captured - move to look around, Esc to release");
        }
    }

    /// Release mouse from FPS mode
    fn release_mouse(&mut self) {
        if let Some(window) = &self.window {
            let _ = window.set_cursor_grab(CursorGrabMode::None);
            window.set_cursor_visible(true);
            self.fps_camera.mouse_captured = false;
            self.last_mouse_pos = None;
        }
    }

    fn render(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            return;
        }

        self.frame_count += 1;

        // Update timing
        let now = Instant::now();
        let delta = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
        self.frame_time = delta;
        self.fps = 1.0 / delta.max(0.001);

        // Update FPS camera movement if in first-person mode
        if self.camera_mode == CameraMode::FirstPerson {
            // Calculate velocity from keyboard input
            let velocity = self.fps_camera.calculate_velocity(
                self.keyboard_state.w,
                self.keyboard_state.s,
                self.keyboard_state.a,
                self.keyboard_state.d,
                self.keyboard_state.f,
                self.keyboard_state.v,
            );

            // Move camera physics object
            if let (Some(physics_world), Some(camera_obj)) =
                (&mut self.physics_world, &mut self.camera_object)
            {
                camera_obj.move_with_velocity(
                    physics_world,
                    velocity,
                    delta,
                    self.config.physics.gravity,
                );
            }
        }

        // Physics simulation with fixed timestep
        if let Some(physics_world) = &mut self.physics_world {
            self.physics_accumulator += delta;
            let timestep = self.config.physics.timestep;

            while self.physics_accumulator >= timestep {
                physics_world.step(timestep);
                self.physics_accumulator -= timestep;
            }
        }

        // Update FPS camera position from physics
        if self.camera_mode == CameraMode::FirstPerson {
            if let (Some(physics_world), Some(camera_obj)) =
                (&self.physics_world, &self.camera_object)
            {
                self.fps_camera.set_position(camera_obj.position(physics_world));
            }
        }

        let window = self.window.as_ref().unwrap();
        let gl = self.gl.as_ref().unwrap();
        let egui_ctx = self.egui_ctx.as_ref().unwrap();
        let egui_state = self.egui_state.as_mut().unwrap();
        let painter = self.painter.as_mut().unwrap();
        let gl_tracer = self.gl_tracer.as_mut().unwrap();
        let gl_context = self.gl_context.as_ref().unwrap();
        let gl_surface = self.gl_surface.as_ref().unwrap();

        let size = window.inner_size();

        unsafe {
            gl.viewport(0, 0, size.width as i32, size.height as i32);
            gl.clear_color(0.1, 0.1, 0.1, 1.0);
            gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);
        }

        // Create camera config based on current mode
        let camera = match self.camera_mode {
            CameraMode::Orbit => CameraConfig {
                position: self.orbit_camera.position(),
                rotation: self.orbit_camera.rotation(),
                vfov: 60.0_f32.to_radians(),
                pitch: self.orbit_camera.pitch(),
                yaw: self.orbit_camera.yaw(),
                target_position: Some(self.orbit_camera.focus),
            },
            CameraMode::FirstPerson => CameraConfig {
                position: self.fps_camera.position(),
                rotation: self.fps_camera.rotation(),
                vfov: 60.0_f32.to_radians(),
                pitch: self.fps_camera.pitch(),
                yaw: self.fps_camera.yaw(),
                target_position: None,
            },
        };

        // Render world using GlTracer (if enabled)
        if self.render_world {
            unsafe {
                gl_tracer.render_to_gl_with_camera(
                    gl,
                    size.width as i32,
                    size.height as i32,
                    &camera,
                );
            }
        }

        // Render dynamic objects (if enabled)
        if self.render_objects {
            if let (Some(mesh_renderer), Some(physics_world)) =
                (self.mesh_renderer.as_ref(), self.physics_world.as_ref())
            {
                for (i, obj) in self.objects.iter().enumerate() {
                    if i >= self.object_mesh_indices.len() {
                        continue;
                    }

                    // Get physics position and rotation
                    let Some(body) = physics_world.get_rigid_body(obj.body_handle) else {
                        continue;
                    };
                    let position = body.translation();
                    let rotation = body.rotation();

                    let position = glam::Vec3::new(position.x, position.y, position.z);
                    let rotation = glam::Quat::from_xyzw(rotation.i, rotation.j, rotation.k, rotation.w);

                    // Mesh is in [0,1] space, centered to [-0.5, 0.5] by renderer
                    // object_size is the desired edge length in normalized world units
                    let scale = self.config.spawning.object_size;
                    unsafe {
                        mesh_renderer.render_mesh_with_scale(
                            gl,
                            self.object_mesh_indices[i],
                            position,
                            rotation,
                            scale,
                            &camera,
                            size.width as i32,
                            size.height as i32,
                        );
                    }
                }
            }
        }

        // Capture UI state before egui run
        let (cam_distance, cam_yaw, cam_pitch, cam_pos, cam_rot) = match self.camera_mode {
            CameraMode::Orbit => (
                self.orbit_camera.distance,
                self.orbit_camera.yaw(),
                self.orbit_camera.pitch(),
                self.orbit_camera.position(),
                self.orbit_camera.rotation(),
            ),
            CameraMode::FirstPerson => (
                0.0, // No distance in FPS mode
                self.fps_camera.yaw(),
                self.fps_camera.pitch(),
                self.fps_camera.position(),
                self.fps_camera.rotation(),
            ),
        };

        let mut ui_state = UiState {
            fps: self.fps,
            frame_time: self.frame_time,
            world_depth: self.world_depth,
            gravity: self.config.physics.gravity,
            timestep: self.config.physics.timestep,
            camera_distance: cam_distance,
            camera_yaw: cam_yaw,
            camera_pitch: cam_pitch,
            camera_pos: cam_pos,
            camera_rot: cam_rot,
            object_count: self.objects.len(),
            render_world: self.render_world,
            render_objects: self.render_objects,
            show_debug_info: self.show_debug_info,
            camera_mode: self.camera_mode,
        };

        // Run egui
        let raw_input = egui_state.take_egui_input(window);
        let full_output = egui_ctx.run(raw_input, |ctx| {
            render_debug_panel(ctx, &mut ui_state);
        });

        egui_state.handle_platform_output(window, full_output.platform_output);

        // Update app state from UI
        self.render_world = ui_state.render_world;
        self.render_objects = ui_state.render_objects;
        self.show_debug_info = ui_state.show_debug_info;

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

        // Capture frame to file if requested (before swap to capture the rendered content)
        if let Some(ref path) = self.capture_frame_path {
            match self.save_framebuffer_to_file(gl, size.width, size.height, path) {
                Ok(()) => println!("[CAPTURE] Frame saved to: {}", path),
                Err(e) => eprintln!("[CAPTURE] Failed to save frame: {}", e),
            }
        }

        gl_surface.swap_buffers(gl_context).unwrap();

        // In debug mode or capture mode, exit after single frame
        if self.debug_mode {
            println!("[DEBUG] Frame {} rendered, exiting", self.frame_count);
            self.cleanup();
            event_loop.exit();
            return;
        }

        window.request_redraw();
    }

    /// Save the current framebuffer to an image file
    fn save_framebuffer_to_file(
        &self,
        gl: &Context,
        width: u32,
        height: u32,
        path: &str,
    ) -> Result<(), String> {
        // Read pixels from framebuffer (RGBA format)
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        unsafe {
            gl.read_pixels(
                0,
                0,
                width as i32,
                height as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(&mut pixels),
            );
        }

        // Convert RGBA to RGB (image crate expects RGB for PNG)
        let rgb_pixels: Vec<u8> = pixels
            .chunks(4)
            .flat_map(|rgba| [rgba[0], rgba[1], rgba[2]])
            .collect();

        // Flip Y-axis (GL origin is bottom-left, image origin is top-left)
        let mut flipped = vec![0u8; rgb_pixels.len()];
        for y in 0..height {
            let src_row = &rgb_pixels[(y * width * 3) as usize..((y + 1) * width * 3) as usize];
            let dst_y = height - 1 - y;
            let dst_row = &mut flipped[(dst_y * width * 3) as usize..((dst_y + 1) * width * 3) as usize];
            dst_row.copy_from_slice(src_row);
        }

        // Save to file
        image::save_buffer(
            path,
            &flipped,
            width,
            height,
            image::ColorType::Rgb8,
        )
        .map_err(|e| e.to_string())
    }

    fn cleanup(&mut self) {
        println!("[DEBUG] Cleaning up resources...");

        // Destroy egui painter first (it holds GL resources)
        if let Some(mut painter) = self.painter.take() {
            if self.gl.is_some() {
                painter.destroy();
            }
        }

        // Clear egui state
        self.egui_state = None;
        self.egui_ctx = None;

        // Destroy GL tracer
        if let Some(mut gl_tracer) = self.gl_tracer.take() {
            if let Some(gl) = &self.gl {
                unsafe {
                    gl_tracer.destroy_gl(gl);
                }
            }
        }

        // Destroy mesh renderer
        if let Some(mut mesh_renderer) = self.mesh_renderer.take() {
            if let Some(gl) = &self.gl {
                unsafe {
                    mesh_renderer.destroy_gl(gl);
                }
            }
        }

        // Clear physics
        self.physics_world = None;
        self.objects.clear();
        self.object_mesh_indices.clear();

        // Clear world
        self.world_cube = None;

        // Clear GL context - important: surface must be released before context
        self.gl = None;
        self.gl_surface = None;
        self.gl_context = None;

        // Clear window last
        self.window = None;

        println!("[DEBUG] Cleanup complete");
    }
}
