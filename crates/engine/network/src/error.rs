//! Network error types.

use thiserror::Error;

/// Network-specific errors.
#[derive(Error, Debug)]
pub enum NetworkError {
    /// Connection failed to establish.
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    /// Connection was closed unexpectedly.
    #[error("connection closed: {0}")]
    ConnectionClosed(String),

    /// Failed to send a message.
    #[error("send failed: {0}")]
    SendFailed(String),

    /// Failed to receive a message.
    #[error("receive failed: {0}")]
    ReceiveFailed(String),

    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Deserialization error.
    #[error("deserialization error: {0}")]
    Deserialization(String),

    /// Connection timeout.
    #[error("connection timeout")]
    Timeout,

    /// Maximum reconnection attempts exceeded.
    #[error("max reconnection attempts exceeded")]
    MaxReconnectsExceeded,

    /// Invalid state for operation.
    #[error("invalid state: {0}")]
    InvalidState(String),

    /// Transport-specific error.
    #[error("transport error: {0}")]
    Transport(String),

    /// TLS/certificate error.
    #[error("TLS error: {0}")]
    Tls(String),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Convenience result type for network operations.
pub type NetworkResult<T> = Result<T, NetworkError>;
