//! Generic orbit camera controller for egui-based applications
//!
//! This module re-exports the orbit controller from the `system` crate.
//! The canonical implementation lives in `system::camera`.

// Re-export orbit controller types from system crate
pub use system::camera::{OrbitController, OrbitControllerConfig};
