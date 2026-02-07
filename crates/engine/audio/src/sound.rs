//! Sound effect types and playback
//!
//! Provides types for loading and playing sound effects, including
//! one-shot sounds and looping sounds.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use crate::{AudioCategory, AudioPosition, Error, Result, Volume};
use tracing::{debug, trace};

/// Handle to a playing sound, used to control or stop playback
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SoundHandle(pub(crate) u64);

impl SoundHandle {
    /// Get the raw handle ID
    pub fn id(&self) -> u64 {
        self.0
    }
}

/// Audio data for a sound effect
///
/// Sounds can be loaded from files and shared between multiple playback instances.
#[derive(Debug, Clone)]
pub struct Sound {
    /// Unique identifier
    id: u64,
    /// Display name (usually filename)
    name: String,
    /// Duration in seconds
    duration: f32,
    /// Sample rate
    sample_rate: u32,
    /// Number of channels (1 = mono, 2 = stereo)
    channels: u16,
    /// Raw audio data (platform-specific representation)
    #[allow(dead_code)]
    data: Arc<SoundData>,
}

/// Platform-specific sound data
#[derive(Debug)]
pub(crate) struct SoundData {
    /// Decoded audio samples (interleaved if stereo)
    #[allow(dead_code)]
    samples: Vec<f32>,
}

impl Sound {
    /// Create a sound from raw audio samples
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
            data: Arc::new(SoundData { samples }),
        }
    }

    /// Load a sound from a file path
    ///
    /// Supported formats depend on the platform:
    /// - Native: WAV, OGG, MP3, FLAC (via rodio)
    /// - WASM: Decoded via Web Audio API
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        debug!("Loading sound: {}", path.display());

        // For now, create a placeholder - actual loading will be implemented
        // with platform-specific backends
        #[cfg(feature = "native")]
        {
            Self::load_native(path, name)
        }

        #[cfg(not(feature = "native"))]
        {
            // Placeholder for WASM or when no audio backend is enabled
            Ok(Self::from_samples(name, vec![], 44100, 2))
        }
    }

    #[cfg(feature = "native")]
    fn load_native(path: &Path, name: String) -> Result<Self> {
        use std::fs::File;
        use std::io::BufReader;

        let file = File::open(path).map_err(|e| Error::LoadFailed(e.to_string()))?;
        let reader = BufReader::new(file);

        let decoder = rodio::Decoder::new(reader)
            .map_err(|e| Error::LoadFailed(format!("Failed to decode: {}", e)))?;

        let sample_rate = decoder.sample_rate();
        let channels = decoder.channels();
        let samples: Vec<f32> = decoder.map(|s| s as f32 / i16::MAX as f32).collect();

        Ok(Self::from_samples(name, samples, sample_rate, channels))
    }

    /// Get the sound's unique ID
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Get the sound's name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the duration in seconds
    pub fn duration(&self) -> f32 {
        self.duration
    }

    /// Get the sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Get the number of channels
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// Check if this is a mono sound (better for spatial audio)
    pub fn is_mono(&self) -> bool {
        self.channels == 1
    }
}

/// Configuration for sound playback
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SoundConfig {
    /// Volume (0.0 to 1.0)
    pub volume: Volume,
    /// Playback speed/pitch (1.0 = normal)
    pub pitch: f32,
    /// Whether to loop the sound
    pub looping: bool,
    /// Audio category for volume grouping
    pub category: AudioCategory,
    /// Delay before playing (in seconds)
    pub delay: f32,
    /// Fade in duration (in seconds)
    pub fade_in: f32,
}

impl Default for SoundConfig {
    fn default() -> Self {
        Self {
            volume: Volume::FULL,
            pitch: 1.0,
            looping: false,
            category: AudioCategory::Effects,
            delay: 0.0,
            fade_in: 0.0,
        }
    }
}

impl SoundConfig {
    /// Create config for a one-shot sound effect
    pub fn one_shot() -> Self {
        Self::default()
    }

    /// Create config for a looping sound
    pub fn looping() -> Self {
        Self {
            looping: true,
            ..Default::default()
        }
    }

    /// Set the volume
    pub fn with_volume(mut self, volume: Volume) -> Self {
        self.volume = volume;
        self
    }

    /// Set the pitch/speed
    pub fn with_pitch(mut self, pitch: f32) -> Self {
        self.pitch = pitch.max(0.1);
        self
    }

    /// Set the category
    pub fn with_category(mut self, category: AudioCategory) -> Self {
        self.category = category;
        self
    }

    /// Set a delay before playing
    pub fn with_delay(mut self, delay: f32) -> Self {
        self.delay = delay.max(0.0);
        self
    }

    /// Set fade in duration
    pub fn with_fade_in(mut self, duration: f32) -> Self {
        self.fade_in = duration.max(0.0);
        self
    }
}

/// Internal state for a playing sound
#[derive(Debug)]
#[allow(dead_code)] // Fields used for audio backend integration
pub(crate) struct PlayingSound {
    pub handle: SoundHandle,
    pub sound_id: u64,
    pub config: SoundConfig,
    pub position: Option<AudioPosition>,
    pub elapsed: f32,
    pub volume_multiplier: f32,
    pub stopped: bool,
}

/// Sound effect player
///
/// Manages playback of sound effects with pooling and lifetime management.
pub struct SoundPlayer {
    /// Maximum simultaneous sounds
    max_sounds: usize,
    /// Currently playing sounds
    playing: HashMap<u64, PlayingSound>,
    /// Next handle ID
    next_handle: u64,
}

impl SoundPlayer {
    /// Create a new sound player
    pub fn new(max_sounds: usize) -> Result<Self> {
        Ok(Self {
            max_sounds,
            playing: HashMap::new(),
            next_handle: 0,
        })
    }

    /// Play a sound effect
    pub fn play(&mut self, sound: &Sound, config: SoundConfig) -> Result<SoundHandle> {
        self.play_internal(sound, config, None)
    }

    /// Play a sound at a 3D position
    pub fn play_spatial(
        &mut self,
        sound: &Sound,
        position: AudioPosition,
        config: SoundConfig,
    ) -> Result<SoundHandle> {
        self.play_internal(sound, config, Some(position))
    }

    fn play_internal(
        &mut self,
        sound: &Sound,
        config: SoundConfig,
        position: Option<AudioPosition>,
    ) -> Result<SoundHandle> {
        // Check if we've hit the sound limit
        if self.playing.len() >= self.max_sounds {
            // Try to remove the oldest non-looping sound
            let oldest = self
                .playing
                .iter()
                .filter(|(_, s)| !s.config.looping)
                .max_by(|a, b| a.1.elapsed.partial_cmp(&b.1.elapsed).unwrap())
                .map(|(id, _)| *id);

            if let Some(id) = oldest {
                self.playing.remove(&id);
                debug!("Removed oldest sound to make room for new sound");
            } else {
                return Err(Error::PlaybackError(
                    "Maximum simultaneous sounds reached".to_string(),
                ));
            }
        }

        let handle = SoundHandle(self.next_handle);
        self.next_handle += 1;

        let playing = PlayingSound {
            handle,
            sound_id: sound.id(),
            config,
            position,
            elapsed: 0.0,
            volume_multiplier: 1.0,
            stopped: false,
        };

        self.playing.insert(handle.0, playing);
        trace!("Playing sound {} (handle: {})", sound.name(), handle.0);

        Ok(handle)
    }

    /// Stop a playing sound
    pub fn stop(&mut self, handle: SoundHandle) {
        if let Some(playing) = self.playing.get_mut(&handle.0) {
            playing.stopped = true;
            debug!("Stopped sound {}", handle.0);
        }
    }

    /// Stop all sounds in a category
    pub fn stop_category(&mut self, category: AudioCategory) {
        for playing in self.playing.values_mut() {
            if playing.config.category == category {
                playing.stopped = true;
            }
        }
        debug!("Stopped all sounds in category {:?}", category);
    }

    /// Stop all sounds
    pub fn stop_all(&mut self) {
        for playing in self.playing.values_mut() {
            playing.stopped = true;
        }
        debug!("Stopped all sounds");
    }

    /// Check if a sound is still playing
    pub fn is_playing(&self, handle: SoundHandle) -> bool {
        self.playing
            .get(&handle.0)
            .map(|s| !s.stopped)
            .unwrap_or(false)
    }

    /// Set volume multiplier for a playing sound
    pub fn set_volume(&mut self, handle: SoundHandle, volume: Volume) {
        if let Some(playing) = self.playing.get_mut(&handle.0) {
            playing.volume_multiplier = volume.value();
        }
    }

    /// Update the player (call each frame)
    pub fn update(&mut self, delta_time: f32) {
        // Update elapsed time and remove finished sounds
        let finished: Vec<u64> = self
            .playing
            .iter_mut()
            .filter_map(|(id, playing)| {
                playing.elapsed += delta_time;

                // Check if sound is finished (either stopped or non-looping and past duration)
                // Note: In a real implementation, we'd track actual audio playback state
                if playing.stopped {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();

        for id in finished {
            self.playing.remove(&id);
        }
    }

    /// Get the number of currently playing sounds
    pub fn playing_count(&self) -> usize {
        self.playing.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sound_from_samples() {
        let samples = vec![0.0f32; 44100]; // 1 second of silence
        let sound = Sound::from_samples("test", samples, 44100, 1);

        assert_eq!(sound.name(), "test");
        assert!((sound.duration() - 1.0).abs() < 0.01);
        assert_eq!(sound.sample_rate(), 44100);
        assert!(sound.is_mono());
    }

    #[test]
    fn sound_config_builder() {
        let config = SoundConfig::one_shot()
            .with_volume(Volume::new(0.5))
            .with_pitch(1.2)
            .with_category(AudioCategory::Ui);

        assert_eq!(config.volume.value(), 0.5);
        assert_eq!(config.pitch, 1.2);
        assert_eq!(config.category, AudioCategory::Ui);
        assert!(!config.looping);
    }

    #[test]
    fn sound_player_playback() {
        let mut player = SoundPlayer::new(10).unwrap();
        let sound = Sound::from_samples("test", vec![0.0; 1000], 44100, 1);

        let handle = player.play(&sound, SoundConfig::default()).unwrap();
        assert!(player.is_playing(handle));
        assert_eq!(player.playing_count(), 1);

        player.stop(handle);
        player.update(0.0);
        assert!(!player.is_playing(handle));
        assert_eq!(player.playing_count(), 0);
    }

    #[test]
    fn sound_player_category_stop() {
        let mut player = SoundPlayer::new(10).unwrap();
        let sound = Sound::from_samples("test", vec![0.0; 1000], 44100, 1);

        let h1 = player
            .play(&sound, SoundConfig::default().with_category(AudioCategory::Effects))
            .unwrap();
        let h2 = player
            .play(&sound, SoundConfig::default().with_category(AudioCategory::Ui))
            .unwrap();

        player.stop_category(AudioCategory::Effects);
        player.update(0.0);

        assert!(!player.is_playing(h1));
        assert!(player.is_playing(h2));
    }
}
