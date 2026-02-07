//! Camera configuration for 3D rendering
//!
//! This module re-exports the camera types from the `core` crate for use in rendering.
//! The canonical camera implementation lives in `core::camera`.

// Re-export all camera types from core crate
pub use core::camera::{
    Camera, DEFAULT_VFOV, FirstPersonController, FirstPersonControllerConfig, OrbitController,
    OrbitControllerConfig,
};
