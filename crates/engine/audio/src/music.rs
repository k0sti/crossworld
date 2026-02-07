//! Music playback with crossfade support
//!
//! Provides music track management with smooth transitions between tracks.

use std::path::Path;
use std::sync::Arc;

use crate::{Result, Volume};
use tracing::{debug, info};

/// A music track that can be played
#[derive(Debug, Clone)]
pub struct MusicTrack {
    /// Unique identifier
    id: u64,
    /// Display name
    name: String,
    /// Duration in seconds
    duration: f32,
    /// Sample rate
    #[allow(dead_code)]
    sample_rate: u32,
    /// Channels
    #[allow(dead_code)]
    channels: u16,
    /// Audio data
    #[allow(dead_code)]
    data: Arc<MusicData>,
}

/// Platform-specific music data
#[derive(Debug)]
struct MusicData {
    /// Path to the music file (for streaming)
    #[allow(dead_code)]
    path: Option<String>,
    /// Decoded samples (for smaller files)
    #[allow(dead_code)]
    samples: Option<Vec<f32>>,
}

impl MusicTrack {
    /// Load a music track from file
    ///
    /// For longer tracks, this may use streaming playback.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let path = path.as_ref();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        debug!("Loading music track: {}", path.display());

        // Placeholder - actual loading depends on platform
        Ok(Self {
            id,
            name,
            duration: 0.0, // Would be set from actual file
            sample_rate: 44100,
            channels: 2,
            data: Arc::new(MusicData {
                path: Some(path.to_string_lossy().to_string()),
                samples: None,
            }),
        })
    }

    /// Create from raw samples
    pub fn from_samples(
        name: impl Into<String>,
        samples: Vec<f32>,
        sample_rate: u32,
        channels: u16,
    ) -> Self {
        static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let duration = samples.len() as f32 / (sample_rate as f32 * channels as f32);

        Self {
            id,
            name: name.into(),
            duration,
            sample_rate,
            channels,
            data: Arc::new(MusicData {
                path: None,
                samples: Some(samples),
            }),
        }
    }

    /// Get track ID
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Get track name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get duration in seconds
    pub fn duration(&self) -> f32 {
        self.duration
    }
}

/// Configuration for music crossfade
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct CrossfadeConfig {
    /// Duration of the crossfade in seconds
    pub duration: f32,
    /// Crossfade curve type
    pub curve: CrossfadeCurve,
}

impl Default for CrossfadeConfig {
    fn default() -> Self {
        Self {
            duration: 2.0,
            curve: CrossfadeCurve::EqualPower,
        }
    }
}

impl CrossfadeConfig {
    /// Create a linear crossfade
    pub fn linear(duration: f32) -> Self {
        Self {
            duration,
            curve: CrossfadeCurve::Linear,
        }
    }

    /// Create an equal-power crossfade (smoother)
    pub fn equal_power(duration: f32) -> Self {
        Self {
            duration,
            curve: CrossfadeCurve::EqualPower,
        }
    }

    /// Calculate volumes for fade-out and fade-in at a given progress (0.0 to 1.0)
    pub fn calculate_volumes(&self, progress: f32) -> (f32, f32) {
        let t = progress.clamp(0.0, 1.0);
        match self.curve {
            CrossfadeCurve::Linear => (1.0 - t, t),
            CrossfadeCurve::EqualPower => {
                // Equal power crossfade: uses sine/cosine for constant power
                let angle = t * std::f32::consts::FRAC_PI_2;
                (angle.cos(), angle.sin())
            }
            CrossfadeCurve::Logarithmic => {
                // Logarithmic curve for more natural fade
                let fade_out = if t < 1.0 { (1.0 - t).sqrt() } else { 0.0 };
                let fade_in = t.sqrt();
                (fade_out, fade_in)
            }
        }
    }
}

/// Crossfade curve types
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CrossfadeCurve {
    /// Linear crossfade (may have slight volume dip)
    Linear,
    /// Equal-power crossfade (constant perceived volume)
    EqualPower,
    /// Logarithmic crossfade (natural fade)
    Logarithmic,
}

/// State of the music player
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicState {
    /// No music playing
    Stopped,
    /// Music is playing normally
    Playing,
    /// Music is paused
    Paused,
    /// Crossfading between tracks
    Crossfading,
}

/// Music playback manager
///
/// Handles playing music tracks with support for crossfading between tracks.
pub struct MusicPlayer {
    /// Current playback state
    state: MusicState,
    /// Currently playing track (or fading out track during crossfade)
    current_track: Option<ActiveTrack>,
    /// Track fading in during crossfade
    next_track: Option<ActiveTrack>,
    /// Crossfade configuration (when crossfading)
    crossfade: Option<CrossfadeState>,
    /// Default crossfade duration
    #[allow(dead_code)] // Will be used when crossfade_to() uses default
    default_crossfade_duration: f32,
}

/// State for an active (playing) track
#[derive(Debug)]
struct ActiveTrack {
    #[allow(dead_code)] // Will be used for track identification in audio backend
    track_id: u64,
    track_name: String,
    volume: Volume,
    current_volume_multiplier: f32,
    elapsed: f32,
    paused: bool,
}

/// State for an ongoing crossfade
#[derive(Debug)]
struct CrossfadeState {
    config: CrossfadeConfig,
    progress: f32,
}

impl MusicPlayer {
    /// Create a new music player
    pub fn new(default_crossfade_duration: f32) -> Result<Self> {
        Ok(Self {
            state: MusicState::Stopped,
            current_track: None,
            next_track: None,
            crossfade: None,
            default_crossfade_duration,
        })
    }

    /// Get the current playback state
    pub fn state(&self) -> MusicState {
        self.state
    }

    /// Check if music is playing
    pub fn is_playing(&self) -> bool {
        matches!(self.state, MusicState::Playing | MusicState::Crossfading)
    }

    /// Play a music track (stops current if playing)
    pub fn play(&mut self, track: &MusicTrack, volume: Volume) -> Result<()> {
        info!("Playing music track: {}", track.name());

        // Stop any current playback
        self.stop_internal();

        self.current_track = Some(ActiveTrack {
            track_id: track.id(),
            track_name: track.name().to_string(),
            volume,
            current_volume_multiplier: 1.0,
            elapsed: 0.0,
            paused: false,
        });
        self.state = MusicState::Playing;

        Ok(())
    }

    /// Crossfade to a new track
    pub fn crossfade_to(
        &mut self,
        track: &MusicTrack,
        volume: Volume,
        config: CrossfadeConfig,
    ) -> Result<()> {
        if self.current_track.is_none() {
            // No current track, just play normally
            return self.play(track, volume);
        }

        info!(
            "Crossfading to music track: {} ({}s)",
            track.name(),
            config.duration
        );

        self.next_track = Some(ActiveTrack {
            track_id: track.id(),
            track_name: track.name().to_string(),
            volume,
            current_volume_multiplier: 0.0,
            elapsed: 0.0,
            paused: false,
        });

        self.crossfade = Some(CrossfadeState {
            config,
            progress: 0.0,
        });
        self.state = MusicState::Crossfading;

        Ok(())
    }

    /// Stop playback
    pub fn stop(&mut self) {
        info!("Stopping music playback");
        self.stop_internal();
    }

    fn stop_internal(&mut self) {
        self.current_track = None;
        self.next_track = None;
        self.crossfade = None;
        self.state = MusicState::Stopped;
    }

    /// Pause playback
    pub fn pause(&mut self) {
        if let Some(ref mut track) = self.current_track {
            track.paused = true;
            self.state = MusicState::Paused;
            debug!("Music paused");
        }
    }

    /// Resume paused playback
    pub fn resume(&mut self) {
        if let Some(ref mut track) = self.current_track {
            if track.paused {
                track.paused = false;
                self.state = if self.crossfade.is_some() {
                    MusicState::Crossfading
                } else {
                    MusicState::Playing
                };
                debug!("Music resumed");
            }
        }
    }

    /// Set volume for current playback
    pub fn set_volume(&mut self, volume: Volume) {
        if let Some(ref mut track) = self.current_track {
            track.volume = volume;
        }
    }

    /// Get the name of the currently playing track
    pub fn current_track_name(&self) -> Option<&str> {
        self.current_track.as_ref().map(|t| t.track_name.as_str())
    }

    /// Update the music player (call each frame)
    pub fn update(&mut self, delta_time: f32) {
        // Update elapsed time
        if let Some(ref mut track) = self.current_track {
            if !track.paused {
                track.elapsed += delta_time;
            }
        }
        if let Some(ref mut track) = self.next_track {
            track.elapsed += delta_time;
        }

        // Handle crossfade
        if let Some(ref mut crossfade) = self.crossfade {
            crossfade.progress += delta_time / crossfade.config.duration;

            let (fade_out, fade_in) = crossfade.config.calculate_volumes(crossfade.progress);

            if let Some(ref mut current) = self.current_track {
                current.current_volume_multiplier = fade_out;
            }
            if let Some(ref mut next) = self.next_track {
                next.current_volume_multiplier = fade_in;
            }

            // Check if crossfade is complete
            if crossfade.progress >= 1.0 {
                debug!("Crossfade complete");
                self.current_track = self.next_track.take();
                if let Some(ref mut track) = self.current_track {
                    track.current_volume_multiplier = 1.0;
                }
                self.crossfade = None;
                self.state = if self.current_track.is_some() {
                    MusicState::Playing
                } else {
                    MusicState::Stopped
                };
            }
        }
    }

    /// Get the effective volume for the current track
    pub fn current_volume(&self) -> f32 {
        self.current_track
            .as_ref()
            .map(|t| t.volume.value() * t.current_volume_multiplier)
            .unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn music_track_creation() {
        let track = MusicTrack::from_samples("test", vec![0.0; 88200], 44100, 2);
        assert_eq!(track.name(), "test");
        assert!((track.duration() - 1.0).abs() < 0.01);
    }

    #[test]
    fn crossfade_config_volumes() {
        let config = CrossfadeConfig::equal_power(2.0);

        // At start, fade out should be full, fade in should be zero
        let (out, in_) = config.calculate_volumes(0.0);
        assert!((out - 1.0).abs() < 0.01);
        assert!(in_ < 0.01);

        // At end, fade out should be zero, fade in should be full
        let (out, in_) = config.calculate_volumes(1.0);
        assert!(out < 0.01);
        assert!((in_ - 1.0).abs() < 0.01);

        // At middle, both should be roughly equal (for equal power)
        let (out, in_) = config.calculate_volumes(0.5);
        assert!((out - in_).abs() < 0.1);
    }

    #[test]
    fn music_player_basic_playback() {
        let mut player = MusicPlayer::new(2.0).unwrap();
        let track = MusicTrack::from_samples("test", vec![0.0; 1000], 44100, 2);

        assert_eq!(player.state(), MusicState::Stopped);

        player.play(&track, Volume::FULL).unwrap();
        assert_eq!(player.state(), MusicState::Playing);
        assert!(player.is_playing());
        assert_eq!(player.current_track_name(), Some("test"));

        player.pause();
        assert_eq!(player.state(), MusicState::Paused);

        player.resume();
        assert_eq!(player.state(), MusicState::Playing);

        player.stop();
        assert_eq!(player.state(), MusicState::Stopped);
    }

    #[test]
    fn music_player_crossfade() {
        let mut player = MusicPlayer::new(2.0).unwrap();
        let track1 = MusicTrack::from_samples("track1", vec![0.0; 1000], 44100, 2);
        let track2 = MusicTrack::from_samples("track2", vec![0.0; 1000], 44100, 2);

        player.play(&track1, Volume::FULL).unwrap();
        assert_eq!(player.current_track_name(), Some("track1"));

        player
            .crossfade_to(&track2, Volume::FULL, CrossfadeConfig::default())
            .unwrap();
        assert_eq!(player.state(), MusicState::Crossfading);

        // Simulate crossfade completion
        for _ in 0..100 {
            player.update(0.05); // 50ms per frame, 5 seconds total
        }

        assert_eq!(player.state(), MusicState::Playing);
        assert_eq!(player.current_track_name(), Some("track2"));
    }
}
