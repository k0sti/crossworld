//! Core types for LLM interactions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Role of a message participant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// System message providing context/instructions
    System,
    /// User input
    User,
    /// Assistant response
    Assistant,
    /// Tool/function result
    Tool,
}

/// A message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// The role of who sent this message
    pub role: Role,
    /// Text content of the message
    pub content: String,
    /// Optional name for multi-user scenarios
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Tool calls made by the assistant
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tool_calls: Vec<ToolCall>,
    /// Tool call ID if this is a tool result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl Message {
    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
            name: None,
            tool_calls: Vec::new(),
            tool_call_id: None,
        }
    }

    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            name: None,
            tool_calls: Vec::new(),
            tool_call_id: None,
        }
    }

    /// Create an assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            name: None,
            tool_calls: Vec::new(),
            tool_call_id: None,
        }
    }

    /// Create an assistant message with tool calls
    pub fn assistant_with_tools(content: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            name: None,
            tool_calls,
            tool_call_id: None,
        }
    }

    /// Create a tool result message
    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: content.into(),
            name: None,
            tool_calls: Vec::new(),
            tool_call_id: Some(tool_call_id.into()),
        }
    }

    /// Add a name to this message
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

/// A tool/function call made by the assistant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique identifier for this tool call
    pub id: String,
    /// Name of the tool to call
    pub name: String,
    /// Arguments as a JSON object
    pub arguments: serde_json::Value,
}

impl ToolCall {
    /// Create a new tool call
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: serde_json::Value,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            arguments,
        }
    }

    /// Parse arguments as a specific type
    pub fn parse_arguments<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_value(self.arguments.clone())
    }
}

/// Definition of a tool that can be called
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Name of the tool
    pub name: String,
    /// Description of what the tool does
    pub description: String,
    /// JSON schema for the parameters
    pub parameters: serde_json::Value,
}

impl ToolDefinition {
    /// Create a new tool definition
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
        }
    }

    /// Create a tool definition with no parameters
    pub fn no_params(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }
}

/// A request to generate a completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// Messages in the conversation
    pub messages: Vec<Message>,
    /// Available tools
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tools: Vec<ToolDefinition>,
    /// Maximum tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Temperature for sampling (0.0 - 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Stop sequences
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub stop: Vec<String>,
    /// Additional provider-specific parameters
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl CompletionRequest {
    /// Create a new completion request with messages
    pub fn new(messages: Vec<Message>) -> Self {
        Self {
            messages,
            tools: Vec::new(),
            max_tokens: None,
            temperature: None,
            stop: Vec::new(),
            extra: HashMap::new(),
        }
    }

    /// Add tools to the request
    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.tools = tools;
        self
    }

    /// Set the maximum tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set the temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Add stop sequences
    pub fn with_stop(mut self, stop: Vec<String>) -> Self {
        self.stop = stop;
        self
    }

    /// Add an extra parameter
    pub fn with_extra(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.extra.insert(key.into(), value);
        self
    }
}

/// Response from a completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// The generated message
    pub message: Message,
    /// Number of tokens in the prompt
    pub prompt_tokens: Option<u32>,
    /// Number of tokens generated
    pub completion_tokens: Option<u32>,
    /// Finish reason
    pub finish_reason: Option<FinishReason>,
    /// Model that was used
    pub model: Option<String>,
}

impl CompletionResponse {
    /// Create a new completion response
    pub fn new(message: Message) -> Self {
        Self {
            message,
            prompt_tokens: None,
            completion_tokens: None,
            finish_reason: None,
            model: None,
        }
    }

    /// Check if the response contains tool calls
    pub fn has_tool_calls(&self) -> bool {
        !self.message.tool_calls.is_empty()
    }

    /// Get the tool calls from the response
    pub fn tool_calls(&self) -> &[ToolCall] {
        &self.message.tool_calls
    }

    /// Get the text content
    pub fn content(&self) -> &str {
        &self.message.content
    }
}

/// Reason why generation stopped
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    /// Reached a natural stopping point
    Stop,
    /// Reached the max token limit
    Length,
    /// Model wants to call tools
    ToolCalls,
    /// Content was filtered
    ContentFilter,
    /// Other/unknown reason
    Other,
}

/// A streaming chunk from completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    /// Delta content (text being added)
    pub delta: String,
    /// Delta tool calls (partial tool call info)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tool_call_deltas: Vec<ToolCallDelta>,
    /// Finish reason if this is the final chunk
    pub finish_reason: Option<FinishReason>,
}

/// Partial tool call information during streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallDelta {
    /// Index of the tool call
    pub index: usize,
    /// Tool call ID (sent in first delta)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Tool name (sent in first delta)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Arguments delta (streamed incrementally)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments_delta: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_construction() {
        let msg = Message::user("Hello");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content, "Hello");

        let msg = Message::assistant("Hi there").with_name("Claude");
        assert_eq!(msg.role, Role::Assistant);
        assert_eq!(msg.name, Some("Claude".to_string()));
    }

    #[test]
    fn test_tool_call_parse() {
        let call = ToolCall::new(
            "call_123",
            "get_weather",
            serde_json::json!({"city": "Tokyo"}),
        );

        #[derive(Deserialize)]
        struct WeatherArgs {
            city: String,
        }

        let args: WeatherArgs = call.parse_arguments().unwrap();
        assert_eq!(args.city, "Tokyo");
    }

    #[test]
    fn test_completion_request_builder() {
        let request = CompletionRequest::new(vec![Message::user("Hello")])
            .with_max_tokens(100)
            .with_temperature(0.7);

        assert_eq!(request.max_tokens, Some(100));
        assert_eq!(request.temperature, Some(0.7));
    }
}
