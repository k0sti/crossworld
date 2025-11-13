use rand::{rngs::OsRng, RngCore};

/// Unique identifier issued to every accepted session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionContext {
    pub session_id: u64,
}

impl SessionContext {
    pub fn new(session_id: u64) -> Self {
        Self { session_id }
    }
}

/// Generates a cryptographically strong random session identifier.
pub fn generate_session_id() -> u64 {
    OsRng.next_u64()
}

/// Helper that builds the canonical message string clients must sign.
pub fn handshake_message(server_url: &str, timestamp: u64) -> String {
    format!("{server_url}{timestamp}")
}
