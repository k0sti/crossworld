use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;

/// Orbit camera component for editor
#[derive(Component, Debug, Clone)]
pub struct OrbitCamera {
    /// Point the camera orbits around
    pub target: Vec3,
    /// Distance from target
    pub distance: f32,
    /// Rotation around vertical axis (yaw)
    pub yaw: f32,
    /// Rotation around horizontal axis (pitch)
    pub pitch: f32,
}

impl OrbitCamera {
    /// Create new orbit camera looking at target from distance
    pub fn new(target: Vec3, distance: f32) -> Self {
        Self {
            target,
            distance,
            yaw: std::f32::consts::FRAC_PI_4,   // 45 degrees
            pitch: std::f32::consts::FRAC_PI_6, // 30 degrees
        }
    }

    /// Calculate transform from current orbit parameters
    pub fn calculate_transform(&self) -> Transform {
        let x = self.distance * self.pitch.cos() * self.yaw.sin();
        let y = self.distance * self.pitch.sin();
        let z = self.distance * self.pitch.cos() * self.yaw.cos();

        let position = self.target + Vec3::new(x, y, z);
        Transform::from_translation(position).looking_at(self.target, Vec3::Y)
    }

    /// Reset to default view
    pub fn reset(&mut self) {
        self.yaw = std::f32::consts::FRAC_PI_4;
        self.pitch = std::f32::consts::FRAC_PI_6;
        self.distance = 15.0;
        self.target = Vec3::ZERO;
    }
}

/// System that handles camera orbit via right-click drag
pub fn orbit_camera(
    mut camera_query: Query<(&mut OrbitCamera, &mut Transform)>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut motion_events: MessageReader<MouseMotion>,
) {
    if !mouse_buttons.pressed(MouseButton::Right) {
        motion_events.clear();
        return;
    }

    let mut delta = Vec2::ZERO;
    for event in motion_events.read() {
        delta += event.delta;
    }

    if delta.length_squared() < 0.001 {
        return;
    }

    for (mut orbit, mut transform) in camera_query.iter_mut() {
        orbit.yaw -= delta.x * 0.005;
        orbit.pitch += delta.y * 0.005;

        // Clamp pitch to avoid gimbal lock
        orbit.pitch = orbit.pitch.clamp(-1.4, 1.4);

        *transform = orbit.calculate_transform();
    }
}

/// System that handles camera zoom via scroll wheel
pub fn zoom_camera(
    mut camera_query: Query<(&mut OrbitCamera, &mut Transform)>,
    mut scroll_events: MessageReader<MouseWheel>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    // Don't zoom if Shift is held (that's for cursor size)
    if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
        scroll_events.clear();
        return;
    }

    let scroll_delta: f32 = scroll_events.read().map(|e| e.y).sum();
    if scroll_delta.abs() < 0.01 {
        return;
    }

    for (mut orbit, mut transform) in camera_query.iter_mut() {
        orbit.distance *= 1.0 - scroll_delta * 0.1;
        orbit.distance = orbit.distance.clamp(2.0, 100.0);
        *transform = orbit.calculate_transform();
    }
}

/// System that handles camera keyboard shortcuts
pub fn camera_keyboard_controls(
    mut camera_query: Query<(&mut OrbitCamera, &mut Transform)>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    for (mut orbit, mut transform) in camera_query.iter_mut() {
        // Home: Reset camera
        if keyboard.just_pressed(KeyCode::Home) {
            orbit.reset();
            *transform = orbit.calculate_transform();
            info!("Camera reset to default");
        }

        // F: Frame scene (fit view)
        if keyboard.just_pressed(KeyCode::KeyF) {
            orbit.target = Vec3::ZERO;
            orbit.distance = 15.0;
            *transform = orbit.calculate_transform();
            info!("Camera framed to scene center");
        }

        // Numpad views
        if keyboard.just_pressed(KeyCode::Numpad1) {
            orbit.yaw = 0.0;
            orbit.pitch = 0.0;
            *transform = orbit.calculate_transform();
            info!("Front view");
        }

        if keyboard.just_pressed(KeyCode::Numpad3) {
            orbit.yaw = std::f32::consts::FRAC_PI_2;
            orbit.pitch = 0.0;
            *transform = orbit.calculate_transform();
            info!("Side view");
        }

        if keyboard.just_pressed(KeyCode::Numpad7) {
            orbit.yaw = 0.0;
            orbit.pitch = std::f32::consts::FRAC_PI_2 - 0.01;
            *transform = orbit.calculate_transform();
            info!("Top view");
        }
    }
}

/// Plugin for camera controls
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (orbit_camera, zoom_camera, camera_keyboard_controls),
        );
    }
}
