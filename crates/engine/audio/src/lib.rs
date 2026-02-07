//! Audio engine for Crossworld
//!
//! This crate provides audio functionality for Crossworld, including:
//! - Sound effect playback (one-shot and looping)
//! - Music playback with crossfade support
//! - 3D spatial audio positioning
//! - Voice chat integration (MoQ)
//!
//! # Architecture
//!
//! The audio system is designed as a platform layer component that abstracts
//! over different audio backends:
//! - Native: Uses `rodio` for desktop platforms
//! - WASM: Uses Web Audio API for browser environments
//!
//! # Example
//!
//! ```rust,ignore
//! use crossworld_audio::{AudioEngine, Sound, SoundConfig};
//!
//! // Create the audio engine
//! let mut engine = AudioEngine::new()?;
//!
//! // Load a sound effect
//! let sound = Sound::load("assets/sounds/footstep.ogg")?;
//!
//! // Play the sound
//! engine.play_sound(&sound, SoundConfig::default())?;
//!
//! // Play positioned sound for 3D audio
//! engine.play_sound_at(&sound, position, SoundConfig::default())?;
//! ```

pub mod engine;
pub mod music;
pub mod sound;
pub mod spatial;
pub mod voice;

pub use engine::{AudioEngine, AudioEngineConfig};
pub use music::{MusicPlayer, MusicTrack, CrossfadeConfig};
pub use sound::{Sound, SoundConfig, SoundHandle, SoundPlayer};
pub use spatial::{AudioListener, SpatialConfig, SpatialSource};
pub use voice::{VoiceChatConfig, VoiceChatIntegration, VoiceParticipant};

use glam::Vec3;

/// Error types for the audio crate
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Audio initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Failed to load audio file: {0}")]
    LoadFailed(String),

    #[error("Playback error: {0}")]
    PlaybackError(String),

    #[error("Invalid audio format: {0}")]
    InvalidFormat(String),

    #[error("Audio device not available")]
    DeviceNotAvailable,

    #[error("Voice chat error: {0}")]
    VoiceChatError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for audio operations
pub type Result<T> = std::result::Result<T, Error>;

/// Volume level (0.0 = silent, 1.0 = full volume)
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Volume(f32);

impl Volume {
    /// Silent (0.0)
    pub const SILENT: Self = Self(0.0);
    /// Full volume (1.0)
    pub const FULL: Self = Self(1.0);

    /// Create a new volume level, clamped to [0.0, 1.0]
    pub fn new(value: f32) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    /// Get the volume value
    pub fn value(&self) -> f32 {
        self.0
    }

    /// Convert to decibels (useful for audio mixing)
    pub fn to_db(&self) -> f32 {
        if self.0 <= 0.0 {
            f32::NEG_INFINITY
        } else {
            20.0 * self.0.log10()
        }
    }

    /// Create from decibel value
    pub fn from_db(db: f32) -> Self {
        Self::new(10.0_f32.powf(db / 20.0))
    }
}

impl Default for Volume {
    fn default() -> Self {
        Self::FULL
    }
}

impl From<f32> for Volume {
    fn from(value: f32) -> Self {
        Self::new(value)
    }
}

/// Audio categories for volume control
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
pub enum AudioCategory {
    /// Master volume (affects all audio)
    Master,
    /// Sound effects (footsteps, impacts, etc.)
    #[default]
    Effects,
    /// Background music
    Music,
    /// Voice chat
    Voice,
    /// Ambient sounds
    Ambient,
    /// UI sounds (clicks, notifications)
    Ui,
}

/// Position in 3D space for spatial audio
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AudioPosition {
    /// World position
    pub position: Vec3,
    /// Velocity (for Doppler effect)
    pub velocity: Vec3,
}

impl AudioPosition {
    /// Create a new audio position
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
        }
    }

    /// Create with velocity for Doppler effect
    pub fn with_velocity(position: Vec3, velocity: Vec3) -> Self {
        Self { position, velocity }
    }
}

impl Default for AudioPosition {
    fn default() -> Self {
        Self::new(Vec3::ZERO)
    }
}

impl From<Vec3> for AudioPosition {
    fn from(position: Vec3) -> Self {
        Self::new(position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_clamping() {
        assert_eq!(Volume::new(-0.5).value(), 0.0);
        assert_eq!(Volume::new(0.5).value(), 0.5);
        assert_eq!(Volume::new(1.5).value(), 1.0);
    }

    #[test]
    fn volume_db_conversion() {
        let vol = Volume::new(1.0);
        assert!((vol.to_db() - 0.0).abs() < 0.001);

        let vol = Volume::new(0.5);
        assert!((vol.to_db() - (-6.02)).abs() < 0.1);

        let reconstructed = Volume::from_db(-6.02);
        assert!((reconstructed.value() - 0.5).abs() < 0.01);
    }

    #[test]
    fn audio_position_creation() {
        let pos = AudioPosition::new(Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(pos.position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(pos.velocity, Vec3::ZERO);

        let pos_with_vel = AudioPosition::with_velocity(
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(0.1, 0.0, 0.0),
        );
        assert_eq!(pos_with_vel.velocity, Vec3::new(0.1, 0.0, 0.0));
    }
}
