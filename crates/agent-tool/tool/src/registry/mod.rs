use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Arc;

use futures::future::BoxFuture;
use provider::ToolDefinition;
use schemars::{JsonSchema, schema_for};
use serde::Serialize;
use serde::de::DeserializeOwned;
use thiserror::Error;

use crate::{ToolCall, ToolError, ToolExecutionResult, ToolExecutor};

fn schema_json<T: JsonSchema>() -> serde_json::Value {
    serde_json::to_value(schema_for!(T)).expect("schema should serialize to JSON")
}

/// Typed semantic contract for a single tool.
pub trait TypedTool: Send + Sync + 'static {
    /// Structured request type accepted by the tool.
    type Request: DeserializeOwned + JsonSchema + Send + 'static;
    /// Structured response type returned by the tool.
    type Response: Serialize + JsonSchema + Send + 'static;

    /// Stable tool name presented to the model.
    fn name(&self) -> &'static str;

    /// Human-readable description of the tool's behavior.
    fn description(&self) -> &'static str;

    /// Executes the tool with a validated typed request.
    fn execute_typed<'a>(
        &'a self,
        request: Self::Request,
    ) -> BoxFuture<'a, Result<Self::Response, ToolError>>;

    /// Builds the provider-visible tool definition.
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(
            self.name(),
            self.description(),
            schema_json::<Self::Request>(),
        )
        .with_output_schema(schema_json::<Self::Response>())
    }
}

/// Erased runtime-facing tool contract shared by typed and JSON-driven tools.
pub trait ErasedTool: Send + Sync {
    /// Stable tool name presented to the model.
    fn name(&self) -> &str;

    /// Provider-visible tool definition.
    fn definition(&self) -> ToolDefinition;

    /// Executes the tool with raw JSON input.
    fn execute<'a>(
        &'a self,
        input: serde_json::Value,
    ) -> BoxFuture<'a, Result<ToolExecutionResult, ToolError>>;
}

impl<T> ErasedTool for T
where
    T: TypedTool,
{
    fn name(&self) -> &str {
        TypedTool::name(self)
    }

    fn definition(&self) -> ToolDefinition {
        TypedTool::definition(self)
    }

    fn execute<'a>(
        &'a self,
        input: serde_json::Value,
    ) -> BoxFuture<'a, Result<ToolExecutionResult, ToolError>> {
        Box::pin(async move {
            let request = serde_json::from_value::<T::Request>(input).map_err(|error| {
                ToolError::new(format!("invalid input for {}: {error}", self.name()))
            })?;
            let response = self.execute_typed(request).await?;
            let output = serde_json::to_value(response).map_err(|error| {
                ToolError::new(format!("failed to encode tool output: {error}"))
            })?;
            Ok(ToolExecutionResult::success(output))
        })
    }
}

/// Registration-time errors for tool registries.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ToolRegistrationError {
    /// The requested stable tool name is already registered.
    #[error("duplicate tool registration: {name}")]
    DuplicateTool { name: String },
}

/// Async handler contract for scratch-authored JSON tools.
pub trait JsonToolHandler: Send + Sync + 'static {
    /// Executes the tool from raw JSON input.
    fn call(
        &self,
        input: serde_json::Value,
    ) -> BoxFuture<'static, Result<ToolExecutionResult, ToolError>>;
}

impl<F, Fut> JsonToolHandler for F
where
    F: Fn(serde_json::Value) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<ToolExecutionResult, ToolError>> + Send + 'static,
{
    fn call(
        &self,
        input: serde_json::Value,
    ) -> BoxFuture<'static, Result<ToolExecutionResult, ToolError>> {
        Box::pin((self)(input))
    }
}

/// Scratch-authored JSON-driven tool.
pub struct JsonTool {
    definition: ToolDefinition,
    handler: Arc<dyn JsonToolHandler>,
}

impl JsonTool {
    /// Creates a new JSON-driven tool from explicit schema metadata.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: serde_json::Value,
        handler: impl JsonToolHandler,
    ) -> Self {
        Self {
            definition: ToolDefinition::new(name, description, input_schema),
            handler: Arc::new(handler),
        }
    }

    /// Adds an explicit output schema to the tool definition.
    pub fn with_output_schema(mut self, output_schema: serde_json::Value) -> Self {
        self.definition = self.definition.with_output_schema(output_schema);
        self
    }
}

impl ErasedTool for JsonTool {
    fn name(&self) -> &str {
        &self.definition.name
    }

    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    fn execute<'a>(
        &'a self,
        input: serde_json::Value,
    ) -> BoxFuture<'a, Result<ToolExecutionResult, ToolError>> {
        let handler = self.handler.clone();
        Box::pin(async move { handler.call(input).await })
    }
}

/// Erased tool registry implementing the runtime-facing executor boundary.
#[derive(Clone, Default)]
pub struct RegistryToolExecutor {
    tools: BTreeMap<String, Arc<dyn ErasedTool>>,
}

impl RegistryToolExecutor {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a typed tool in the registry.
    pub fn register<T>(&mut self, tool: Arc<T>) -> Result<(), ToolRegistrationError>
    where
        T: TypedTool,
    {
        let tool: Arc<dyn ErasedTool> = tool;
        self.register_erased(tool)
    }

    /// Registers a scratch-authored JSON-driven tool in the registry.
    pub fn register_json(&mut self, tool: JsonTool) -> Result<(), ToolRegistrationError> {
        self.register_erased(Arc::new(tool))
    }

    /// Registers an already-erased tool in the registry.
    pub fn register_erased(
        &mut self,
        tool: Arc<dyn ErasedTool>,
    ) -> Result<(), ToolRegistrationError> {
        let name = tool.name().to_string();
        if self.tools.contains_key(&name) {
            return Err(ToolRegistrationError::DuplicateTool { name });
        }
        self.tools.insert(name, tool);
        Ok(())
    }

    /// Returns the stable tool names in registration order.
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }
}

impl ToolExecutor for RegistryToolExecutor {
    fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|tool| tool.definition()).collect()
    }

    fn execute<'a>(
        &'a self,
        call: ToolCall,
    ) -> BoxFuture<'a, Result<ToolExecutionResult, ToolError>> {
        Box::pin(async move {
            let tool = self
                .tools
                .get(&call.name)
                .ok_or_else(|| ToolError::new(format!("unknown tool: {}", call.name)))?;
            tool.execute(call.input).await
        })
    }
}

#[cfg(test)]
mod tests {
    use futures::future::BoxFuture;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    use super::{JsonTool, RegistryToolExecutor, ToolRegistrationError, TypedTool};
    use crate::{ToolCall, ToolError, ToolExecutionResult, ToolExecutor};
    use std::sync::Arc;

    #[derive(Debug, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    struct FakeRequest {
        /// Message to echo back.
        message: String,
    }

    #[derive(Debug, Serialize, JsonSchema)]
    #[serde(transparent)]
    #[schemars(description = "Echoed message text.")]
    struct FakeResponse(String);

    struct FakeTool;

    impl TypedTool for FakeTool {
        type Request = FakeRequest;
        type Response = FakeResponse;

        fn name(&self) -> &'static str {
            "fake_echo"
        }

        fn description(&self) -> &'static str {
            "Echoes the provided message."
        }

        fn execute_typed<'a>(
            &'a self,
            request: Self::Request,
        ) -> BoxFuture<'a, Result<Self::Response, ToolError>> {
            Box::pin(async move { Ok(FakeResponse(request.message)) })
        }
    }

    #[tokio::test]
    async fn registry_exposes_typed_schemas_and_executes_tools() {
        let mut registry = RegistryToolExecutor::new();
        registry
            .register(Arc::new(FakeTool))
            .expect("tool registration should succeed");

        let definitions = registry.definitions();
        assert_eq!(definitions[0].name, "fake_echo");
        assert_eq!(
            definitions[0].input_schema["required"],
            serde_json::json!(["message"])
        );
        assert_eq!(
            definitions[0].output_schema.as_ref().unwrap()["type"],
            "string"
        );
        assert_eq!(
            definitions[0].input_schema["properties"]["message"]["description"],
            "Message to echo back."
        );

        let result = registry
            .execute(ToolCall {
                id: "call-1".into(),
                name: "fake_echo".into(),
                input: serde_json::json!({ "message": "hello" }),
            })
            .await
            .expect("tool should execute");
        assert_eq!(result.output, serde_json::json!("hello"));
    }

    #[tokio::test]
    async fn registry_supports_json_tools() {
        let mut registry = RegistryToolExecutor::new();
        registry
            .register_json(
                JsonTool::new(
                    "json_echo",
                    "Echoes JSON input",
                    json!({
                        "type": "object",
                        "properties": { "message": { "type": "string" } },
                        "required": ["message"]
                    }),
                    |input: serde_json::Value| async move {
                        Ok(ToolExecutionResult::success(json!({
                            "message": input["message"].clone()
                        })))
                    },
                )
                .with_output_schema(json!({
                    "type": "object",
                    "properties": { "message": { "type": "string" } },
                    "required": ["message"]
                })),
            )
            .expect("json tool registration should succeed");

        let result = registry
            .execute(ToolCall {
                id: "call-1".into(),
                name: "json_echo".into(),
                input: json!({ "message": "hello" }),
            })
            .await
            .expect("json tool should execute");
        assert_eq!(result.output["message"], "hello");
    }

    #[test]
    fn registry_rejects_duplicate_names_without_panicking() {
        let mut registry = RegistryToolExecutor::new();
        registry
            .register(Arc::new(FakeTool))
            .expect("initial registration should succeed");

        let error = registry
            .register(Arc::new(FakeTool))
            .expect_err("duplicate registration should fail");
        assert_eq!(
            error,
            ToolRegistrationError::DuplicateTool {
                name: "fake_echo".into()
            }
        );
    }
}
