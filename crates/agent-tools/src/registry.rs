use std::collections::BTreeMap;
use std::sync::Arc;

use agent_runtime::{RuntimeError, ToolCall, ToolExecutionResult, ToolExecutor};
use futures::future::BoxFuture;
use provider::ToolDefinition;
use schemars::{JsonSchema, schema_for};
use serde::Serialize;
use serde::de::DeserializeOwned;

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
    ) -> BoxFuture<'a, Result<Self::Response, RuntimeError>>;

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

trait ErasedTool: Send + Sync {
    fn name(&self) -> &'static str;
    fn definition(&self) -> ToolDefinition;
    fn execute<'a>(
        &'a self,
        input: serde_json::Value,
    ) -> BoxFuture<'a, Result<ToolExecutionResult, RuntimeError>>;
}

impl<T> ErasedTool for T
where
    T: TypedTool,
{
    fn name(&self) -> &'static str {
        TypedTool::name(self)
    }

    fn definition(&self) -> ToolDefinition {
        TypedTool::definition(self)
    }

    fn execute<'a>(
        &'a self,
        input: serde_json::Value,
    ) -> BoxFuture<'a, Result<ToolExecutionResult, RuntimeError>> {
        Box::pin(async move {
            let request = serde_json::from_value::<T::Request>(input).map_err(|error| {
                RuntimeError::Tool(format!("invalid input for {}: {error}", self.name()))
            })?;
            let response = self.execute_typed(request).await?;
            let output = serde_json::to_value(response).map_err(|error| {
                RuntimeError::Tool(format!("failed to encode tool output: {error}"))
            })?;
            Ok(ToolExecutionResult::success(output))
        })
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

    /// Registers a tool in the registry.
    pub fn register<T>(&mut self, tool: Arc<T>)
    where
        T: TypedTool,
    {
        let tool: Arc<dyn ErasedTool> = tool;
        let name = tool.name().to_string();
        let previous = self.tools.insert(name.clone(), tool);
        assert!(previous.is_none(), "duplicate tool registration: {name}");
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
    ) -> BoxFuture<'a, Result<ToolExecutionResult, RuntimeError>> {
        Box::pin(async move {
            let tool = self
                .tools
                .get(&call.name)
                .ok_or_else(|| RuntimeError::Tool(format!("unknown tool: {}", call.name)))?;
            tool.execute(call.input).await
        })
    }
}

#[cfg(test)]
mod tests {
    use agent_runtime::{RuntimeError, ToolExecutor};
    use futures::future::BoxFuture;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    use super::{RegistryToolExecutor, TypedTool};
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
        ) -> BoxFuture<'a, Result<Self::Response, RuntimeError>> {
            Box::pin(async move { Ok(FakeResponse(request.message)) })
        }
    }

    #[tokio::test]
    async fn registry_exposes_typed_schemas_and_executes_tools() {
        let mut registry = RegistryToolExecutor::new();
        registry.register(Arc::new(FakeTool));

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
            .execute(agent_runtime::ToolCall {
                id: "call-1".into(),
                name: "fake_echo".into(),
                input: serde_json::json!({ "message": "hello" }),
            })
            .await
            .expect("tool should execute");
        assert_eq!(result.output, serde_json::json!("hello"));
    }
}
