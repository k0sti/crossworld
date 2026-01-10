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

use glam::Vec2;
use glow::Context;
use std::collections::HashSet;
use winit::event::WindowEvent;
use winit::keyboard::KeyCode;
use winit::window::Window;

pub mod camera;
pub mod cli;
pub mod controller;

// Lua configuration module
pub mod lua_config;

// Re-export input types from core
pub use core::input::{
    ControllerInfo, ControllerInput, CursorMode, GamepadState, MouseButtonType, MouseButtons,
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

#[cfg(feature = "runtime")]
pub use egui_integration::EguiIntegration;
#[cfg(feature = "runtime")]
pub use note_overlay::render_note_overlay;
#[cfg(feature = "runtime")]
pub use review_overlay::{render_review_overlay, ReviewAction};
#[cfg(feature = "runtime")]
pub use runner::{create_event_loop, run_app, AppConfig, AppRuntime, DebugMode, ReviewConfig};

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

    /// Inject a mouse position event
    ///
    /// Used for automated testing and config-driven input.
    pub fn inject_mouse_pos(&mut self, x: f32, y: f32) {
        let new_pos = Vec2::new(x, y);
        if let Some(old_pos) = self.mouse_pos {
            self.mouse_delta = new_pos - old_pos;
        }
        self.mouse_pos = Some(new_pos);
    }

    /// Inject a mouse click event
    ///
    /// Used for automated testing and config-driven input.
    pub fn inject_mouse_click(&mut self, button: MouseButtonType, pressed: bool) {
        match button {
            MouseButtonType::Left => self.mouse_buttons.left = pressed,
            MouseButtonType::Right => self.mouse_buttons.right = pressed,
            MouseButtonType::Middle => self.mouse_buttons.middle = pressed,
        }
    }

    /// Inject a key press event
    ///
    /// Used for automated testing and config-driven input.
    pub fn inject_key(&mut self, key: KeyCode, pressed: bool) {
        if pressed {
            self.keys.insert(key);
        } else {
            self.keys.remove(&key);
        }
    }

    /// Clear all input state
    pub fn clear(&mut self) {
        self.keys.clear();
        self.mouse_pos = None;
        self.mouse_delta = Vec2::ZERO;
        self.raw_mouse_delta = Vec2::ZERO;
        self.scroll_delta = Vec2::ZERO;
        self.mouse_buttons = MouseButtons::default();
        self.gamepad = None;
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
