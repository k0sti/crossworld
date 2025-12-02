use glow::*;
use glutin::config::ConfigTemplateBuilder;
use glutin::context::{ContextApi, ContextAttributesBuilder, Version};
use glutin::display::GetGlDisplay;
use glutin::prelude::*;
use glutin::surface::{SurfaceAttributesBuilder, WindowSurface};
use glutin_winit::DisplayBuilder;
use raw_window_handle::HasWindowHandle;
use std::error::Error;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

use glam::{Mat4, Vec3};
use serde::Deserialize;
use std::rc::Rc;

use cube::{Cube, parse_csm, load_vox_to_cube};
use crossworld_physics::{rapier3d::prelude::*, PhysicsWorld, VoxelColliderBuilder};
use renderer::{GlCubeTracer, CameraConfig};

/// Configuration loaded from config.toml
#[derive(Debug, Deserialize, Clone)]
struct ProtoGlConfig {
    world: WorldConfig,
    physics: PhysicsConfig,
    spawning: SpawningConfig,
    rendering: RenderConfig,
}

#[derive(Debug, Deserialize, Clone)]
struct WorldConfig {
    macro_depth: u32,
    micro_depth: u32,
    border_depth: u32,
    #[serde(default = "default_border_materials")]
    border_materials: [u8; 4],
    root_cube: String,
}

fn default_border_materials() -> [u8; 4] {
    [32, 32, 0, 0] // Bottom: bedrock, Top: air
}

#[derive(Debug, Deserialize, Clone)]
struct PhysicsConfig {
    gravity: f32,
    timestep: f32,
}

#[derive(Debug, Deserialize, Clone)]
struct SpawningConfig {
    spawn_count: u32,
    models_path: String,
    min_height: f32,
    max_height: f32,
    spawn_radius: f32,
}

#[derive(Debug, Deserialize, Clone)]
struct RenderConfig {
    viewport_width: u32,
    viewport_height: u32,
    camera_distance: f32,
}

impl Default for ProtoGlConfig {
    fn default() -> Self {
        Self {
            world: WorldConfig {
                macro_depth: 3,
                micro_depth: 4,
                border_depth: 1,
                border_materials: [32, 32, 0, 0],
                root_cube: ">a [5 5 4 9 5 5 0 0]".to_string(),
            },
            physics: PhysicsConfig {
                gravity: -9.81,
                timestep: 0.016666,
            },
            spawning: SpawningConfig {
                spawn_count: 10,
                models_path: "assets/models/".to_string(),
                min_height: 10.0,
                max_height: 30.0,
                spawn_radius: 20.0,
            },
            rendering: RenderConfig {
                viewport_width: 800,
                viewport_height: 600,
                camera_distance: 30.0,
            },
        }
    }
}

/// Orbit camera for viewing the scene
struct OrbitCamera {
    focus: Vec3,
    yaw: f32,
    pitch: f32,
    distance: f32,
    dragging: bool,
    last_mouse_pos: Option<(f32, f32)>,
}

impl OrbitCamera {
    fn new(distance: f32) -> Self {
        Self {
            focus: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.5,
            distance,
            dragging: false,
            last_mouse_pos: None,
        }
    }

    fn view_matrix(&self) -> Mat4 {
        let x = self.distance * self.pitch.cos() * self.yaw.sin();
        let y = self.distance * self.pitch.sin();
        let z = self.distance * self.pitch.cos() * self.yaw.cos();

        Mat4::look_at_rh(
            self.focus + Vec3::new(x, y, z),
            self.focus,
            Vec3::Y,
        )
    }

    fn position(&self) -> Vec3 {
        let x = self.distance * self.pitch.cos() * self.yaw.sin();
        let y = self.distance * self.pitch.sin();
        let z = self.distance * self.pitch.cos() * self.yaw.cos();
        self.focus + Vec3::new(x, y, z)
    }

    fn rotation(&self) -> glam::Quat {
        // Calculate the direction from camera to focus
        let pos = self.position();
        let dir = (self.focus - pos).normalize();

        // Create a rotation that looks at the focus point
        glam::Quat::from_rotation_arc(Vec3::NEG_Z, dir)
    }

    fn handle_mouse_drag(&mut self, delta_x: f32, delta_y: f32) {
        self.yaw -= delta_x * 0.003;
        self.pitch -= delta_y * 0.003;
        self.pitch = self.pitch.clamp(-1.5, 1.5);
    }

    fn handle_scroll(&mut self, delta: f32) {
        self.distance -= delta;
        self.distance = self.distance.clamp(5.0, 100.0);
    }
}

/// A voxel model loaded from a .vox file
struct VoxModel {
    cube: Rc<Cube<u8>>,
    name: String,
    depth: u32,
}

/// A dynamic cube object in the physics simulation
struct CubeObject {
    /// Voxel data
    cube: Rc<Cube<u8>>,
    /// Rapier rigid body handle
    body_handle: RigidBodyHandle,
    /// Rapier collider handle
    collider_handle: ColliderHandle,
    /// Model source (for identification)
    model_name: String,
    /// Octree depth for rendering
    depth: u32,
}

struct ProtoGlApp {
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
    gl_tracer: Option<GlCubeTracer>,
    camera: OrbitCamera,

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
}

impl Default for ProtoGlApp {
    fn default() -> Self {
        // Load config from file or use defaults
        let config = load_config().unwrap_or_else(|e| {
            eprintln!("Warning: Failed to load config.toml: {}", e);
            eprintln!("Using default configuration");
            ProtoGlConfig::default()
        });

        let camera = OrbitCamera::new(config.rendering.camera_distance);

        Self {
            window: None,
            gl_context: None,
            gl_surface: None,
            gl: None,
            egui_ctx: None,
            egui_state: None,
            painter: None,
            gl_tracer: None,
            camera,
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
        let mut gl_tracer = GlCubeTracer::new(world_cube_rc.clone());

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

        // Initialize physics world
        let gravity = Vec3::new(0.0, self.config.physics.gravity, 0.0);
        let mut physics_world = PhysicsWorld::new(gravity);

        // Create world collider (static terrain)
        let world_collider = VoxelColliderBuilder::from_cube(&world_cube_rc, world_depth);
        let world_body = RigidBodyBuilder::fixed().build();
        let world_body_handle = physics_world.add_rigid_body(world_body);
        physics_world.add_collider(world_collider, world_body_handle);

        // Load models and spawn dynamic cubes
        let models = load_vox_models(&self.config.spawning.models_path);
        let objects = spawn_cube_objects(&self.config.spawning, &models, &mut physics_world);

        println!("Proto-GL Physics Viewer initialized!");
        println!("  World depth: {}", world_depth);
        println!("  Camera distance: {:.1}", self.camera.distance);
        println!("  Gravity: {:.2}", self.config.physics.gravity);
        println!("  Physics timestep: {:.4}", self.config.physics.timestep);
        println!("  Spawned objects: {}", objects.len());

        self.window = Some(window);
        self.gl_context = Some(gl_context);
        self.gl_surface = Some(gl_surface);
        self.gl = Some(gl);
        self.egui_ctx = Some(egui_ctx);
        self.egui_state = Some(egui_state);
        self.painter = Some(painter);
        self.gl_tracer = Some(gl_tracer);
        self.world_cube = Some(world_cube);
        self.world_depth = world_depth;
        self.physics_world = Some(physics_world);
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
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Right {
                    self.camera.dragging = state == ElementState::Pressed;
                    if !self.camera.dragging {
                        self.camera.last_mouse_pos = None;
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if self.camera.dragging {
                    if let Some((last_x, last_y)) = self.camera.last_mouse_pos {
                        let delta_x = position.x as f32 - last_x;
                        let delta_y = position.y as f32 - last_y;
                        self.camera.handle_mouse_drag(delta_x, delta_y);
                    }
                    self.camera.last_mouse_pos = Some((position.x as f32, position.y as f32));

                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll_delta = match delta {
                    MouseScrollDelta::LineDelta(_x, y) => y * 2.0,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32 * 0.01,
                };
                self.camera.handle_scroll(scroll_delta);

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                self.render();
            }
            _ => {}
        }
    }
}

impl ProtoGlApp {
    fn render(&mut self) {
        if self.window.is_none() {
            return;
        }

        // Update timing
        let now = Instant::now();
        let delta = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
        self.frame_time = delta;
        self.fps = 1.0 / delta.max(0.001);

        // Physics simulation with fixed timestep
        if let Some(physics_world) = &mut self.physics_world {
            self.physics_accumulator += delta;
            let timestep = self.config.physics.timestep;

            while self.physics_accumulator >= timestep {
                physics_world.step(timestep);
                self.physics_accumulator -= timestep;
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

        // Create camera config from orbit camera
        let camera = CameraConfig {
            position: self.camera.position(),
            rotation: self.camera.rotation(),
            vfov: 60.0_f32.to_radians(),
        };

        // Render world using GlCubeTracer (if enabled)
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

        // TODO: Render dynamic objects here (if render_objects is true)

        // Capture UI state before egui run
        let fps = self.fps;
        let frame_time = self.frame_time;
        let world_depth = self.world_depth;
        let gravity = self.config.physics.gravity;
        let timestep = self.config.physics.timestep;
        let camera_distance = self.camera.distance;
        let camera_yaw = self.camera.yaw;
        let camera_pitch = self.camera.pitch;
        let camera_pos = self.camera.position();
        let camera_rot = self.camera.rotation();
        let object_count = self.objects.len();
        let mut render_world = self.render_world;
        let mut render_objects = self.render_objects;
        let mut show_debug_info = self.show_debug_info;

        // Run egui
        let raw_input = egui_state.take_egui_input(window);
        let full_output = egui_ctx.run(raw_input, |ctx| {
            egui::SidePanel::right("controls").show(ctx, |ui| {
                ui.heading("Proto-GL Viewer");

                ui.separator();
                ui.label(format!("FPS: {:.1}", fps));
                ui.label(format!("Frame time: {:.2} ms", frame_time * 1000.0));
                ui.label(format!("Objects: {}", object_count));

                ui.separator();
                ui.heading("Configuration");
                ui.label(format!("World depth: {}", world_depth));
                ui.label(format!("Gravity: {:.2}", gravity));
                ui.label(format!("Timestep: {:.4}", timestep));

                ui.separator();
                ui.heading("Camera");
                ui.label(format!("Distance: {:.1}", camera_distance));
                ui.label(format!("Yaw: {:.2}", camera_yaw));
                ui.label(format!("Pitch: {:.2}", camera_pitch));

                ui.separator();
                ui.heading("Rendering");
                ui.checkbox(&mut render_world, "Render World");
                ui.checkbox(&mut render_objects, "Render Objects");
                ui.checkbox(&mut show_debug_info, "Show Debug Info");

                if show_debug_info {
                    ui.separator();
                    ui.heading("Debug Info");
                    ui.label(format!("Cam Pos: ({:.1}, {:.1}, {:.1})",
                        camera_pos.x, camera_pos.y, camera_pos.z));
                    ui.label(format!("Cam Rot: ({:.2}, {:.2}, {:.2}, {:.2})",
                        camera_rot.x, camera_rot.y, camera_rot.z, camera_rot.w));
                }

                ui.separator();
                if ui.button("Reset Scene").clicked() {
                    println!("Reset scene (not yet implemented)");
                }

                ui.separator();
                ui.label("Controls:");
                ui.label("• Right-click drag: Rotate camera");
                ui.label("• Mouse wheel: Zoom");
            });
        });

        egui_state.handle_platform_output(window, full_output.platform_output);

        // Update app state from UI
        self.render_world = render_world;
        self.render_objects = render_objects;
        self.show_debug_info = show_debug_info;

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

        window.request_redraw();
    }
}

/// Load configuration from config.toml
fn load_config() -> Result<ProtoGlConfig, Box<dyn Error>> {
    let config_path = "crates/proto-gl/config.toml";
    let config_str = std::fs::read_to_string(config_path)?;
    let config: ProtoGlConfig = toml::from_str(&config_str)?;
    Ok(config)
}

/// Generate world cube from configuration
fn generate_world(config: &WorldConfig) -> (Cube<u8>, u32) {
    use std::rc::Rc;

    // Parse CSM
    let parse_result = parse_csm(&config.root_cube);
    let mut cube = match parse_result {
        Ok(result) => result.root,
        Err(e) => {
            eprintln!("Warning: Failed to parse CSM: {}", e);
            eprintln!("Using simple octree default");
            Cube::cubes([
                Rc::new(Cube::solid(5)),
                Rc::new(Cube::solid(5)),
                Rc::new(Cube::solid(4)),
                Rc::new(Cube::solid(9)),
                Rc::new(Cube::solid(5)),
                Rc::new(Cube::solid(5)),
                Rc::new(Cube::solid(0)),
                Rc::new(Cube::solid(0)),
            ]);
            Cube::solid(8)
        }
    };

    // Calculate total depth
    let total_depth = config.macro_depth + config.micro_depth + config.border_depth;

    // Apply border layers if needed
    if config.border_depth > 0 {
        cube = add_border_layers(cube, config.border_depth, config.border_materials);
    }

    (cube, total_depth)
}

/// Add border layers to cube (copied pattern from proto)
fn add_border_layers(cube: Cube<u8>, border_depth: u32, materials: [u8; 4]) -> Cube<u8> {
    use std::rc::Rc;

    let mut result = cube;
    for _ in 0..border_depth {
        let child = Rc::new(result.clone());
        result = Cube::cubes([
            Rc::new(Cube::solid(materials[0])), // Bottom layer
            child.clone(),
            child.clone(),
            child.clone(),
            child.clone(),
            child.clone(),
            child.clone(),
            Rc::new(Cube::solid(materials[1])), // Top layer
        ]);
    }
    result
}

/// Calculate the depth of a cube (how many levels of octree)
fn calculate_cube_depth(cube: &Cube<u8>) -> u32 {
    fn depth_recursive(cube: &Cube<u8>) -> u32 {
        match cube {
            Cube::Solid(_) => 0,
            Cube::Cubes(children) => {
                1 + children
                    .iter()
                    .map(|c| depth_recursive(c))
                    .max()
                    .unwrap_or(0)
            }
            Cube::Quad { quads, .. } => {
                1 + quads
                    .iter()
                    .map(|c| depth_recursive(c))
                    .max()
                    .unwrap_or(0)
            }
            Cube::Layers { layers, .. } => {
                1 + layers
                    .iter()
                    .map(|c| depth_recursive(c))
                    .max()
                    .unwrap_or(0)
            }
        }
    }
    depth_recursive(cube)
}

/// Convert Cube<i32> to Cube<u8> by mapping material IDs
fn convert_cube_i32_to_u8(cube: &Cube<i32>) -> Cube<u8> {
    match cube {
        Cube::Solid(v) => {
            let u8_val = if *v > 0 { (*v as u8).max(1) } else { 0 };
            Cube::Solid(u8_val)
        }
        Cube::Cubes(children) => {
            let converted_children: Box<[Rc<Cube<u8>>; 8]> = Box::new([
                Rc::new(convert_cube_i32_to_u8(&children[0])),
                Rc::new(convert_cube_i32_to_u8(&children[1])),
                Rc::new(convert_cube_i32_to_u8(&children[2])),
                Rc::new(convert_cube_i32_to_u8(&children[3])),
                Rc::new(convert_cube_i32_to_u8(&children[4])),
                Rc::new(convert_cube_i32_to_u8(&children[5])),
                Rc::new(convert_cube_i32_to_u8(&children[6])),
                Rc::new(convert_cube_i32_to_u8(&children[7])),
            ]);
            Cube::Cubes(converted_children)
        }
        Cube::Quad { axis, quads } => {
            let converted_quads: [Rc<Cube<u8>>; 4] = [
                Rc::new(convert_cube_i32_to_u8(&quads[0])),
                Rc::new(convert_cube_i32_to_u8(&quads[1])),
                Rc::new(convert_cube_i32_to_u8(&quads[2])),
                Rc::new(convert_cube_i32_to_u8(&quads[3])),
            ];
            Cube::Quad {
                axis: *axis,
                quads: converted_quads,
            }
        }
        Cube::Layers { axis, layers } => {
            let converted_layers: [Rc<Cube<u8>>; 2] = [
                Rc::new(convert_cube_i32_to_u8(&layers[0])),
                Rc::new(convert_cube_i32_to_u8(&layers[1])),
            ];
            Cube::Layers {
                axis: *axis,
                layers: converted_layers,
            }
        }
    }
}

/// Load .vox models from a directory
fn load_vox_models(models_path: &str) -> Vec<VoxModel> {
    use std::fs;
    use std::path::Path;

    let mut models = Vec::new();

    // Check if directory exists
    let path = Path::new(models_path);
    if !path.exists() || !path.is_dir() {
        eprintln!("Warning: Models directory not found: {}", models_path);
        eprintln!("Creating fallback simple cube models");

        // Create a few simple cube models as fallback
        models.push(VoxModel {
            cube: Rc::new(Cube::solid(5)), // Grass
            name: "simple_cube_grass".to_string(),
            depth: 0,
        });
        models.push(VoxModel {
            cube: Rc::new(Cube::solid(4)), // Stone
            name: "simple_cube_stone".to_string(),
            depth: 0,
        });
        models.push(VoxModel {
            cube: Rc::new(Cube::solid(9)), // Wood
            name: "simple_cube_wood".to_string(),
            depth: 0,
        });

        return models;
    }

    // Load .vox files from directory
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let file_path = entry.path();
            if file_path.extension().map_or(false, |ext| ext == "vox") {
                // Read file bytes
                let bytes = match fs::read(&file_path) {
                    Ok(b) => b,
                    Err(e) => {
                        eprintln!("Warning: Failed to read {}: {}", file_path.display(), e);
                        continue;
                    }
                };

                // Load with center alignment
                match load_vox_to_cube(&bytes, Vec3::splat(0.5)) {
                    Ok(cube_i32) => {
                        // Convert Cube<i32> to Cube<u8>
                        let cube = convert_cube_i32_to_u8(&cube_i32);

                        // Calculate depth from cube size
                        let depth = calculate_cube_depth(&cube);
                        let name = file_path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        models.push(VoxModel {
                            cube: Rc::new(cube),
                            name,
                            depth,
                        });
                        println!("Loaded model: {} (depth {})", file_path.display(), depth);
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to load {}: {}", file_path.display(), e);
                    }
                }
            }
        }
    }

    // If no models loaded, use fallback
    if models.is_empty() {
        eprintln!("Warning: No .vox models found in {}", models_path);
        eprintln!("Using fallback simple cube models");
        models.push(VoxModel {
            cube: Rc::new(Cube::solid(5)),
            name: "fallback_cube".to_string(),
            depth: 0,
        });
    }

    println!("Loaded {} model(s)", models.len());
    models
}

/// Spawn dynamic cube objects with physics
fn spawn_cube_objects(
    config: &SpawningConfig,
    models: &[VoxModel],
    physics_world: &mut PhysicsWorld,
) -> Vec<CubeObject> {
    use rand::Rng;

    let mut objects = Vec::new();
    let mut rng = rand::thread_rng();

    for i in 0..config.spawn_count {
        // Random position
        let x = rng.gen_range(-config.spawn_radius..config.spawn_radius);
        let y = rng.gen_range(config.min_height..config.max_height);
        let z = rng.gen_range(-config.spawn_radius..config.spawn_radius);

        // Random model
        let model = &models[i as usize % models.len()];

        // Create physics body
        let rb = RigidBodyBuilder::dynamic()
            .translation(vector![x, y, z])
            .build();
        let rb_handle = physics_world.add_rigid_body(rb);

        // Create collider from voxel cube
        let collider = VoxelColliderBuilder::from_cube(&model.cube, model.depth);
        let coll_handle = physics_world.add_collider(collider, rb_handle);

        objects.push(CubeObject {
            cube: model.cube.clone(),
            body_handle: rb_handle,
            collider_handle: coll_handle,
            model_name: model.name.clone(),
            depth: model.depth,
        });
    }

    println!("Spawned {} dynamic cubes", objects.len());
    objects
}

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = ProtoGlApp::default();

    event_loop.run_app(&mut app)?;
    Ok(())
}
