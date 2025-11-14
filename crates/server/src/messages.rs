use serde::{Deserialize, Serialize};

/// Messages sent over reliable streams (QUIC streams)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReliableMessage {
    /// Player joining the server
    Join {
        npub: String,
        display_name: Option<String>,
        avatar_url: Option<String>,
        position: [f32; 3],
    },
    /// Player leaving the server
    Leave {
        npub: String,
    },
    /// Chat message
    ChatMessage {
        from: String,
        content: String,
        timestamp: u64,
    },
    /// Game event (voxel edit, etc.)
    GameEvent {
        event_type: String,
        data: Vec<u8>,
    },
    /// Server command
    ServerCommand {
        command: String,
        args: Vec<String>,
    },
    /// Kick notification
    Kick {
        reason: String,
    },
}

/// Messages sent over unreliable datagrams (UDP-like)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnreliableMessage {
    /// Position update for single player
    Position {
        x: f32,
        y: f32,
        z: f32,
        rx: f32,  // Quaternion rotation
        ry: f32,
        rz: f32,
        rw: f32,
        seq: u32,  // Sequence number for ordering
    },
    /// Batch position updates
    Batch {
        positions: Vec<CompactPosition>,
        timestamp: u64,
    },
    /// Ping/Pong for latency measurement
    Ping {
        timestamp: u64,
    },
    Pong {
        timestamp: u64,
    },
}

/// Compact position format for efficient broadcasting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactPosition {
    /// Player hex ID
    pub id: String,
    /// Position [x, y, z]
    pub pos: [f32; 3],
    /// Rotation quaternion [x, y, z, w]
    pub rot: [f32; 4],
    /// Velocity [x, y, z]
    pub vel: [f32; 3],
    /// Animation state
    pub anim: u8,
}

/// Player identity information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerIdentity {
    pub npub: String,
    pub hex_id: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

/// Player state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub position: glam::Vec3,
    pub rotation: glam::Quat,
    pub velocity: glam::Vec3,
    pub animation_state: AnimationState,
}

/// Animation states
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum AnimationState {
    Idle = 0,
    Walking = 1,
    Running = 2,
    Jumping = 3,
    Falling = 4,
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
