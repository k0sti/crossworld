// Proto-GL Physics Viewer Library
// Modular structure for the physics testing viewer

pub mod app;
pub mod camera;
pub mod config;
pub mod models;
pub mod physics;
pub mod ui;
pub mod world;

// Re-export commonly used types
pub use app::ProtoGlApp;
pub use camera::OrbitCamera;
pub use config::{ProtoGlConfig, WorldConfig, PhysicsConfig, SpawningConfig, RenderConfig};
pub use models::{VoxModel, CubeObject};
