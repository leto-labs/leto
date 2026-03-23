#[cfg(feature = "native")]
pub mod native;

use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};

use brain_types::{BrainError, Tool, ToolDef};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalSessionAction {
    SendKeys,
    Capture,
    Close,
}

#[derive(Debug, Clone)]
pub struct TerminalSessionRequest {
    pub session_id: String,
    pub action: TerminalSessionAction,
    pub keystrokes: Option<String>,
    pub keys: Option<Vec<String>>,
    pub min_wait_seconds: f64,
    pub working_directory: Option<String>,
    pub rows: u16,
    pub cols: u16,
    pub max_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalSessionObservation {
    pub timed_out: bool,
    pub terminal_state: String,
    pub observation: String,
    pub shell_exited: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
}

pub trait TerminalSessionDriver: Send + Sync {
    fn execute(
        &self,
        request: TerminalSessionRequest,
    ) -> BoxFuture<'_, Result<TerminalSessionObservation, BrainError>>;
}

pub struct TerminalSessionTool<T: TerminalSessionDriver> {
    driver: T,
}

impl<T: TerminalSessionDriver> TerminalSessionTool<T> {
    pub fn new(driver: T) -> Self {
        Self { driver }
    }
}

impl<T: TerminalSessionDriver> Tool for TerminalSessionTool<T> {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "terminal_session".into(),
            description: "Interact with a persistent terminal session using verbatim keystrokes and bounded screen capture.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Stable session identifier for the persistent terminal."
                    },
                    "action": {
                        "type": "string",
                        "enum": ["send_keys", "capture", "close"],
                        "description": "Terminal session operation to perform."
                    },
                    "keystrokes": {
                        "type": "string",
                        "description": "Verbatim keystrokes to send when action is send_keys."
                    },
                    "keys": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Optional logical key tokens to send when action is send_keys, such as C-c, C-d, or Enter."
                    },
                    "min_wait_seconds": {
                        "type": "number",
                        "description": "Minimum time to wait after sending keys before capturing output."
                    },
                    "working_directory": {
                        "type": "string",
                        "description": "Optional working directory used when lazily starting a session."
                    },
                    "rows": {
                        "type": "integer",
                        "description": "Terminal row count used when lazily starting a session."
                    },
                    "cols": {
                        "type": "integer",
                        "description": "Terminal column count used when lazily starting a session."
                    },
                    "max_bytes": {
                        "type": "integer",
                        "description": "Maximum number of bytes to keep in returned observation strings."
                    }
                },
                "required": ["session_id", "action"]
            }),
        }
    }

    fn execute(&self, args: serde_json::Value) -> BoxFuture<'_, Result<String, BrainError>> {
        Box::pin(async move {
            let session_id = args
                .get("session_id")
                .and_then(|value| value.as_str())
                .ok_or_else(|| BrainError::ToolFailed {
                    tool: "terminal_session".into(),
                    reason: "missing required parameter 'session_id'".into(),
                })?
                .to_owned();

            let action = match args.get("action").and_then(|value| value.as_str()) {
                Some("send_keys") => TerminalSessionAction::SendKeys,
                Some("capture") => TerminalSessionAction::Capture,
                Some("close") => TerminalSessionAction::Close,
                Some(other) => {
                    return Err(BrainError::ToolFailed {
                        tool: "terminal_session".into(),
                        reason: format!(
                            "unsupported action '{other}' (expected send_keys, capture, or close)"
                        ),
                    });
                }
                None => {
                    return Err(BrainError::ToolFailed {
                        tool: "terminal_session".into(),
                        reason: "missing required parameter 'action'".into(),
                    });
                }
            };

            let keystrokes = args
                .get("keystrokes")
                .and_then(|value| value.as_str())
                .map(ToOwned::to_owned);
            let keys = args.get("keys").and_then(|value| {
                value.as_array().map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(ToOwned::to_owned))
                        .collect::<Vec<_>>()
                })
            });
            if matches!(action, TerminalSessionAction::SendKeys)
                && keystrokes.is_none()
                && keys.as_ref().is_none_or(Vec::is_empty)
            {
                return Err(BrainError::ToolFailed {
                    tool: "terminal_session".into(),
                    reason: "missing required parameter 'keystrokes' or 'keys' for send_keys"
                        .into(),
                });
            }

            let min_wait_seconds = args
                .get("min_wait_seconds")
                .and_then(|value| value.as_f64())
                .unwrap_or(0.0);
            let working_directory = args
                .get("working_directory")
                .and_then(|value| value.as_str())
                .map(ToOwned::to_owned);
            let rows = args
                .get("rows")
                .and_then(|value| value.as_u64())
                .map(|value| value as u16)
                .unwrap_or(40);
            let cols = args
                .get("cols")
                .and_then(|value| value.as_u64())
                .map(|value| value as u16)
                .unwrap_or(160);
            let max_bytes = args
                .get("max_bytes")
                .and_then(|value| value.as_u64())
                .map(|value| value as usize)
                .unwrap_or(10_000);

            tracing::info!(session_id, ?action, "terminal_session invoked");
            let observation = self
                .driver
                .execute(TerminalSessionRequest {
                    session_id,
                    action,
                    keystrokes,
                    keys,
                    min_wait_seconds,
                    working_directory,
                    rows,
                    cols,
                    max_bytes,
                })
                .await?;

            serde_json::to_string(&observation).map_err(BrainError::from)
        })
    }
}
