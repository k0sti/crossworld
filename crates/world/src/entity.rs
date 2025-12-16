//! Entity system for objects in the world
//!
//! An Entity is an object with position, rotation, and updateable logic.

use crossworld_physics::Object;
use glam::{Quat, Vec3};

/// Behavior/logic trait for entities
///
/// Implement this trait to add custom update behavior to entities.
pub trait Logic {
    /// Update the entity's state
    ///
    /// Called each frame with the time delta in seconds.
    fn update(&mut self, dt: f32);
}

/// An entity in the world with position, rotation, and optional behavior.
///
/// Entities implement the `Object` trait for transform access.
#[derive(Debug, Clone)]
pub struct Entity {
    /// Position in world space
    pub position: Vec3,
    /// Rotation as quaternion
    pub rotation: Quat,
}

impl Entity {
    /// Create a new entity at the origin
    pub fn new() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        }
    }

    /// Create a new entity at the given position
    pub fn at_position(position: Vec3) -> Self {
        Self {
            position,
            rotation: Quat::IDENTITY,
        }
    }

    /// Create a new entity with position and rotation
    pub fn with_transform(position: Vec3, rotation: Quat) -> Self {
        Self { position, rotation }
    }
}

impl Default for Entity {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for Entity {
    fn position(&self) -> Vec3 {
        self.position
    }

    fn rotation(&self) -> Quat {
        self.rotation
    }

    fn set_position(&mut self, position: Vec3) {
        self.position = position;
    }

    fn set_rotation(&mut self, rotation: Quat) {
        self.rotation = rotation;
    }
}

impl Logic for Entity {
    fn update(&mut self, _dt: f32) {
        // Default: no-op
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_default() {
        let entity = Entity::new();
        assert_eq!(entity.position(), Vec3::ZERO);
        assert_eq!(entity.rotation(), Quat::IDENTITY);
    }

    #[test]
    fn test_entity_at_position() {
        let pos = Vec3::new(1.0, 2.0, 3.0);
        let entity = Entity::at_position(pos);
        assert_eq!(entity.position(), pos);
    }

    #[test]
    fn test_entity_object_trait() {
        let mut entity = Entity::new();
        let new_pos = Vec3::new(5.0, 10.0, 15.0);
        entity.set_position(new_pos);
        assert_eq!(entity.position(), new_pos);
    }
}
