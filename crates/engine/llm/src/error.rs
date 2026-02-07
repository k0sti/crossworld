//! Error types for the LLM crate

/// Errors that can occur during LLM operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The client is not connected or initialized
    #[error("Client not connected")]
    NotConnected,

    /// Request timed out
    #[error("Request timed out after {0:?}")]
    Timeout(std::time::Duration),

    /// Task was cancelled
    #[error("Task cancelled: {0}")]
    Cancelled(String),

    /// Invalid tool call
    #[error("Invalid tool call: {0}")]
    InvalidToolCall(String),

    /// Tool execution failed
    #[error("Tool execution failed: {0}")]
    ToolExecutionFailed(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Channel send/receive error
    #[error("Channel error: {0}")]
    Channel(String),

    /// Provider-specific error
    #[error("Provider error: {0}")]
    Provider(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded, retry after {retry_after:?}")]
    RateLimited {
        retry_after: Option<std::time::Duration>,
    },

    /// The model returned an unexpected response format
    #[error("Unexpected response format: {0}")]
    UnexpectedFormat(String),
}

/// Result type for LLM operations
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Returns true if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Error::Timeout(_) | Error::RateLimited { .. } | Error::Channel(_)
        )
    }
}

/// Convenience trait for converting channel errors
impl<T> From<tokio::sync::mpsc::error::SendError<T>> for Error {
    fn from(err: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Error::Channel(format!("Send error: {}", err))
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for Error {
    fn from(err: tokio::sync::oneshot::error::RecvError) -> Self {
        Error::Channel(format!("Receive error: {}", err))
    }
}
