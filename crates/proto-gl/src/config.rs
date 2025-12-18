use serde::Deserialize;
use std::error::Error;

/// Configuration loaded from config.toml
#[derive(Debug, Deserialize, Clone)]
pub struct ProtoGlConfig {
    pub world: WorldConfig,
    pub physics: PhysicsConfig,
    pub spawning: SpawningConfig,
    pub rendering: RenderConfig,
    #[serde(default)]
    pub fps: FpsConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WorldConfig {
    pub macro_depth: u32,
    pub micro_depth: u32,
    pub border_depth: u32,
    #[serde(default = "default_border_materials")]
    pub border_materials: [u8; 4],
    pub root_cube: String,
}

fn default_border_materials() -> [u8; 4] {
    [32, 32, 0, 0] // Bottom: bedrock, Top: air
}

#[derive(Debug, Deserialize, Clone)]
pub struct PhysicsConfig {
    pub gravity: f32,
    pub timestep: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SpawningConfig {
    pub spawn_count: u32,
    pub models_path: String,
    pub min_height: f32,
    pub max_height: f32,
    pub spawn_radius: f32,
    #[serde(default = "default_object_size")]
    pub object_size: f32,
}

fn default_object_size() -> f32 {
    0.5
}

#[derive(Debug, Deserialize, Clone)]
pub struct RenderConfig {
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub camera_distance: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FpsConfig {
    /// Movement speed in units per second
    pub move_speed: f32,
    /// Mouse sensitivity for look-around
    pub mouse_sensitivity: f32,
    /// Camera height offset (eye level)
    pub eye_height: f32,
    /// Camera capsule radius for collision
    pub collision_radius: f32,
    /// Spawn position for FPS camera
    pub spawn_position: [f32; 3],
}

impl Default for FpsConfig {
    fn default() -> Self {
        Self {
            move_speed: 5.0,
            mouse_sensitivity: 0.003,
            eye_height: 1.7,
            collision_radius: 0.3,
            spawn_position: [0.5, 0.8, 0.5],
        }
    }
}

impl Default for ProtoGlConfig {
    fn default() -> Self {
        Self {
            world: WorldConfig {
                macro_depth: 3,
                micro_depth: 4,
                border_depth: 1,
                border_materials: [32, 32, 0, 0],
                root_cube: ">a [5 5 4 9 5 5 0 0]".to_string(),
            },
            physics: PhysicsConfig {
                gravity: -9.81,
                timestep: 0.016666,
            },
            spawning: SpawningConfig {
                spawn_count: 10,
                models_path: "assets/models/".to_string(),
                // Heights and spawn radius in [0,1]³ world space
                min_height: 0.6,
                max_height: 0.9,
                spawn_radius: 0.3,
                object_size: 0.05,
            },
            rendering: RenderConfig {
                viewport_width: 1000,
                viewport_height: 750,
                // Camera distance appropriate for [0,1]³ world
                camera_distance: 2.0,
            },
            fps: FpsConfig::default(),
        }
    }
}

/// Load configuration from config.toml
pub fn load_config() -> Result<ProtoGlConfig, Box<dyn Error>> {
    let config_path = "crates/proto-gl/config.toml";
    let config_str = std::fs::read_to_string(config_path)?;
    let config: ProtoGlConfig = toml::from_str(&config_str)?;
    Ok(config)
}
