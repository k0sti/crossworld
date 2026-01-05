//! Hot-reload runtime for the game library
//!
//! This binary watches the game library for changes and hot-reloads it.

use app::{
    create_controller_backend, App, ControllerBackend, CursorMode, FrameContext,
    InputState, CREATE_APP_SYMBOL,
};
use glam::Vec2;
use glow::{Context, HasContext};
use glutin::config::ConfigTemplateBuilder;
use glutin::context::{ContextApi, ContextAttributesBuilder, Version};
use glutin::display::GetGlDisplay;
use glutin::prelude::*;
use glutin::surface::{SurfaceAttributesBuilder, WindowSurface};
use glutin_winit::DisplayBuilder;
use libloading::Library;
use notify_debouncer_mini::{new_debouncer, notify::*, DebouncedEvent};
use raw_window_handle::HasWindowHandle;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{CursorGrabMode, Window, WindowId};

#[cfg(target_os = "linux")]
use winit::platform::x11::EventLoopBuilderExtX11;

/// Get the platform-specific library filename
fn get_lib_name() -> &'static str {
    #[cfg(target_os = "linux")]
    return "libgame.so";

    #[cfg(target_os = "macos")]
    return "libgame.dylib";

    #[cfg(target_os = "windows")]
    return "game.dll";
}

/// Get the path to the game library
fn get_lib_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // Go from crates/game to crates
    path.pop(); // Go from crates to workspace root
    path.push("target");
    path.push("debug");
    path.push(get_lib_name());
    path
}

struct GameRuntime {
    window: Option<Window>,
    gl_context: Option<glutin::context::PossiblyCurrentContext>,
    gl_surface: Option<glutin::surface::Surface<WindowSurface>>,
    gl: Option<Arc<Context>>,
    app_lib: Option<Library>,
    app_instance: Option<Box<dyn App>>,

    // Timing
    start_time: Instant,
    last_update: Instant,
    frame_count: u64,

    // Input state
    input_state: InputState,
    last_mouse_pos: Option<Vec2>,

    // Controller backend
    controller_backend: Option<Box<dyn ControllerBackend>>,

    // Egui integration
    egui: Option<app::EguiIntegration>,

    // File watching
    file_events: Arc<
        Mutex<
            Receiver<
                std::result::Result<Vec<DebouncedEvent>, notify_debouncer_mini::notify::Error>,
            >,
        >,
    >,
    last_reload_time: Option<Duration>,
}

impl GameRuntime {
    fn new(
        file_events: Arc<
            Mutex<
                Receiver<
                    std::result::Result<
                        Vec<DebouncedEvent>,
                        notify_debouncer_mini::notify::Error,
                    >,
                >,
            >,
        >,
    ) -> Self {
        let controller_backend = create_controller_backend();

        Self {
            window: None,
            gl_context: None,
            gl_surface: None,
            gl: None,
            app_lib: None,
            app_instance: None,
            start_time: Instant::now(),
            last_update: Instant::now(),
            frame_count: 0,
            input_state: InputState::default(),
            last_mouse_pos: None,
            controller_backend,
            egui: None,
            file_events,
            last_reload_time: None,
        }
    }

    /// Reset per-frame input deltas
    fn reset_frame_deltas(&mut self) {
        self.input_state.mouse_delta = Vec2::ZERO;
        self.input_state.raw_mouse_delta = Vec2::ZERO;
        self.input_state.scroll_delta = Vec2::ZERO;
    }
}

impl GameRuntime {
    /// Load the app library and create the app instance
    unsafe fn load_app(&mut self) {
        let lib_path = get_lib_path();

        println!(
            "[GameRuntime] Loading app library from: {}",
            lib_path.display()
        );

        if !lib_path.exists() {
            eprintln!(
                "[GameRuntime] ERROR: App library not found at: {}",
                lib_path.display()
            );
            return;
        }

        let lib = match Library::new(&lib_path) {
            Ok(lib) => lib,
            Err(e) => {
                eprintln!("[GameRuntime] ERROR: Failed to load library: {}", e);
                return;
            }
        };

        let create_app: libloading::Symbol<unsafe extern "C" fn() -> *mut dyn App> =
            match lib.get(CREATE_APP_SYMBOL) {
                Ok(func) => func,
                Err(e) => {
                    eprintln!(
                        "[GameRuntime] ERROR: Failed to find create_app symbol: {}",
                        e
                    );
                    return;
                }
            };

        let app_ptr = create_app();
        if app_ptr.is_null() {
            eprintln!("[GameRuntime] ERROR: create_app returned null");
            return;
        }

        let mut app = Box::from_raw(app_ptr);

        // Initialize the app if we have a GL context
        if let (Some(window), Some(gl)) = (self.window.as_ref(), self.gl.as_ref()) {
            println!("[GameRuntime] Calling app.init()");
            let size = window.inner_size();
            let ctx = FrameContext {
                gl,
                window,
                delta_time: 0.0,
                elapsed: 0.0,
                frame: 0,
                size: (size.width, size.height),
            };
            app.init(&ctx);
        }

        self.app_instance = Some(app);
        self.app_lib = Some(lib);

        println!("[GameRuntime] App loaded successfully");
    }

    /// Unload the current app library
    unsafe fn unload_app(&mut self) {
        println!("[GameRuntime] Unloading app library");

        if let (Some(mut app), Some(window), Some(gl)) =
            (self.app_instance.take(), self.window.as_ref(), self.gl.as_ref())
        {
            println!("[GameRuntime] Calling app.shutdown()");
            let size = window.inner_size();
            let ctx = FrameContext {
                gl,
                window,
                delta_time: 0.0,
                elapsed: self.start_time.elapsed().as_secs_f32(),
                frame: self.frame_count,
                size: (size.width, size.height),
            };
            app.shutdown(&ctx);
            drop(app);
        }

        if let Some(lib) = self.app_lib.take() {
            drop(lib);
            println!("[GameRuntime] App library unloaded");
        }
    }

    /// Reload the app library
    unsafe fn reload_app(&mut self) {
        let start_time = Instant::now();
        println!("[GameRuntime] Hot-reload triggered!");

        // Capture current window size before unloading
        let current_size = self.window.as_ref().map(|w| w.inner_size());

        self.unload_app();

        // Small delay to ensure file is fully written
        std::thread::sleep(Duration::from_millis(50));

        self.load_app();

        let reload_time = start_time.elapsed();
        self.last_reload_time = Some(reload_time);

        // Send resize event to new app instance to sync window size
        if let (Some(app), Some(size)) = (self.app_instance.as_mut(), current_size) {
            app.on_event(&WindowEvent::Resized(size));
        }

        if self.app_instance.is_some() {
            println!(
                "[GameRuntime] Hot-reload successful in {:.2}ms",
                reload_time.as_secs_f64() * 1000.0
            );
        } else {
            eprintln!(
                "[GameRuntime] Hot-reload failed after {:.2}ms",
                reload_time.as_secs_f64() * 1000.0
            );
            eprintln!("[GameRuntime] Continuing with previous version (if any)");
        }
    }
}

impl ApplicationHandler for GameRuntime {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        println!("[GameRuntime] Initializing window and GL context");

        let window_attributes = Window::default_attributes()
            .with_title("Hot-Reload Demo - Rotating Cube")
            .with_inner_size(winit::dpi::LogicalSize::new(800, 600));

        let template = ConfigTemplateBuilder::new()
            .with_alpha_size(8)
            .with_transparency(false);

        let display_builder =
            DisplayBuilder::new().with_window_attributes(Some(window_attributes));

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

        // Request OpenGL 4.3 for compute shader support
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
            std::num::NonZeroU32::new(size.width).unwrap(),
            std::num::NonZeroU32::new(size.height).unwrap(),
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

        println!("[GameRuntime] OpenGL context created successfully");

        // Initialize egui integration
        let egui = unsafe { app::EguiIntegration::new(&window, Arc::clone(&gl)) };

        self.window = Some(window);
        self.gl_context = Some(gl_context);
        self.gl_surface = Some(gl_surface);
        self.gl = Some(gl);
        self.egui = Some(egui);
        self.start_time = Instant::now();

        // Load the app library
        unsafe {
            self.load_app();
            if self.app_instance.is_none() {
                eprintln!("[GameRuntime] Failed to load app, exiting...");
                event_loop.exit();
                return;
            }
        }

        self.last_update = Instant::now();
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        use winit::event::DeviceEvent;

        // Accumulate raw mouse motion for FPS camera
        if let DeviceEvent::MouseMotion { delta } = event {
            self.input_state.raw_mouse_delta.x += delta.0 as f32;
            self.input_state.raw_mouse_delta.y += delta.1 as f32;
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // Let egui handle events first
        if let (Some(window), Some(egui)) = (self.window.as_ref(), self.egui.as_mut()) {
            if egui.on_window_event(window, &event) {
                return;
            }
        }

        // Process input events into InputState
        match &event {
            WindowEvent::CursorMoved { position, .. } => {
                let new_pos = Vec2::new(position.x as f32, position.y as f32);
                if let Some(last_pos) = self.last_mouse_pos {
                    self.input_state.mouse_delta = new_pos - last_pos;
                }
                self.last_mouse_pos = Some(new_pos);
                self.input_state.mouse_pos = Some(new_pos);
            }
            WindowEvent::CursorLeft { .. } => {
                self.input_state.mouse_pos = None;
                self.last_mouse_pos = None;
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = *state == ElementState::Pressed;
                match button {
                    MouseButton::Left => self.input_state.mouse_buttons.left = pressed,
                    MouseButton::Right => self.input_state.mouse_buttons.right = pressed,
                    MouseButton::Middle => self.input_state.mouse_buttons.middle = pressed,
                    _ => {}
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => Vec2::new(*x, *y),
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        Vec2::new(pos.x as f32, pos.y as f32) / 10.0
                    }
                };
                self.input_state.scroll_delta += scroll;
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(keycode) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => {
                            self.input_state.keys.insert(keycode);
                        }
                        ElementState::Released => {
                            self.input_state.keys.remove(&keycode);
                        }
                    }
                }
            }
            _ => {}
        }

        // Let app handle remaining events
        if let Some(app) = &mut self.app_instance {
            app.on_event(&event);
        }

        match event {
            WindowEvent::CloseRequested => {
                println!("[GameRuntime] Close requested");
                unsafe {
                    self.unload_app();
                }
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let (Some(gl_surface), Some(gl_context)) =
                    (self.gl_surface.as_ref(), self.gl_context.as_ref())
                {
                    gl_surface.resize(
                        gl_context,
                        std::num::NonZeroU32::new(size.width).unwrap(),
                        std::num::NonZeroU32::new(size.height).unwrap(),
                    );
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                // Check for file changes
                let should_reload = {
                    let lib_path = get_lib_path();
                    let mut needs_reload = false;
                    if let Ok(rx) = self.file_events.lock() {
                        while let Ok(events_result) = rx.try_recv() {
                            if let Ok(events) = events_result {
                                for event in events {
                                    if event.path == lib_path {
                                        println!(
                                            "[FileWatcher] Detected change: {}",
                                            event.path.display()
                                        );
                                        needs_reload = true;
                                    }
                                }
                            }
                        }
                    }
                    needs_reload
                };

                if should_reload {
                    unsafe {
                        self.reload_app();
                    }
                }

                if let (
                    Some(window),
                    Some(gl),
                    Some(app),
                    Some(gl_context),
                    Some(gl_surface),
                ) = (
                    self.window.as_ref(),
                    self.gl.as_ref(),
                    self.app_instance.as_mut(),
                    self.gl_context.as_ref(),
                    self.gl_surface.as_ref(),
                ) {
                    // Calculate timing
                    let now = Instant::now();
                    let delta_time = (now - self.last_update).as_secs_f32();
                    self.last_update = now;
                    let elapsed = self.start_time.elapsed().as_secs_f32();
                    let size = window.inner_size();

                    // Poll controller backend
                    if let Some(backend) = &mut self.controller_backend {
                        backend.poll();
                        if let Some(controller) = backend.get_first_controller() {
                            self.input_state.gamepad = Some(controller.gamepad.clone());
                        } else {
                            self.input_state.gamepad = None;
                        }
                    }

                    // Build frame context
                    let ctx = FrameContext {
                        gl,
                        window,
                        delta_time,
                        elapsed,
                        frame: self.frame_count,
                        size: (size.width, size.height),
                    };

                    // Update app logic
                    app.update(&ctx, &self.input_state);

                    // Get cursor mode before we borrow app mutably for UI
                    let cursor_mode = app.cursor_mode();

                    // Render app
                    app.render(&ctx);

                    // Apply cursor mode
                    match cursor_mode {
                        CursorMode::Normal => {
                            window.set_cursor_visible(true);
                            let _ = window.set_cursor_grab(CursorGrabMode::None);
                        }
                        CursorMode::Hidden => {
                            window.set_cursor_visible(false);
                            let _ = window.set_cursor_grab(CursorGrabMode::None);
                        }
                        CursorMode::Grabbed => {
                            window.set_cursor_visible(false);
                            if window.set_cursor_grab(CursorGrabMode::Locked).is_err() {
                                let _ = window.set_cursor_grab(CursorGrabMode::Confined);
                            }
                        }
                    }

                    // Render egui UI
                    if let Some(egui) = &mut self.egui {
                        unsafe {
                            gl.disable(glow::DEPTH_TEST);
                            gl.enable(glow::BLEND);
                            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
                        }

                        egui.run(window, [size.width, size.height], |egui_ctx| {
                            app.ui(&ctx, egui_ctx);
                        });

                        unsafe {
                            gl.enable(glow::DEPTH_TEST);
                            gl.disable(glow::BLEND);
                        }
                    }

                    gl_surface.swap_buffers(gl_context).unwrap();
                    window.request_redraw();

                    self.frame_count += 1;
                    self.reset_frame_deltas();
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

impl Drop for GameRuntime {
    fn drop(&mut self) {
        unsafe {
            self.unload_app();
        }
    }
}

fn main() {
    println!("[GameRuntime] Starting hot-reload runtime");

    let lib_path = get_lib_path();
    println!(
        "[GameRuntime] Watching for changes: {}",
        lib_path.display()
    );

    // Setup file watcher for hot-reload
    let (tx, rx) = channel();
    let mut _debouncer =
        new_debouncer(Duration::from_millis(100), tx).expect("Failed to create file watcher");

    // Watch the parent directory (watching the file directly doesn't work well with atomic writes)
    let watch_dir = lib_path.parent().unwrap();
    _debouncer
        .watcher()
        .watch(watch_dir, RecursiveMode::NonRecursive)
        .expect("Failed to watch directory");

    println!("[GameRuntime] File watcher initialized");

    let file_events = Arc::new(Mutex::new(rx));

    #[cfg(target_os = "linux")]
    let event_loop = {
        let mut builder = EventLoop::builder();
        builder.with_x11();
        builder.build().expect("Failed to create event loop")
    };

    #[cfg(not(target_os = "linux"))]
    let event_loop = EventLoop::new().expect("Failed to create event loop");

    event_loop.set_control_flow(ControlFlow::Poll);

    let mut game_runtime = GameRuntime::new(file_events);

    event_loop
        .run_app(&mut game_runtime)
        .expect("Event loop error");
}
