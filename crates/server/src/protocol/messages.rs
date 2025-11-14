use cube::{Cube, CubeCoord};
use serde::{Deserialize, Serialize};

/// Client -> Server handshake message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HandshakeAck {
    pub session_id: u64,
    pub world_info: WorldInfo,
    pub auth_level: AuthLevel,
}

/// Description of the world served by this instance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorldInfo {
    pub world_id: String,
    pub max_depth: u32,
    pub macro_depth: u32,
    pub border_depth: u32,
}

/// Client -> Server request for a portion of the world.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorldRequest {
    pub session_id: u64,
    pub coord: CubeCoord,
    pub subscribe: bool,
}

/// Server -> Client response containing world data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorldData {
    pub coord: CubeCoord,
    pub root: Cube<u8>,
    pub subscription_id: Option<u64>,
}

/// Client -> Server edit request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorldEdit {
    pub session_id: u64,
    pub operation: EditOperation,
    pub transaction_id: u64,
}

/// Supported edit operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EditOperation {
    SetCube { coord: CubeCoord, cube: Cube<u8> },
}

/// Server -> Client edit acknowledgement.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorldEditAck {
    pub transaction_id: u64,
    pub result: EditResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EditResult {
    Success,
    Error(EditError),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EditError {
    Unauthorized,
    InvalidCoordinates,
    QuotaExceeded,
    ServerError(String),
}

/// Broadcast update sent to subscribed clients.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorldUpdate {
    pub subscription_id: u64,
    pub operation: EditOperation,
    pub author: String,
    pub timestamp: u64,
}

/// Messages clients can send after the initial handshake.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ClientMessage {
    WorldRequest(WorldRequest),
    WorldEdit(WorldEdit),
    Disconnect,
}

/// Messages the server sends back to clients.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServerMessage {
    HandshakeAck(HandshakeAck),
    WorldData(WorldData),
    WorldEditAck(WorldEditAck),
    WorldUpdate(WorldUpdate),
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip<T: Serialize + for<'de> Deserialize<'de> + PartialEq + std::fmt::Debug>(value: T) {
        let bytes = bincode::serialize(&value).expect("serialize");
        let decoded: T = bincode::deserialize(&bytes).expect("deserialize");
        assert_eq!(value, decoded);
    }

    #[test]
    fn client_message_roundtrip() {
        let coord = CubeCoord::new(cube::glam::IVec3::new(1, 2, 3), 4);
        let msg = ClientMessage::WorldRequest(WorldRequest {
            session_id: 42,
            coord,
            subscribe: true,
        });
        roundtrip(msg);
    }

    #[test]
    fn server_message_roundtrip() {
        let coord = CubeCoord::new(cube::glam::IVec3::new(0, 0, 0), 0);
        let msg = ServerMessage::WorldData(WorldData {
            coord,
            root: Cube::Solid(7),
            subscription_id: Some(99),
        });
        roundtrip(msg);
    }
}
