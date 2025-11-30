use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
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
        .add_systems(Startup, setup)
        .add_systems(Update, debug_info)
        .run();
}

/// Initial setup system
fn setup(
    mut commands: Commands,
    config: Res<ProtoConfig>,
) {
    info!("Setting up scene...");

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

/// Debug info display
fn debug_info(
    diagnostics: Res<DiagnosticsStore>,
) {
    // Display FPS every second (simplified - would use proper timing in full implementation)
    if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(value) = fps.smoothed() {
            if value < 45.0 {
                warn!("FPS: {:.1}", value);
            }
        }
    }
}
