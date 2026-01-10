//! Generic orbit camera controller for egui-based applications
//!
//! This module re-exports the orbit controller from the `app` crate.
//! The canonical implementation lives in `app::camera`.

// Re-export orbit controller types from app crate
pub use app::camera::{OrbitController, OrbitControllerConfig};
