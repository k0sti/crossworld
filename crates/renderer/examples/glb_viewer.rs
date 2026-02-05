//! GLB Model Viewer Example
//!
//! A simple Bevy application that loads and renders GLB/GLTF models with
//! free camera controls (WASD + mouse look).
//!
//! # Usage
//! ```bash
//! cargo run -p renderer --example glb_viewer
//! ```
//!
//! # Controls
//! - WASD: Move forward/backward/left/right
//! - Space: Move up
//! - Shift: Move down
//! - Right-click + drag: Look around
//! - Scroll: Zoom in/out
//! - Home: Reset camera
//! - F: Frame model (fit to view)

use bevy::input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::window::WindowResolution;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, FRAC_PI_6};

// ============================================================================
// Camera Controller
// ============================================================================

/// First-person camera controller with orbit capability
#[derive(Component, Debug)]
pub struct FreeCamera {
    /// Movement speed (units per second)
    pub move_speed: f32,
    /// Look sensitivity
    pub look_sensitivity: f32,
    /// Current yaw (horizontal rotation)
    pub yaw: f32,
    /// Current pitch (vertical rotation)
    pub pitch: f32,
    /// Whether right mouse button is held for looking
    pub looking: bool,
}

impl Default for FreeCamera {
    fn default() -> Self {
        Self {
            move_speed: 5.0,
            look_sensitivity: 0.003,
            yaw: FRAC_PI_4,
            pitch: FRAC_PI_6,
            looking: false,
        }
    }
}

impl FreeCamera {
    /// Calculate forward direction from yaw/pitch
    pub fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            -self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        )
        .normalize()
    }

    /// Calculate right direction from yaw
    pub fn right(&self) -> Vec3 {
        Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin()).normalize()
    }
}

/// System that handles camera movement via WASD + Space/Shift
fn camera_movement(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &FreeCamera)>,
) {
    for (mut transform, camera) in query.iter_mut() {
        let mut velocity = Vec3::ZERO;

        let forward = camera.forward();
        let right = camera.right();
        let up = Vec3::Y;

        // WASD movement
        if keyboard.pressed(KeyCode::KeyW) {
            velocity += forward;
        }
        if keyboard.pressed(KeyCode::KeyS) {
            velocity -= forward;
        }
        if keyboard.pressed(KeyCode::KeyA) {
            velocity -= right;
        }
        if keyboard.pressed(KeyCode::KeyD) {
            velocity += right;
        }
        if keyboard.pressed(KeyCode::Space) {
            velocity += up;
        }
        if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
            velocity -= up;
        }

        // Normalize and apply speed
        if velocity.length_squared() > 0.0 {
            velocity = velocity.normalize() * camera.move_speed * time.delta_secs();
            transform.translation += velocity;
        }
    }
}

/// System that handles camera look via right-click drag
fn camera_look(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut motion_events: MessageReader<MouseMotion>,
    mut query: Query<(&mut Transform, &mut FreeCamera)>,
) {
    // Check if right mouse button is pressed
    let looking = mouse_buttons.pressed(MouseButton::Right);

    for (mut transform, mut camera) in query.iter_mut() {
        camera.looking = looking;

        if !looking {
            // Clear events when not looking
            motion_events.clear();
            continue;
        }

        // Accumulate mouse delta
        let mut delta = Vec2::ZERO;
        for event in motion_events.read() {
            delta += event.delta;
        }

        if delta.length_squared() < 0.001 {
            continue;
        }

        // Apply rotation
        camera.yaw -= delta.x * camera.look_sensitivity;
        camera.pitch += delta.y * camera.look_sensitivity;

        // Clamp pitch to avoid gimbal lock
        camera.pitch = camera.pitch.clamp(-FRAC_PI_2 + 0.1, FRAC_PI_2 - 0.1);

        // Update transform rotation
        transform.rotation = Quat::from_euler(EulerRot::YXZ, -camera.yaw, camera.pitch, 0.0);
    }
}

/// System that handles camera zoom via scroll wheel
fn camera_zoom(
    mut scroll_events: MessageReader<MouseWheel>,
    mut query: Query<(&mut Transform, &FreeCamera)>,
) {
    let scroll_delta: f32 = scroll_events
        .read()
        .map(|e| match e.unit {
            MouseScrollUnit::Line => e.y * 0.5,
            MouseScrollUnit::Pixel => e.y * 0.01,
        })
        .sum();

    if scroll_delta.abs() < 0.01 {
        return;
    }

    for (mut transform, camera) in query.iter_mut() {
        // Move forward/backward based on scroll
        let forward = camera.forward();
        transform.translation += forward * scroll_delta * camera.move_speed * 0.5;
    }
}

/// System that handles camera keyboard shortcuts
fn camera_shortcuts(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &mut FreeCamera)>,
    model_bounds: Res<ModelBounds>,
) {
    for (mut transform, mut camera) in query.iter_mut() {
        // Home: Reset camera to default position
        if keyboard.just_pressed(KeyCode::Home) {
            camera.yaw = FRAC_PI_4;
            camera.pitch = FRAC_PI_6;
            transform.translation = model_bounds.center + Vec3::new(0.0, 2.0, 10.0);
            transform.rotation = Quat::from_euler(EulerRot::YXZ, -camera.yaw, camera.pitch, 0.0);
            info!("Camera reset to default");
        }

        // F: Frame model (fit to view)
        if keyboard.just_pressed(KeyCode::KeyF) {
            let distance = model_bounds.radius * 2.5;
            camera.yaw = FRAC_PI_4;
            camera.pitch = FRAC_PI_6;

            // Position camera to see the whole model
            let offset = Vec3::new(
                distance * camera.yaw.sin() * camera.pitch.cos(),
                distance * camera.pitch.sin() + model_bounds.center.y,
                distance * camera.yaw.cos() * camera.pitch.cos(),
            );
            transform.translation = model_bounds.center + offset;
            transform.rotation = Quat::from_euler(EulerRot::YXZ, -camera.yaw, camera.pitch, 0.0);
            info!("Camera framed to model");
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

    // Spawn the camera with free camera controller
    let camera = FreeCamera::default();
    let initial_transform = Transform::from_xyz(0.0, 5.0, 15.0).with_rotation(Quat::from_euler(
        EulerRot::YXZ,
        -camera.yaw,
        camera.pitch,
        0.0,
    ));

    commands.spawn((Camera3d::default(), initial_transform, camera));

    // Add ambient light
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 500.0,
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
    // The model path is relative to the assets directory
    let model_path = "models/glb/grandmas_house/scene.gltf#Scene0";
    info!("Loading model: {}", model_path);

    let scene_handle: Handle<Scene> = asset_server.load(model_path);
    model_state.handle = Some(scene_handle);

    info!("GLB viewer setup complete!");
    info!("Controls:");
    info!("  WASD - Move");
    info!("  Space/Shift - Up/Down");
    info!("  Right-click + drag - Look around");
    info!("  Scroll - Zoom");
    info!("  Home - Reset camera");
    info!("  F - Frame model");
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
fn calculate_bounds(mut bounds: ResMut<ModelBounds>, query: Query<&GlobalTransform, With<Mesh3d>>) {
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
            "Model bounds calculated: center={:?}, radius={:.2}",
            bounds.center, bounds.radius
        );
    }
}

/// System to log camera info periodically (placeholder for UI)
fn debug_info(
    time: Res<Time>,
    camera_query: Query<(&Transform, &FreeCamera)>,
    model_state: Res<ModelLoadState>,
    bounds: Res<ModelBounds>,
    mut last_log: Local<f32>,
) {
    // Log every 5 seconds
    *last_log += time.delta_secs();
    if *last_log < 5.0 {
        return;
    }
    *last_log = 0.0;

    if let Ok((transform, camera)) = camera_query.single() {
        let pos = transform.translation;
        info!(
            "Camera pos: ({:.1}, {:.1}, {:.1}), looking: {}, model loaded: {}",
            pos.x, pos.y, pos.z, camera.looking, model_state.spawned
        );
        if bounds.loaded {
            info!(
                "Model center: {:?}, radius: {:.1}",
                bounds.center, bounds.radius
            );
        }
    }
}

/// Plugin that bundles all camera controls
pub struct FreeCameraPlugin;

impl Plugin for FreeCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (camera_movement, camera_look, camera_zoom, camera_shortcuts),
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
        .add_plugins(FreeCameraPlugin)
        .init_resource::<ModelBounds>()
        .init_resource::<ModelLoadState>()
        .add_systems(Startup, setup)
        .add_systems(Update, (spawn_model, calculate_bounds, debug_info))
        .run();
}
