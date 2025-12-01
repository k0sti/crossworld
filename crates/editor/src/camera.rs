use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;

/// Component for orbit camera behavior
#[derive(Component)]
pub struct OrbitCamera {
    /// Target point to orbit around
    pub target: Vec3,
    /// Distance from target
    pub distance: f32,
    /// Horizontal rotation angle (radians)
    pub azimuth: f32,
    /// Vertical rotation angle (radians, clamped between -PI/2 and PI/2)
    pub elevation: f32,
    /// Min/max distance limits
    pub distance_min: f32,
    pub distance_max: f32,
    /// Smoothing factor for camera movement (0 = no smoothing, 1 = instant)
    pub smoothing: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            distance: 15.0,
            azimuth: std::f32::consts::FRAC_PI_4,   // 45 degrees
            elevation: std::f32::consts::FRAC_PI_6, // 30 degrees
            distance_min: 1.0,
            distance_max: 100.0,
            smoothing: 0.0, // Increased from 0.1 for more responsive camera
        }
    }
}

impl OrbitCamera {
    pub fn new(target: Vec3, distance: f32) -> Self {
        Self {
            target,
            distance,
            ..default()
        }
    }

    /// Calculate camera position based on target, distance, and angles
    pub fn calculate_position(&self) -> Vec3 {
        let x = self.distance * self.elevation.cos() * self.azimuth.cos();
        let y = self.distance * self.elevation.sin();
        let z = self.distance * self.elevation.cos() * self.azimuth.sin();
        self.target + Vec3::new(x, y, z)
    }

    /// Calculate camera transform looking at target
    pub fn calculate_transform(&self) -> Transform {
        let position = self.calculate_position();
        Transform::from_translation(position).looking_at(self.target, Vec3::Y)
    }
}

/// System that handles orbit camera controls
pub fn orbit_camera_controller(
    mut query: Query<(&mut OrbitCamera, &mut Transform)>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: MessageReader<MouseMotion>,
    mut mouse_wheel: MessageReader<MouseWheel>,
    time: Res<Time>,
) {
    // Calculate total mouse delta for this frame
    let mouse_delta: Vec2 = mouse_motion.read().map(|e| e.delta).sum();

    // Calculate total scroll delta for this frame
    let scroll_delta: f32 = mouse_wheel.read().map(|e| e.y).sum();

    for (mut camera, mut transform) in query.iter_mut() {
        // Right-click drag to rotate
        if mouse_buttons.pressed(MouseButton::Right) && mouse_delta.length_squared() > 0.0 {
            let rotation_speed = 0.01; // Increased from 0.005 for more responsive rotation
            camera.azimuth -= mouse_delta.x * rotation_speed;
            camera.elevation += mouse_delta.y * rotation_speed;

            // Clamp elevation to prevent flipping
            camera.elevation = camera.elevation.clamp(
                -std::f32::consts::FRAC_PI_2 + 0.01,
                std::f32::consts::FRAC_PI_2 - 0.01,
            );
        }

        // Middle-click drag to pan (DISABLED - cube should stay fixed at origo)
        // NOTE: Panning is disabled to keep the cube centered at the origin.
        // If panning is needed in the future, it should move the camera position
        // while keeping the target fixed at Vec3::ZERO.
        if false && mouse_buttons.pressed(MouseButton::Middle) && mouse_delta.length_squared() > 0.0
        {
            let pan_speed = 0.01 * camera.distance;

            // Calculate right and up vectors in camera space
            let forward = (camera.target - camera.calculate_position()).normalize();
            let right = forward.cross(Vec3::Y).normalize();
            let up = right.cross(forward).normalize();

            // Pan the target
            camera.target -= right * mouse_delta.x * pan_speed;
            camera.target += up * mouse_delta.y * pan_speed;
        }

        // Scroll wheel to zoom
        if scroll_delta.abs() > 0.01 {
            let zoom_speed = 0.1 * camera.distance;
            camera.distance -= scroll_delta * zoom_speed;
            camera.distance = camera
                .distance
                .clamp(camera.distance_min, camera.distance_max);
        }

        // Smooth camera movement
        let target_transform = camera.calculate_transform();

        if camera.smoothing > 0.0 {
            let t = (camera.smoothing * 10.0 * time.delta_secs()).min(1.0);
            transform.translation = transform.translation.lerp(target_transform.translation, t);
            transform.rotation = transform.rotation.slerp(target_transform.rotation, t);
        } else {
            *transform = target_transform;
        }
    }
}

/// System to handle camera keyboard shortcuts
pub fn camera_keyboard_shortcuts(
    mut query: Query<&mut OrbitCamera>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    for mut camera in query.iter_mut() {
        // F key: frame scene (reset to default view)
        if keyboard.just_pressed(KeyCode::KeyF) {
            camera.target = Vec3::ZERO;
            camera.distance = 15.0;
            camera.azimuth = std::f32::consts::FRAC_PI_4; // 45 degrees
            camera.elevation = std::f32::consts::FRAC_PI_6; // 30 degrees
            info!("Camera reset to frame scene");
        }

        // Home key: reset camera to default position
        if keyboard.just_pressed(KeyCode::Home) {
            camera.target = Vec3::ZERO;
            camera.distance = 15.0;
            camera.azimuth = std::f32::consts::FRAC_PI_4;
            camera.elevation = std::f32::consts::FRAC_PI_6;
            info!("Camera reset to home position");
        }

        // Numpad views (orthographic-style)
        // Numpad 1: Front view
        if keyboard.just_pressed(KeyCode::Numpad1) {
            camera.azimuth = 0.0;
            camera.elevation = 0.0;
            info!("Camera: Front view");
        }

        // Numpad 3: Side view (right)
        if keyboard.just_pressed(KeyCode::Numpad3) {
            camera.azimuth = std::f32::consts::FRAC_PI_2; // 90 degrees
            camera.elevation = 0.0;
            info!("Camera: Right side view");
        }

        // Numpad 7: Top view
        if keyboard.just_pressed(KeyCode::Numpad7) {
            camera.azimuth = 0.0;
            camera.elevation = std::f32::consts::FRAC_PI_2 - 0.01; // Almost 90 degrees
            info!("Camera: Top view");
        }
    }
}

/// Plugin for camera controls
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (orbit_camera_controller, camera_keyboard_shortcuts));
    }
}
