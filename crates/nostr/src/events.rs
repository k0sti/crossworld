//! Nostr event types for Crossworld
//!
//! Defines event structures based on doc/nostr.md specification:
//! - Kind 30311: Live Event (world/server configuration)
//! - Kind 1311: Live Chat Message
//! - Kind 30317: Avatar State
//! - Kind 1317: Position Update
//! - Kind 30078: World Model / Server Discovery

use crate::{Error, Result};
use serde::{Deserialize, Serialize};

/// Default d-tag for Crossworld live events
pub const CROSSWORLD_D_TAG: &str = "crossworld";

/// Default d-tag for development environment
pub const CROSSWORLD_DEV_D_TAG: &str = "crossworld-dev";

// ============================================================================
// Live Event (Kind 30311)
// ============================================================================

/// Live Event (Kind 30311) - Server/world configuration and discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveEvent {
    /// World identifier (d-tag)
    pub d_tag: String,
    /// Human-readable world name
    pub title: Option<String>,
    /// World description
    pub summary: Option<String>,
    /// Status: "live" or "ended"
    pub status: String,
    /// MoQ relay URL for voice chat
    pub streaming_url: Option<String>,
    /// Nostr relay URLs for chat
    pub relay_urls: Vec<String>,
    /// Topic hashtags
    pub hashtags: Vec<String>,
    /// Current participant count
    pub participants: Option<u32>,
}

impl LiveEvent {
    /// Event kind for live events (NIP-53)
    pub const KIND: u16 = 30311;

    /// Create a new live event with default values
    pub fn new(d_tag: &str) -> Self {
        Self {
            d_tag: d_tag.to_string(),
            title: Some("Crossworld".to_string()),
            summary: Some("Crossworld Nostr Metaverse".to_string()),
            status: "live".to_string(),
            streaming_url: None,
            relay_urls: Vec::new(),
            hashtags: vec!["crossworld".to_string(), "metaverse".to_string()],
            participants: None,
        }
    }

    /// Get the address tag (a-tag) for this event
    pub fn address_tag(&self, pubkey: &str) -> String {
        format!("30311:{}:{}", pubkey, self.d_tag)
    }
}

// ============================================================================
// Live Chat Message (Kind 1311)
// ============================================================================

/// Live Chat Message (Kind 1311) - In-world chat messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Reference to live event (kind:pubkey:d-tag)
    pub live_event_address: String,
    /// Message content
    pub content: String,
    /// Mentioned user pubkeys
    pub mentioned_pubkeys: Vec<String>,
    /// Reply to event ID (optional)
    pub reply_to: Option<String>,
}

impl ChatMessage {
    /// Event kind for live chat messages (NIP-53)
    pub const KIND: u16 = 1311;

    /// Create a new chat message
    pub fn new(live_event_address: &str, content: &str) -> Self {
        Self {
            live_event_address: live_event_address.to_string(),
            content: content.to_string(),
            mentioned_pubkeys: Vec::new(),
            reply_to: None,
        }
    }
}

// ============================================================================
// Avatar State (Kind 30317)
// ============================================================================

/// Avatar State (Kind 30317) - Persistent avatar configuration per world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvatarState {
    /// Application identifier (d-tag)
    pub d_tag: String,
    /// Reference to world event (30311:pubkey:d-tag)
    pub world_address: Option<String>,
    /// Avatar type: "vox" or "glb"
    pub avatar_type: String,
    /// Model ID from models.json
    pub avatar_id: Option<String>,
    /// Custom model URL (if avatar_id not used)
    pub avatar_url: Option<String>,
    /// Position in world
    pub position: Position,
    /// Status: "active" or "inactive"
    pub status: String,
    /// Voice connection status
    pub voice: Option<String>,
    /// Microphone status
    pub mic: Option<String>,
}

impl AvatarState {
    /// Event kind for avatar state
    pub const KIND: u16 = 30317;

    /// Create a new avatar state with default values
    pub fn new() -> Self {
        Self {
            d_tag: CROSSWORLD_D_TAG.to_string(),
            world_address: None,
            avatar_type: "vox".to_string(),
            avatar_id: Some("chr_army1".to_string()),
            avatar_url: None,
            position: Position::default(),
            status: "active".to_string(),
            voice: None,
            mic: None,
        }
    }

    /// Set the world this avatar belongs to
    pub fn with_world(mut self, server_pubkey: &str, d_tag: &str) -> Self {
        self.world_address = Some(format!("30311:{}:{}", server_pubkey, d_tag));
        self
    }

    /// Set the avatar model
    pub fn with_avatar(mut self, avatar_id: &str) -> Self {
        self.avatar_id = Some(avatar_id.to_string());
        self
    }

    /// Set the position
    pub fn with_position(mut self, x: f32, y: f32, z: f32) -> Self {
        self.position = Position { x, y, z };
        self
    }
}

impl Default for AvatarState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Position Update (Kind 1317)
// ============================================================================

/// Position Update (Kind 1317) - Real-time movement synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionUpdate {
    /// Reference to avatar state (30317:pubkey:d-tag)
    pub avatar_address: String,
    /// Reference to world (30311:pubkey:d-tag)
    pub world_address: String,
    /// Current position
    pub position: Position,
    /// Optional rotation quaternion
    pub rotation: Option<Rotation>,
    /// Movement animation hint
    pub move_style: MoveStyle,
    /// Expiration timestamp (NIP-40)
    pub expiration: u64,
}

impl PositionUpdate {
    /// Event kind for position updates
    pub const KIND: u16 = 1317;

    /// Default expiration time in seconds (60s)
    pub const DEFAULT_EXPIRATION_SECS: u64 = 60;

    /// Create a new position update
    pub fn new(avatar_address: &str, world_address: &str, position: Position) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            avatar_address: avatar_address.to_string(),
            world_address: world_address.to_string(),
            position,
            rotation: None,
            move_style: MoveStyle::Walk,
            expiration: now + Self::DEFAULT_EXPIRATION_SECS,
        }
    }
}

/// 3D position
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Position {
    /// Create a new position
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Parse from JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(Error::Serialization)
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// Rotation quaternion
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Rotation {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Default for Rotation {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        }
    }
}

impl Rotation {
    /// Create a new rotation quaternion
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    /// Identity rotation (no rotation)
    pub fn identity() -> Self {
        Self::default()
    }
}

/// Movement animation style
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MoveStyle {
    /// Normal walking
    #[default]
    Walk,
    /// Running (shift+move)
    Run,
    /// Teleport with fade effect
    TeleportFade,
    /// Teleport with scale effect
    TeleportScale,
    /// Teleport with spin effect
    TeleportSpin,
}

impl MoveStyle {
    /// Convert to string for Nostr tag
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Walk => "walk",
            Self::Run => "run",
            Self::TeleportFade => "teleport:fade",
            Self::TeleportScale => "teleport:scale",
            Self::TeleportSpin => "teleport:spin",
        }
    }
}

// ============================================================================
// World Model (Kind 30078)
// ============================================================================

/// World Model (Kind 30078) - Voxel world data storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldModel {
    /// Model identifier (d-tag)
    pub d_tag: String,
    /// Human-readable world name
    pub title: Option<String>,
    /// World description
    pub description: Option<String>,
    /// Preview image URL
    pub thumbnail: Option<String>,
    /// Reference to live event (optional)
    pub live_event_address: Option<String>,
    /// CSM (CubeScript Model) content
    pub csm_content: String,
}

impl WorldModel {
    /// Event kind for world models (NIP-78)
    pub const KIND: u16 = 30078;

    /// Create a new world model
    pub fn new(d_tag: &str, csm_content: &str) -> Self {
        Self {
            d_tag: d_tag.to_string(),
            title: None,
            description: None,
            thumbnail: None,
            live_event_address: None,
            csm_content: csm_content.to_string(),
        }
    }

    /// Set the title
    pub fn with_title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    /// Set the description
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_live_event() {
        let event = LiveEvent::new("test-world");
        assert_eq!(event.d_tag, "test-world");
        assert_eq!(event.status, "live");

        let address = event.address_tag("abc123");
        assert_eq!(address, "30311:abc123:test-world");
    }

    #[test]
    fn test_avatar_state() {
        let avatar = AvatarState::new()
            .with_world("abc123", "crossworld-dev")
            .with_avatar("chr_robot1")
            .with_position(1.0, 2.0, 3.0);

        assert_eq!(avatar.d_tag, CROSSWORLD_D_TAG);
        assert_eq!(
            avatar.world_address,
            Some("30311:abc123:crossworld-dev".to_string())
        );
        assert_eq!(avatar.avatar_id, Some("chr_robot1".to_string()));
        assert_eq!(avatar.position.x, 1.0);
    }

    #[test]
    fn test_position_json() {
        let pos = Position::new(1.5, 2.5, 3.5);
        let json = pos.to_json();
        let parsed = Position::from_json(&json).unwrap();

        assert_eq!(pos.x, parsed.x);
        assert_eq!(pos.y, parsed.y);
        assert_eq!(pos.z, parsed.z);
    }

    #[test]
    fn test_move_style() {
        assert_eq!(MoveStyle::Walk.as_str(), "walk");
        assert_eq!(MoveStyle::Run.as_str(), "run");
        assert_eq!(MoveStyle::TeleportFade.as_str(), "teleport:fade");
    }

    #[test]
    fn test_world_model() {
        let model = WorldModel::new("my-castle", ">d [1 2 3]")
            .with_title("My Castle")
            .with_description("A medieval castle");

        assert_eq!(model.d_tag, "my-castle");
        assert_eq!(model.title, Some("My Castle".to_string()));
        assert_eq!(model.csm_content, ">d [1 2 3]");
    }
}
