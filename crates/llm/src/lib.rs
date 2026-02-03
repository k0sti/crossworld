//! LLM inference client abstraction for Crossworld
//!
//! This crate provides a unified interface for interacting with Large Language Models (LLMs),
//! supporting:
//!
//! - **AI inference client abstraction**: Provider-agnostic client trait for completions
//! - **Background task management**: Spawn and manage async LLM tasks with cancellation
//! - **Tool call support**: Define and execute tools for agent patterns
//! - **Text streaming**: Stream completions chunk-by-chunk
//!
//! # Example
//!
//! ```rust,ignore
//! use crossworld_llm::{
//!     Message, CompletionRequest, ToolDefinitionBuilder, ToolRegistry, spawn_task,
//! };
//!
//! // Create a completion request
//! let request = CompletionRequest::new(vec![
//!     Message::system("You are a helpful assistant."),
//!     Message::user("What's the weather in Tokyo?"),
//! ]);
//!
//! // Define tools
//! let weather_tool = ToolDefinitionBuilder::new("get_weather")
//!     .description("Get the current weather for a city")
//!     .string_param("city", "The city name", true)
//!     .build();
//!
//! // Add tools to the request
//! let request = request.with_tools(vec![weather_tool]);
//!
//! // Spawn as a background task
//! let handle = spawn_task(|ctx| async move {
//!     // Use your LLM client here
//!     // client.complete(request).await
//!     ctx.check_cancelled()?;
//!     Ok(())
//! });
//!
//! // Wait for completion or cancel
//! handle.cancel();
//! ```
//!
//! # Modules
//!
//! - [`client`]: LLM client trait and streaming interfaces
//! - [`error`]: Error types for LLM operations
//! - [`task`]: Background task management with cancellation
//! - [`tools`]: Tool definition and execution for agents
//! - [`types`]: Core types (messages, completions, tool calls)

pub mod client;
pub mod error;
pub mod task;
pub mod tools;
pub mod types;

// Re-export commonly used types
pub use client::{ChunkStream, ClientConfig, LlmClient, StreamAccumulator};
pub use error::{Error, Result};
pub use task::{
    spawn_task, spawn_task_with_timeout, TaskBuilder, TaskContext, TaskHandle, TaskId, TaskStatus,
};
pub use tools::{FnTool, ToolDefinitionBuilder, ToolHandler, ToolRegistry, ToolResult};
pub use types::{
    CompletionRequest, CompletionResponse, FinishReason, Message, Role, StreamChunk, ToolCall,
    ToolCallDelta, ToolDefinition,
};

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::client::{ChunkStream, ClientConfig, LlmClient, StreamAccumulator};
    pub use crate::error::{Error, Result};
    pub use crate::task::{
        spawn_task, spawn_task_with_timeout, TaskContext, TaskHandle, TaskStatus,
    };
    pub use crate::tools::{ToolDefinitionBuilder, ToolHandler, ToolRegistry};
    pub use crate::types::{
        CompletionRequest, CompletionResponse, Message, Role, ToolCall, ToolDefinition,
    };
}
