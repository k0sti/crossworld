//! Application runner for OpenGL applications
//!
//! Provides the window creation, OpenGL context setup, and event loop
//! management that is common across all applications.

use glam::Vec2;
use glow::{Context, HasContext};
use glutin::config::ConfigTemplateBuilder;
use glutin::context::{ContextApi, ContextAttributesBuilder, Version};
use glutin::display::GetGlDisplay;
use glutin::prelude::*;
use glutin::surface::{SurfaceAttributesBuilder, WindowSurface};
use glutin_winit::DisplayBuilder;
use raw_window_handle::HasWindowHandle;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{CursorGrabMode, Window, WindowId};

#[cfg(target_os = "linux")]
use winit::platform::x11::EventLoopBuilderExtX11;

use crate::{
    create_controller_backend, App, ControllerBackend, CursorMode, FrameContext, InputState,
};

use super::EguiIntegration;

/// Debug mode configuration
#[derive(Debug, Clone, Copy)]
pub struct DebugMode {
    /// Number of frames to run before exiting
    pub frames: u64,
    /// Path to save the final frame screenshot
    pub output_path: &'static str,
}

impl DebugMode {
    /// Create a new debug mode configuration
    pub fn new(frames: u64) -> Self {
        Self {
            frames,
            output_path: "output/frame_last.png",
        }
    }
}

/// Review panel configuration
#[derive(Clone, Debug)]
pub struct ReviewConfig {
    /// Path to the review document file (None if created from text)
    pub file_path: Option<std::path::PathBuf>,
    /// Document content (loaded from file or provided directly, Arc for efficient cloning)
    pub content: std::sync::Arc<str>,
    /// User comment input buffer
    pub comment: String,
}

impl ReviewConfig {
    /// Create a new review configuration by loading a file
    pub fn from_file(file_path: std::path::PathBuf) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(&file_path)?;
        Ok(Self {
            file_path: Some(file_path),
            content: content.into(),
            comment: String::new(),
        })
    }

    /// Create a new review configuration from a text string
    pub fn from_text(content: impl Into<String>) -> Self {
        Self {
            file_path: None,
            content: content.into().into(),
            comment: String::new(),
        }
    }
}

/// Configuration for the application window
#[derive(Clone)]
pub struct AppConfig {
    /// Window title
    pub title: String,
    /// Initial window width
    pub width: u32,
    /// Initial window height
    pub height: u32,
    /// OpenGL major version
    pub gl_major: u8,
    /// OpenGL minor version
    pub gl_minor: u8,
    /// Optional debug mode configuration
    pub debug_mode: Option<DebugMode>,
    /// Optional note message to display as overlay (supports markdown)
    pub note: Option<String>,
    /// Optional review panel configuration
    pub review: Option<ReviewConfig>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            title: "Application".to_string(),
            width: 800,
            height: 600,
            gl_major: 4,
            gl_minor: 3,
            debug_mode: None,
            note: None,
            review: None,
        }
    }
}

impl AppConfig {
    /// Create a new configuration with the given title
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Default::default()
        }
    }

    /// Set the window size
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Set the OpenGL version
    pub fn with_gl_version(mut self, major: u8, minor: u8) -> Self {
        self.gl_major = major;
        self.gl_minor = minor;
        self
    }

    /// Enable debug mode with the specified number of frames
    pub fn with_debug_mode(mut self, frames: u64) -> Self {
        self.debug_mode = Some(DebugMode::new(frames));
        self
    }

    /// Set a note message to display as overlay (supports markdown)
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// Set a review panel from a text message
    pub fn with_review_text(mut self, content: impl Into<String>) -> Self {
        self.review = Some(ReviewConfig::from_text(content));
        self
    }

    /// Set a review panel from a file path
    pub fn with_review_file(mut self, file_path: std::path::PathBuf) -> std::io::Result<Self> {
        self.review = Some(ReviewConfig::from_file(file_path)?);
        Ok(self)
    }
}

/// Runtime state for an application
pub struct AppRuntime<A: App> {
    config: AppConfig,
    app: A,
    window: Option<Window>,
    gl_context: Option<glutin::context::PossiblyCurrentContext>,
    gl_surface: Option<glutin::surface::Surface<WindowSurface>>,
    gl: Option<Arc<Context>>,

    // Timing
    start_time: Instant,
    last_update: Instant,
    frame_count: u64,

    // Input state (accumulated between frames)
    input_state: InputState,
    last_mouse_pos: Option<Vec2>,

    // Egui integration
    egui: Option<EguiIntegration>,

    // Controller backend
    controller_backend: Option<Box<dyn ControllerBackend>>,

    initialized: bool,
}

impl<A: App> AppRuntime<A> {
    /// Create a new runtime with the given app and configuration
    pub fn new(app: A, config: AppConfig) -> Self {
        let controller_backend = create_controller_backend();

        Self {
            config,
            app,
            window: None,
            gl_context: None,
            gl_surface: None,
            gl: None,
            start_time: Instant::now(),
            last_update: Instant::now(),
            frame_count: 0,
            input_state: InputState::default(),
            last_mouse_pos: None,
            egui: None,
            controller_backend,
            initialized: false,
        }
    }

    /// Get a reference to the window
    pub fn window(&self) -> Option<&Window> {
        self.window.as_ref()
    }

    /// Get a reference to the GL context
    pub fn gl(&self) -> Option<&Arc<Context>> {
        self.gl.as_ref()
    }

    /// Get a mutable reference to the app
    pub fn app_mut(&mut self) -> &mut A {
        &mut self.app
    }

    /// Reset per-frame input deltas (called after processing)
    fn reset_frame_deltas(&mut self) {
        self.input_state.mouse_delta = Vec2::ZERO;
        self.input_state.raw_mouse_delta = Vec2::ZERO;
        self.input_state.scroll_delta = Vec2::ZERO;
    }

    /// Apply cursor mode from app
    fn apply_cursor_mode(&self, window: &Window) {
        let mode = self.app.cursor_mode();
        match mode {
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
                // Try Locked first, fall back to Confined
                if window.set_cursor_grab(CursorGrabMode::Locked).is_err() {
                    let _ = window.set_cursor_grab(CursorGrabMode::Confined);
                }
            }
        }
    }

    /// Capture the current framebuffer and save to file
    fn capture_frame(
        &self,
        gl: &Context,
        size: winit::dpi::PhysicalSize<u32>,
        output_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use image::{ImageBuffer, Rgba};

        let width = size.width as usize;
        let height = size.height as usize;
        let mut pixels = vec![0u8; width * height * 4];

        unsafe {
            gl.read_pixels(
                0,
                0,
                size.width as i32,
                size.height as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut pixels)),
            );
        }

        // Flip image vertically (OpenGL has origin at bottom-left)
        let mut flipped = vec![0u8; width * height * 4];
        for y in 0..height {
            let src_offset = y * width * 4;
            let dst_offset = (height - 1 - y) * width * 4;
            flipped[dst_offset..dst_offset + width * 4]
                .copy_from_slice(&pixels[src_offset..src_offset + width * 4]);
        }

        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_raw(width as u32, height as u32, flipped)
                .ok_or("Failed to create image buffer")?;

        img.save(output_path)?;
        Ok(())
    }
}

impl<A: App> ApplicationHandler for AppRuntime<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        println!("[AppRuntime] Initializing window and GL context");

        let window_attributes = Window::default_attributes()
            .with_title(&self.config.title)
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.config.width,
                self.config.height,
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
            .with_context_api(ContextApi::OpenGl(Some(Version::new(
                self.config.gl_major,
                self.config.gl_minor,
            ))))
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

        println!("[AppRuntime] OpenGL context created successfully");

        // Initialize egui integration
        let egui = unsafe { EguiIntegration::new(&window, Arc::clone(&gl)) };

        // Initialize the app
        if !self.initialized {
            let ctx = FrameContext {
                gl: &gl,
                window: &window,
                delta_time: 0.0,
                elapsed: 0.0,
                frame: 0,
                size: (size.width, size.height),
            };
            self.app.init(&ctx);
            self.initialized = true;
            println!("[AppRuntime] App initialized");
        }

        self.window = Some(window);
        self.gl_context = Some(gl_context);
        self.gl_surface = Some(gl_surface);
        self.gl = Some(gl);
        self.egui = Some(egui);
        self.start_time = Instant::now();
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
        // Always update mouse button state to keep it synchronized
        // This prevents the state from becoming stuck when egui consumes events
        if let WindowEvent::MouseInput { state, button, .. } = &event {
            let pressed = *state == ElementState::Pressed;
            match button {
                MouseButton::Left => self.input_state.mouse_buttons.left = pressed,
                MouseButton::Right => self.input_state.mouse_buttons.right = pressed,
                MouseButton::Middle => self.input_state.mouse_buttons.middle = pressed,
                _ => {}
            }
        }

        // Let egui handle events first
        if let (Some(window), Some(egui)) = (self.window.as_ref(), self.egui.as_mut()) {
            if egui.on_window_event(window, &event) {
                // Egui consumed the event
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
            WindowEvent::MouseInput { .. } => {
                // Mouse button state already handled above before egui
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
        self.app.on_event(&event);

        match event {
            WindowEvent::CloseRequested => {
                println!("[AppRuntime] Close requested");
                if let (Some(window), Some(gl)) = (self.window.as_ref(), self.gl.as_ref()) {
                    let size = window.inner_size();
                    let ctx = FrameContext {
                        gl,
                        window,
                        delta_time: 0.0,
                        elapsed: self.start_time.elapsed().as_secs_f32(),
                        frame: self.frame_count,
                        size: (size.width, size.height),
                    };
                    self.app.shutdown(&ctx);
                }
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
                if let (Some(window), Some(gl), Some(gl_context), Some(gl_surface)) = (
                    self.window.as_ref(),
                    self.gl.as_ref(),
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
                        // Get gamepad state from first controller
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
                    self.app.update(&ctx, &self.input_state);

                    // Check if app wants to exit
                    if self.app.should_exit() {
                        println!("[AppRuntime] App requested exit");
                        self.app.shutdown(&ctx);
                        event_loop.exit();
                        return;
                    }

                    // Apply cursor mode
                    self.apply_cursor_mode(window);

                    // Render app
                    self.app.render(&ctx);

                    // Render egui UI
                    if let Some(egui) = &mut self.egui {
                        // Setup GL state for egui
                        unsafe {
                            gl.disable(glow::DEPTH_TEST);
                            gl.enable(glow::BLEND);
                            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
                        }

                        // Get note for rendering in the closure
                        let note = self.config.note.clone();
                        let mut review = self.config.review.clone();
                        let mut review_action = super::review_overlay::ReviewAction::None;

                        egui.run(window, [size.width, size.height], |egui_ctx| {
                            self.app.ui(&ctx, egui_ctx);

                            // Render note overlay if configured
                            if let Some(ref note_text) = note {
                                super::note_overlay::render_note_overlay(egui_ctx, note_text);
                            }

                            // Render review overlay if configured
                            if let Some(ref mut review_config) = review {
                                review_action = super::review_overlay::render_review_overlay(
                                    egui_ctx,
                                    review_config,
                                );
                            }
                        });

                        // Update review config from closure
                        if let Some(review_config) = review {
                            self.config.review = Some(review_config);
                        }

                        // Handle review action - output commands to stdout
                        match review_action {
                            super::review_overlay::ReviewAction::None => {
                                // Keep running
                            }
                            super::review_overlay::ReviewAction::Approve => {
                                println!("APPROVE");
                                self.app.shutdown(&ctx);
                                event_loop.exit();
                                return;
                            }
                            super::review_overlay::ReviewAction::ContinueWithFeedback(msg) => {
                                println!("CONTINUE: {}", msg);
                                self.app.shutdown(&ctx);
                                event_loop.exit();
                                return;
                            }
                            super::review_overlay::ReviewAction::Spawn(title) => {
                                println!("SPAWN: {}", title);
                                self.app.shutdown(&ctx);
                                event_loop.exit();
                                return;
                            }
                            super::review_overlay::ReviewAction::Discard => {
                                println!("DISCARD");
                                self.app.shutdown(&ctx);
                                event_loop.exit();
                                return;
                            }
                            super::review_overlay::ReviewAction::Rebase => {
                                println!("REBASE");
                                self.app.shutdown(&ctx);
                                event_loop.exit();
                                return;
                            }
                            super::review_overlay::ReviewAction::Merge => {
                                println!("MERGE");
                                self.app.shutdown(&ctx);
                                event_loop.exit();
                                return;
                            }
                            super::review_overlay::ReviewAction::Complete => {
                                // Complete = REBASE + MERGE + APPROVE (merge first, then mark done)
                                println!("REBASE");
                                println!("MERGE");
                                println!("APPROVE");
                                self.app.shutdown(&ctx);
                                event_loop.exit();
                                return;
                            }
                            super::review_overlay::ReviewAction::Cancel => {
                                // Exit without printing any command
                                self.app.shutdown(&ctx);
                                event_loop.exit();
                                return;
                            }
                        }

                        // Restore GL state
                        unsafe {
                            gl.enable(glow::DEPTH_TEST);
                            gl.disable(glow::BLEND);
                        }
                    }

                    gl_surface.swap_buffers(gl_context).unwrap();

                    self.frame_count += 1;

                    // Check debug mode exit condition
                    if let Some(debug_mode) = self.config.debug_mode {
                        if self.frame_count >= debug_mode.frames {
                            println!(
                                "[DEBUG] Frame {}/{} - Capturing screenshot and exiting",
                                self.frame_count, debug_mode.frames
                            );

                            // Create output directory if it doesn't exist
                            if let Some(parent) =
                                std::path::Path::new(debug_mode.output_path).parent()
                            {
                                let _ = std::fs::create_dir_all(parent);
                            }

                            // Capture framebuffer and save
                            if let Err(e) = self.capture_frame(gl, size, debug_mode.output_path) {
                                eprintln!("[DEBUG] Failed to capture frame: {}", e);
                            } else {
                                println!("[DEBUG] Screenshot saved to: {}", debug_mode.output_path);
                            }

                            // Shutdown and exit
                            let ctx = FrameContext {
                                gl,
                                window,
                                delta_time,
                                elapsed,
                                frame: self.frame_count,
                                size: (size.width, size.height),
                            };
                            self.app.shutdown(&ctx);
                            event_loop.exit();
                            return;
                        } else {
                            println!("[DEBUG] Frame {}/{}", self.frame_count, debug_mode.frames);
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

/// Run an application with the given configuration
///
/// This creates the event loop, window, GL context, and runs the app.
pub fn run_app<A: App + 'static>(app: A, config: AppConfig) {
    #[cfg(target_os = "linux")]
    let event_loop = {
        let mut builder = EventLoop::builder();
        builder.with_x11();
        builder.build().expect("Failed to create event loop")
    };

    #[cfg(not(target_os = "linux"))]
    let event_loop = EventLoop::new().expect("Failed to create event loop");

    event_loop.set_control_flow(ControlFlow::Poll);

    let mut runtime = AppRuntime::new(app, config);

    event_loop.run_app(&mut runtime).expect("Event loop error");
}

/// Create an event loop (useful for custom runtime implementations)
pub fn create_event_loop() -> EventLoop<()> {
    #[cfg(target_os = "linux")]
    {
        let mut builder = EventLoop::builder();
        builder.with_x11();
        builder.build().expect("Failed to create event loop")
    }

    #[cfg(not(target_os = "linux"))]
    {
        EventLoop::new().expect("Failed to create event loop")
    }
}
