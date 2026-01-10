//! Camera module - re-exports from core
//!
//! This module now re-exports the camera implementation from the core crate.
//! The camera implementation has been moved to allow WASM compilation.

pub use core::camera::*;

// Egui integration helper functions for orbit controller (requires runtime feature)
#[cfg(feature = "runtime")]
pub mod egui_ext {
    use super::*;

    /// Handle egui response for camera orbit and zoom
    ///
    /// Call this with the egui response from the viewport area where
    /// the user can drag to orbit the camera.
    pub fn handle_orbit_response(
        controller: &OrbitController,
        response: &egui::Response,
        camera: &mut Camera,
    ) {
        // Handle orbit via mouse drag
        if response.dragged() {
            let delta = response.drag_delta();
            let yaw_delta = -delta.x * controller.config.mouse_sensitivity;
            let pitch_delta = -delta.y * controller.config.mouse_sensitivity;
            camera.orbit(controller.target, yaw_delta, pitch_delta);
        }

        // Handle zoom via scroll
        if response.hovered() {
            let scroll_delta = response.ctx.input(|i| i.smooth_scroll_delta.y);
            if scroll_delta.abs() > 0.01 {
                apply_orbit_zoom(controller, camera, scroll_delta);
            }
        }
    }

    /// Handle egui input context for scroll-only zoom (when not using response)
    ///
    /// Useful when the scroll should work anywhere, not just on a specific response
    pub fn handle_orbit_scroll(
        controller: &OrbitController,
        ctx: &egui::Context,
        camera: &mut Camera,
    ) {
        let scroll_delta = ctx.input(|i| i.smooth_scroll_delta.y);
        if scroll_delta.abs() > 0.01 {
            apply_orbit_zoom(controller, camera, scroll_delta);
        }
    }

    /// Apply zoom by moving camera toward/away from target
    fn apply_orbit_zoom(controller: &OrbitController, camera: &mut Camera, scroll_delta: f32) {
        let to_target = controller.target - camera.position;
        let distance = to_target.length();
        let zoom_amount = scroll_delta * controller.config.zoom_sensitivity * 0.01;
        let new_distance = (distance - zoom_amount)
            .clamp(controller.config.min_distance, controller.config.max_distance);
        let zoom_factor = new_distance / distance;
        camera.position = controller.target - to_target * zoom_factor;
    }
}
