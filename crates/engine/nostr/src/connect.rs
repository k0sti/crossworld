//! NIP-46 Nostr Connect (Remote Signing) support
//!
//! This module provides functionality for connecting to a remote Nostr signer
//! (like Amber) using NIP-46. The signer scans a QR code containing a
//! `nostrconnect://` URI and approves the connection.
//!
//! # Example
//!
//! ```rust,ignore
//! use crossworld_nostr::connect::NostrConnectSession;
//! use std::time::Duration;
//!
//! // Create a new session
//! let session = NostrConnectSession::new("wss://relay.nsec.app", Duration::from_secs(120))?;
//!
//! // Get the URI for QR code generation
//! let uri = session.connect_uri();
//! println!("Scan this: {}", uri);
//!
//! // Wait for connection (blocking)
//! let (pubkey, signer) = session.wait_for_connection().await?;
//! println!("Connected as: {}", pubkey.to_bech32()?);
//! ```

use nostr::{Keys, PublicKey, RelayUrl, ToBech32};
use nostr_connect::prelude::{NostrConnect, NostrConnectURI};
use std::time::Duration;

/// Default relay for NIP-46 connections
pub const DEFAULT_RELAY: &str = "wss://relay.nsec.app";

/// Default timeout for connection attempts (2 minutes)
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);

/// Application name shown to the signer
pub const APP_NAME: &str = "Crossworld";

/// A NIP-46 connection session
///
/// Handles the lifecycle of a NIP-46 connection:
/// 1. Generate client keys and connection URI
/// 2. Wait for signer to connect and approve
/// 3. Provide access to the connected signer for signing operations
pub struct NostrConnectSession {
    /// The relay URL for NIP-46 communication
    relay_url: String,
    /// Connection timeout
    timeout: Duration,
    /// Client keys for this session
    client_keys: Keys,
    /// The nostrconnect:// URI to display as QR code
    uri: NostrConnectURI,
}

impl NostrConnectSession {
    /// Create a new NIP-46 connection session
    ///
    /// # Arguments
    /// * `relay_url` - The relay URL for NIP-46 communication (e.g., "wss://relay.nsec.app")
    /// * `timeout` - Maximum time to wait for connection
    ///
    /// # Returns
    /// A new session ready for connection
    pub fn new(relay_url: &str, timeout: Duration) -> crate::Result<Self> {
        let client_keys = Keys::generate();
        let relay = RelayUrl::parse(relay_url)
            .map_err(|e| crate::Error::NostrProtocol(format!("Invalid relay URL: {}", e)))?;

        let uri = NostrConnectURI::client(client_keys.public_key(), vec![relay], APP_NAME);

        Ok(Self {
            relay_url: relay_url.to_string(),
            timeout,
            client_keys,
            uri,
        })
    }

    /// Create a new session with default relay and timeout
    pub fn with_defaults() -> crate::Result<Self> {
        Self::new(DEFAULT_RELAY, DEFAULT_TIMEOUT)
    }

    /// Get the connection URI string for QR code generation
    ///
    /// This is the `nostrconnect://` URI that the signer (Amber) should scan.
    pub fn connect_uri(&self) -> String {
        self.uri.to_string()
    }

    /// Get the client's public key for this session
    pub fn client_pubkey(&self) -> PublicKey {
        self.client_keys.public_key()
    }

    /// Get the relay URL
    pub fn relay_url(&self) -> &str {
        &self.relay_url
    }

    /// Wait for the signer to connect and return the connected session
    ///
    /// This is a blocking async operation that:
    /// 1. Connects to the relay
    /// 2. Subscribes to NostrConnect events
    /// 3. Waits for the signer to send an ACK (when user approves in Amber)
    /// 4. Returns the user's public key and the connected signer
    ///
    /// # Returns
    /// A tuple of (user's public key, connected NostrConnect signer)
    pub async fn wait_for_connection(self) -> crate::Result<ConnectedSession> {
        use nostr::NostrSigner;

        // Create NostrConnect client
        let connect =
            NostrConnect::new(self.uri, self.client_keys, self.timeout, None).map_err(|e| {
                crate::Error::NostrProtocol(format!("Failed to create NostrConnect: {}", e))
            })?;

        // Wait for connection - this blocks until signer approves
        let user_pubkey = NostrSigner::get_public_key(&connect)
            .await
            .map_err(|e| crate::Error::NostrProtocol(format!("Connection failed: {}", e)))?;

        let npub = user_pubkey
            .to_bech32()
            .map_err(|e| crate::Error::NostrProtocol(format!("Failed to encode pubkey: {}", e)))?;

        Ok(ConnectedSession {
            user_pubkey,
            npub,
            relay_url: self.relay_url,
            signer: connect,
        })
    }
}

/// A successfully connected NIP-46 session
pub struct ConnectedSession {
    /// The connected user's public key
    pub user_pubkey: PublicKey,
    /// The user's npub (bech32-encoded public key)
    pub npub: String,
    /// The relay URL used for this connection
    pub relay_url: String,
    /// The connected signer (can be used for signing operations)
    signer: NostrConnect,
}

impl ConnectedSession {
    /// Get a reference to the signer for signing operations
    pub fn signer(&self) -> &NostrConnect {
        &self.signer
    }

    /// Sign an unsigned event
    pub async fn sign_event(&self, unsigned: nostr::UnsignedEvent) -> crate::Result<nostr::Event> {
        use nostr::NostrSigner;
        self.signer
            .sign_event(unsigned)
            .await
            .map_err(|e| crate::Error::NostrProtocol(format!("Signing failed: {}", e)))
    }

    /// Get the bunker URI for future reconnection
    pub async fn bunker_uri(&self) -> crate::Result<String> {
        self.signer
            .bunker_uri()
            .await
            .map(|uri| uri.to_string())
            .map_err(|e| crate::Error::NostrProtocol(format!("Failed to get bunker URI: {}", e)))
    }

    /// Shutdown the connection (consumes self)
    pub async fn shutdown(self) {
        self.signer.shutdown().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = NostrConnectSession::new("wss://relay.nsec.app", Duration::from_secs(60));
        assert!(session.is_ok());
    }

    #[test]
    fn test_connect_uri_format() {
        let session = NostrConnectSession::with_defaults().unwrap();
        let uri = session.connect_uri();
        assert!(uri.starts_with("nostrconnect://"));
        assert!(uri.contains("relay="));
        assert!(uri.contains("Crossworld"));
    }

    #[test]
    fn test_client_pubkey() {
        let session = NostrConnectSession::with_defaults().unwrap();
        let pubkey = session.client_pubkey();
        assert!(!pubkey.to_hex().is_empty());
    }
}
