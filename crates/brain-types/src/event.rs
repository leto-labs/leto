//! Runtime event types.
//!
//! `Event::Atif` is additive to the normal operational stream. It carries
//! transcript/export completion records for consumers that care about ATIF,
//! while token/tool/progress/turn events remain the main live runtime
//! interface.

use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::error::BrainErrorCode;
use crate::message::Message;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AtifEvent {
    /// Emitted when ATIF tracking starts for a session. This advertises the
    /// target schema and root agent metadata before any completed steps exist.
    TrajectoryStarted {
        schema_version: atif::SchemaVersion,
        session_id: String,
        agent: atif::Agent,
    },
    /// Emitted when a new transcript step from the current turn is fully known.
    StepCompleted {
        step: atif::Step,
    },
    /// Emitted once per completed turn with aggregate metrics for the current
    /// trajectory view.
    FinalMetrics {
        final_metrics: atif::FinalMetrics,
    },
    /// Emitted with the full transcript as of the end of the turn.
    TrajectoryCompleted {
        trajectory: atif::Trajectory,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Event {
    // -- existing --
    Token {
        delta: String,
    },
    ToolCallStart {
        id: String,
        name: String,
        arguments: serde_json::Value,
    },
    ToolCallDone {
        id: String,
        result: String,
        is_error: bool,
    },
    MessageDone {
        message: Message,
    },
    TurnDone {
        iterations: u32,
        prompt_tokens: Option<u32>,
        completion_tokens: Option<u32>,
        cache_read_tokens: Option<u32>,
        cache_write_tokens: Option<u32>,
        reasoning_tokens: Option<u32>,
        total_tokens: u32,
    },
    Error {
        code: BrainErrorCode,
        message: String,
        recoverable: bool,
    },

    // -- streaming tool call args --
    ToolCallDelta {
        id: String,
        name: String,
        arguments_delta: String,
    },

    // -- tool approval flow --
    ToolCallPending {
        id: String,
        name: String,
        arguments: serde_json::Value,
    },
    ToolCallApproved {
        id: String,
    },
    ToolCallRejected {
        id: String,
        reason: String,
    },

    // -- progress --
    Progress {
        phase: String,
        message: String,
        percent: Option<f32>,
    },

    // -- session lifecycle --
    SessionStart {
        session_id: Ulid,
    },
    SessionResume {
        session_id: Ulid,
    },

    // -- hardening/runtime --
    Retry {
        attempt: u32,
        max: u32,
        error: String,
    },
    Compaction {
        original_messages: usize,
        summary_tokens: usize,
    },
    DoomLoopWarning {
        tool_name: String,
        repetitions: u32,
    },
    Atif {
        event: AtifEvent,
    },
}

#[cfg(test)]
mod tests {
    use super::{AtifEvent, Event};

    #[test]
    fn serialize_retry_event() {
        let event = Event::Retry {
            attempt: 1,
            max: 3,
            error: "network".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: Event = serde_json::from_str(&json).unwrap();
        assert!(json.contains("\"type\":\"retry\""));

        match parsed {
            Event::Retry {
                attempt,
                max,
                error,
            } => {
                assert_eq!(attempt, 1);
                assert_eq!(max, 3);
                assert_eq!(error, "network");
            }
            _ => panic!("expected retry event"),
        }
    }

    #[test]
    fn serialize_compaction_event() {
        let event = Event::Compaction {
            original_messages: 12,
            summary_tokens: 42,
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: Event = serde_json::from_str(&json).unwrap();
        assert!(json.contains("\"type\":\"compaction\""));

        match parsed {
            Event::Compaction {
                original_messages,
                summary_tokens,
            } => {
                assert_eq!(original_messages, 12);
                assert_eq!(summary_tokens, 42);
            }
            _ => panic!("expected compaction event"),
        }
    }

    #[test]
    fn serialize_doom_loop_warning_event() {
        let event = Event::DoomLoopWarning {
            tool_name: "search".into(),
            repetitions: 5,
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: Event = serde_json::from_str(&json).unwrap();
        assert!(json.contains("\"type\":\"doom_loop_warning\""));

        match parsed {
            Event::DoomLoopWarning {
                tool_name,
                repetitions,
            } => {
                assert_eq!(tool_name, "search");
                assert_eq!(repetitions, 5);
            }
            _ => panic!("expected doom loop warning event"),
        }
    }

    #[test]
    fn serialize_atif_event() {
        let event = Event::Atif {
            event: AtifEvent::FinalMetrics {
                final_metrics: atif::FinalMetrics {
                    total_prompt_tokens: Some(10),
                    total_completion_tokens: Some(5),
                    total_cached_tokens: None,
                    total_cost_usd: Some(0.01),
                    total_steps: Some(2),
                    extra: None,
                },
            },
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: Event = serde_json::from_str(&json).unwrap();
        assert!(json.contains("\"type\":\"atif\""));
        assert!(json.contains("\"kind\":\"final_metrics\""));

        match parsed {
            Event::Atif {
                event: AtifEvent::FinalMetrics { final_metrics },
            } => {
                assert_eq!(final_metrics.total_prompt_tokens, Some(10));
                assert_eq!(final_metrics.total_steps, Some(2));
            }
            _ => panic!("expected atif event"),
        }
    }
}
