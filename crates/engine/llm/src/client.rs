//! LLM client abstraction and streaming interface

use crate::error::{Error, Result};
use crate::types::{CompletionRequest, CompletionResponse, StreamChunk};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// A boxed stream of stream chunks
pub type ChunkStream = Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>;

/// Trait for LLM inference clients
///
/// This trait provides a unified interface for different LLM providers,
/// supporting both blocking and streaming completions.
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Get the name of this client/provider
    fn name(&self) -> &str;

    /// Get the default model for this client
    fn default_model(&self) -> &str;

    /// Check if the client is connected and ready
    async fn is_ready(&self) -> bool;

    /// Generate a completion (blocking until complete)
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;

    /// Generate a completion with streaming response
    async fn complete_stream(&self, request: CompletionRequest) -> Result<ChunkStream>;
}

/// Configuration for an LLM client
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// API endpoint URL
    pub endpoint: Option<String>,
    /// API key for authentication
    pub api_key: Option<String>,
    /// Default model to use
    pub model: String,
    /// Request timeout
    pub timeout: std::time::Duration,
    /// Maximum retries for transient failures
    pub max_retries: u32,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            endpoint: None,
            api_key: None,
            model: String::new(),
            timeout: std::time::Duration::from_secs(60),
            max_retries: 3,
        }
    }
}

impl ClientConfig {
    /// Create a new configuration with the specified model
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            ..Default::default()
        }
    }

    /// Set the API endpoint
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    /// Set the API key
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Set the request timeout
    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the maximum retries
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }
}

/// Helper to accumulate streaming chunks into a complete response
#[derive(Default)]
pub struct StreamAccumulator {
    content: String,
    tool_calls: Vec<crate::types::ToolCall>,
    tool_call_buffers: std::collections::HashMap<usize, ToolCallBuffer>,
    finish_reason: Option<crate::types::FinishReason>,
}

#[derive(Default)]
struct ToolCallBuffer {
    id: Option<String>,
    name: Option<String>,
    arguments: String,
}

impl StreamAccumulator {
    /// Create a new accumulator
    pub fn new() -> Self {
        Self::default()
    }

    /// Process a stream chunk
    pub fn process(&mut self, chunk: StreamChunk) {
        // Accumulate text content
        self.content.push_str(&chunk.delta);

        // Process tool call deltas
        for delta in chunk.tool_call_deltas {
            let buffer = self.tool_call_buffers.entry(delta.index).or_default();

            if let Some(id) = delta.id {
                buffer.id = Some(id);
            }
            if let Some(name) = delta.name {
                buffer.name = Some(name);
            }
            if let Some(args_delta) = delta.arguments_delta {
                buffer.arguments.push_str(&args_delta);
            }
        }

        // Store finish reason
        if chunk.finish_reason.is_some() {
            self.finish_reason = chunk.finish_reason;
        }
    }

    /// Finalize and get the complete response
    pub fn finish(mut self) -> Result<CompletionResponse> {
        // Convert tool call buffers to actual tool calls
        let mut indices: Vec<_> = self.tool_call_buffers.keys().copied().collect();
        indices.sort();

        for idx in indices {
            if let Some(buffer) = self.tool_call_buffers.remove(&idx) {
                let id = buffer
                    .id
                    .ok_or_else(|| Error::UnexpectedFormat("Missing tool call ID".to_string()))?;
                let name = buffer
                    .name
                    .ok_or_else(|| Error::UnexpectedFormat("Missing tool call name".to_string()))?;
                let arguments: serde_json::Value = serde_json::from_str(&buffer.arguments)
                    .map_err(|e| {
                        Error::UnexpectedFormat(format!("Invalid tool arguments: {}", e))
                    })?;

                self.tool_calls
                    .push(crate::types::ToolCall::new(id, name, arguments));
            }
        }

        let message = if self.tool_calls.is_empty() {
            crate::types::Message::assistant(self.content)
        } else {
            crate::types::Message::assistant_with_tools(self.content, self.tool_calls)
        };

        Ok(CompletionResponse {
            message,
            prompt_tokens: None,
            completion_tokens: None,
            finish_reason: self.finish_reason,
            model: None,
        })
    }

    /// Get the current accumulated content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Check if any tool calls are being accumulated
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_call_buffers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FinishReason, StreamChunk, ToolCallDelta};

    #[test]
    fn test_stream_accumulator_text() {
        let mut acc = StreamAccumulator::new();

        acc.process(StreamChunk {
            delta: "Hello".to_string(),
            tool_call_deltas: vec![],
            finish_reason: None,
        });

        acc.process(StreamChunk {
            delta: " world!".to_string(),
            tool_call_deltas: vec![],
            finish_reason: Some(FinishReason::Stop),
        });

        let response = acc.finish().unwrap();
        assert_eq!(response.content(), "Hello world!");
        assert_eq!(response.finish_reason, Some(FinishReason::Stop));
    }

    #[test]
    fn test_stream_accumulator_tool_calls() {
        let mut acc = StreamAccumulator::new();

        // First chunk with tool call start
        acc.process(StreamChunk {
            delta: "".to_string(),
            tool_call_deltas: vec![ToolCallDelta {
                index: 0,
                id: Some("call_1".to_string()),
                name: Some("get_weather".to_string()),
                arguments_delta: Some("{\"".to_string()),
            }],
            finish_reason: None,
        });

        // Second chunk with arguments continuation
        acc.process(StreamChunk {
            delta: "".to_string(),
            tool_call_deltas: vec![ToolCallDelta {
                index: 0,
                id: None,
                name: None,
                arguments_delta: Some("city\": \"Tokyo\"}".to_string()),
            }],
            finish_reason: Some(FinishReason::ToolCalls),
        });

        let response = acc.finish().unwrap();
        assert!(response.has_tool_calls());
        assert_eq!(response.tool_calls().len(), 1);
        assert_eq!(response.tool_calls()[0].name, "get_weather");
    }

    #[test]
    fn test_client_config_builder() {
        let config = ClientConfig::new("gpt-4")
            .with_endpoint("https://api.openai.com/v1")
            .with_api_key("sk-xxx")
            .with_timeout(std::time::Duration::from_secs(30))
            .with_max_retries(5);

        assert_eq!(config.model, "gpt-4");
        assert_eq!(
            config.endpoint,
            Some("https://api.openai.com/v1".to_string())
        );
        assert_eq!(config.api_key, Some("sk-xxx".to_string()));
        assert_eq!(config.timeout, std::time::Duration::from_secs(30));
        assert_eq!(config.max_retries, 5);
    }
}
