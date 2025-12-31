use glow::Context;
use std::sync::Arc;
use winit::event::WindowEvent;

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
}

/// Function signature for creating a new App instance from the dynamic library
pub type CreateAppFn = unsafe extern "C" fn() -> *mut dyn App;

/// Export symbol name for the create_app function
pub const CREATE_APP_SYMBOL: &[u8] = b"create_app";
