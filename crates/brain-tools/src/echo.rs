use futures::future::BoxFuture;

use brain_types::{BrainError, Tool, ToolDef};

pub struct EchoTool;

impl Tool for EchoTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "echo".into(),
            description: "Echoes back the input message. For testing.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "The message to echo back"
                    }
                },
                "required": ["message"]
            }),
        }
    }

    fn execute(&self, args: serde_json::Value) -> BoxFuture<'_, Result<String, BrainError>> {
        Box::pin(async move {
            tracing::info!(args = %args, "echo tool invoked");
            let message = args
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            Ok(message.to_string())
        })
    }
}
