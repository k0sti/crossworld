//! GLB Model Viewer Example
//!
//! A Bevy application that loads and renders GLB/GLTF models with
//! orbit camera controls.
//!
//! # Usage
//! ```bash
//! cargo run -p renderer --example bevy_glb_viewer
//! ```
//!
//! # Controls
//! - Right-click + drag: Orbit camera around model
//! - Scroll: Zoom in/out
//! - WASD: Pan camera
//! - Home: Reset camera to default view
//! - F: Frame model (fit to view)

use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::window::WindowResolution;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, FRAC_PI_6};

// ============================================================================
// Orbit Camera (adapted from editor-bevy)
// ============================================================================

/// Orbit camera component - orbits around a target point
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
            yaw: FRAC_PI_4,   // 45 degrees
            pitch: FRAC_PI_6, // 30 degrees
        }
    }

    /// Calculate transform from current orbit parameters
    pub fn calculate_transform(&self) -> Transform {
        // Spherical to cartesian coordinates
        let x = self.distance * self.pitch.cos() * self.yaw.sin();
        let y = self.distance * self.pitch.sin();
        let z = self.distance * self.pitch.cos() * self.yaw.cos();

        let position = self.target + Vec3::new(x, y, z);
        Transform::from_translation(position).looking_at(self.target, Vec3::Y)
    }

    /// Reset to default view
    pub fn reset(&mut self) {
        self.yaw = FRAC_PI_4;
        self.pitch = FRAC_PI_6;
        self.distance = 15.0;
        self.target = Vec3::ZERO;
    }
}

/// System that handles camera orbit via right-click drag
fn orbit_camera(
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
        orbit.pitch = orbit.pitch.clamp(-FRAC_PI_2 + 0.1, FRAC_PI_2 - 0.1);

        *transform = orbit.calculate_transform();
    }
}

/// System that handles camera zoom via scroll wheel
fn zoom_camera(
    mut camera_query: Query<(&mut OrbitCamera, &mut Transform)>,
    mut scroll_events: MessageReader<MouseWheel>,
) {
    let scroll_delta: f32 = scroll_events.read().map(|e| e.y).sum();
    if scroll_delta.abs() < 0.01 {
        return;
    }

    for (mut orbit, mut transform) in camera_query.iter_mut() {
        orbit.distance *= 1.0 - scroll_delta * 0.1;
        orbit.distance = orbit.distance.clamp(1.0, 200.0);
        *transform = orbit.calculate_transform();
    }
}

/// System that handles camera pan via WASD
fn pan_camera(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut camera_query: Query<(&mut OrbitCamera, &mut Transform)>,
) {
    let pan_speed = 10.0;
    let dt = time.delta_secs();

    for (mut orbit, mut transform) in camera_query.iter_mut() {
        let forward = (orbit.target - transform.translation).normalize();
        let right = forward.cross(Vec3::Y).normalize();
        let up = Vec3::Y;

        let mut pan_delta = Vec3::ZERO;

        // WASD for horizontal panning
        if keyboard.pressed(KeyCode::KeyW) {
            pan_delta += forward;
        }
        if keyboard.pressed(KeyCode::KeyS) {
            pan_delta -= forward;
        }
        if keyboard.pressed(KeyCode::KeyA) {
            pan_delta -= right;
        }
        if keyboard.pressed(KeyCode::KeyD) {
            pan_delta += right;
        }
        // Q/E for vertical panning
        if keyboard.pressed(KeyCode::KeyQ) || keyboard.pressed(KeyCode::Space) {
            pan_delta += up;
        }
        if keyboard.pressed(KeyCode::KeyE) || keyboard.pressed(KeyCode::ShiftLeft) {
            pan_delta -= up;
        }

        if pan_delta.length_squared() > 0.0 {
            pan_delta = pan_delta.normalize() * pan_speed * dt;
            orbit.target += pan_delta;
            *transform = orbit.calculate_transform();
        }
    }
}

/// System that handles camera keyboard shortcuts
fn camera_shortcuts(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut camera_query: Query<(&mut OrbitCamera, &mut Transform)>,
    bounds: Res<ModelBounds>,
) {
    for (mut orbit, mut transform) in camera_query.iter_mut() {
        // Home: Reset camera to default
        if keyboard.just_pressed(KeyCode::Home) {
            orbit.reset();
            if bounds.loaded {
                orbit.target = bounds.center;
                orbit.distance = bounds.radius * 2.5;
            }
            *transform = orbit.calculate_transform();
            info!("Camera reset");
        }

        // F: Frame model (fit to view)
        if keyboard.just_pressed(KeyCode::KeyF) && bounds.loaded {
            orbit.target = bounds.center;
            orbit.distance = bounds.radius * 2.5;
            *transform = orbit.calculate_transform();
            info!("Camera framed to model");
        }

        // Numpad views
        if keyboard.just_pressed(KeyCode::Numpad1) {
            orbit.yaw = 0.0;
            orbit.pitch = 0.0;
            *transform = orbit.calculate_transform();
            info!("Front view");
        }

        if keyboard.just_pressed(KeyCode::Numpad3) {
            orbit.yaw = FRAC_PI_2;
            orbit.pitch = 0.0;
            *transform = orbit.calculate_transform();
            info!("Side view");
        }

        if keyboard.just_pressed(KeyCode::Numpad7) {
            orbit.yaw = 0.0;
            orbit.pitch = FRAC_PI_2 - 0.01;
            *transform = orbit.calculate_transform();
            info!("Top view");
        }
    }
}

// ============================================================================
// Model Loading
// ============================================================================

/// Resource to track loaded model bounds for camera framing
#[derive(Resource, Default)]
pub struct ModelBounds {
    pub center: Vec3,
    pub radius: f32,
    pub loaded: bool,
}

/// Resource to track model loading state
#[derive(Resource, Default)]
pub struct ModelLoadState {
    pub handle: Option<Handle<Scene>>,
    pub spawned: bool,
}

/// Setup the scene with camera, lighting, and model
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut model_state: ResMut<ModelLoadState>,
) {
    info!("Setting up GLB viewer...");

    // Spawn the camera with orbit control
    let orbit_camera = OrbitCamera::new(Vec3::ZERO, 20.0);
    commands.spawn((
        Camera3d::default(),
        orbit_camera.calculate_transform(),
        orbit_camera,
    ));

    // Add ambient light
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 300.0,
        affects_lightmapped_meshes: true,
    });

    // Add directional light (sun)
    commands.spawn((
        DirectionalLight {
            illuminance: 15000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(10.0, 20.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Add a second directional light for fill
    commands.spawn((
        DirectionalLight {
            illuminance: 5000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(-10.0, 10.0, -5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Load the GLB model
    let model_path = "models/glb/grandmas_house/scene.gltf#Scene0";
    info!("Loading model: {}", model_path);

    let scene_handle: Handle<Scene> = asset_server.load(model_path);
    model_state.handle = Some(scene_handle);

    info!("GLB viewer setup complete!");
    info!("Controls:");
    info!("  Right-click + drag: Orbit camera");
    info!("  Scroll: Zoom in/out");
    info!("  WASD: Pan camera");
    info!("  Q/E or Space/Shift: Pan up/down");
    info!("  Home: Reset camera");
    info!("  F: Frame model");
    info!("  Numpad 1/3/7: Front/Side/Top views");
}

/// System to spawn the model once it's loaded
fn spawn_model(
    mut commands: Commands,
    mut model_state: ResMut<ModelLoadState>,
    scenes: Res<Assets<Scene>>,
) {
    if model_state.spawned {
        return;
    }

    if let Some(handle) = &model_state.handle
        && scenes.get(handle).is_some()
    {
        info!("Model loaded, spawning scene...");
        commands.spawn((SceneRoot(handle.clone()), Transform::default()));
        model_state.spawned = true;
        info!("Model spawned!");
    }
}

/// System to calculate model bounds after spawning (for camera framing)
fn calculate_bounds(
    mut bounds: ResMut<ModelBounds>,
    mut camera_query: Query<(&mut OrbitCamera, &mut Transform)>,
    query: Query<&GlobalTransform, With<Mesh3d>>,
) {
    if bounds.loaded || query.is_empty() {
        return;
    }

    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);

    for transform in query.iter() {
        let pos = transform.translation();
        min = min.min(pos);
        max = max.max(pos);
    }

    if min.x < f32::MAX {
        bounds.center = (min + max) * 0.5;
        bounds.radius = (max - min).length() * 0.5;
        bounds.loaded = true;
        info!(
            "Model bounds: center={:?}, radius={:.2}",
            bounds.center, bounds.radius
        );

        // Auto-frame model on load
        for (mut orbit, mut transform) in camera_query.iter_mut() {
            orbit.target = bounds.center;
            orbit.distance = bounds.radius * 2.5;
            *transform = orbit.calculate_transform();
        }
    }
}

/// Plugin that bundles all camera controls
pub struct OrbitCameraPlugin;

impl Plugin for OrbitCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (orbit_camera, zoom_camera, pan_camera, camera_shortcuts),
        );
    }
}

// ============================================================================
// Main
// ============================================================================

fn main() {
    println!("GLB Model Viewer");
    println!("================");
    println!();

    // Get the assets directory - it should be at the repository root
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let assets_path = std::path::Path::new(manifest_dir)
        .parent() // crates/
        .and_then(|p| p.parent()) // crossworld/
        .map(|p| p.join("assets"))
        .expect("Could not find assets directory");

    println!("Assets path: {}", assets_path.display());

    // Check if the model exists
    let model_path = assets_path.join("models/glb/grandmas_house/scene.gltf");
    if model_path.exists() {
        println!("Model found: {}", model_path.display());
    } else {
        eprintln!("Warning: Model not found at {}", model_path.display());
        eprintln!("Make sure the grandmas_house model is in assets/models/glb/");
    }

    println!();

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "GLB Model Viewer".to_string(),
                        resolution: WindowResolution::new(1280, 720),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: assets_path.to_string_lossy().to_string(),
                    ..default()
                }),
        )
        .add_plugins(OrbitCameraPlugin)
        .init_resource::<ModelBounds>()
        .init_resource::<ModelLoadState>()
        .add_systems(Startup, setup)
        .add_systems(Update, (spawn_model, calculate_bounds))
        .run();
}
