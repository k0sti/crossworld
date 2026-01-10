mod character_controller;
mod collider;
pub mod collision;
mod cube_object;
pub mod sdf;
pub mod terrain;
mod world;
pub mod world_collider;

// Object trait - use local definition for WASM, re-export from app for native
#[cfg(target_arch = "wasm32")]
mod object;

// Only compile WASM bindings when "wasm" feature is enabled
#[cfg(feature = "wasm")]
mod wasm;

// Native-only utilities for Bevy integration
#[cfg(not(target_arch = "wasm32"))]
pub mod native;

pub use character_controller::{CharacterController, CharacterControllerConfig, RaycastHit};
pub use collider::{
    create_box_collider, create_capsule_collider, create_sphere_collider, VoxelColliderBuilder,
};
pub use cube_object::CubeObject;
pub use world::PhysicsWorld;

// Re-export Object trait - from app on native, from local module on WASM
#[cfg(not(target_arch = "wasm32"))]
pub use app::Object;
#[cfg(target_arch = "wasm32")]
pub use object::Object;

// Re-export for convenience
pub use glam;
pub use rapier3d;

#[cfg(feature = "wasm")]
pub use wasm::WasmPhysicsWorld;
