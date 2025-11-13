use cube::{Cube, CubeCoord};
use serde::{Deserialize, Serialize};

/// Client -> Server handshake message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Handshake {
    pub npub: String,
    pub timestamp: u64,
    pub signature: Vec<u8>,
    pub display_name: Option<String>,
}

/// Authorization level granted to a session.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthLevel {
    ReadOnly,
    User,
    Admin,
}

/// Server -> Client handshake acknowledgement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeAck {
    pub session_id: u64,
    pub world_info: WorldInfo,
    pub auth_level: AuthLevel,
}

/// Description of the world served by this instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldInfo {
    pub world_id: String,
    pub max_depth: u32,
    pub macro_depth: u32,
    pub border_depth: u32,
}

/// Client -> Server request for a portion of the world.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldRequest {
    pub session_id: u64,
    pub coord: CubeCoord,
    pub subscribe: bool,
}

/// Server -> Client response containing world data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldData {
    pub coord: CubeCoord,
    pub root: Cube<u8>,
    pub subscription_id: Option<u64>,
}

/// Client -> Server edit request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldEdit {
    pub session_id: u64,
    pub operation: EditOperation,
    pub transaction_id: u64,
}

/// Supported edit operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditOperation {
    SetCube { coord: CubeCoord, cube: Cube<u8> },
}

/// Server -> Client edit acknowledgement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldEditAck {
    pub transaction_id: u64,
    pub result: EditResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditResult {
    Success,
    Error(EditError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditError {
    Unauthorized,
    InvalidCoordinates,
    QuotaExceeded,
    ServerError(String),
}

/// Broadcast update sent to subscribed clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldUpdate {
    pub subscription_id: u64,
    pub operation: EditOperation,
    pub author: String,
    pub timestamp: u64,
}

/// Messages clients can send after the initial handshake.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    WorldRequest(WorldRequest),
    WorldEdit(WorldEdit),
    Disconnect,
}

/// Messages the server sends back to clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    HandshakeAck(HandshakeAck),
    WorldData(WorldData),
    WorldEditAck(WorldEditAck),
    WorldUpdate(WorldUpdate),
    Error(String),
}
