//! Crossworld server crate.
//!
//! This crate provides the foundational building blocks for the Crossworld
//! multiplayer server described in `docs/server.md`.  The modules exposed here
//! cover authentication, protocol definitions, configurable storage-backed world
//! management, and an async networking layer built around WebTransport.

pub mod auth;
pub mod config;
pub mod network;
pub mod protocol;
pub mod world;

pub use auth::{AuthConfig, AuthManager};
pub use config::{ServerConfig, WorldConfig};
pub use network::{ServerError, WebTransportServer};
pub use protocol::{
    messages::{
        ClientMessage, EditError, EditOperation, EditResult, Handshake, HandshakeAck,
        ServerMessage, WorldData, WorldEdit, WorldEditAck, WorldInfo, WorldRequest, WorldUpdate,
    },
    AuthLevel,
};
pub use world::{storage::FileStorage, WorldState};
