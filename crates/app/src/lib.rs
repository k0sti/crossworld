//! Application framework for Crossworld native applications
//!
//! This crate provides the core abstractions for building native applications:
//!
//! - [`App`] trait: The main interface for application logic
//! - [`ControllerBackend`] trait: Gamepad input handling
//!
//! With the `runtime` feature enabled, additional utilities are available:
//! - [`AppRunner`]: Window creation and event loop management
//! - [`EguiIntegration`]: Egui UI rendering integration

use glow::Context;
use std::sync::Arc;
use winit::event::WindowEvent;
use winit::window::CursorGrabMode;

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

/// Application trait for hot-reloadable game code
///
/// This trait defines the lifecycle hooks that game code must implement.
/// The runtime will call these methods at appropriate times during the
/// application lifecycle and hot-reload process.
pub trait App {
    /// Initialize the application
    ///
    /// Called once when the game library is first loaded, and again after
    /// each hot-reload. Use this to create OpenGL resources (buffers, shaders,
    /// textures) and initialize game state.
    ///
    /// # Safety
    /// The GL context must be current when this is called.
    unsafe fn init(&mut self, gl: Arc<Context>);

    /// Uninitialize the application
    ///
    /// Called before unloading the game library during hot-reload. Use this
    /// to clean up OpenGL resources and prevent leaks.
    ///
    /// # Safety
    /// The GL context must be current when this is called.
    unsafe fn uninit(&mut self, gl: Arc<Context>);

    /// Handle window events
    ///
    /// Called for each window event (resize, keyboard, mouse, etc.).
    fn event(&mut self, event: &WindowEvent);

    /// Handle raw mouse motion (for infinite mouse movement)
    ///
    /// Called when raw mouse motion is detected. This provides infinite
    /// mouse movement not constrained by window boundaries.
    ///
    /// # Arguments
    /// * `delta` - Raw mouse delta (x, y) in pixels
    fn mouse_motion(&mut self, _delta: (f64, f64)) {
        // Default: do nothing
    }

    /// Update game logic
    ///
    /// Called each frame before rendering. Use this for game logic, physics,
    /// animation updates, etc.
    ///
    /// # Arguments
    /// * `delta_time` - Time elapsed since last update in seconds
    fn update(&mut self, delta_time: f32);

    /// Render the current frame
    ///
    /// Called each frame after update. Use this to issue OpenGL draw calls.
    ///
    /// # Safety
    /// The GL context must be current when this is called.
    unsafe fn render(&mut self, gl: Arc<Context>);

    /// Get the desired cursor grab mode
    ///
    /// Called each frame to check if the cursor should be grabbed/hidden.
    /// Return None to use default behavior (cursor visible and free).
    /// Return Some((grab_mode, visible)) to control cursor behavior.
    fn cursor_state(&self) -> Option<(CursorGrabMode, bool)> {
        None
    }
}

/// Function signature for creating a new App instance from the dynamic library
pub type CreateAppFn = unsafe extern "C" fn() -> *mut dyn App;

/// Export symbol name for the create_app function
pub const CREATE_APP_SYMBOL: &[u8] = b"create_app";
