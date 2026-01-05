//! Generic orbit camera controller for egui-based applications
//!
//! Provides reusable camera orbit and zoom control that can be used
//! across different applications (renderer, testbed, etc.)

use crate::Camera;
use glam::Vec3;

/// Configuration for orbit camera controller
#[derive(Debug, Clone)]
pub struct OrbitControllerConfig {
    /// Mouse sensitivity for orbit rotation (radians per pixel)
    pub mouse_sensitivity: f32,
    /// Zoom sensitivity (units per scroll unit)
    pub zoom_sensitivity: f32,
    /// Minimum zoom distance from target
    pub min_distance: f32,
    /// Maximum zoom distance from target
    pub max_distance: f32,
}

impl Default for OrbitControllerConfig {
    fn default() -> Self {
        Self {
            mouse_sensitivity: 0.005,
            zoom_sensitivity: 0.5,
            min_distance: 0.5,
            max_distance: 50.0,
        }
    }
}

/// Generic orbit camera controller that works with egui responses
///
/// Handles:
/// - Mouse drag for orbit rotation around target
/// - Mouse scroll for zoom in/out
///
/// # Example
/// ```ignore
/// let controller = OrbitController::new(Vec3::ZERO, OrbitControllerConfig::default());
///
/// // In your egui rendering:
/// let response = ui.allocate_rect(rect, egui::Sense::drag());
/// controller.handle_input(&response, &mut camera);
/// ```
pub struct OrbitController {
    /// Target point to orbit around
    pub target: Vec3,
    /// Configuration for sensitivity and limits
    pub config: OrbitControllerConfig,
}

impl OrbitController {
    /// Create a new orbit controller with the given target and config
    pub fn new(target: Vec3, config: OrbitControllerConfig) -> Self {
        Self { target, config }
    }

    /// Create a new orbit controller with default configuration
    pub fn with_target(target: Vec3) -> Self {
        Self::new(target, OrbitControllerConfig::default())
    }

    /// Handle egui response for camera orbit and zoom
    ///
    /// Call this with the egui response from the viewport area where
    /// the user can drag to orbit the camera.
    pub fn handle_response(&self, response: &egui::Response, camera: &mut Camera) {
        // Handle orbit via mouse drag
        if response.dragged() {
            let delta = response.drag_delta();
            let yaw_delta = -delta.x * self.config.mouse_sensitivity;
            let pitch_delta = -delta.y * self.config.mouse_sensitivity;
            camera.orbit(self.target, yaw_delta, pitch_delta);
        }

        // Handle zoom via scroll
        if response.hovered() {
            let scroll_delta = response.ctx.input(|i| i.smooth_scroll_delta.y);
            if scroll_delta.abs() > 0.01 {
                self.apply_zoom(camera, scroll_delta);
            }
        }
    }

    /// Handle egui input context for scroll-only zoom (when not using response)
    ///
    /// Useful when the scroll should work anywhere, not just on a specific response
    pub fn handle_scroll(&self, ctx: &egui::Context, camera: &mut Camera) {
        let scroll_delta = ctx.input(|i| i.smooth_scroll_delta.y);
        if scroll_delta.abs() > 0.01 {
            self.apply_zoom(camera, scroll_delta);
        }
    }

    /// Apply zoom by moving camera toward/away from target
    fn apply_zoom(&self, camera: &mut Camera, scroll_delta: f32) {
        let to_target = self.target - camera.position;
        let distance = to_target.length();
        let zoom_amount = scroll_delta * self.config.zoom_sensitivity * 0.01;
        let new_distance = (distance - zoom_amount).clamp(self.config.min_distance, self.config.max_distance);
        let zoom_factor = new_distance / distance;
        camera.position = self.target - to_target * zoom_factor;
    }

    /// Update the target point
    pub fn set_target(&mut self, target: Vec3) {
        self.target = target;
    }
}
