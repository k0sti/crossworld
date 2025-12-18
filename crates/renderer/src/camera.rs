//! Camera configuration for 3D rendering
//!
//! Provides camera positioning, orientation, and movement utilities for
//! both first-person and orbital camera controls.

use crossworld_physics::Object;

/// Camera for 3D rendering
///
/// Implements the `Object` trait for position/rotation access.
#[derive(Debug, Clone)]
pub struct Camera {
    /// Camera position in world space
    pub position: glam::Vec3,
    /// Camera rotation (orientation)
    pub rotation: glam::Quat,
    /// Vertical field of view in radians
    pub vfov: f32,
    /// Pitch angle in radians (rotation around X axis)
    pub pitch: f32,
    /// Yaw angle in radians (rotation around Y axis)
    pub yaw: f32,
    /// Optional target position for look-at cameras
    pub target_position: Option<glam::Vec3>,
}

/// Default vertical field of view: 60 degrees
pub const DEFAULT_VFOV: f32 = 60.0 * std::f32::consts::PI / 180.0;

impl Default for Camera {
    fn default() -> Self {
        let position = glam::Vec3::new(3.0, 2.0, 3.0);
        let target = glam::Vec3::ZERO;
        let forward = (target - position).normalize();

        // Calculate pitch and yaw from forward vector
        let yaw = forward.z.atan2(forward.x);
        let pitch = forward.y.asin();

        let rotation = glam::Quat::from_rotation_arc(glam::Vec3::NEG_Z, forward);

        Self {
            position,
            rotation,
            vfov: DEFAULT_VFOV,
            pitch,
            yaw,
            target_position: Some(target),
        }
    }
}

impl Copy for Camera {}

impl Camera {
    /// Create camera with position looking at target
    pub fn look_at(position: glam::Vec3, target: glam::Vec3, up: glam::Vec3) -> Self {
        // Build camera basis the same way as the raytracer does
        let forward = (target - position).normalize();
        let right = forward.cross(up).normalize();
        let cam_up = right.cross(forward);

        // Build rotation matrix from basis vectors and convert to quaternion
        // In camera space: right=+X, up=+Y, forward=-Z (OpenGL convention)
        let rotation_matrix = glam::Mat3::from_cols(right, cam_up, -forward);
        let rotation = glam::Quat::from_mat3(&rotation_matrix);

        // Calculate pitch and yaw from forward vector
        let yaw = forward.z.atan2(forward.x);
        let pitch = forward.y.asin();

        Self {
            position,
            rotation,
            vfov: DEFAULT_VFOV,
            pitch,
            yaw,
            target_position: Some(target),
        }
    }

    /// Set the camera to look at a specific target position
    pub fn set_look_at(&mut self, target: glam::Vec3) {
        // Build camera basis the same way as the raytracer does
        let up = glam::Vec3::Y; // Use world up
        let forward = (target - self.position).normalize();
        let right = forward.cross(up).normalize();
        let cam_up = right.cross(forward);

        // Build rotation matrix from basis vectors and convert to quaternion
        let rotation_matrix = glam::Mat3::from_cols(right, cam_up, -forward);
        self.rotation = glam::Quat::from_mat3(&rotation_matrix);

        // Update pitch and yaw
        self.yaw = forward.z.atan2(forward.x);
        self.pitch = forward.y.asin();
        self.target_position = Some(target);
    }

    /// Create camera from pitch and yaw angles
    pub fn from_pitch_yaw(position: glam::Vec3, pitch: f32, yaw: f32) -> Self {
        let rotation = glam::Quat::from_euler(glam::EulerRot::YXZ, yaw, pitch, 0.0);

        Self {
            position,
            rotation,
            vfov: DEFAULT_VFOV,
            pitch,
            yaw,
            target_position: None,
        }
    }

    /// Update camera rotation from pitch and yaw
    pub fn update_from_pitch_yaw(&mut self) {
        self.rotation = glam::Quat::from_euler(glam::EulerRot::YXZ, self.yaw, self.pitch, 0.0);
        // Clear target when manually rotating
        self.target_position = None;
    }

    /// Get the forward direction vector
    pub fn forward(&self) -> glam::Vec3 {
        self.rotation * glam::Vec3::NEG_Z
    }

    /// Get the right direction vector
    pub fn right(&self) -> glam::Vec3 {
        self.rotation * glam::Vec3::X
    }

    /// Get the up direction vector
    pub fn up(&self) -> glam::Vec3 {
        self.rotation * glam::Vec3::Y
    }

    /// Get the target point the camera is looking at (1 unit forward)
    pub fn target(&self) -> glam::Vec3 {
        self.position + self.forward()
    }

    /// Rotate camera by yaw (around Y axis) and pitch (around local X axis)
    /// This rotates the camera in place (first-person style)
    #[allow(dead_code)]
    pub fn rotate(&mut self, yaw_delta: f32, pitch_delta: f32) {
        // Update pitch and yaw
        self.yaw += yaw_delta;
        self.pitch += pitch_delta;

        // Clamp pitch to prevent gimbal lock
        const MAX_PITCH: f32 = 89.0 * std::f32::consts::PI / 180.0;
        self.pitch = self.pitch.clamp(-MAX_PITCH, MAX_PITCH);

        // Update rotation from pitch and yaw
        self.update_from_pitch_yaw();
    }

    /// Orbit camera around a target point
    /// yaw_delta: rotation around world Y-axis (horizontal mouse movement)
    /// pitch_delta: rotation around camera's local right axis (vertical mouse movement)
    pub fn orbit(&mut self, target: glam::Vec3, yaw_delta: f32, pitch_delta: f32) {
        // Calculate vector from target to camera
        let mut offset = self.position - target;
        let distance = offset.length();

        // Step 1: Apply pitch rotation around camera's CURRENT local right axis (vertical angle)
        // This must happen FIRST, using the original camera orientation
        if pitch_delta.abs() > 0.0001 {
            // Get the current right vector (perpendicular to both up and forward)
            let forward = -offset.normalize();
            let right = forward.cross(glam::Vec3::Y).normalize();

            // Only apply pitch if right vector is valid (not looking straight up/down)
            if right.length_squared() > 0.0001 {
                let pitch_rotation = glam::Quat::from_axis_angle(right, pitch_delta);
                offset = pitch_rotation * offset;

                // Clamp to prevent flipping over the poles
                let new_y = offset.y;
                let xz_length = (offset.x * offset.x + offset.z * offset.z).sqrt();
                let angle_from_horizontal = new_y.atan2(xz_length);

                // Clamp angle to [-85°, 85°] to prevent gimbal lock
                const MAX_ANGLE: f32 = 85.0 * std::f32::consts::PI / 180.0;
                if angle_from_horizontal.abs() > MAX_ANGLE {
                    let clamped_angle = angle_from_horizontal.clamp(-MAX_ANGLE, MAX_ANGLE);
                    let new_y = distance * clamped_angle.sin();
                    let new_xz = distance * clamped_angle.cos();
                    let xz_ratio = new_xz / xz_length;
                    offset = glam::Vec3::new(offset.x * xz_ratio, new_y, offset.z * xz_ratio);
                }
            }
        }

        // Step 2: Apply yaw rotation around world Y-axis (horizontal orbit)
        // This happens AFTER pitch, so horizontal orbit is always around world Y
        if yaw_delta.abs() > 0.0001 {
            let yaw_rotation = glam::Quat::from_axis_angle(glam::Vec3::Y, yaw_delta);
            offset = yaw_rotation * offset;
        }

        // Ensure we maintain the same distance
        offset = offset.normalize() * distance;

        // Update position
        self.position = target + offset;

        // Update rotation to look at target
        let forward = (target - self.position).normalize();
        self.rotation = glam::Quat::from_rotation_arc(glam::Vec3::NEG_Z, forward);

        // Update pitch and yaw from the new rotation
        self.yaw = forward.z.atan2(forward.x);
        self.pitch = forward.y.asin();
        self.target_position = Some(target);
    }

    /// Move camera relative to its current orientation
    #[allow(dead_code)]
    pub fn translate_local(&mut self, offset: glam::Vec3) {
        self.position += self.right() * offset.x;
        self.position += self.up() * offset.y;
        self.position += self.forward() * offset.z;
    }

    /// Zoom by moving camera forward/backward along view direction
    /// Note: For orbit cameras, consider zooming toward/away from target instead
    #[allow(dead_code)]
    pub fn zoom(&mut self, delta: f32) {
        self.position += self.forward() * delta;
    }

    /// Create a camera looking at a target object
    ///
    /// # Arguments
    /// * `target` - The object to look at (implements Object trait)
    /// * `offset` - Offset from target position to place camera
    /// * `up` - Up vector for camera orientation
    pub fn looking_at(target: &dyn Object, offset: glam::Vec3, up: glam::Vec3) -> Self {
        let camera_position = target.position() + offset;
        Self::look_at(camera_position, target.position(), up)
    }
}

impl Object for Camera {
    fn position(&self) -> glam::Vec3 {
        self.position
    }

    fn rotation(&self) -> glam::Quat {
        self.rotation
    }

    fn set_position(&mut self, position: glam::Vec3) {
        self.position = position;
    }

    fn set_rotation(&mut self, rotation: glam::Quat) {
        self.rotation = rotation;
    }
}
