use provider::RequestOptions;
use serde::{Deserialize, Serialize};

/// Configuration controlling runtime ATIF emission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AtifConfig {
    /// Whether the runtime should emit native ATIF lifecycle events.
    pub emit_events: bool,
}

impl Default for AtifConfig {
    fn default() -> Self {
        Self { emit_events: false }
    }
}

/// Configuration controlling transcript compaction support.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CompactionConfig {
    /// Whether loops should consider runtime-provided compaction advice.
    pub enabled: bool,
    /// Transcript pressure ratio at which the runtime should recommend compaction.
    pub threshold_ratio: f32,
    /// Number of trailing transcript messages to preserve verbatim.
    pub preserve_recent_messages: usize,
    /// Optional model override used for summary generation subcalls.
    pub summary_model: Option<String>,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold_ratio: 0.8,
            preserve_recent_messages: 6,
            summary_model: None,
        }
    }
}

/// Configuration controlling repeated-tool doom-loop detection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DoomLoopConfig {
    /// Whether the runtime should track repeated tool-call signatures.
    pub enabled: bool,
    /// Advisory repetition threshold that emits warnings to loops and observers.
    pub threshold: u32,
    /// Whether the runtime should abort once the same handled signature repeats again.
    pub error_after_handled_repetition: bool,
    /// Optional hard-stop threshold that aborts the turn after too many repeats.
    pub hard_error_threshold: Option<u32>,
    /// Maximum recent signature history retained for repetition tracking.
    pub signature_window: usize,
}

impl Default for DoomLoopConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold: 3,
            error_after_handled_repetition: false,
            hard_error_threshold: None,
            signature_window: 8,
        }
    }
}

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
    /// ATIF emission settings.
    pub atif: AtifConfig,
    /// Transcript compaction settings.
    pub compaction: CompactionConfig,
    /// Doom-loop detection settings.
    pub doom_loop: DoomLoopConfig,
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
            atif: AtifConfig::default(),
            compaction: CompactionConfig::default(),
            doom_loop: DoomLoopConfig::default(),
        }
    }
}
