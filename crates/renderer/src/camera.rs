//! Camera configuration for 3D rendering
//!
//! This module re-exports the camera types from the `app` crate for use in rendering.
//! The canonical camera implementation lives in `app::camera`.

// Re-export all camera types from app crate
pub use app::camera::{
    Camera, CameraMode, DEFAULT_VFOV, FirstPersonController, FirstPersonControllerConfig, Object,
    OrbitController, OrbitControllerConfig,
};
