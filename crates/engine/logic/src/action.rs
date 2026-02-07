//! Actions that can be dispatched by rules

use glam::IVec3;
use serde::{Deserialize, Serialize};

/// Action to perform when a rule matches
///
/// Actions describe the changes to apply when rule conditions are met.
/// Multiple actions can be combined in a single rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Action {
    /// Set voxel material at offset from matched position
    SetVoxel {
        /// Offset from matched position
        offset: IVec3,
        /// Material value to set
        material: u8,
    },

    /// Clear (remove) voxel at offset from matched position
    ClearVoxel {
        /// Offset from matched position
        offset: IVec3,
    },

    /// Fill a region with a material
    FillRegion {
        /// Minimum corner offset (inclusive)
        min: IVec3,
        /// Maximum corner offset (inclusive)
        max: IVec3,
        /// Material to fill with
        material: u8,
    },

    /// Replace one material with another in a region
    Replace {
        /// Minimum corner offset (inclusive)
        min: IVec3,
        /// Maximum corner offset (inclusive)
        max: IVec3,
        /// Material to replace
        from: u8,
        /// Material to replace with
        to: u8,
    },

    /// Copy a region to another location
    CopyRegion {
        /// Source minimum corner offset
        src_min: IVec3,
        /// Source maximum corner offset
        src_max: IVec3,
        /// Destination offset
        dst_offset: IVec3,
    },

    /// Spawn an entity at a position
    Spawn {
        /// Offset from matched position
        offset: IVec3,
        /// Entity type identifier
        entity_type: String,
    },

    /// Emit an event (for external handlers)
    Emit {
        /// Event name
        event: String,
        /// Optional payload data (JSON-serializable)
        payload: Option<String>,
    },

    /// No-op action (useful for rules that only trigger events)
    None,
}

impl Action {
    /// Create a set voxel action at the matched position
    pub fn set(material: u8) -> Self {
        Action::SetVoxel {
            offset: IVec3::ZERO,
            material,
        }
    }

    /// Create a set voxel action at an offset
    pub fn set_at(offset: IVec3, material: u8) -> Self {
        Action::SetVoxel { offset, material }
    }

    /// Create a clear voxel action at the matched position
    pub fn clear() -> Self {
        Action::ClearVoxel {
            offset: IVec3::ZERO,
        }
    }

    /// Create a clear voxel action at an offset
    pub fn clear_at(offset: IVec3) -> Self {
        Action::ClearVoxel { offset }
    }

    /// Create a fill region action
    pub fn fill(min: IVec3, max: IVec3, material: u8) -> Self {
        Action::FillRegion { min, max, material }
    }

    /// Create a replace action for a single position
    pub fn replace(from: u8, to: u8) -> Self {
        Action::Replace {
            min: IVec3::ZERO,
            max: IVec3::ZERO,
            from,
            to,
        }
    }

    /// Create a spawn action at the matched position
    pub fn spawn(entity_type: impl Into<String>) -> Self {
        Action::Spawn {
            offset: IVec3::ZERO,
            entity_type: entity_type.into(),
        }
    }

    /// Create an emit event action
    pub fn emit(event: impl Into<String>) -> Self {
        Action::Emit {
            event: event.into(),
            payload: None,
        }
    }

    /// Create an emit event action with payload
    pub fn emit_with_payload(event: impl Into<String>, payload: impl Into<String>) -> Self {
        Action::Emit {
            event: event.into(),
            payload: Some(payload.into()),
        }
    }
}

impl Default for Action {
    fn default() -> Self {
        Action::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_builders() {
        let a = Action::set(5);
        assert!(
            matches!(a, Action::SetVoxel { offset, material } if offset == IVec3::ZERO && material == 5)
        );

        let a = Action::clear_at(IVec3::Y);
        assert!(matches!(a, Action::ClearVoxel { offset } if offset == IVec3::Y));
    }

    #[test]
    fn test_action_serialization() {
        let action = Action::FillRegion {
            min: IVec3::new(-1, -1, -1),
            max: IVec3::new(1, 1, 1),
            material: 3,
        };

        let json = serde_json::to_string(&action).unwrap();
        let deserialized: Action = serde_json::from_str(&json).unwrap();

        assert_eq!(action, deserialized);
    }
}
