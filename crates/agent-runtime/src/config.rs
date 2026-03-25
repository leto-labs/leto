use provider::RequestOptions;
use serde::{Deserialize, Serialize};

/// Shared runtime configuration for a storeless in-memory session engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RuntimeConfig {
    /// Maximum provider/tool iterations allowed within one turn.
    pub max_iterations: u32,
    /// Maximum retry count for retryable provider failures before visible output.
    pub max_retries: u32,
    /// Fixed backoff between provider retries in milliseconds.
    pub retry_backoff_ms: u64,
    /// Maximum serialized tool output size retained in transcript updates.
    pub tool_output_max_bytes: usize,
    /// Optional model override for provider requests.
    pub model: Option<String>,
    /// Shared provider request options forwarded into each provider call.
    pub request: RequestOptions,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_iterations: 20,
            max_retries: 0,
            retry_backoff_ms: 100,
            tool_output_max_bytes: 16_000,
            model: None,
            request: RequestOptions::default(),
        }
    }
}
