//! Application framework - the App trait and per-frame context.
//!
//! This is the canonical home of the application lifecycle abstractions used
//! across Crossworld native applications:
//!
//! - [`App`] trait: the main interface for application logic
//! - [`FrameContext`]: per-frame context with GL, window, timing info
//!
//! With the `runtime` feature enabled, the runner ([`crate::runner`]) drives
//! these via window creation and the event loop.

use crate::input::{CursorMode, InputState};
use glow::Context;
use winit::event::WindowEvent;
use winit::window::Window;

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

#[cfg(test)]
mod tests {
    #[test]
    fn frame_context_aspect_ratio() {
        let width = 1920u32;
        let height = 1080u32;
        let aspect = width as f32 / height as f32;
        assert!((aspect - 1.778).abs() < 0.01);
    }
}
