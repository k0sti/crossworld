use glam::{Mat4, Quat, Vec3};

/// Camera mode selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraMode {
    Orbit,
    FirstPerson,
}

impl Default for CameraMode {
    fn default() -> Self {
        Self::Orbit
    }
}

/// Orbit camera for viewing the scene
/// Uses vector-based rotation instead of euler angles
pub struct OrbitCamera {
    pub focus: Vec3,
    /// Camera orientation as quaternion
    pub orientation: Quat,
    pub distance: f32,
    pub dragging: bool,
    pub last_mouse_pos: Option<(f32, f32)>,
}

impl OrbitCamera {
    pub fn new(distance: f32) -> Self {
        // Start looking at focus from a slight angle
        let initial_dir = Vec3::new(0.0, 0.5, 1.0).normalize();
        let orientation = Quat::from_rotation_arc(Vec3::NEG_Z, -initial_dir);

        Self {
            // World cube is in [0, 1] space, focus on center
            focus: Vec3::splat(0.5),
            orientation,
            distance,
            dragging: false,
            last_mouse_pos: None,
        }
    }

    /// Get the forward direction (from camera toward focus)
    pub fn forward(&self) -> Vec3 {
        self.orientation * Vec3::NEG_Z
    }

    /// Get the right direction
    pub fn right(&self) -> Vec3 {
        self.orientation * Vec3::X
    }

    /// Get the up direction
    pub fn up(&self) -> Vec3 {
        self.orientation * Vec3::Y
    }

    #[allow(dead_code)]
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position(), self.focus, Vec3::Y)
    }

    pub fn position(&self) -> Vec3 {
        // Camera is at focus - forward * distance
        self.focus - self.forward() * self.distance
    }

    pub fn rotation(&self) -> Quat {
        self.orientation
    }

    /// For compatibility with UI - extract approximate yaw
    pub fn yaw(&self) -> f32 {
        let fwd = self.forward();
        fwd.x.atan2(fwd.z)
    }

    /// For compatibility with UI - extract approximate pitch
    pub fn pitch(&self) -> f32 {
        let fwd = self.forward();
        fwd.y.asin()
    }

    pub fn handle_mouse_drag(&mut self, delta_x: f32, delta_y: f32) {
        let sensitivity = 0.003;

        // Rotate around world Y axis (yaw) - always uses world up
        let yaw_rotation = Quat::from_axis_angle(Vec3::Y, -delta_x * sensitivity);

        // Rotate around camera's local X axis (pitch)
        let pitch_rotation = Quat::from_axis_angle(self.right(), -delta_y * sensitivity);

        // Apply rotations: yaw first (world space), then pitch (local space)
        self.orientation = yaw_rotation * self.orientation;
        self.orientation = pitch_rotation * self.orientation;
        self.orientation = self.orientation.normalize();

        // Clamp pitch to prevent flipping over
        let fwd = self.forward();
        let max_pitch = 1.5_f32; // ~86 degrees
        if fwd.y.abs() > max_pitch.sin() {
            // Reconstruct orientation with clamped pitch
            let yaw = fwd.x.atan2(fwd.z);
            let pitch = fwd.y.asin().clamp(-max_pitch, max_pitch);

            let yaw_quat = Quat::from_rotation_y(yaw);
            let pitch_quat = Quat::from_rotation_x(pitch);
            self.orientation = yaw_quat * pitch_quat;
        }
    }

    pub fn handle_scroll(&mut self, delta: f32) {
        self.distance -= delta * 0.1;
        self.distance = self.distance.clamp(0.5, 10.0);
    }
}

/// First-person camera controller
///
/// Uses vector-based rotation with WASD keys for horizontal movement,
/// F/V for up/down, and mouse for look-around.
pub struct FirstPersonCamera {
    /// Camera position (set by physics)
    pub position: Vec3,
    /// Camera orientation as quaternion
    pub orientation: Quat,
    /// Movement speed multiplier
    pub move_speed: f32,
    /// Mouse sensitivity
    pub sensitivity: f32,
    /// Is mouse captured for look-around
    pub mouse_captured: bool,
    /// Vertical velocity for physics
    pub vertical_velocity: f32,
}

impl FirstPersonCamera {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            orientation: Quat::IDENTITY,
            move_speed: 5.0,
            sensitivity: 0.003,
            mouse_captured: false,
            vertical_velocity: 0.0,
        }
    }

    /// Get camera position
    pub fn position(&self) -> Vec3 {
        self.position
    }

    /// Get camera rotation as quaternion
    pub fn rotation(&self) -> Quat {
        self.orientation
    }

    /// Get forward direction (where camera is looking)
    pub fn forward(&self) -> Vec3 {
        self.orientation * Vec3::NEG_Z
    }

    /// Get right direction
    pub fn right(&self) -> Vec3 {
        self.orientation * Vec3::X
    }

    /// Get up direction
    pub fn up(&self) -> Vec3 {
        self.orientation * Vec3::Y
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

    /// For compatibility with UI - extract approximate yaw
    pub fn yaw(&self) -> f32 {
        let fwd = self.forward();
        fwd.x.atan2(fwd.z)
    }

    /// For compatibility with UI - extract approximate pitch
    pub fn pitch(&self) -> f32 {
        let fwd = self.forward();
        fwd.y.asin()
    }

    /// Handle mouse movement for look-around
    pub fn handle_mouse_move(&mut self, delta_x: f32, delta_y: f32) {
        if !self.mouse_captured {
            return;
        }

        // Rotate around world Y axis (yaw) - always uses world up for FPS camera
        let yaw_rotation = Quat::from_axis_angle(Vec3::Y, -delta_x * self.sensitivity);

        // Rotate around camera's local X axis (pitch)
        let pitch_rotation = Quat::from_axis_angle(self.right(), -delta_y * self.sensitivity);

        // Apply rotations
        self.orientation = yaw_rotation * self.orientation;
        self.orientation = pitch_rotation * self.orientation;
        self.orientation = self.orientation.normalize();

        // Clamp pitch to prevent flipping
        let fwd = self.forward();
        let max_pitch = 1.5_f32; // ~86 degrees
        if fwd.y.abs() > max_pitch.sin() {
            // Reconstruct orientation with clamped pitch
            let yaw = fwd.x.atan2(fwd.z);
            let pitch = fwd.y.asin().clamp(-max_pitch, max_pitch);

            let yaw_quat = Quat::from_rotation_y(yaw);
            let pitch_quat = Quat::from_rotation_x(pitch);
            self.orientation = yaw_quat * pitch_quat;
        }
    }

    /// Calculate movement velocity from input state
    ///
    /// # Arguments
    /// * `forward` - W key pressed
    /// * `backward` - S key pressed
    /// * `left` - A key pressed
    /// * `right` - D key pressed
    /// * `up` - F key pressed
    /// * `down` - V key pressed
    ///
    /// # Returns
    /// Movement velocity vector in world space
    pub fn calculate_velocity(
        &self,
        forward: bool,
        backward: bool,
        left: bool,
        right: bool,
        up: bool,
        down: bool,
    ) -> Vec3 {
        let mut velocity = Vec3::ZERO;

        // Use XZ-projected directions for horizontal movement
        let fwd = self.forward_xz();
        let rgt = self.right_xz();

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
            velocity = velocity.normalize() * self.move_speed;
        }

        velocity
    }

    /// Update position from physics
    pub fn set_position(&mut self, position: Vec3) {
        self.position = position;
    }
}
