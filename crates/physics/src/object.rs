//! Object trait for types with position and rotation in 3D space
//!
//! This module defines the Object trait for WASM builds.
//! For native builds, the trait is re-exported from the app crate.

use glam::{Quat, Vec3};

/// Base trait for any object with position and rotation in 3D space.
///
/// This trait provides a common interface for objects that have a transform
/// (position and rotation). It's designed for objects where the transform
/// can be accessed without external context.
///
/// Implemented by:
/// - `Camera` in app/renderer crate
/// - `Entity` and `Avatar` in world crate
///
/// Note: Physics objects like `CubeObject` and `CharacterController` require
/// a `&PhysicsWorld` reference to access their transform, so they don't
/// implement this trait directly.
pub trait Object {
    /// Get the current position
    fn position(&self) -> Vec3;

    /// Get the current rotation as a quaternion
    fn rotation(&self) -> Quat;

    /// Set the position
    fn set_position(&mut self, position: Vec3);

    /// Set the rotation
    fn set_rotation(&mut self, rotation: Quat);
}
