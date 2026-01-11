//! Trellis HTTP client
//!
//! This module provides the HTTP client for communicating with the Trellis inference server.

use crate::types::{GenerationRequest, ServerStatus, TrellisResult};

/// Trellis HTTP client
#[allow(dead_code)]
pub struct TrellisClient {
    base_url: String,
    client: reqwest::Client,
}

impl TrellisClient {
    /// Create a new Trellis client
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: reqwest::Client::new(),
        }
    }

    /// Check server health
    pub async fn health_check(&self) -> TrellisResult<ServerStatus> {
        todo!("Implement health check")
    }

    /// Generate 3D model from prompt
    pub async fn generate(&self, _request: &GenerationRequest) -> TrellisResult<()> {
        todo!("Implement generate")
    }
}
