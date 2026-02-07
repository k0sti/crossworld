//! Voice chat integration for MoQ (Media over QUIC)
//!
//! This module provides types and abstractions for voice chat functionality.
//! The actual MoQ protocol implementation is handled by the TypeScript layer
//! (using @kixelated/moq and @kixelated/hang), but this module provides:
//!
//! - Configuration types for voice chat settings
//! - Participant tracking and state management
//! - Spatial voice chat positioning
//! - Volume and mute controls
//!
//! # Architecture
//!
//! Voice chat in Crossworld uses a hybrid architecture:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    TypeScript Layer                          │
//! ├─────────────────────────────────────────────────────────────┤
//! │  @kixelated/moq         │  @kixelated/hang                  │
//! │  - QUIC transport       │  - Audio broadcasting             │
//! │  - MoQ protocol         │  - Opus encoding/decoding         │
//! │  - Announcements        │  - MediaStream handling           │
//! ├─────────────────────────────────────────────────────────────┤
//! │                    Voice Manager (TS)                        │
//! │  - Connection management                                     │
//! │  - Participant discovery (Nostr + MoQ)                      │
//! │  - Audio routing                                            │
//! └────────────────────────┬────────────────────────────────────┘
//!                          │
//!                          ▼ WASM bridge (future)
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Rust Audio Crate                          │
//! ├─────────────────────────────────────────────────────────────┤
//! │  VoiceChatIntegration                                        │
//! │  - Participant state tracking                               │
//! │  - Spatial audio positioning                                │
//! │  - Volume/mute management                                   │
//! │  - Speaking detection state                                 │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # MoQ Integration Plan
//!
//! The integration with MoQ will follow these steps:
//!
//! 1. **Phase 1 (Current)**: Type definitions and state management
//!    - Define configuration and participant types
//!    - Implement local state tracking
//!    - Prepare for WASM<->JS interop
//!
//! 2. **Phase 2**: TypeScript bridge
//!    - Expose Rust types via wasm-bindgen
//!    - Create JS bindings for participant updates
//!    - Integrate with existing VoiceManager
//!
//! 3. **Phase 3**: Spatial voice processing
//!    - Process voice audio through spatial audio system
//!    - Apply distance attenuation to voice streams
//!    - Add 3D panning based on participant positions
//!
//! 4. **Phase 4**: Advanced features
//!    - Voice activity detection (VAD)
//!    - Echo cancellation coordination
//!    - Bandwidth adaptation based on participant count

use std::collections::HashMap;

use crate::{AudioPosition, Volume};
use glam::Vec3;
use tracing::debug;

/// Configuration for voice chat
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VoiceChatConfig {
    /// Input (microphone) volume
    pub input_volume: Volume,
    /// Output (speakers) volume
    pub output_volume: Volume,
    /// Whether the local microphone is muted
    pub muted: bool,
    /// Whether to enable spatial voice chat
    pub spatial_enabled: bool,
    /// Maximum distance for voice chat (if spatial)
    pub max_distance: f32,
    /// Reference distance for voice attenuation
    pub ref_distance: f32,
    /// Voice activity detection threshold (0.0 - 1.0)
    pub vad_threshold: f32,
    /// Whether to show speaking indicators
    pub show_speaking_indicators: bool,
}

impl Default for VoiceChatConfig {
    fn default() -> Self {
        Self {
            input_volume: Volume::FULL,
            output_volume: Volume::FULL,
            muted: false,
            spatial_enabled: true,
            max_distance: 50.0,
            ref_distance: 2.0,
            vad_threshold: 0.1,
            show_speaking_indicators: true,
        }
    }
}

/// State of a voice chat participant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ParticipantState {
    /// Connected but not transmitting
    #[default]
    Idle,
    /// Currently speaking
    Speaking,
    /// Microphone is muted
    Muted,
    /// Connection issues (high latency, packet loss)
    Degraded,
    /// Disconnecting
    Leaving,
}

/// Discovery source for a participant
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DiscoverySource {
    /// Discovered via Nostr (kind 30315 ClientStatus)
    Nostr,
    /// Discovered via MoQ announcements
    Moq,
    /// Discovered via both methods
    Both,
}

/// A voice chat participant
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VoiceParticipant {
    /// Unique identifier (typically npub)
    pub id: String,
    /// Display name
    pub display_name: Option<String>,
    /// Current state
    pub state: ParticipantState,
    /// World position (for spatial audio)
    pub position: Option<Vec3>,
    /// Volume adjustment for this participant
    pub volume: Volume,
    /// Whether locally muted by the user
    pub locally_muted: bool,
    /// Discovery source
    pub discovery_source: DiscoverySource,
    /// MoQ broadcast path (e.g., "crossworld/voice/crossworld-dev/npub1...")
    pub broadcast_path: Option<String>,
    /// Speaking level (0.0 - 1.0) for visual indicators
    pub speaking_level: f32,
    /// Last update timestamp (Unix milliseconds)
    pub last_update: u64,
}

impl VoiceParticipant {
    /// Create a new participant
    pub fn new(id: impl Into<String>, discovery_source: DiscoverySource) -> Self {
        Self {
            id: id.into(),
            display_name: None,
            state: ParticipantState::Idle,
            position: None,
            volume: Volume::FULL,
            locally_muted: false,
            discovery_source,
            broadcast_path: None,
            speaking_level: 0.0,
            last_update: 0,
        }
    }

    /// Check if participant is currently speaking
    pub fn is_speaking(&self) -> bool {
        self.state == ParticipantState::Speaking
    }

    /// Check if participant can be heard
    pub fn is_audible(&self) -> bool {
        !self.locally_muted
            && self.state != ParticipantState::Muted
            && self.volume.value() > 0.0
    }

    /// Get effective volume (considering local mute and volume adjustment)
    pub fn effective_volume(&self) -> Volume {
        if self.locally_muted {
            Volume::SILENT
        } else {
            self.volume
        }
    }

    /// Calculate distance-based volume (for spatial audio)
    pub fn spatial_volume(&self, listener_position: Vec3, config: &VoiceChatConfig) -> Volume {
        if !config.spatial_enabled {
            return self.effective_volume();
        }

        let Some(pos) = self.position else {
            return self.effective_volume();
        };

        let distance = (pos - listener_position).length();

        if distance >= config.max_distance {
            return Volume::SILENT;
        }

        // Inverse distance attenuation
        let attenuation = if distance <= config.ref_distance {
            1.0
        } else {
            config.ref_distance / distance
        };

        Volume::new(self.effective_volume().value() * attenuation)
    }
}

/// Voice chat integration manager
///
/// Tracks voice chat participants and manages local state.
/// This works in conjunction with the TypeScript VoiceManager.
pub struct VoiceChatIntegration {
    /// Configuration
    config: VoiceChatConfig,
    /// Connected participants
    participants: HashMap<String, VoiceParticipant>,
    /// Local participant ID (own npub)
    local_id: Option<String>,
    /// Whether voice chat is connected
    connected: bool,
    /// Current connection status message
    status_message: String,
}

impl VoiceChatIntegration {
    /// Create a new voice chat integration
    pub fn new(config: VoiceChatConfig) -> Self {
        Self {
            config,
            participants: HashMap::new(),
            local_id: None,
            connected: false,
            status_message: "Not connected".to_string(),
        }
    }

    /// Get current configuration
    pub fn config(&self) -> &VoiceChatConfig {
        &self.config
    }

    /// Update configuration
    pub fn set_config(&mut self, config: VoiceChatConfig) {
        self.config = config;
    }

    /// Set the local participant ID
    pub fn set_local_id(&mut self, id: impl Into<String>) {
        self.local_id = Some(id.into());
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Update connection status
    pub fn set_connected(&mut self, connected: bool, status: impl Into<String>) {
        self.connected = connected;
        self.status_message = status.into();
        debug!("Voice chat connection status: {} - {}", connected, self.status_message);
    }

    /// Get connection status message
    pub fn status_message(&self) -> &str {
        &self.status_message
    }

    /// Toggle local mute
    pub fn toggle_mute(&mut self) {
        self.config.muted = !self.config.muted;
        debug!("Voice chat mute: {}", self.config.muted);
    }

    /// Set local mute state
    pub fn set_muted(&mut self, muted: bool) {
        self.config.muted = muted;
    }

    /// Check if locally muted
    pub fn is_muted(&self) -> bool {
        self.config.muted
    }

    /// Add or update a participant
    pub fn update_participant(&mut self, participant: VoiceParticipant) {
        let id = participant.id.clone();
        debug!("Updated voice participant: {}", id);
        self.participants.insert(id, participant);
    }

    /// Remove a participant
    pub fn remove_participant(&mut self, id: &str) {
        self.participants.remove(id);
        debug!("Removed voice participant: {}", id);
    }

    /// Get a participant by ID
    pub fn get_participant(&self, id: &str) -> Option<&VoiceParticipant> {
        self.participants.get(id)
    }

    /// Get a mutable participant by ID
    pub fn get_participant_mut(&mut self, id: &str) -> Option<&mut VoiceParticipant> {
        self.participants.get_mut(id)
    }

    /// Get all participants
    pub fn participants(&self) -> impl Iterator<Item = &VoiceParticipant> {
        self.participants.values()
    }

    /// Get participant count
    pub fn participant_count(&self) -> usize {
        self.participants.len()
    }

    /// Get participants who are currently speaking
    pub fn speaking_participants(&self) -> impl Iterator<Item = &VoiceParticipant> {
        self.participants.values().filter(|p| p.is_speaking())
    }

    /// Update participant position (for spatial audio)
    pub fn update_participant_position(&mut self, id: &str, position: Vec3) {
        if let Some(participant) = self.participants.get_mut(id) {
            participant.position = Some(position);
        }
    }

    /// Update participant speaking state
    pub fn update_participant_speaking(&mut self, id: &str, speaking: bool, level: f32) {
        if let Some(participant) = self.participants.get_mut(id) {
            participant.state = if speaking {
                ParticipantState::Speaking
            } else {
                ParticipantState::Idle
            };
            participant.speaking_level = level;
        }
    }

    /// Mute a specific participant locally
    pub fn mute_participant(&mut self, id: &str, muted: bool) {
        if let Some(participant) = self.participants.get_mut(id) {
            participant.locally_muted = muted;
            debug!("Participant {} locally muted: {}", id, muted);
        }
    }

    /// Set volume for a specific participant
    pub fn set_participant_volume(&mut self, id: &str, volume: Volume) {
        if let Some(participant) = self.participants.get_mut(id) {
            participant.volume = volume;
        }
    }

    /// Get audio position for a participant (for spatial audio source)
    pub fn get_participant_audio_position(&self, id: &str) -> Option<AudioPosition> {
        self.participants
            .get(id)
            .and_then(|p| p.position)
            .map(AudioPosition::new)
    }

    /// Clear all participants (on disconnect)
    pub fn clear_participants(&mut self) {
        self.participants.clear();
        debug!("Cleared all voice participants");
    }

    /// Update the integration (call each frame)
    ///
    /// This can be used to timeout stale participants, etc.
    pub fn update(&mut self, _delta_time: f32) {
        // Future: implement participant timeout logic
        // For now, TypeScript handles participant lifecycle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn participant_creation() {
        let participant = VoiceParticipant::new("npub1test", DiscoverySource::Both);
        assert_eq!(participant.id, "npub1test");
        assert_eq!(participant.state, ParticipantState::Idle);
        assert!(!participant.is_speaking());
        assert!(participant.is_audible());
    }

    #[test]
    fn participant_spatial_volume() {
        let mut participant = VoiceParticipant::new("test", DiscoverySource::Moq);
        participant.position = Some(Vec3::new(10.0, 0.0, 0.0));

        let config = VoiceChatConfig {
            spatial_enabled: true,
            ref_distance: 2.0,
            max_distance: 50.0,
            ..Default::default()
        };

        let listener_pos = Vec3::ZERO;
        let volume = participant.spatial_volume(listener_pos, &config);

        // At distance 10 with ref_distance 2, attenuation should be 2/10 = 0.2
        assert!((volume.value() - 0.2).abs() < 0.01);
    }

    #[test]
    fn voice_chat_integration() {
        let mut voice = VoiceChatIntegration::new(VoiceChatConfig::default());

        assert!(!voice.is_connected());
        assert!(!voice.is_muted());

        voice.set_connected(true, "Connected to relay");
        assert!(voice.is_connected());

        let participant = VoiceParticipant::new("user1", DiscoverySource::Nostr);
        voice.update_participant(participant);
        assert_eq!(voice.participant_count(), 1);

        voice.update_participant_speaking("user1", true, 0.8);
        let p = voice.get_participant("user1").unwrap();
        assert!(p.is_speaking());
        assert_eq!(p.speaking_level, 0.8);

        voice.mute_participant("user1", true);
        let p = voice.get_participant("user1").unwrap();
        assert!(!p.is_audible());

        voice.remove_participant("user1");
        assert_eq!(voice.participant_count(), 0);
    }

    #[test]
    fn voice_chat_mute_toggle() {
        let mut voice = VoiceChatIntegration::new(VoiceChatConfig::default());

        assert!(!voice.is_muted());
        voice.toggle_mute();
        assert!(voice.is_muted());
        voice.toggle_mute();
        assert!(!voice.is_muted());
    }
}
