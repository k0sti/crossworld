//! Hot-reload runtime for the game library
//!
//! This binary watches the game library for changes and hot-reloads it.
//!
//! Usage:
//!   cargo run -p game                              # Normal hot-reload mode
//!   cargo run -p game -- --debug N                 # Run N frames with debug output
//!   cargo run -p game -- --review "Message"        # Run with review panel (inline message)
//!   cargo run -p game -- --review-file PATH        # Run with review panel (from file)
//!
//! In debug mode, additional logging is output for world/cube state verification.

#![allow(clippy::type_complexity)]

use app::{
    create_controller_backend, App, ControllerBackend, CursorMode, FrameContext, InputState,
    CREATE_APP_SYMBOL,
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

/// Debug mode configuration
#[derive(Debug, Clone)]
struct DebugConfig {
    /// Number of frames to run before exiting
    frames: u64,
}

/// Review panel configuration
#[derive(Debug, Clone)]
struct ReviewConfig {
    /// Path to the review document (None if created from text)
    #[allow(dead_code)]
    path: Option<PathBuf>,
    /// Content of the review document
    content: String,
    /// User comment input buffer
    comment: String,
}

/// Command line configuration
#[derive(Debug, Clone, Default)]
struct CliConfig {
    debug: Option<DebugConfig>,
    review: Option<ReviewConfig>,
}

fn parse_args() -> CliConfig {
    let args: Vec<String> = std::env::args().collect();
    let mut config = CliConfig::default();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--debug" => {
                if i + 1 < args.len() {
                    match args[i + 1].parse::<u64>() {
                        Ok(n) => {
                            config.debug = Some(DebugConfig { frames: n });
                            println!("[Game] Debug mode: running {} frames", n);
                        }
                        Err(_) => {
                            eprintln!("Error: --debug requires a number of frames");
                            std::process::exit(1);
                        }
                    }
                    i += 1;
                } else {
                    eprintln!("Error: --debug requires a number of frames");
                    std::process::exit(1);
                }
            }
            "--review" | "-r" => {
                if i + 1 < args.len() {
                    let content = args[i + 1].clone();
                    config.review = Some(ReviewConfig {
                        path: None,
                        content,
                        comment: String::new(),
                    });
                    println!("[Game] Review message: {}", &args[i + 1]);
                    i += 1;
                } else {
                    eprintln!("Error: --review requires a message");
                    std::process::exit(1);
                }
            }
            "--review-file" => {
                if i + 1 < args.len() {
                    let path = PathBuf::from(&args[i + 1]);
                    match std::fs::read_to_string(&path) {
                        Ok(content) => {
                            config.review = Some(ReviewConfig {
                                path: Some(path.clone()),
                                content,
                                comment: String::new(),
                            });
                            println!("[Game] Review document: {}", path.display());
                        }
                        Err(e) => {
                            eprintln!("Error: Failed to read review document: {}", e);
                            std::process::exit(1);
                        }
                    }
                    i += 1;
                } else {
                    eprintln!("Error: --review-file requires a file path");
                    std::process::exit(1);
                }
            }
            "--help" | "-h" => {
                println!("Usage: game [OPTIONS]");
                println!();
                println!("Options:");
                println!("  --debug N           Run only N frames with debug output");
                println!("  --review MESSAGE    Display a review panel with markdown message");
                println!("  --review-file PATH  Display a review panel with markdown document from file");
                println!("  --help              Show this help message");
                std::process::exit(0);
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                eprintln!("Use --help for usage information");
                std::process::exit(1);
            }
        }
        i += 1;
    }

    config
}

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
    last_lib_modified: Option<std::time::SystemTime>,

    // CLI configuration
    cli_config: CliConfig,
}

impl GameRuntime {
    fn new(
        file_events: Arc<
            Mutex<
                Receiver<
                    std::result::Result<Vec<DebouncedEvent>, notify_debouncer_mini::notify::Error>,
                >,
            >,
        >,
        cli_config: CliConfig,
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
            last_lib_modified: None,
            cli_config,
        }
    }

    /// Check if debug mode is enabled
    fn is_debug_mode(&self) -> bool {
        self.cli_config.debug.is_some()
    }

    /// Check if we should exit due to debug frame limit
    fn should_exit_debug(&self) -> bool {
        if let Some(ref debug) = self.cli_config.debug {
            self.frame_count >= debug.frames
        } else {
            false
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

        // Track modification time to prevent spurious reloads
        if let Ok(metadata) = std::fs::metadata(&lib_path) {
            if let Ok(modified) = metadata.modified() {
                self.last_lib_modified = Some(modified);
            }
        }

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

        if let (Some(mut app), Some(window), Some(gl)) = (
            self.app_instance.take(),
            self.window.as_ref(),
            self.gl.as_ref(),
        ) {
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

        // Drop old egui before unloading app to clean up GL resources properly
        self.egui = None;

        self.unload_app();

        // Small delay to ensure file is fully written
        std::thread::sleep(Duration::from_millis(50));

        self.load_app();

        // Reinitialize egui after reloading app with fresh GL state
        if let (Some(window), Some(gl)) = (self.window.as_ref(), self.gl.as_ref()) {
            self.egui = Some(app::EguiIntegration::new(window, Arc::clone(gl)));
        }

        let reload_time = start_time.elapsed();
        self.last_reload_time = Some(reload_time);

        // Clear any pending file events to prevent immediate reload cycles
        if let Ok(rx) = self.file_events.lock() {
            while rx.try_recv().is_ok() {}
        }

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
                                        // Only reload if modification time actually changed
                                        if let Ok(metadata) = std::fs::metadata(&lib_path) {
                                            if let Ok(modified) = metadata.modified() {
                                                let has_changed = self
                                                    .last_lib_modified
                                                    .map(|last| modified > last)
                                                    .unwrap_or(true);

                                                if has_changed {
                                                    println!(
                                                        "[FileWatcher] Detected change: {}",
                                                        event.path.display()
                                                    );
                                                    self.last_lib_modified = Some(modified);
                                                    needs_reload = true;
                                                }
                                            }
                                        }
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

                if let (Some(window), Some(gl), Some(app), Some(gl_context), Some(gl_surface)) = (
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
                    let mut should_exit = false;
                    let mut exit_comment: Option<String> = None;

                    if let Some(egui) = &mut self.egui {
                        unsafe {
                            gl.disable(glow::DEPTH_TEST);
                            gl.enable(glow::BLEND);
                            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
                        }

                        let review_config = &mut self.cli_config.review;

                        egui.run(window, [size.width, size.height], |egui_ctx| {
                            app.ui(&ctx, egui_ctx);

                            // Render review overlay if configured
                            if let Some(ref mut review) = review_config {
                                use egui::{
                                    Align2, Area, Color32, Frame, Margin, RichText, ScrollArea,
                                    TextEdit,
                                };

                                Area::new(egui::Id::new("review_panel"))
                                    .anchor(Align2::RIGHT_TOP, [-10.0, 10.0])
                                    .show(egui_ctx, |ui| {
                                        Frame::popup(ui.style())
                                            .fill(Color32::from_rgba_unmultiplied(20, 20, 30, 240))
                                            .inner_margin(Margin::same(16))
                                            .show(ui, |ui| {
                                                ui.set_max_width(400.0);
                                                ui.set_max_height(500.0);

                                                ui.heading(
                                                    RichText::new("Review").color(Color32::WHITE),
                                                );
                                                ui.separator();

                                                ScrollArea::vertical().max_height(300.0).show(
                                                    ui,
                                                    |ui| {
                                                        ui.label(
                                                            RichText::new(&*review.content)
                                                                .color(Color32::LIGHT_GRAY),
                                                        );
                                                    },
                                                );

                                                ui.separator();
                                                ui.label(
                                                    RichText::new("Response:")
                                                        .color(Color32::WHITE),
                                                );
                                                ui.add(
                                                    TextEdit::multiline(&mut review.comment)
                                                        .desired_width(380.0)
                                                        .desired_rows(4)
                                                        .hint_text("Enter your response..."),
                                                );

                                                ui.horizontal(|ui| {
                                                    if ui.button("Submit").clicked() {
                                                        should_exit = true;
                                                        exit_comment = Some(review.comment.clone());
                                                    }
                                                    if ui.button("Cancel").clicked() {
                                                        should_exit = true;
                                                    }
                                                });
                                            });
                                    });
                            }
                        });

                        unsafe {
                            gl.enable(glow::DEPTH_TEST);
                            gl.disable(glow::BLEND);
                        }
                    }

                    gl_surface.swap_buffers(gl_context).unwrap();

                    self.frame_count += 1;

                    // Check debug mode exit condition
                    if self.should_exit_debug() {
                        if let Some(ref debug) = self.cli_config.debug {
                            println!(
                                "[DEBUG] Frame {}/{} - Exiting debug mode",
                                self.frame_count, debug.frames
                            );
                        }
                        unsafe {
                            self.unload_app();
                        }
                        event_loop.exit();
                        return;
                    }

                    // Check review panel exit
                    if should_exit {
                        if let Some(comment) = exit_comment {
                            println!("{}", comment);
                        }
                        unsafe {
                            self.unload_app();
                        }
                        event_loop.exit();
                        return;
                    }

                    // Debug frame logging
                    if self.is_debug_mode() {
                        if let Some(ref debug) = self.cli_config.debug {
                            println!("[DEBUG] Frame {}/{}", self.frame_count, debug.frames);
                        }
                    }

                    window.request_redraw();
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
    // Parse command line arguments
    let cli_config = parse_args();

    // Set environment variable for debug mode so the library can detect it
    if cli_config.debug.is_some() {
        std::env::set_var("GAME_DEBUG", "1");
    }

    println!("[GameRuntime] Starting hot-reload runtime");

    let lib_path = get_lib_path();
    println!("[GameRuntime] Watching for changes: {}", lib_path.display());

    // Setup file watcher for hot-reload
    let (tx, rx) = channel();
    let mut _debouncer =
        new_debouncer(Duration::from_millis(500), tx).expect("Failed to create file watcher");

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

    let mut game_runtime = GameRuntime::new(file_events, cli_config);

    event_loop
        .run_app(&mut game_runtime)
        .expect("Event loop error");
}
