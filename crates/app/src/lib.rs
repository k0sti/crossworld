//! Application framework for Crossworld native applications
//!
//! This crate provides the core abstractions for building native applications:
//!
//! - [`App`] trait: The main interface for application logic
//! - [`FrameContext`]: Per-frame context with GL, window, timing info
//! - [`InputState`]: Unified input state snapshot
//! - [`ControllerBackend`] trait: Gamepad input handling
//!
//! With the `runtime` feature enabled, additional utilities are available:
//! - [`AppRuntime`]: Window creation and event loop management
//! - Integrated egui UI rendering via the `ui()` method

use glam::Vec2;
use glow::Context;
use std::collections::HashSet;
use winit::event::WindowEvent;
use winit::keyboard::KeyCode;
use winit::window::Window;

pub mod controller;

pub use controller::{
    create_controller_backend, ControllerBackend, ControllerInfo, ControllerInput, GamepadState,
};

#[cfg(feature = "gilrs")]
pub use controller::GilrsBackend;

// Runtime module (requires runtime feature)
#[cfg(feature = "runtime")]
mod egui_integration;
#[cfg(feature = "runtime")]
mod runner;

#[cfg(feature = "runtime")]
pub use egui_integration::EguiIntegration;
#[cfg(feature = "runtime")]
pub use runner::{create_event_loop, run_app, AppConfig, AppRuntime};

// Re-export egui when runtime feature is enabled
#[cfg(feature = "runtime")]
pub use egui;

/// Mouse button state flags
#[derive(Debug, Clone, Copy, Default)]
pub struct MouseButtons {
    pub left: bool,
    pub right: bool,
    pub middle: bool,
}

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

/// Input state snapshot for the current frame
///
/// Contains all input information aggregated from events.
#[derive(Debug, Clone, Default)]
pub struct InputState {
    /// Currently pressed keys
    pub keys: HashSet<KeyCode>,
    /// Mouse position in window coordinates
    pub mouse_pos: Option<Vec2>,
    /// Mouse delta since last frame (from window events)
    pub mouse_delta: Vec2,
    /// Raw mouse motion (for FPS camera, not constrained by window)
    pub raw_mouse_delta: Vec2,
    /// Scroll delta
    pub scroll_delta: Vec2,
    /// Mouse buttons currently held
    pub mouse_buttons: MouseButtons,
    /// Gamepad state (if connected)
    pub gamepad: Option<GamepadState>,
}

impl InputState {
    /// Check if a key is currently pressed
    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.keys.contains(&key)
    }

    /// Check if left mouse button is pressed
    pub fn is_left_mouse_pressed(&self) -> bool {
        self.mouse_buttons.left
    }

    /// Check if right mouse button is pressed
    pub fn is_right_mouse_pressed(&self) -> bool {
        self.mouse_buttons.right
    }
}

/// Cursor mode for the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorMode {
    /// Visible, free movement (default)
    #[default]
    Normal,
    /// Hidden but not grabbed
    Hidden,
    /// Hidden and confined/locked (for FPS camera)
    Grabbed,
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

    /// Request cursor mode (optional)
    ///
    /// Called each frame to check cursor behavior.
    fn cursor_mode(&self) -> CursorMode {
        CursorMode::Normal
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
