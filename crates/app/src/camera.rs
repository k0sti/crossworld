//! Generic 3D camera system with support for orbit and first-person modes
//!
//! This module provides a unified camera system that can be used across
//! different applications (game, editor, testbed, etc.).
//!
//! # Components
//!
//! - [`Camera`]: Core camera struct with position and rotation (quaternion only)
//! - [`OrbitController`]: Controller for orbit camera behavior
//! - [`FirstPersonController`]: Controller for first-person camera behavior

use glam::{Quat, Vec3};

// ============================================================================
// Camera
// ============================================================================

/// Default vertical field of view: 60 degrees
pub const DEFAULT_VFOV: f32 = 60.0 * std::f32::consts::PI / 180.0;

/// Maximum pitch angle to prevent gimbal lock (~86 degrees)
const MAX_PITCH: f32 = 1.5;

/// Camera for 3D rendering
///
/// A minimal camera with position and rotation (quaternion).
/// Pitch and yaw are derived from the quaternion on-demand.
///
/// # Coordinate System
///
/// Uses OpenGL convention:
/// - +X is right
/// - +Y is up
/// - -Z is forward (into the screen)
///
/// # Examples
///
/// ```
/// use app::camera::{Camera, DEFAULT_VFOV};
/// use glam::Vec3;
///
/// // Create camera looking at origin from position (3, 2, 3)
/// let camera = Camera::look_at(
///     Vec3::new(3.0, 2.0, 3.0),
///     Vec3::ZERO,
///     Vec3::Y,
/// );
///
/// // Get camera direction vectors
/// let forward = camera.forward();
/// let right = camera.right();
/// let up = camera.up();
///
/// // Get derived angles
/// let pitch = camera.pitch();
/// let yaw = camera.yaw();
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    /// Camera position in world space
    pub position: Vec3,
    /// Camera rotation (orientation) as quaternion - source of truth
    pub rotation: Quat,
    /// Vertical field of view in radians
    pub vfov: f32,
}

impl Default for Camera {
    fn default() -> Self {
        let position = Vec3::new(3.0, 2.0, 3.0);
        let target = Vec3::ZERO;
        Self::look_at(position, target, Vec3::Y)
    }
}

impl Camera {
    /// Create a new camera with default settings at the given position
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            rotation: Quat::IDENTITY,
            vfov: DEFAULT_VFOV,
        }
    }

    /// Create camera with position looking at target
    pub fn look_at(position: Vec3, target: Vec3, up: Vec3) -> Self {
        let forward = (target - position).normalize();
        let right = forward.cross(up).normalize();
        let cam_up = right.cross(forward);

        // Build rotation matrix from basis vectors and convert to quaternion
        // In camera space: right=+X, up=+Y, forward=-Z (OpenGL convention)
        let rotation_matrix = glam::Mat3::from_cols(right, cam_up, -forward);
        let rotation = Quat::from_mat3(&rotation_matrix);

        Self {
            position,
            rotation,
            vfov: DEFAULT_VFOV,
        }
    }

    /// Set the camera to look at a specific target position
    pub fn set_look_at(&mut self, target: Vec3) {
        let up = Vec3::Y;
        let forward = (target - self.position).normalize();
        let right = forward.cross(up).normalize();
        let cam_up = right.cross(forward);

        let rotation_matrix = glam::Mat3::from_cols(right, cam_up, -forward);
        self.rotation = Quat::from_mat3(&rotation_matrix);
    }

    /// Create camera from pitch and yaw angles
    pub fn from_pitch_yaw(position: Vec3, pitch: f32, yaw: f32) -> Self {
        let rotation = Quat::from_euler(glam::EulerRot::YXZ, yaw, pitch, 0.0);

        Self {
            position,
            rotation,
            vfov: DEFAULT_VFOV,
        }
    }

    /// Get the forward direction vector
    pub fn forward(&self) -> Vec3 {
        self.rotation * Vec3::NEG_Z
    }

    /// Get the right direction vector
    pub fn right(&self) -> Vec3 {
        self.rotation * Vec3::X
    }

    /// Get the up direction vector
    pub fn up(&self) -> Vec3 {
        self.rotation * Vec3::Y
    }

    /// Get forward direction projected onto XZ plane (for movement)
    pub fn forward_xz(&self) -> Vec3 {
        let fwd = self.forward();
        Vec3::new(fwd.x, 0.0, fwd.z).normalize_or_zero()
    }

    /// Get right direction projected onto XZ plane (for movement)
    pub fn right_xz(&self) -> Vec3 {
        let rgt = self.right();
        Vec3::new(rgt.x, 0.0, rgt.z).normalize_or_zero()
    }

    /// Extract pitch angle from rotation (derived, not stored)
    pub fn pitch(&self) -> f32 {
        self.forward().y.asin()
    }

    /// Extract yaw angle from rotation (derived, not stored)
    pub fn yaw(&self) -> f32 {
        let fwd = self.forward();
        fwd.x.atan2(fwd.z)
    }

    /// Get the target point the camera is looking at (1 unit forward)
    pub fn target(&self) -> Vec3 {
        self.position + self.forward()
    }

    /// Rotate camera in place (first-person style)
    ///
    /// Applies yaw around world Y-axis, then pitch around camera's local X-axis.
    /// Pitch is clamped to prevent flipping.
    pub fn rotate(&mut self, yaw_delta: f32, pitch_delta: f32) {
        // Rotate around world Y axis (yaw)
        let yaw_rotation = Quat::from_axis_angle(Vec3::Y, yaw_delta);

        // Rotate around camera's local X axis (pitch)
        let pitch_rotation = Quat::from_axis_angle(self.right(), pitch_delta);

        // Apply rotations
        self.rotation = yaw_rotation * self.rotation;
        self.rotation = pitch_rotation * self.rotation;
        self.rotation = self.rotation.normalize();

        // Clamp pitch to prevent flipping
        self.clamp_pitch();
    }

    /// Orbit camera around a target point
    ///
    /// Applies pitch around camera's local right axis, then yaw around world Y-axis.
    /// Maintains distance from target.
    pub fn orbit(&mut self, target: Vec3, yaw_delta: f32, pitch_delta: f32) {
        // Calculate vector from target to camera
        let mut offset = self.position - target;
        let distance = offset.length();

        // Apply pitch rotation around camera's local right axis FIRST
        // (must use original offset to get correct right vector)
        if pitch_delta.abs() > 0.0001 {
            let forward = -offset.normalize();
            let right = forward.cross(Vec3::Y).normalize();

            if right.length_squared() > 0.0001 {
                let pitch_rotation = Quat::from_axis_angle(right, pitch_delta);
                offset = pitch_rotation * offset;

                // Clamp to prevent flipping over the poles
                let xz_length = (offset.x * offset.x + offset.z * offset.z).sqrt();
                let angle = offset.y.atan2(xz_length);

                if angle.abs() > MAX_PITCH {
                    let clamped = angle.clamp(-MAX_PITCH, MAX_PITCH);
                    let new_y = distance * clamped.sin();
                    let new_xz = distance * clamped.cos();
                    let ratio = new_xz / xz_length;
                    offset = Vec3::new(offset.x * ratio, new_y, offset.z * ratio);
                }
            }
        }

        // Apply yaw rotation around world Y-axis SECOND
        if yaw_delta.abs() > 0.0001 {
            let yaw_rotation = Quat::from_axis_angle(Vec3::Y, yaw_delta);
            offset = yaw_rotation * offset;
        }

        // Maintain distance and update position
        offset = offset.normalize() * distance;
        self.position = target + offset;

        // Update rotation to look at target
        let forward = (target - self.position).normalize();
        self.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, forward);
    }

    /// Move camera relative to its current orientation
    pub fn translate_local(&mut self, offset: Vec3) {
        self.position += self.right() * offset.x;
        self.position += self.up() * offset.y;
        self.position += self.forward() * offset.z;
    }

    /// Zoom by moving camera forward/backward along view direction
    pub fn zoom(&mut self, delta: f32) {
        self.position += self.forward() * delta;
    }

    /// Clamp pitch to prevent camera flipping
    fn clamp_pitch(&mut self) {
        let fwd = self.forward();
        if fwd.y.abs() > MAX_PITCH.sin() {
            let yaw = fwd.x.atan2(fwd.z);
            let pitch = fwd.y.asin().clamp(-MAX_PITCH, MAX_PITCH);

            let yaw_quat = Quat::from_rotation_y(yaw);
            let pitch_quat = Quat::from_rotation_x(pitch);
            self.rotation = yaw_quat * pitch_quat;
        }
    }
}

// ============================================================================
// Orbit Controller
// ============================================================================

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

/// Generic orbit camera controller
///
/// Handles:
/// - Mouse drag for orbit rotation around target
/// - Mouse scroll for zoom in/out
///
/// # Example
/// ```ignore
/// use app::camera::{Camera, OrbitController, OrbitControllerConfig};
/// use glam::Vec3;
///
/// let mut camera = Camera::look_at(Vec3::new(5.0, 3.0, 5.0), Vec3::ZERO, Vec3::Y);
/// let controller = OrbitController::new(Vec3::ZERO, OrbitControllerConfig::default());
///
/// // On mouse drag:
/// controller.rotate(yaw_delta, pitch_delta, &mut camera);
///
/// // On scroll:
/// controller.zoom(scroll_delta, &mut camera);
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

    /// Apply rotation directly
    ///
    /// # Arguments
    /// * `yaw_delta` - Horizontal rotation in radians
    /// * `pitch_delta` - Vertical rotation in radians
    pub fn rotate(&self, yaw_delta: f32, pitch_delta: f32, camera: &mut Camera) {
        camera.orbit(self.target, yaw_delta, pitch_delta);
    }

    /// Apply zoom directly
    ///
    /// # Arguments
    /// * `zoom_delta` - Zoom amount (positive = closer, negative = farther)
    pub fn zoom(&self, zoom_delta: f32, camera: &mut Camera) {
        self.apply_zoom(camera, zoom_delta);
    }

    /// Apply zoom by moving camera toward/away from target
    fn apply_zoom(&self, camera: &mut Camera, scroll_delta: f32) {
        let to_target = self.target - camera.position;
        let distance = to_target.length();
        let zoom_amount = scroll_delta * self.config.zoom_sensitivity * 0.01;
        let new_distance =
            (distance - zoom_amount).clamp(self.config.min_distance, self.config.max_distance);
        let zoom_factor = new_distance / distance;
        camera.position = self.target - to_target * zoom_factor;
    }

    /// Update the target point
    pub fn set_target(&mut self, target: Vec3) {
        self.target = target;
    }

    /// Get the current distance from camera to target
    pub fn distance(&self, camera: &Camera) -> f32 {
        (camera.position - self.target).length()
    }

    /// Set camera distance from target (along current direction)
    pub fn set_distance(&self, camera: &mut Camera, distance: f32) {
        let direction = (camera.position - self.target).normalize();
        camera.position = self.target + direction * distance;
    }
}

// ============================================================================
// First-Person Controller
// ============================================================================

/// Configuration for first-person camera controller
#[derive(Debug, Clone)]
pub struct FirstPersonControllerConfig {
    /// Mouse sensitivity for look rotation
    pub mouse_sensitivity: f32,
    /// Movement speed in units per second
    pub move_speed: f32,
}

impl Default for FirstPersonControllerConfig {
    fn default() -> Self {
        Self {
            mouse_sensitivity: 0.003,
            move_speed: 5.0,
        }
    }
}

/// First-person camera controller
///
/// Handles:
/// - Mouse movement for look-around
/// - WASD movement (horizontal)
/// - Vertical movement (up/down keys)
///
/// # Example
/// ```ignore
/// use app::camera::{Camera, FirstPersonController, FirstPersonControllerConfig};
/// use glam::Vec3;
///
/// let mut camera = Camera::from_pitch_yaw(Vec3::new(0.0, 2.0, 5.0), 0.0, 0.0);
/// let mut controller = FirstPersonController::new(FirstPersonControllerConfig::default());
///
/// // On mouse movement (when captured):
/// controller.handle_mouse_move(&mut camera, delta_x, delta_y);
///
/// // On update:
/// let velocity = controller.calculate_velocity(&camera, forward, backward, left, right, up, down);
/// camera.position += velocity * delta_time;
/// ```
pub struct FirstPersonController {
    /// Configuration
    pub config: FirstPersonControllerConfig,
    /// Whether mouse is captured for look-around
    pub mouse_captured: bool,
}

impl FirstPersonController {
    /// Create a new first-person controller with the given config
    pub fn new(config: FirstPersonControllerConfig) -> Self {
        Self {
            config,
            mouse_captured: false,
        }
    }

    /// Create a new first-person controller with default config
    pub fn with_defaults() -> Self {
        Self::new(FirstPersonControllerConfig::default())
    }

    /// Handle mouse movement for look-around
    ///
    /// Only applies rotation if mouse is captured.
    pub fn handle_mouse_move(&self, camera: &mut Camera, delta_x: f32, delta_y: f32) {
        if !self.mouse_captured {
            return;
        }

        let yaw_delta = -delta_x * self.config.mouse_sensitivity;
        let pitch_delta = -delta_y * self.config.mouse_sensitivity;

        camera.rotate(yaw_delta, pitch_delta);
    }

    /// Apply camera rotation directly (for gamepad input, etc.)
    pub fn apply_rotation(&self, camera: &mut Camera, delta_x: f32, delta_y: f32) {
        camera.rotate(delta_x, delta_y);
    }

    /// Calculate movement velocity from input state
    ///
    /// Returns velocity in world space. Uses XZ-projected directions for
    /// horizontal movement to keep the player on the ground plane.
    ///
    /// # Arguments
    /// * `camera` - Camera to get direction from
    /// * `forward` - W key or forward input
    /// * `backward` - S key or backward input
    /// * `left` - A key or left input
    /// * `right` - D key or right input
    /// * `up` - Space or up input
    /// * `down` - Shift or down input
    #[allow(clippy::too_many_arguments)]
    pub fn calculate_velocity(
        &self,
        camera: &Camera,
        forward: bool,
        backward: bool,
        left: bool,
        right: bool,
        up: bool,
        down: bool,
    ) -> Vec3 {
        let mut velocity = Vec3::ZERO;

        // Use XZ-projected directions for horizontal movement
        let fwd = camera.forward_xz();
        let rgt = camera.right_xz();

        if forward {
            velocity += fwd;
        }
        if backward {
            velocity -= fwd;
        }
        if left {
            velocity -= rgt;
        }
        if right {
            velocity += rgt;
        }
        if up {
            velocity.y += 1.0;
        }
        if down {
            velocity.y -= 1.0;
        }

        // Normalize if moving diagonally, then apply speed
        if velocity.length_squared() > 0.0 {
            velocity = velocity.normalize() * self.config.move_speed;
        }

        velocity
    }

    /// Toggle mouse capture state
    pub fn toggle_mouse_capture(&mut self) {
        self.mouse_captured = !self.mouse_captured;
    }

    /// Set mouse capture state
    pub fn set_mouse_captured(&mut self, captured: bool) {
        self.mouse_captured = captured;
    }
}

// ============================================================================
// Egui Integration (optional, when runtime feature is enabled)
// ============================================================================

#[cfg(feature = "runtime")]
impl OrbitController {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_default() {
        let camera = Camera::default();
        assert!((camera.vfov - DEFAULT_VFOV).abs() < 0.001);
    }

    #[test]
    fn test_camera_look_at() {
        let camera = Camera::look_at(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::Y);

        // Camera should be looking toward origin (negative Z)
        let forward = camera.forward();
        assert!(forward.z < 0.0);
    }

    #[test]
    fn test_camera_from_pitch_yaw() {
        let camera = Camera::from_pitch_yaw(Vec3::ZERO, 0.0, 0.0);

        // With zero pitch/yaw, forward should be -Z
        let forward = camera.forward();
        assert!((forward.z - (-1.0)).abs() < 0.001);
    }

    #[test]
    fn test_orbit_controller() {
        let mut camera = Camera::look_at(Vec3::new(5.0, 0.0, 0.0), Vec3::ZERO, Vec3::Y);
        let controller = OrbitController::with_target(Vec3::ZERO);

        let initial_pos = camera.position;

        // Rotate a bit
        controller.rotate(0.1, 0.0, &mut camera);

        // Position should have changed
        assert!((camera.position - initial_pos).length() > 0.01);

        // But distance should be the same
        let dist_before = initial_pos.length();
        let dist_after = camera.position.length();
        assert!((dist_before - dist_after).abs() < 0.01);
    }

    #[test]
    fn test_first_person_controller() {
        let mut camera = Camera::from_pitch_yaw(Vec3::ZERO, 0.0, 0.0);
        let mut controller = FirstPersonController::with_defaults();

        // Without capture, mouse move should not affect camera
        let initial_rotation = camera.rotation;
        controller.handle_mouse_move(&mut camera, 100.0, 50.0);
        assert_eq!(camera.rotation, initial_rotation);

        // With capture, mouse move should affect camera
        controller.mouse_captured = true;
        controller.handle_mouse_move(&mut camera, 100.0, 50.0);
        assert_ne!(camera.rotation, initial_rotation);
    }

    #[test]
    fn test_velocity_calculation() {
        let camera = Camera::from_pitch_yaw(Vec3::ZERO, 0.0, 0.0);
        let controller = FirstPersonController::with_defaults();

        // Forward should give positive velocity in forward direction
        let velocity =
            controller.calculate_velocity(&camera, true, false, false, false, false, false);
        assert!(velocity.z < 0.0); // Forward is -Z

        // Up should give positive Y velocity
        let velocity =
            controller.calculate_velocity(&camera, false, false, false, false, true, false);
        assert!(velocity.y > 0.0);
    }
}
