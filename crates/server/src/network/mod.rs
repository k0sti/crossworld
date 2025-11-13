pub mod broadcast;
mod io;
pub mod session;
pub mod webtransport;

pub use broadcast::BroadcastHub;
pub use session::{ClientSession, RateLimiter};
pub use webtransport::WebTransportServer;

use crate::{auth::AuthError, world::WorldError};
use thiserror::Error;

/// Errors emitted by the networking layer.
#[derive(Debug, Error)]
pub enum ServerError {
    #[error("authentication failed: {0}")]
    Auth(#[from] AuthError),
    #[error("world error: {0}")]
    World(#[from] WorldError),
    #[error("serialization error: {0}")]
    Serialization(#[from] bincode::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("session mismatch")]
    InvalidSession,
    #[error("connection closed")]
    ConnectionClosed,
    #[error("transport error: {0}")]
    Transport(String),
}

pub type Result<T> = std::result::Result<T, ServerError>;
