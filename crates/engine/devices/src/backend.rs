//! Controller backend trait for platform-specific implementations
//!
//! This module defines the trait that platform-specific controller backends
//! must implement (e.g., gilrs for desktop, Web Gamepad API for browsers).

use crate::gamepad::{ControllerInfo, ControllerInput};

/// Trait for controller backends (e.g., gilrs, SDL2, Web Gamepad API)
///
/// Implementations of this trait provide platform-specific gamepad/controller
/// support. The backend is responsible for:
/// - Polling for controller events
/// - Maintaining controller state
/// - Providing access to connected controllers
///
/// # Example
///
/// ```ignore
/// let mut backend = GilrsBackend::new().unwrap();
///
/// // In your game loop:
/// backend.poll();
///
/// if let Some(controller) = backend.get_first_controller() {
///     let movement = controller.gamepad.get_movement();
///     // Use movement for player control
/// }
/// ```
pub trait ControllerBackend {
    /// Poll for controller events and update state
    ///
    /// Should be called each frame to process any pending controller events.
    fn poll(&mut self);

    /// Get list of all connected controllers
    ///
    /// Returns information about each connected controller including
    /// its ID, name, and connection status.
    fn enumerate(&self) -> Vec<ControllerInfo>;

    /// Get mutable reference to controller input state for a specific controller
    ///
    /// The `id` corresponds to the `ControllerInfo.id` from `enumerate()`.
    fn get_controller(&mut self, id: usize) -> Option<&mut ControllerInput>;

    /// Get the first connected controller (convenience method)
    ///
    /// Useful for single-player games that only need one controller.
    fn get_first_controller(&mut self) -> Option<&mut ControllerInput>;

    /// Check if any controllers are connected
    fn has_controllers(&self) -> bool {
        !self.enumerate().is_empty()
    }

    /// Get the number of connected controllers
    fn controller_count(&self) -> usize {
        self.enumerate().len()
    }
}

/// A no-op controller backend for platforms without controller support
///
/// This implementation can be used as a fallback when no controller
/// backend is available, allowing code to compile and run without
/// conditional compilation everywhere.
pub struct NullBackend;

impl NullBackend {
    /// Create a new null backend
    pub fn new() -> Self {
        Self
    }
}

impl Default for NullBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl ControllerBackend for NullBackend {
    fn poll(&mut self) {
        // No-op
    }

    fn enumerate(&self) -> Vec<ControllerInfo> {
        Vec::new()
    }

    fn get_controller(&mut self, _id: usize) -> Option<&mut ControllerInput> {
        None
    }

    fn get_first_controller(&mut self) -> Option<&mut ControllerInput> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_backend() {
        let mut backend = NullBackend::new();

        backend.poll(); // Should not panic
        assert!(backend.enumerate().is_empty());
        assert!(backend.get_controller(0).is_none());
        assert!(backend.get_first_controller().is_none());
        assert!(!backend.has_controllers());
        assert_eq!(backend.controller_count(), 0);
    }
}
