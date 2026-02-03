//! Tool call support for agent patterns

use crate::error::{Error, Result};
use crate::types::{ToolCall, ToolDefinition};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Trait for implementing tool handlers
#[async_trait]
pub trait ToolHandler: Send + Sync {
    /// Get the tool definition
    fn definition(&self) -> ToolDefinition;

    /// Execute the tool with the given arguments
    async fn execute(&self, arguments: Value) -> Result<Value>;

    /// Validate arguments before execution (optional)
    fn validate(&self, arguments: &Value) -> Result<()> {
        let _ = arguments;
        Ok(())
    }
}

/// A registry for managing available tools
#[derive(Default)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn ToolHandler>>,
}

impl ToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a tool handler
    pub fn register<T: ToolHandler + 'static>(&mut self, handler: T) {
        let name = handler.definition().name.clone();
        self.tools.insert(name, Arc::new(handler));
    }

    /// Register a tool handler with a custom name
    pub fn register_as<T: ToolHandler + 'static>(&mut self, name: impl Into<String>, handler: T) {
        self.tools.insert(name.into(), Arc::new(handler));
    }

    /// Get a tool handler by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn ToolHandler>> {
        self.tools.get(name).cloned()
    }

    /// Check if a tool is registered
    pub fn contains(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Get all tool definitions
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|h| h.definition()).collect()
    }

    /// Get the names of all registered tools
    pub fn names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Execute a tool call
    pub async fn execute(&self, tool_call: &ToolCall) -> Result<Value> {
        let handler = self
            .get(&tool_call.name)
            .ok_or_else(|| Error::InvalidToolCall(format!("Unknown tool: {}", tool_call.name)))?;

        handler.validate(&tool_call.arguments)?;
        handler.execute(tool_call.arguments.clone()).await
    }

    /// Execute multiple tool calls in parallel
    pub async fn execute_all(&self, tool_calls: &[ToolCall]) -> Vec<(String, Result<Value>)> {
        let futures: Vec<_> = tool_calls
            .iter()
            .map(|tc| {
                let id = tc.id.clone();
                let registry = self;
                let tool_call = tc.clone();
                async move {
                    let result = registry.execute(&tool_call).await;
                    (id, result)
                }
            })
            .collect();

        futures::future::join_all(futures).await
    }
}

/// Result of executing a tool call
#[derive(Debug, Clone)]
pub struct ToolResult {
    /// The ID of the tool call this is a result for
    pub tool_call_id: String,
    /// Whether the execution was successful
    pub success: bool,
    /// The result value (or error message)
    pub value: Value,
}

impl ToolResult {
    /// Create a successful tool result
    pub fn success(tool_call_id: impl Into<String>, value: Value) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            success: true,
            value,
        }
    }

    /// Create a failed tool result
    pub fn failure(tool_call_id: impl Into<String>, error: impl std::fmt::Display) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            success: false,
            value: Value::String(error.to_string()),
        }
    }

    /// Convert the result to a string for message content
    pub fn to_content(&self) -> String {
        match &self.value {
            Value::String(s) => s.clone(),
            other => serde_json::to_string_pretty(other).unwrap_or_else(|_| other.to_string()),
        }
    }
}

/// A simple function-based tool handler
pub struct FnTool<F> {
    definition: ToolDefinition,
    handler: F,
}

impl<F, Fut> FnTool<F>
where
    F: Fn(Value) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<Value>> + Send,
{
    /// Create a new function-based tool
    pub fn new(definition: ToolDefinition, handler: F) -> Self {
        Self {
            definition,
            handler,
        }
    }
}

#[async_trait]
impl<F, Fut> ToolHandler for FnTool<F>
where
    F: Fn(Value) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<Value>> + Send,
{
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn execute(&self, arguments: Value) -> Result<Value> {
        (self.handler)(arguments).await
    }
}

/// Builder for creating tool definitions with a fluent API
pub struct ToolDefinitionBuilder {
    name: String,
    description: String,
    properties: serde_json::Map<String, Value>,
    required: Vec<String>,
}

impl ToolDefinitionBuilder {
    /// Start building a tool definition
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            properties: serde_json::Map::new(),
            required: Vec::new(),
        }
    }

    /// Set the description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Add a string parameter
    pub fn string_param(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        let name = name.into();
        self.properties.insert(
            name.clone(),
            serde_json::json!({
                "type": "string",
                "description": description.into()
            }),
        );
        if required {
            self.required.push(name);
        }
        self
    }

    /// Add a number parameter
    pub fn number_param(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        let name = name.into();
        self.properties.insert(
            name.clone(),
            serde_json::json!({
                "type": "number",
                "description": description.into()
            }),
        );
        if required {
            self.required.push(name);
        }
        self
    }

    /// Add a boolean parameter
    pub fn bool_param(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        let name = name.into();
        self.properties.insert(
            name.clone(),
            serde_json::json!({
                "type": "boolean",
                "description": description.into()
            }),
        );
        if required {
            self.required.push(name);
        }
        self
    }

    /// Add an array parameter
    pub fn array_param(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        item_type: &str,
        required: bool,
    ) -> Self {
        let name = name.into();
        self.properties.insert(
            name.clone(),
            serde_json::json!({
                "type": "array",
                "description": description.into(),
                "items": { "type": item_type }
            }),
        );
        if required {
            self.required.push(name);
        }
        self
    }

    /// Add an enum parameter
    pub fn enum_param(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        values: &[&str],
        required: bool,
    ) -> Self {
        let name = name.into();
        self.properties.insert(
            name.clone(),
            serde_json::json!({
                "type": "string",
                "description": description.into(),
                "enum": values
            }),
        );
        if required {
            self.required.push(name);
        }
        self
    }

    /// Build the tool definition
    pub fn build(self) -> ToolDefinition {
        ToolDefinition {
            name: self.name,
            description: self.description,
            parameters: serde_json::json!({
                "type": "object",
                "properties": self.properties,
                "required": self.required
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoTool;

    #[async_trait]
    impl ToolHandler for EchoTool {
        fn definition(&self) -> ToolDefinition {
            ToolDefinitionBuilder::new("echo")
                .description("Echoes the input")
                .string_param("message", "Message to echo", true)
                .build()
        }

        async fn execute(&self, arguments: Value) -> Result<Value> {
            let message = arguments
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("no message");
            Ok(Value::String(format!("Echo: {}", message)))
        }
    }

    #[tokio::test]
    async fn test_tool_registry() {
        let mut registry = ToolRegistry::new();
        registry.register(EchoTool);

        assert!(registry.contains("echo"));
        assert_eq!(registry.names(), vec!["echo"]);

        let definitions = registry.definitions();
        assert_eq!(definitions.len(), 1);
        assert_eq!(definitions[0].name, "echo");
    }

    #[tokio::test]
    async fn test_tool_execution() {
        let mut registry = ToolRegistry::new();
        registry.register(EchoTool);

        let call = ToolCall::new("call_1", "echo", serde_json::json!({"message": "Hello"}));

        let result = registry.execute(&call).await.unwrap();
        assert_eq!(result, Value::String("Echo: Hello".to_string()));
    }

    #[tokio::test]
    async fn test_unknown_tool() {
        let registry = ToolRegistry::new();
        let call = ToolCall::new("call_1", "unknown", serde_json::json!({}));

        let result = registry.execute(&call).await;
        assert!(matches!(result, Err(Error::InvalidToolCall(_))));
    }

    #[test]
    fn test_tool_definition_builder() {
        let def = ToolDefinitionBuilder::new("test_tool")
            .description("A test tool")
            .string_param("name", "The name", true)
            .number_param("count", "The count", false)
            .bool_param("enabled", "Is enabled", true)
            .build();

        assert_eq!(def.name, "test_tool");
        assert_eq!(def.description, "A test tool");

        let params = &def.parameters;
        assert!(params.get("properties").is_some());
        assert!(params.get("required").is_some());
    }

    #[tokio::test]
    async fn test_fn_tool() {
        let definition = ToolDefinitionBuilder::new("add")
            .description("Adds two numbers")
            .number_param("a", "First number", true)
            .number_param("b", "Second number", true)
            .build();

        let tool = FnTool::new(definition, |args: Value| async move {
            let a = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let b = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
            Ok(Value::Number(serde_json::Number::from_f64(a + b).unwrap()))
        });

        let result = tool
            .execute(serde_json::json!({"a": 2, "b": 3}))
            .await
            .unwrap();
        assert_eq!(result, serde_json::json!(5.0));
    }
}
