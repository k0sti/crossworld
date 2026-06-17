//! Camera module - re-exports from core
//!
//! The canonical camera implementation lives in [`core::camera`].

pub use core::camera::*;

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn reexports_core_camera_types() {
        let camera = Camera::from_pitch_yaw(Vec3::ZERO, 0.0, 0.0);
        let _view = camera.view_matrix();
        let _ = FirstPersonController::default();
        let _ = OrbitController::with_target(Vec3::ZERO);
    }
}
