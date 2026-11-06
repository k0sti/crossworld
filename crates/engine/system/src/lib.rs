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
//! - [`app`]: Application trait and per-frame context
//! - [`runner`]: Window creation and event loop (with the `runtime` feature)
//! - [`input`]: Input state and controller types (re-exported from `core`)
//! - [`camera`]: 3D camera system (re-exported from `core`)
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
pub mod cli;
pub mod controller;
pub mod input;
pub mod path;
pub mod platform;
pub mod timer;
pub mod window;

// Application framework types (canonical home — formerly the `app` crate)
pub use app::{App, CreateAppFn, FrameContext, CREATE_APP_SYMBOL};
pub use controller::{create_controller_backend, ControllerBackend};
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

#[cfg(feature = "gilrs")]
pub use controller::GilrsBackend;

// Runtime modules (require the `runtime` feature: window, event loop, egui)
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

// Re-export egui when runtime feature is enabled
#[cfg(feature = "runtime")]
pub use egui;
