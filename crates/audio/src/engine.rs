//! Core audio engine implementation
//!
//! The AudioEngine is the main entry point for all audio operations.
//! It manages audio device initialization, volume controls, and coordinates
//! between the different audio subsystems (sound effects, music, spatial audio).

use std::collections::HashMap;

use crate::{
    music::{CrossfadeConfig, MusicPlayer, MusicTrack},
    sound::{Sound, SoundConfig, SoundHandle, SoundPlayer},
    spatial::{AudioListener, SpatialConfig, SpatialSource},
    AudioCategory, AudioPosition, Result, Volume,
};
use glam::Vec3;
use tracing::{debug, info, warn};

/// Configuration for the audio engine
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioEngineConfig {
    /// Master volume
    pub master_volume: Volume,
    /// Per-category volume settings
    pub category_volumes: HashMap<AudioCategory, Volume>,
    /// Spatial audio configuration
    pub spatial_config: SpatialConfig,
    /// Default crossfade duration for music (in seconds)
    pub default_crossfade_duration: f32,
    /// Maximum number of simultaneous sounds
    pub max_simultaneous_sounds: usize,
}

impl Default for AudioEngineConfig {
    fn default() -> Self {
        let mut category_volumes = HashMap::new();
        category_volumes.insert(AudioCategory::Master, Volume::FULL);
        category_volumes.insert(AudioCategory::Effects, Volume::new(0.8));
        category_volumes.insert(AudioCategory::Music, Volume::new(0.6));
        category_volumes.insert(AudioCategory::Voice, Volume::FULL);
        category_volumes.insert(AudioCategory::Ambient, Volume::new(0.5));
        category_volumes.insert(AudioCategory::Ui, Volume::new(0.7));

        Self {
            master_volume: Volume::FULL,
            category_volumes,
            spatial_config: SpatialConfig::default(),
            default_crossfade_duration: 2.0,
            max_simultaneous_sounds: 32,
        }
    }
}

/// The main audio engine
///
/// Manages all audio playback including sound effects, music, and spatial audio.
/// The engine abstracts over platform-specific audio backends.
pub struct AudioEngine {
    /// Engine configuration
    config: AudioEngineConfig,
    /// Sound effect player
    sound_player: SoundPlayer,
    /// Music player
    music_player: MusicPlayer,
    /// Audio listener (for spatial audio)
    listener: AudioListener,
    /// Active spatial sources
    spatial_sources: HashMap<u64, SpatialSource>,
    /// Next spatial source ID
    next_source_id: u64,
    /// Whether the engine is initialized
    initialized: bool,
}

impl AudioEngine {
    /// Create a new audio engine with default configuration
    pub fn new() -> Result<Self> {
        Self::with_config(AudioEngineConfig::default())
    }

    /// Create a new audio engine with custom configuration
    pub fn with_config(config: AudioEngineConfig) -> Result<Self> {
        info!("Initializing audio engine");

        let sound_player = SoundPlayer::new(config.max_simultaneous_sounds)?;
        let music_player = MusicPlayer::new(config.default_crossfade_duration)?;
        let listener = AudioListener::new(config.spatial_config.clone());

        Ok(Self {
            config,
            sound_player,
            music_player,
            listener,
            spatial_sources: HashMap::new(),
            next_source_id: 0,
            initialized: true,
        })
    }

    /// Check if the engine is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    // -------------------------------------------------------------------------
    // Volume Control
    // -------------------------------------------------------------------------

    /// Set the master volume
    pub fn set_master_volume(&mut self, volume: Volume) {
        self.config.master_volume = volume;
        debug!("Master volume set to {}", volume.value());
    }

    /// Get the master volume
    pub fn master_volume(&self) -> Volume {
        self.config.master_volume
    }

    /// Set volume for a specific category
    pub fn set_category_volume(&mut self, category: AudioCategory, volume: Volume) {
        self.config.category_volumes.insert(category, volume);
        debug!("Category {:?} volume set to {}", category, volume.value());
    }

    /// Get volume for a specific category
    pub fn category_volume(&self, category: AudioCategory) -> Volume {
        self.config
            .category_volumes
            .get(&category)
            .copied()
            .unwrap_or(Volume::FULL)
    }

    /// Calculate effective volume for a category (master * category)
    pub fn effective_volume(&self, category: AudioCategory) -> Volume {
        let master = self.config.master_volume.value();
        let category_vol = self.category_volume(category).value();
        Volume::new(master * category_vol)
    }

    // -------------------------------------------------------------------------
    // Sound Effect Playback
    // -------------------------------------------------------------------------

    /// Play a sound effect
    pub fn play_sound(&mut self, sound: &Sound, config: SoundConfig) -> Result<SoundHandle> {
        let effective_vol = self.effective_volume(config.category);
        let volume = config.volume;
        let adjusted_config = config.with_volume(Volume::new(
            volume.value() * effective_vol.value(),
        ));
        self.sound_player.play(sound, adjusted_config)
    }

    /// Play a sound at a specific 3D position
    pub fn play_sound_at(
        &mut self,
        sound: &Sound,
        position: impl Into<AudioPosition>,
        config: SoundConfig,
    ) -> Result<SoundHandle> {
        let position = position.into();
        let effective_vol = self.effective_volume(config.category);

        // Calculate spatial attenuation based on listener position
        let distance = (position.position - self.listener.position().position).length();
        let attenuation = self.config.spatial_config.calculate_attenuation(distance);

        let volume = config.volume;
        let adjusted_config = config.with_volume(Volume::new(
            volume.value() * effective_vol.value() * attenuation,
        ));

        self.sound_player.play_spatial(sound, position, adjusted_config)
    }

    /// Stop a playing sound
    pub fn stop_sound(&mut self, handle: SoundHandle) {
        self.sound_player.stop(handle);
    }

    /// Stop all sounds in a category
    pub fn stop_category(&mut self, category: AudioCategory) {
        self.sound_player.stop_category(category);
    }

    /// Stop all sound effects
    pub fn stop_all_sounds(&mut self) {
        self.sound_player.stop_all();
    }

    // -------------------------------------------------------------------------
    // Music Playback
    // -------------------------------------------------------------------------

    /// Play a music track
    pub fn play_music(&mut self, track: &MusicTrack) -> Result<()> {
        let volume = self.effective_volume(AudioCategory::Music);
        self.music_player.play(track, volume)
    }

    /// Play a music track with crossfade from the current track
    pub fn crossfade_to(&mut self, track: &MusicTrack, config: CrossfadeConfig) -> Result<()> {
        let volume = self.effective_volume(AudioCategory::Music);
        self.music_player.crossfade_to(track, volume, config)
    }

    /// Stop the current music
    pub fn stop_music(&mut self) {
        self.music_player.stop();
    }

    /// Pause the current music
    pub fn pause_music(&mut self) {
        self.music_player.pause();
    }

    /// Resume paused music
    pub fn resume_music(&mut self) {
        self.music_player.resume();
    }

    /// Check if music is currently playing
    pub fn is_music_playing(&self) -> bool {
        self.music_player.is_playing()
    }

    // -------------------------------------------------------------------------
    // Spatial Audio
    // -------------------------------------------------------------------------

    /// Update the audio listener position
    pub fn set_listener_position(&mut self, position: impl Into<AudioPosition>) {
        self.listener.set_position(position.into());
    }

    /// Update the audio listener orientation
    pub fn set_listener_orientation(&mut self, forward: Vec3, up: Vec3) {
        self.listener.set_orientation(forward, up);
    }

    /// Create a spatial audio source
    pub fn create_spatial_source(&mut self, position: impl Into<AudioPosition>) -> u64 {
        let id = self.next_source_id;
        self.next_source_id += 1;

        let source = SpatialSource::new(id, position.into(), self.config.spatial_config.clone());
        self.spatial_sources.insert(id, source);

        debug!("Created spatial source {}", id);
        id
    }

    /// Update a spatial source position
    pub fn update_spatial_source(&mut self, id: u64, position: impl Into<AudioPosition>) {
        if let Some(source) = self.spatial_sources.get_mut(&id) {
            source.set_position(position.into());
        } else {
            warn!("Attempted to update non-existent spatial source {}", id);
        }
    }

    /// Remove a spatial source
    pub fn remove_spatial_source(&mut self, id: u64) {
        self.spatial_sources.remove(&id);
        debug!("Removed spatial source {}", id);
    }

    // -------------------------------------------------------------------------
    // Engine Update
    // -------------------------------------------------------------------------

    /// Update the audio engine (should be called each frame)
    ///
    /// This updates spatial audio calculations, crossfades, and cleans up
    /// finished sounds.
    pub fn update(&mut self, delta_time: f32) {
        // Update music player (for crossfades)
        self.music_player.update(delta_time);

        // Update sound player (cleanup finished sounds)
        self.sound_player.update(delta_time);

        // Update spatial sources relative to listener
        for source in self.spatial_sources.values_mut() {
            source.update_relative_to(&self.listener);
        }
    }

    /// Shutdown the audio engine and release resources
    pub fn shutdown(&mut self) {
        info!("Shutting down audio engine");
        self.stop_all_sounds();
        self.stop_music();
        self.spatial_sources.clear();
        self.initialized = false;
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        if self.initialized {
            self.shutdown();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_creation() {
        let engine = AudioEngine::new();
        assert!(engine.is_ok());
        let engine = engine.unwrap();
        assert!(engine.is_initialized());
    }

    #[test]
    fn volume_control() {
        let mut engine = AudioEngine::new().unwrap();

        engine.set_master_volume(Volume::new(0.5));
        assert_eq!(engine.master_volume().value(), 0.5);

        engine.set_category_volume(AudioCategory::Effects, Volume::new(0.8));
        let effective = engine.effective_volume(AudioCategory::Effects);
        assert!((effective.value() - 0.4).abs() < 0.001); // 0.5 * 0.8 = 0.4
    }

    #[test]
    fn spatial_source_management() {
        let mut engine = AudioEngine::new().unwrap();

        let id1 = engine.create_spatial_source(Vec3::new(1.0, 0.0, 0.0));
        let id2 = engine.create_spatial_source(Vec3::new(-1.0, 0.0, 0.0));

        assert_ne!(id1, id2);

        engine.update_spatial_source(id1, Vec3::new(2.0, 0.0, 0.0));
        engine.remove_spatial_source(id1);

        // id1 should be gone, id2 should still exist
        assert!(!engine.spatial_sources.contains_key(&id1));
        assert!(engine.spatial_sources.contains_key(&id2));
    }
}
