//! Application runner for OpenGL applications
//!
//! Provides the window creation, OpenGL context setup, and event loop
//! management that is common across all applications.

use glow::Context;
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
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{CursorGrabMode, Window, WindowId};

#[cfg(target_os = "linux")]
use winit::platform::x11::EventLoopBuilderExtX11;

use crate::App;

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
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            title: "Application".to_string(),
            width: 800,
            height: 600,
            gl_major: 4,
            gl_minor: 3,
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
}

/// Runtime state for an application
pub struct AppRuntime<A: App> {
    config: AppConfig,
    app: A,
    window: Option<Window>,
    gl_context: Option<glutin::context::PossiblyCurrentContext>,
    gl_surface: Option<glutin::surface::Surface<WindowSurface>>,
    gl: Option<Arc<Context>>,
    last_update: Instant,
    initialized: bool,
}

impl<A: App> AppRuntime<A> {
    /// Create a new runtime with the given app and configuration
    pub fn new(app: A, config: AppConfig) -> Self {
        Self {
            config,
            app,
            window: None,
            gl_context: None,
            gl_surface: None,
            gl: None,
            last_update: Instant::now(),
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

        // Initialize the app
        if !self.initialized {
            unsafe {
                self.app.init(Arc::clone(&gl));
            }
            self.initialized = true;
            println!("[AppRuntime] App initialized");
        }

        self.window = Some(window);
        self.gl_context = Some(gl_context);
        self.gl_surface = Some(gl_surface);
        self.gl = Some(gl);
        self.last_update = Instant::now();
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        use winit::event::DeviceEvent;

        // Forward raw mouse motion to app
        if let DeviceEvent::MouseMotion { delta } = event {
            self.app.mouse_motion(delta);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // Forward events to app
        self.app.event(&event);

        match event {
            WindowEvent::CloseRequested => {
                println!("[AppRuntime] Close requested");
                if let Some(gl) = &self.gl {
                    unsafe {
                        self.app.uninit(Arc::clone(gl));
                    }
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
                    let now = Instant::now();
                    let delta_time = (now - self.last_update).as_secs_f32();
                    self.last_update = now;

                    // Update app logic
                    self.app.update(delta_time);

                    // Apply cursor state changes
                    if let Some((grab_mode, visible)) = self.app.cursor_state() {
                        window.set_cursor_visible(visible);
                        if window.set_cursor_grab(grab_mode).is_err() {
                            // Fall back to Confined if Locked isn't supported
                            if matches!(grab_mode, CursorGrabMode::Locked) {
                                let _ = window.set_cursor_grab(CursorGrabMode::Confined);
                            }
                        }
                    } else {
                        window.set_cursor_visible(true);
                        let _ = window.set_cursor_grab(CursorGrabMode::None);
                    }

                    // Render
                    unsafe {
                        self.app.render(Arc::clone(gl));
                    }

                    gl_surface.swap_buffers(gl_context).unwrap();
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
