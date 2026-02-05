//! System crate - Unified platform abstraction layer for Crossworld
//!
//! This crate provides the core system abstractions for building Crossworld
//! applications across different platforms (Native, Web/WASM).
//!
//! # Modules
//!
//! - [`platform`]: Platform detection and capabilities
//! - [`timer`]: High-resolution timing utilities
//! - [`path`]: Cross-platform path resolution
//! - [`window`]: Window handle abstraction
//! - [`app`]: Application trait and framework context
//! - [`input`]: Input state management (keyboard, mouse, gamepad)
//! - [`camera`]: 3D camera system with orbit and first-person controllers
//!
//! # Features
//!
//! - `runtime`: Enables the AppRuntime and EguiIntegration (requires glutin, egui)
//! - `gilrs`: Enables gamepad support via the gilrs backend
//!
//! # Example
//!
//! ```ignore
//! use system::{App, FrameContext, InputState, Camera};
//! use glam::Vec3;
//!
//! struct MyApp {
//!     camera: Camera,
//! }
//!
//! impl App for MyApp {
//!     fn init(&mut self, ctx: &FrameContext) {
//!         // Initialize OpenGL resources
//!     }
//!
//!     fn shutdown(&mut self, ctx: &FrameContext) {
//!         // Clean up OpenGL resources
//!     }
//!
//!     fn update(&mut self, ctx: &FrameContext, input: &InputState) {
//!         // Update game logic
//!     }
//!
//!     fn render(&mut self, ctx: &FrameContext) {
//!         // Render the frame
//!     }
//! }
//! ```

pub mod app;
pub mod camera;
pub mod input;
pub mod path;
pub mod platform;
pub mod timer;
pub mod window;

// Re-export commonly used types at crate root
pub use app::{App, CreateAppFn, FrameContext, CREATE_APP_SYMBOL};
pub use camera::{
    Camera, FirstPersonController, FirstPersonControllerConfig, OrbitController,
    OrbitControllerConfig, DEFAULT_VFOV,
};
pub use input::{
    ControllerInfo, ControllerInput, CursorMode, GamepadState, InputState, MouseButtonType,
    MouseButtons,
};
pub use path::{normalize_path, path_to_url, PathResolver};
pub use platform::{Platform, PlatformCapabilities};
pub use timer::{Duration, FrameTimer, Instant};
pub use window::{WindowConfig, WindowHandle};

// Re-export egui when runtime feature is enabled
#[cfg(feature = "runtime")]
pub use egui;
