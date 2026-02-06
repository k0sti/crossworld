//! Transport protocol traits.
//!
//! This module defines the core abstractions for network transports,
//! allowing different implementations (WebTransport, WebSocket, etc.)
//! to be used interchangeably.

use crate::error::NetworkResult;
use crate::message::{ReliableMessage, UnreliableMessage};
use std::future::Future;
use std::pin::Pin;

/// A reliable channel for ordered, guaranteed delivery.
///
/// Used for messages that must arrive in order (join, leave, chat).
pub trait ReliableChannel: Send + Sync {
    /// Send a reliable message.
    fn send(
        &self,
        message: ReliableMessage,
    ) -> Pin<Box<dyn Future<Output = NetworkResult<()>> + Send + '_>>;

    /// Receive a reliable message.
    fn receive(&self) -> Pin<Box<dyn Future<Output = NetworkResult<ReliableMessage>> + Send + '_>>;

    /// Close the channel.
    fn close(&self) -> Pin<Box<dyn Future<Output = NetworkResult<()>> + Send + '_>>;
}

/// An unreliable channel for unordered, best-effort delivery.
///
/// Used for frequent updates where latest state matters more than
/// guaranteed delivery (position updates).
pub trait UnreliableChannel: Send + Sync {
    /// Send an unreliable message (datagram).
    fn send(
        &self,
        message: UnreliableMessage,
    ) -> Pin<Box<dyn Future<Output = NetworkResult<()>> + Send + '_>>;

    /// Receive an unreliable message (datagram).
    fn receive(
        &self,
    ) -> Pin<Box<dyn Future<Output = NetworkResult<UnreliableMessage>> + Send + '_>>;

    /// Get the maximum datagram size supported.
    fn max_datagram_size(&self) -> usize;
}

/// Transport configuration.
#[derive(Debug, Clone)]
pub struct TransportConfig {
    /// Address to connect to (for client) or bind to (for server).
    pub address: String,

    /// TLS certificate path (PEM format).
    pub cert_path: Option<String>,

    /// TLS private key path (PEM format).
    pub key_path: Option<String>,

    /// Connection timeout in milliseconds.
    pub connect_timeout_ms: u64,

    /// Keep-alive interval in milliseconds.
    pub keepalive_interval_ms: u64,

    /// Maximum idle timeout in milliseconds.
    pub idle_timeout_ms: u64,

    /// Allow self-signed certificates (development only).
    pub allow_insecure: bool,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            address: "127.0.0.1:4433".to_string(),
            cert_path: None,
            key_path: None,
            connect_timeout_ms: 10_000,
            keepalive_interval_ms: 5_000,
            idle_timeout_ms: 30_000,
            allow_insecure: false,
        }
    }
}

/// A network transport abstraction.
///
/// Provides both reliable (stream) and unreliable (datagram) channels.
pub trait Transport: Send + Sync {
    /// The reliable channel type for this transport.
    type Reliable: ReliableChannel;

    /// The unreliable channel type for this transport.
    type Unreliable: UnreliableChannel;

    /// Get the reliable channel.
    fn reliable(&self) -> &Self::Reliable;

    /// Get the unreliable channel.
    fn unreliable(&self) -> &Self::Unreliable;

    /// Check if the transport is connected.
    fn is_connected(&self) -> bool;

    /// Get the remote address.
    fn remote_address(&self) -> Option<String>;

    /// Close the transport.
    fn close(&self) -> Pin<Box<dyn Future<Output = NetworkResult<()>> + Send + '_>>;
}

/// Server-side transport listener.
///
/// Accepts incoming connections and returns transport instances.
pub trait TransportListener: Send + Sync {
    /// The transport type produced by this listener.
    type Transport: Transport;

    /// Accept the next incoming connection.
    fn accept(&self) -> Pin<Box<dyn Future<Output = NetworkResult<Self::Transport>> + Send + '_>>;

    /// Get the local address the listener is bound to.
    fn local_address(&self) -> NetworkResult<String>;
}

/// Client-side transport connector.
///
/// Establishes connections to servers.
pub trait TransportConnector: Send + Sync {
    /// The transport type produced by this connector.
    type Transport: Transport;

    /// Connect to the specified address.
    fn connect(
        &self,
        config: TransportConfig,
    ) -> Pin<Box<dyn Future<Output = NetworkResult<Self::Transport>> + Send + '_>>;
}
