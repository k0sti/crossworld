//! Input device abstractions for Crossworld
//!
//! This crate provides platform-agnostic abstractions for input devices
//! and can be compiled both natively and to WebAssembly.
//!
//! # Modules
//!
//! - [`mouse`]: Mouse and cursor input types
//! - [`gamepad`]: Gamepad/controller state and input processing
//! - [`keyboard`]: Keyboard input abstraction
//! - [`touch`]: Touch input support (placeholder for mobile)
//! - [`sensors`]: Sensor abstractions (accelerometer, compass - placeholder)
//! - [`backend`]: Controller backend trait for platform-specific implementations
//!
//! # Feature Flags
//!
//! - `gilrs`: Enable gilrs-based gamepad backend (native only)

pub mod backend;
pub mod gamepad;
pub mod keyboard;
pub mod mouse;
pub mod sensors;
pub mod touch;

// Re-export commonly used types at crate root
pub use backend::ControllerBackend;
pub use gamepad::{ControllerInfo, ControllerInput, GamepadState};
pub use keyboard::{KeyState, KeyboardState};
pub use mouse::{CursorMode, MouseButtonType, MouseButtons};
pub use sensors::{Accelerometer, Compass, SensorState};
pub use touch::{TouchPhase, TouchPoint, TouchState};

// Backend implementations
#[cfg(feature = "gilrs")]
mod gilrs_backend;

#[cfg(feature = "gilrs")]
pub use gilrs_backend::GilrsBackend;

/// Create the default controller backend based on enabled features
#[cfg(feature = "gilrs")]
pub fn create_controller_backend() -> Option<Box<dyn ControllerBackend>> {
    match GilrsBackend::new() {
        Ok(backend) => Some(Box::new(backend)),
        Err(e) => {
            eprintln!("[Controller] Failed to initialize gilrs backend: {}", e);
            None
        }
    }
}

/// Create the default controller backend (returns None when no backend available)
#[cfg(not(feature = "gilrs"))]
pub fn create_controller_backend() -> Option<Box<dyn ControllerBackend>> {
    None
}
