use bevy::prelude::*;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy_rapier3d::prelude::*;
use crossworld_world::WorldCube;
use serde::Deserialize;
use std::path::Path;

/// Configuration loaded from config.toml
#[derive(Debug, Deserialize, Resource)]
struct ProtoConfig {
    world: WorldConfig,
    physics: PhysicsConfig,
    spawning: SpawningConfig,
    player: PlayerConfig,
}

#[derive(Debug, Deserialize)]
struct WorldConfig {
    macro_depth: u32,
    micro_depth: u32,
    border_depth: u32,
    seed: u32,
}

#[derive(Debug, Deserialize)]
struct PhysicsConfig {
    gravity: f32,
    timestep: f32,
}

#[derive(Debug, Deserialize)]
struct SpawningConfig {
    spawn_count: u32,
    models_path: String,
    min_height: f32,
    max_height: f32,
    spawn_radius: f32,
}

#[derive(Debug, Deserialize)]
struct PlayerConfig {
    move_speed: f32,
    jump_force: f32,
    camera_distance: f32,
}

impl Default for ProtoConfig {
    fn default() -> Self {
        Self {
            world: WorldConfig {
                macro_depth: 3,
                micro_depth: 4,
                border_depth: 1,
                seed: 12345,
            },
            physics: PhysicsConfig {
                gravity: -9.81,
                timestep: 0.016666,
            },
            spawning: SpawningConfig {
                spawn_count: 20,
                models_path: "packages/app/dist/assets/models/vox/".to_string(),
                min_height: 20.0,
                max_height: 50.0,
                spawn_radius: 30.0,
            },
            player: PlayerConfig {
                move_speed: 5.0,
                jump_force: 8.0,
                camera_distance: 10.0,
            },
        }
    }
}

/// Component for the world entity
#[derive(Component)]
struct WorldEntity;

/// Camera controller resource for orbit camera
#[derive(Resource)]
struct CameraController {
    pub focus: Vec3,
    pub radius: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub sensitivity: f32,
    pub zoom_speed: f32,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            focus: Vec3::ZERO,
            radius: 20.0,
            pitch: -0.4, // Looking down slightly
            yaw: 0.0,
            sensitivity: 0.003,
            zoom_speed: 1.0,
        }
    }
}

fn load_config() -> ProtoConfig {
    let config_path = Path::new("crates/proto/config.toml");

    if config_path.exists() {
        match std::fs::read_to_string(config_path) {
            Ok(contents) => match toml::from_str(&contents) {
                Ok(config) => {
                    info!("Loaded configuration from {:?}", config_path);
                    return config;
                }
                Err(e) => {
                    warn!("Failed to parse config.toml: {}, using defaults", e);
                }
            },
            Err(e) => {
                warn!("Failed to read config.toml: {}, using defaults", e);
            }
        }
    } else {
        warn!("config.toml not found at {:?}, using defaults", config_path);
    }

    ProtoConfig::default()
}

fn main() {
    // Load configuration
    let config = load_config();

    info!("=== Bevy Physics Prototype ===");
    info!("World depth: macro={}, micro={}", config.world.macro_depth, config.world.micro_depth);
    info!("Spawn count: {}", config.spawning.spawn_count);

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .insert_resource(config)
        .init_resource::<CameraController>()
        .add_systems(Startup, setup)
        .add_systems(Update, camera_controls)
        .run();
}

/// Initial setup system
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<ProtoConfig>,
) {
    info!("Setting up scene...");

    // Generate world from config
    info!("Generating world: macro_depth={}, micro_depth={}, seed={}",
        config.world.macro_depth, config.world.micro_depth, config.world.seed);

    let world_cube = WorldCube::new(
        config.world.macro_depth,
        config.world.micro_depth,
        config.world.border_depth,
        config.world.seed,
    );

    // Generate mesh
    let geometry_data = world_cube.generate_mesh();

    info!("World mesh generated: {} vertices, {} indices",
        geometry_data.vertices.len() / 3,
        geometry_data.indices.len());

    // Convert to Bevy mesh
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());

    // Positions (Vec3)
    let positions: Vec<[f32; 3]> = geometry_data
        .vertices
        .chunks(3)
        .map(|chunk| [chunk[0], chunk[1], chunk[2]])
        .collect();

    // Normals (Vec3)
    let normals: Vec<[f32; 3]> = geometry_data
        .normals
        .chunks(3)
        .map(|chunk| [chunk[0], chunk[1], chunk[2]])
        .collect();

    // Colors (Vec3)
    let colors: Vec<[f32; 3]> = geometry_data
        .colors
        .chunks(3)
        .map(|chunk| [chunk[0], chunk[1], chunk[2]])
        .collect();

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(geometry_data.indices.clone()));

    // Spawn world entity with physics
    info!("Spawning world entity with collider...");

    commands.spawn((
        WorldEntity,
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
        RigidBody::Fixed,
        // TODO: Add collider using VoxelColliderBuilder
    ));

    // Add lighting
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Add camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(10.0, 10.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    info!("Scene setup complete");
    info!("Configuration: gravity={}, timestep={}", config.physics.gravity, config.physics.timestep);
}

/// Camera controls system - orbit camera with mouse
fn camera_controls(
    mut controller: ResMut<CameraController>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut mouse_wheel: EventReader<MouseWheel>,
    mut query: Query<&mut Transform, With<Camera3d>>,
) {
    // Right mouse button drag to rotate
    if mouse_buttons.pressed(MouseButton::Right) {
        for motion in mouse_motion.read() {
            controller.yaw -= motion.delta.x * controller.sensitivity;
            controller.pitch -= motion.delta.y * controller.sensitivity;

            // Clamp pitch to avoid gimbal lock
            controller.pitch = controller.pitch.clamp(-1.5, 1.5);
        }
    } else {
        // Clear events if not using them
        mouse_motion.clear();
    }

    // Mouse wheel to zoom
    for wheel in mouse_wheel.read() {
        controller.radius -= wheel.y * controller.zoom_speed;
        controller.radius = controller.radius.clamp(5.0, 100.0);
    }

    // Update camera transform
    if let Ok(mut transform) = query.get_single_mut() {
        // Calculate camera position from spherical coordinates
        let x = controller.radius * controller.pitch.cos() * controller.yaw.sin();
        let y = controller.radius * controller.pitch.sin();
        let z = controller.radius * controller.pitch.cos() * controller.yaw.cos();

        transform.translation = controller.focus + Vec3::new(x, y, z);
        transform.look_at(controller.focus, Vec3::Y);
    }
}
