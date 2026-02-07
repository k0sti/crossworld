//! Network message types.
//!
//! These are the core message types exchanged between client and server.
//! They are transport-agnostic and can be serialized over any transport.

use serde::{Deserialize, Serialize};

/// Messages sent over reliable streams (ordered, guaranteed delivery).
///
/// Used for important state changes that must arrive in order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReliableMessage {
    /// Player joining the server.
    Join {
        npub: String,
        display_name: Option<String>,
        avatar_url: Option<String>,
        position: [f32; 3],
    },

    /// Player leaving the server.
    Leave { npub: String },

    /// Chat message.
    ChatMessage {
        from: String,
        content: String,
        timestamp: u64,
    },

    /// Game event (voxel edit, etc.).
    GameEvent { event_type: String, data: Vec<u8> },

    /// Server command/response.
    ServerCommand { command: String, args: Vec<String> },

    /// Kick notification.
    Kick { reason: String },
}

/// Messages sent over unreliable datagrams (unordered, best-effort).
///
/// Used for frequent updates where latest state matters more than reliability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnreliableMessage {
    /// Position update for single player.
    Position {
        x: f32,
        y: f32,
        z: f32,
        rx: f32, // Quaternion rotation
        ry: f32,
        rz: f32,
        rw: f32,
        seq: u32, // Sequence number for ordering
    },

    /// Batch position updates (server -> client).
    Batch {
        positions: Vec<CompactPosition>,
        timestamp: u64,
    },

    /// Ping for latency measurement.
    Ping { timestamp: u64 },

    /// Pong response.
    Pong { timestamp: u64 },
}

/// Compact position format for efficient broadcasting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactPosition {
    /// Player hex ID.
    pub id: String,
    /// Position [x, y, z].
    pub pos: [f32; 3],
    /// Rotation quaternion [x, y, z, w].
    pub rot: [f32; 4],
    /// Velocity [x, y, z].
    pub vel: [f32; 3],
    /// Animation state.
    pub anim: u8,
}

/// Player identity information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerIdentity {
    pub npub: String,
    pub hex_id: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

/// Player state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub position: glam::Vec3,
    pub rotation: glam::Quat,
    pub velocity: glam::Vec3,
    pub animation_state: AnimationState,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            position: glam::Vec3::ZERO,
            rotation: glam::Quat::IDENTITY,
            velocity: glam::Vec3::ZERO,
            animation_state: AnimationState::Idle,
        }
    }
}

/// Animation states.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum AnimationState {
    #[default]
    Idle = 0,
    Walking = 1,
    Running = 2,
    Jumping = 3,
    Falling = 4,
}

impl From<u8> for AnimationState {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Idle,
            1 => Self::Walking,
            2 => Self::Running,
            3 => Self::Jumping,
            4 => Self::Falling,
            _ => Self::Idle,
        }
    }
}

impl From<AnimationState> for u8 {
    fn from(value: AnimationState) -> Self {
        value as u8
    }
}

/// Serialize a message to bytes using bincode.
pub fn serialize<T: Serialize>(message: &T) -> Result<Vec<u8>, crate::error::NetworkError> {
    bincode::serialize(message)
        .map_err(|e| crate::error::NetworkError::Serialization(e.to_string()))
}

/// Deserialize bytes to a message using bincode.
pub fn deserialize<'a, T: Deserialize<'a>>(
    bytes: &'a [u8],
) -> Result<T, crate::error::NetworkError> {
    bincode::deserialize(bytes)
        .map_err(|e| crate::error::NetworkError::Deserialization(e.to_string()))
}
