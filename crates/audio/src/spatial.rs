//! 3D spatial audio support
//!
//! Provides types for positioning audio sources in 3D space relative to
//! a listener, with distance attenuation and panning.

use crate::AudioPosition;
use glam::Vec3;

/// Distance attenuation model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum DistanceModel {
    /// No distance attenuation
    None,
    /// Linear attenuation: 1 - (distance - refDistance) / (maxDistance - refDistance)
    Linear,
    /// Inverse distance: refDistance / (refDistance + rolloff * (distance - refDistance))
    #[default]
    Inverse,
    /// Exponential: (distance / refDistance) ^ -rolloff
    Exponential,
}

/// Configuration for spatial audio
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpatialConfig {
    /// Distance attenuation model
    pub distance_model: DistanceModel,
    /// Reference distance for attenuation (at this distance, volume is unchanged)
    pub ref_distance: f32,
    /// Maximum distance for attenuation (beyond this, volume is zero or constant)
    pub max_distance: f32,
    /// Rolloff factor (higher = faster attenuation)
    pub rolloff_factor: f32,
    /// Whether to apply Doppler effect
    pub doppler_enabled: bool,
    /// Doppler factor (1.0 = physically accurate)
    pub doppler_factor: f32,
    /// Speed of sound in units per second (default: 343 m/s)
    pub speed_of_sound: f32,
}

impl Default for SpatialConfig {
    fn default() -> Self {
        Self {
            distance_model: DistanceModel::Inverse,
            ref_distance: 1.0,
            max_distance: 100.0,
            rolloff_factor: 1.0,
            doppler_enabled: false,
            doppler_factor: 1.0,
            speed_of_sound: 343.0,
        }
    }
}

impl SpatialConfig {
    /// Calculate distance attenuation factor (0.0 to 1.0)
    pub fn calculate_attenuation(&self, distance: f32) -> f32 {
        let distance = distance.max(self.ref_distance);

        match self.distance_model {
            DistanceModel::None => 1.0,
            DistanceModel::Linear => {
                let clamped = distance.min(self.max_distance);
                1.0 - self.rolloff_factor
                    * (clamped - self.ref_distance)
                    / (self.max_distance - self.ref_distance)
            }
            DistanceModel::Inverse => {
                self.ref_distance
                    / (self.ref_distance
                        + self.rolloff_factor * (distance - self.ref_distance).max(0.0))
            }
            DistanceModel::Exponential => {
                (distance / self.ref_distance).powf(-self.rolloff_factor)
            }
        }
        .clamp(0.0, 1.0)
    }

    /// Calculate Doppler pitch shift
    ///
    /// Returns a pitch multiplier (1.0 = no change, > 1.0 = higher pitch, < 1.0 = lower pitch)
    pub fn calculate_doppler(
        &self,
        listener_velocity: Vec3,
        source_velocity: Vec3,
        direction_to_source: Vec3,
    ) -> f32 {
        if !self.doppler_enabled || direction_to_source.length_squared() < 0.001 {
            return 1.0;
        }

        let dir_normalized = direction_to_source.normalize();

        // Project velocities onto the direction vector (toward source)
        // Positive = moving toward source, Negative = moving away from source
        let listener_toward_source = listener_velocity.dot(dir_normalized);
        let source_toward_source = source_velocity.dot(dir_normalized);

        // Doppler formula: f' = f * (c + vL) / (c - vS)
        // where:
        // - c = speed of sound
        // - vL = listener velocity toward source (positive = approaching source)
        // - vS = source velocity toward source (positive = moving away from listener)
        //
        // When listener moves toward source (vL > 0): pitch increases
        // When source moves toward listener (vS < 0): pitch increases (we use -vS in denominator)
        let c = self.speed_of_sound;
        let factor = self.doppler_factor;

        let numerator = c + factor * listener_toward_source;
        let denominator = c - factor * source_toward_source;

        if denominator.abs() < 0.001 {
            return 1.0; // Avoid division by zero
        }

        (numerator / denominator).clamp(0.5, 2.0) // Limit extreme shifts
    }
}

/// Audio listener (typically attached to the camera/player)
#[derive(Debug, Clone)]
pub struct AudioListener {
    /// Current position
    position: AudioPosition,
    /// Forward direction
    forward: Vec3,
    /// Up direction
    up: Vec3,
    /// Spatial audio configuration
    config: SpatialConfig,
}

impl AudioListener {
    /// Create a new listener at the origin
    pub fn new(config: SpatialConfig) -> Self {
        Self {
            position: AudioPosition::default(),
            forward: Vec3::NEG_Z,
            up: Vec3::Y,
            config,
        }
    }

    /// Get current position
    pub fn position(&self) -> AudioPosition {
        self.position
    }

    /// Set position
    pub fn set_position(&mut self, position: AudioPosition) {
        self.position = position;
    }

    /// Get forward direction
    pub fn forward(&self) -> Vec3 {
        self.forward
    }

    /// Get up direction
    pub fn up(&self) -> Vec3 {
        self.up
    }

    /// Set orientation
    pub fn set_orientation(&mut self, forward: Vec3, up: Vec3) {
        self.forward = forward.normalize_or_zero();
        self.up = up.normalize_or_zero();
    }

    /// Get the right direction
    pub fn right(&self) -> Vec3 {
        self.forward.cross(self.up).normalize_or_zero()
    }

    /// Get the spatial config
    pub fn config(&self) -> &SpatialConfig {
        &self.config
    }

    /// Calculate panning values for a source position
    ///
    /// Returns (left, right) volume multipliers for stereo panning
    pub fn calculate_panning(&self, source_position: Vec3) -> (f32, f32) {
        let to_source = source_position - self.position.position;
        if to_source.length_squared() < 0.001 {
            return (1.0, 1.0); // Source is at listener position
        }

        let right = self.right();
        let pan = to_source.normalize().dot(right);

        // Convert pan (-1 to 1) to left/right gains
        // Using constant-power panning for smooth transitions
        let angle = (pan + 1.0) * 0.25 * std::f32::consts::PI;
        let left = angle.cos();
        let right = angle.sin();

        (left, right)
    }
}

/// A spatial audio source
#[derive(Debug, Clone)]
pub struct SpatialSource {
    /// Unique identifier
    id: u64,
    /// Current position
    position: AudioPosition,
    /// Spatial configuration
    config: SpatialConfig,
    /// Cached attenuation value
    cached_attenuation: f32,
    /// Cached panning (left, right)
    cached_panning: (f32, f32),
    /// Cached Doppler pitch shift
    cached_doppler: f32,
}

impl SpatialSource {
    /// Create a new spatial source
    pub fn new(id: u64, position: AudioPosition, config: SpatialConfig) -> Self {
        Self {
            id,
            position,
            config,
            cached_attenuation: 1.0,
            cached_panning: (1.0, 1.0),
            cached_doppler: 1.0,
        }
    }

    /// Get source ID
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Get current position
    pub fn position(&self) -> AudioPosition {
        self.position
    }

    /// Set position
    pub fn set_position(&mut self, position: AudioPosition) {
        self.position = position;
    }

    /// Get cached attenuation
    pub fn attenuation(&self) -> f32 {
        self.cached_attenuation
    }

    /// Get cached panning
    pub fn panning(&self) -> (f32, f32) {
        self.cached_panning
    }

    /// Get cached Doppler pitch shift
    pub fn doppler(&self) -> f32 {
        self.cached_doppler
    }

    /// Update spatial calculations relative to a listener
    pub fn update_relative_to(&mut self, listener: &AudioListener) {
        let to_source = self.position.position - listener.position.position;
        let distance = to_source.length();

        // Update attenuation
        self.cached_attenuation = self.config.calculate_attenuation(distance);

        // Update panning
        self.cached_panning = listener.calculate_panning(self.position.position);

        // Update Doppler
        self.cached_doppler = self.config.calculate_doppler(
            listener.position.velocity,
            self.position.velocity,
            to_source,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_attenuation_linear() {
        let config = SpatialConfig {
            distance_model: DistanceModel::Linear,
            ref_distance: 1.0,
            max_distance: 10.0,
            rolloff_factor: 1.0,
            ..Default::default()
        };

        assert!((config.calculate_attenuation(1.0) - 1.0).abs() < 0.001);
        assert!((config.calculate_attenuation(5.5) - 0.5).abs() < 0.001);
        assert!((config.calculate_attenuation(10.0) - 0.0).abs() < 0.001);
    }

    #[test]
    fn distance_attenuation_inverse() {
        let config = SpatialConfig {
            distance_model: DistanceModel::Inverse,
            ref_distance: 1.0,
            rolloff_factor: 1.0,
            ..Default::default()
        };

        assert!((config.calculate_attenuation(1.0) - 1.0).abs() < 0.001);
        assert!((config.calculate_attenuation(2.0) - 0.5).abs() < 0.001);
        assert!((config.calculate_attenuation(4.0) - 0.25).abs() < 0.001);
    }

    #[test]
    fn listener_panning() {
        let listener = AudioListener::new(SpatialConfig::default());

        // Source directly in front (no panning)
        let (left, right) = listener.calculate_panning(Vec3::new(0.0, 0.0, -5.0));
        assert!((left - right).abs() < 0.1);

        // Source to the right
        let (left, right) = listener.calculate_panning(Vec3::new(5.0, 0.0, 0.0));
        assert!(right > left);

        // Source to the left
        let (left, right) = listener.calculate_panning(Vec3::new(-5.0, 0.0, 0.0));
        assert!(left > right);
    }

    #[test]
    fn spatial_source_update() {
        let config = SpatialConfig::default();
        let listener = AudioListener::new(config.clone());
        let mut source = SpatialSource::new(0, AudioPosition::new(Vec3::new(5.0, 0.0, 0.0)), config);

        source.update_relative_to(&listener);

        // Should have some attenuation at distance 5
        assert!(source.attenuation() < 1.0);
        assert!(source.attenuation() > 0.0);

        // Should be panned to the right
        let (left, right) = source.panning();
        assert!(right > left);
    }

    #[test]
    fn doppler_effect() {
        let config = SpatialConfig {
            doppler_enabled: true,
            doppler_factor: 1.0,
            speed_of_sound: 343.0,
            ..Default::default()
        };

        // Source approaching listener
        let pitch = config.calculate_doppler(
            Vec3::ZERO,                  // listener stationary
            Vec3::new(0.0, 0.0, -50.0),  // source moving toward listener
            Vec3::new(0.0, 0.0, -1.0),   // direction to source
        );
        assert!(pitch > 1.0, "Approaching source should have higher pitch");

        // Source moving away from listener
        let pitch = config.calculate_doppler(
            Vec3::ZERO,                  // listener stationary
            Vec3::new(0.0, 0.0, 50.0),   // source moving away from listener
            Vec3::new(0.0, 0.0, -1.0),   // direction to source
        );
        assert!(pitch < 1.0, "Receding source should have lower pitch");
    }
}
