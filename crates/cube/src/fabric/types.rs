//! Type definitions for the fabric system

use serde::{Deserialize, Serialize};

/// Configuration for fabric generation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FabricConfig {
    /// Magnitude at origin (|Q| < 1 = inside/solid)
    /// Typical value: 0.5
    pub root_magnitude: f32,

    /// Magnitude at max distance (|Q| > 1 = outside/air)
    /// Typical value: 2.0
    pub boundary_magnitude: f32,

    /// Distance where |Q| = 1.0 (surface), relative to world half-size
    /// Typical value: 0.8 (sphere surface at 80% of world half-size)
    pub surface_radius: f32,

    /// Additive states per depth level for noise/variation
    pub additive_states: Vec<AdditiveState>,

    /// Default maximum rendering depth
    pub max_depth: u32,
}

impl Default for FabricConfig {
    fn default() -> Self {
        Self {
            root_magnitude: 0.5,
            boundary_magnitude: 2.0,
            surface_radius: 0.8,
            additive_states: vec![
                AdditiveState::default(),    // depth 0
                AdditiveState::new(0.1, 0.05), // depth 1
                AdditiveState::new(0.2, 0.1),  // depth 2
                AdditiveState::new(0.3, 0.15), // depth 3
                AdditiveState::new(0.4, 0.2),  // depth 4
            ],
            max_depth: 5,
        }
    }
}

/// Additive state per depth level
/// Applied as noise/variation to quaternion values at each depth
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AdditiveState {
    /// Rotation noise in radians (applied as random axis rotation)
    pub rotation: f32,

    /// Magnitude noise (added to base magnitude)
    pub magnitude: f32,
}

impl Default for AdditiveState {
    fn default() -> Self {
        Self {
            rotation: 0.0,
            magnitude: 0.0,
        }
    }
}

impl AdditiveState {
    pub fn new(rotation: f32, magnitude: f32) -> Self {
        Self {
            rotation,
            magnitude,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fabric_config_default() {
        let config = FabricConfig::default();
        assert_eq!(config.root_magnitude, 0.5);
        assert_eq!(config.boundary_magnitude, 2.0);
        assert_eq!(config.surface_radius, 0.8);
        assert_eq!(config.max_depth, 5);
        assert!(!config.additive_states.is_empty());
    }

    #[test]
    fn test_additive_state_default() {
        let state = AdditiveState::default();
        assert_eq!(state.rotation, 0.0);
        assert_eq!(state.magnitude, 0.0);
    }

    #[test]
    fn test_additive_state_new() {
        let state = AdditiveState::new(0.5, 0.25);
        assert_eq!(state.rotation, 0.5);
        assert_eq!(state.magnitude, 0.25);
    }
}
