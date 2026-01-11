//! Nostr integration library for Crossworld
//!
//! This crate provides Nostr protocol support for Crossworld, including:
//! - Key management (generation, import/export, storage)
//! - Event types specific to Crossworld (live events, avatar state, position updates)
//! - Account state management for the editor
//! - NIP-46 remote signing support (with `nip46` feature)
//!
//! # Example
//!
//! ```rust,ignore
//! use crossworld_nostr::{NostrAccount, KeyManager};
//!
//! // Create or load an account
//! let key_manager = KeyManager::new();
//! let account = key_manager.generate_account()?;
//!
//! // Get public key for display
//! println!("Logged in as: {}", account.npub());
//! ```

pub mod account;
pub mod events;
pub mod keys;

#[cfg(feature = "nip46")]
pub mod connect;

pub use account::{AccountState, NostrAccount};
pub use events::{AvatarState, LiveEvent, PositionUpdate, WorldModel};
pub use keys::KeyManager;

// Re-export common nostr types for convenience
pub use nostr::{FromBech32, PublicKey, ToBech32};

#[cfg(feature = "nip46")]
pub use connect::{ConnectedSession, NostrConnectSession};

/// Error types for the nostr crate
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid key format: {0}")]
    InvalidKey(String),

    #[error("Key file error: {0}")]
    KeyFile(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Nostr protocol error: {0}")]
    NostrProtocol(String),

    #[error("No keys loaded")]
    NoKeys,
}

/// Result type for nostr operations
pub type Result<T> = std::result::Result<T, Error>;
