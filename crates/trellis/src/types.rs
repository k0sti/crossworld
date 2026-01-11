//! Trellis types
//!
//! Request/response types for the Trellis inference server.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Trellis error type
#[derive(Debug, Error)]
pub enum TrellisError {
    /// HTTP client error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Server returned an error
    #[error("Server error: {0}")]
    Server(String),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Invalid response from server
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

/// Result type alias for Trellis operations
pub type TrellisResult<T> = Result<T, TrellisError>;

/// Server health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStatus {
    /// Server status string
    pub status: String,
    /// Whether Trellis model is available
    pub trellis_available: bool,
    /// Whether GPU is available
    pub gpu_available: bool,
    /// GPU name (if available)
    pub gpu_name: Option<String>,
    /// Whether model is loaded
    pub model_loaded: bool,
    /// Error message (if any)
    pub error: Option<String>,
}

impl ServerStatus {
    /// Check if server is ready
    pub fn is_ready(&self) -> bool {
        self.status == "ready" && self.trellis_available && self.model_loaded
    }
}

/// Generation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRequest {
    /// Text prompt
    pub prompt: String,
    /// Optional image input (base64 encoded)
    pub image: Option<String>,
    /// Random seed for reproducibility
    pub seed: Option<i32>,
}

impl GenerationRequest {
    /// Create a new generation request from text prompt
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            image: None,
            seed: None,
        }
    }

    /// Set image input (base64 encoded)
    pub fn with_image(mut self, image: String) -> Self {
        self.image = Some(image);
        self
    }

    /// Set random seed
    pub fn with_seed(mut self, seed: i32) -> Self {
        self.seed = Some(seed);
        self
    }
}
