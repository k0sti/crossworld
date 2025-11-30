mod character_controller;
mod collider;
mod cube_object;
mod world;

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

// Re-export for convenience
pub use glam;
pub use rapier3d;

#[cfg(feature = "wasm")]
pub use wasm::WasmPhysicsWorld;
