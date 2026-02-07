//! Network transport abstractions for Crossworld.
//!
//! This crate provides:
//! - Transport protocol traits (WebTransport, WebSocket)
//! - Connection lifecycle management
//! - Reconnection strategies
//! - Message serialization/deserialization

pub mod connection;
pub mod error;
pub mod message;
pub mod transport;

#[cfg(feature = "webtransport")]
pub mod webtransport;

pub use connection::{ConnectionEvent, ConnectionInfo, ConnectionState};
pub use error::{NetworkError, NetworkResult};
pub use message::{
    AnimationState, CompactPosition, PlayerIdentity, PlayerState, ReliableMessage,
    UnreliableMessage,
};
pub use transport::{ReliableChannel, Transport, UnreliableChannel};
