//! Avatar - an Entity with Nostr identity

use crate::entity::{Entity, Logic};
use crate::identity::Identity;
use crossworld_physics::Object;
use glam::{Quat, Vec3};

pub mod manager;
#[allow(unused_imports)]
pub use manager::AvatarManager;

/// Avatar: an Entity with Nostr identity
///
/// Represents a player in the world with both physical presence
/// (position, rotation) and social identity (npub, display name).
#[derive(Debug, Clone)]
pub struct Avatar {
    /// The underlying entity (position/rotation)
    entity: Entity,
    /// Nostr public key
    npub: String,
    /// Display name from Nostr profile
    display_name: Option<String>,
    /// Avatar image URL from Nostr profile
    avatar_url: Option<String>,
}

impl Avatar {
    /// Create a new avatar with the given npub
    pub fn new(npub: impl Into<String>) -> Self {
        Self {
            entity: Entity::new(),
            npub: npub.into(),
            display_name: None,
            avatar_url: None,
        }
    }

    /// Create an avatar at a specific position
    pub fn at_position(npub: impl Into<String>, position: Vec3) -> Self {
        Self {
            entity: Entity::at_position(position),
            npub: npub.into(),
            display_name: None,
            avatar_url: None,
        }
    }

    /// Set display name
    pub fn with_display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = Some(name.into());
        self
    }

    /// Set avatar URL
    pub fn with_avatar_url(mut self, url: impl Into<String>) -> Self {
        self.avatar_url = Some(url.into());
        self
    }

    /// Get mutable reference to underlying entity
    pub fn entity_mut(&mut self) -> &mut Entity {
        &mut self.entity
    }

    /// Get reference to underlying entity
    pub fn entity(&self) -> &Entity {
        &self.entity
    }
}

impl Object for Avatar {
    fn position(&self) -> Vec3 {
        self.entity.position()
    }

    fn rotation(&self) -> Quat {
        self.entity.rotation()
    }

    fn set_position(&mut self, position: Vec3) {
        self.entity.set_position(position);
    }

    fn set_rotation(&mut self, rotation: Quat) {
        self.entity.set_rotation(rotation);
    }
}

impl Logic for Avatar {
    fn update(&mut self, dt: f32) {
        self.entity.update(dt);
    }
}

impl Identity for Avatar {
    fn npub(&self) -> &str {
        &self.npub
    }

    fn display_name(&self) -> Option<&str> {
        self.display_name.as_deref()
    }

    fn avatar_url(&self) -> Option<&str> {
        self.avatar_url.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avatar_new() {
        let avatar = Avatar::new("npub1test...");
        assert_eq!(avatar.npub(), "npub1test...");
        assert_eq!(avatar.position(), Vec3::ZERO);
    }

    #[test]
    fn test_avatar_with_profile() {
        let avatar = Avatar::new("npub1test...")
            .with_display_name("Alice")
            .with_avatar_url("https://example.com/avatar.png");

        assert_eq!(avatar.display_name(), Some("Alice"));
        assert_eq!(avatar.avatar_url(), Some("https://example.com/avatar.png"));
    }

    #[test]
    fn test_avatar_position() {
        let pos = Vec3::new(10.0, 5.0, 20.0);
        let avatar = Avatar::at_position("npub1test...", pos);
        assert_eq!(avatar.position(), pos);
    }
}
