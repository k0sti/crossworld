//! Application framework for Crossworld native applications
//!
//! This crate provides the core abstractions for building native applications:
//!
//! - [`App`] trait: The main interface for application logic
//! - [`FrameContext`]: Per-frame context with GL, window, timing info
//! - [`InputState`]: Unified input state snapshot
//! - [`ControllerBackend`] trait: Gamepad input handling
//! - [`camera`]: Generic 3D camera system with orbit and first-person modes
//!
//! With the `runtime` feature enabled, additional utilities are available:
//! - [`AppRuntime`]: Window creation and event loop management
//! - Integrated egui UI rendering via the `ui()` method

use glow::Context;
use winit::event::WindowEvent;
use winit::window::Window;

pub mod camera;
pub mod cli;
pub mod controller;

// Re-export input types from core
pub use core::input::{
    ControllerInfo, ControllerInput, CursorMode, GamepadState, InputState, MouseButtonType,
    MouseButtons,
};

pub use controller::{create_controller_backend, ControllerBackend};

// Re-export camera types at crate root for convenience
pub use camera::{
    Camera, FirstPersonController, FirstPersonControllerConfig, OrbitController,
    OrbitControllerConfig, DEFAULT_VFOV,
};

#[cfg(feature = "gilrs")]
pub use controller::GilrsBackend;

// Runtime module (requires runtime feature)
#[cfg(feature = "runtime")]
mod egui_integration;
#[cfg(feature = "runtime")]
mod note_overlay;
#[cfg(feature = "runtime")]
mod review_overlay;
#[cfg(feature = "runtime")]
mod runner;
// TODO: Fix compilation errors in hot_reload module
// #[cfg(feature = "runtime")]
// pub mod hot_reload;

#[cfg(feature = "runtime")]
pub use egui_integration::EguiIntegration;
#[cfg(feature = "runtime")]
pub use note_overlay::render_note_overlay;
#[cfg(feature = "runtime")]
pub use review_overlay::{render_review_overlay, ReviewAction};
#[cfg(feature = "runtime")]
pub use runner::{create_event_loop, run_app, AppConfig, AppRuntime, DebugMode, ReviewConfig};
// #[cfg(feature = "runtime")]
// pub use hot_reload::{HotReloadConfig, HotReloadLibrary};

// Re-export egui when runtime feature is enabled
#[cfg(feature = "runtime")]
pub use egui;

/// Frame context passed to update/render methods
///
/// Contains all per-frame information apps need without storing it themselves.
pub struct FrameContext<'a> {
    /// OpenGL context
    pub gl: &'a Context,
    /// Window reference (for DPI, size, etc.)
    pub window: &'a Window,
    /// Time since last frame in seconds
    pub delta_time: f32,
    /// Total elapsed time since app start in seconds
    pub elapsed: f32,
    /// Current frame number
    pub frame: u64,
    /// Window size in pixels (width, height)
    pub size: (u32, u32),
}

impl<'a> FrameContext<'a> {
    /// Create a new frame context
    pub fn new(
        gl: &'a Context,
        window: &'a Window,
        delta_time: f32,
        elapsed: f32,
        frame: u64,
        size: (u32, u32),
    ) -> Self {
        Self {
            gl,
            window,
            delta_time,
            elapsed,
            frame,
            size,
        }
    }

    /// Get the aspect ratio of the window
    #[inline]
    pub fn aspect_ratio(&self) -> f32 {
        if self.size.1 > 0 {
            self.size.0 as f32 / self.size.1 as f32
        } else {
            1.0
        }
    }

    /// Get the window width
    #[inline]
    pub fn width(&self) -> u32 {
        self.size.0
    }

    /// Get the window height
    #[inline]
    pub fn height(&self) -> u32 {
        self.size.1
    }

    /// Get the current frames per second based on delta_time
    #[inline]
    pub fn fps(&self) -> f32 {
        if self.delta_time > 0.0 {
            1.0 / self.delta_time
        } else {
            0.0
        }
    }
}

/// Application trait for hot-reloadable game code
///
/// This trait defines the lifecycle hooks that game code must implement.
/// The runtime will call these methods at appropriate times during the
/// application lifecycle and hot-reload process.
///
/// # Example
///
/// ```ignore
/// struct MyApp {
///     mesh_renderer: MeshRenderer,
/// }
///
/// impl App for MyApp {
///     fn init(&mut self, ctx: &FrameContext) {
///         self.mesh_renderer.init_gl(ctx.gl).unwrap();
///     }
///
///     fn update(&mut self, ctx: &FrameContext, input: &InputState) {
///         if input.is_key_pressed(KeyCode::KeyW) {
///             // Move forward
///         }
///     }
///
///     fn render(&mut self, ctx: &FrameContext) {
///         self.mesh_renderer.render(ctx.gl, ...);
///     }
///
///     fn ui(&mut self, ctx: &FrameContext, egui: &egui::Context) {
///         egui::Window::new("Debug").show(egui, |ui| {
///             ui.label(format!("FPS: {:.0}", 1.0 / ctx.delta_time));
///         });
///     }
/// }
/// ```
pub trait App {
    /// Initialize the application
    ///
    /// Called once when the game library is first loaded, and again after
    /// each hot-reload. Use this to create OpenGL resources (buffers, shaders,
    /// textures) and initialize game state.
    fn init(&mut self, ctx: &FrameContext);

    /// Cleanup before destruction
    ///
    /// Called before unloading the game library during hot-reload. Use this
    /// to clean up OpenGL resources and prevent leaks.
    fn shutdown(&mut self, ctx: &FrameContext);

    /// Handle a window event (optional)
    ///
    /// Called for each window event not handled by the framework.
    /// Return true to consume the event (prevent further processing).
    fn on_event(&mut self, _event: &WindowEvent) -> bool {
        false
    }

    /// Update game logic
    ///
    /// Called each frame before rendering. Use this for game logic, physics,
    /// animation updates, etc.
    fn update(&mut self, ctx: &FrameContext, input: &InputState);

    /// Render the frame
    ///
    /// Called each frame after update. Use this to issue OpenGL draw calls.
    fn render(&mut self, ctx: &FrameContext);

    /// Render UI (optional)
    ///
    /// Called after render with the egui context. Use this for UI rendering.
    /// The runtime handles all egui setup, input, and rendering.
    #[cfg(feature = "runtime")]
    fn ui(&mut self, _ctx: &FrameContext, _egui: &egui::Context) {
        // Default: no UI
    }

    /// Post-render callback (optional)
    ///
    /// Called after all rendering is complete (including UI).
    /// Use this for post-processing effects that should apply to the entire frame.
    fn post_render(&mut self, _ctx: &FrameContext) {
        // Default: no post-render
    }

    /// Request cursor mode (optional)
    ///
    /// Called each frame to check cursor behavior.
    fn cursor_mode(&self) -> CursorMode {
        CursorMode::Normal
    }

    /// Request to exit the application (optional)
    ///
    /// Called each frame after update. Return true to request application exit.
    /// This is useful for automated testing or scripted runs.
    fn should_exit(&self) -> bool {
        false
    }
}

/// Function signature for creating a new App instance from the dynamic library
///
/// Note: This uses `dyn App` which isn't strictly FFI-safe, but this is only used
/// for hot-reload between Rust code, not for interop with other languages.
#[allow(improper_ctypes_definitions)]
pub type CreateAppFn = unsafe extern "C" fn() -> *mut dyn App;

/// Export symbol name for the create_app function
pub const CREATE_APP_SYMBOL: &[u8] = b"create_app";
