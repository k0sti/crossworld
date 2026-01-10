//! Game controller (gamepad) backend support
//!
//! This module provides gamepad backend implementations (gilrs).
//! The core gamepad types are re-exported from the core crate.

// Re-export core gamepad types
pub use core::input::{ControllerInfo, ControllerInput, GamepadState};

/// Trait for controller backends (e.g., gilrs, SDL2, etc.)
pub trait ControllerBackend {
    /// Poll for controller events and update state
    /// Returns true if any events were processed
    fn poll(&mut self);

    /// Get list of all connected controllers
    fn enumerate(&self) -> Vec<ControllerInfo>;

    /// Get mutable reference to controller input state for a specific controller
    fn get_controller(&mut self, id: usize) -> Option<&mut ControllerInput>;

    /// Get the first connected controller (convenience method)
    fn get_first_controller(&mut self) -> Option<&mut ControllerInput>;

    /// Check if any controllers are connected
    fn has_controllers(&self) -> bool {
        !self.enumerate().is_empty()
    }
}

#[cfg(feature = "gilrs")]
#[path = "gilrs_backend.rs"]
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

#[cfg(not(feature = "gilrs"))]
pub fn create_controller_backend() -> Option<Box<dyn ControllerBackend>> {
    None
}
