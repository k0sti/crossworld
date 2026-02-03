//! Condition patterns for rule matching

use glam::IVec3;
use serde::{Deserialize, Serialize};

/// Condition for rule matching
///
/// Conditions are evaluated relative to a target voxel position.
/// Multiple conditions in a rule are combined with AND logic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Condition {
    /// Match if voxel at offset has specific material
    MaterialAt {
        /// Offset from target position
        offset: IVec3,
        /// Material value to match
        material: u8,
    },

    /// Match if voxel at offset is empty (material == 0)
    EmptyAt {
        /// Offset from target position
        offset: IVec3,
    },

    /// Match if voxel at offset is solid (material != 0)
    SolidAt {
        /// Offset from target position
        offset: IVec3,
    },

    /// Match if material at offset is in the given range
    MaterialInRange {
        /// Offset from target position
        offset: IVec3,
        /// Minimum material value (inclusive)
        min: u8,
        /// Maximum material value (inclusive)
        max: u8,
    },

    /// Match if position is within depth range
    DepthInRange {
        /// Minimum depth (inclusive)
        min: u32,
        /// Maximum depth (inclusive)
        max: u32,
    },

    /// Match based on neighbor count
    NeighborCount {
        /// Minimum number of solid neighbors (0-6)
        min: u8,
        /// Maximum number of solid neighbors (0-6)
        max: u8,
    },

    /// Logical AND of multiple conditions
    And(Vec<Condition>),

    /// Logical OR of multiple conditions
    Or(Vec<Condition>),

    /// Negate a condition
    Not(Box<Condition>),

    /// Always matches (useful for default rules)
    Always,

    /// Never matches (useful for disabled rules)
    Never,
}

impl Condition {
    /// Create a condition that matches a specific material at the target position
    pub fn material(material: u8) -> Self {
        Condition::MaterialAt {
            offset: IVec3::ZERO,
            material,
        }
    }

    /// Create a condition that matches empty space at the target position
    pub fn empty() -> Self {
        Condition::EmptyAt {
            offset: IVec3::ZERO,
        }
    }

    /// Create a condition that matches solid voxel at the target position
    pub fn solid() -> Self {
        Condition::SolidAt {
            offset: IVec3::ZERO,
        }
    }

    /// Create a condition that matches a material at an offset
    pub fn material_at(offset: IVec3, material: u8) -> Self {
        Condition::MaterialAt { offset, material }
    }

    /// Create a condition that matches empty space at an offset
    pub fn empty_at(offset: IVec3) -> Self {
        Condition::EmptyAt { offset }
    }

    /// Create a condition that matches solid voxel at an offset
    pub fn solid_at(offset: IVec3) -> Self {
        Condition::SolidAt { offset }
    }

    /// Combine this condition with another using AND
    pub fn and(self, other: Condition) -> Self {
        match (self, other) {
            (Condition::And(mut a), Condition::And(b)) => {
                a.extend(b);
                Condition::And(a)
            }
            (Condition::And(mut a), other) => {
                a.push(other);
                Condition::And(a)
            }
            (this, Condition::And(mut a)) => {
                a.insert(0, this);
                Condition::And(a)
            }
            (a, b) => Condition::And(vec![a, b]),
        }
    }

    /// Combine this condition with another using OR
    pub fn or(self, other: Condition) -> Self {
        match (self, other) {
            (Condition::Or(mut a), Condition::Or(b)) => {
                a.extend(b);
                Condition::Or(a)
            }
            (Condition::Or(mut a), other) => {
                a.push(other);
                Condition::Or(a)
            }
            (this, Condition::Or(mut a)) => {
                a.insert(0, this);
                Condition::Or(a)
            }
            (a, b) => Condition::Or(vec![a, b]),
        }
    }

    /// Negate this condition
    pub fn not(self) -> Self {
        Condition::Not(Box::new(self))
    }
}

impl Default for Condition {
    fn default() -> Self {
        Condition::Always
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_condition_builders() {
        let c = Condition::material(5);
        assert!(
            matches!(c, Condition::MaterialAt { offset, material } if offset == IVec3::ZERO && material == 5)
        );

        let c = Condition::empty_at(IVec3::Y);
        assert!(matches!(c, Condition::EmptyAt { offset } if offset == IVec3::Y));
    }

    #[test]
    fn test_condition_combinators() {
        let c1 = Condition::material(1);
        let c2 = Condition::empty_at(IVec3::Y);
        let combined = c1.and(c2);

        assert!(matches!(combined, Condition::And(v) if v.len() == 2));
    }

    #[test]
    fn test_condition_serialization() {
        let condition = Condition::MaterialAt {
            offset: IVec3::new(1, 2, 3),
            material: 42,
        };

        let json = serde_json::to_string(&condition).unwrap();
        let deserialized: Condition = serde_json::from_str(&json).unwrap();

        assert_eq!(condition, deserialized);
    }
}
