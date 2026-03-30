use std::path::PathBuf;

use agent_core::ProviderModelInfo;
use agent_runtime::RuntimeConfig;
use agent_store::{CredentialEntry, CredentialHealth, ProjectConfig, SessionId};
use provider::{Message, ProviderCapabilities};
use serde::{Deserialize, Serialize};

/// Basic health payload exposed by canonical health endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub healthy: bool,
    pub version: String,
}

impl HealthResponse {
    /// Returns the current package version for the running crate.
    pub fn current() -> Self {
        Self {
            healthy: true,
            version: env!("CARGO_PKG_VERSION").to_owned(),
        }
    }
}

/// Aggregate server status for the canonical API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentServerStatus {
    pub provider_names: Vec<String>,
    pub loop_names: Vec<String>,
    pub default_provider_name: String,
    pub default_loop_name: String,
    pub project_count: usize,
    pub session_count: usize,
}

/// Lightweight agent inventory entry exposed by the canonical API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentInfoRecord {
    /// Stable agent identifier.
    pub name: String,
    /// Human-readable summary when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Request body for creating a project.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateProjectRequest {
    pub name: Option<String>,
    pub root: Option<PathBuf>,
    #[serde(default)]
    pub config: Option<ProjectConfig>,
}

/// Request body for root-based project lookup or resolution.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProjectRootRequest {
    pub root: PathBuf,
}

/// Request body for creating a session with optional initial overrides.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CreateSessionRequest {
    pub title: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub loop_name: Option<String>,
}

/// Request body for updating credential health.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdateCredentialHealthRequest {
    pub health: CredentialHealth,
}

/// Request body for starting a turn through the canonical API.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TurnRequest {
    pub input: Vec<Message>,
}

/// Request body for starting multiple turns for one session sequentially.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct BatchTurnRequest {
    pub turns: Vec<TurnRequest>,
}

/// One tool-call transcript item to append through the canonical API.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolCallRecord {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
    pub output: serde_json::Value,
    #[serde(default)]
    pub is_error: bool,
}

/// Request body for appending tool-call history to one session transcript.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ToolCallRequest {
    pub calls: Vec<ToolCallRecord>,
}

/// Effective runtime view for one session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRuntimeView {
    pub config: RuntimeConfig,
    pub current_loop_name: String,
    pub current_model_id: Option<String>,
}

/// Provider summary for the canonical catalog endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCatalogEntry {
    pub name: String,
    pub model_ids: Vec<String>,
}

/// Owned model-pricing metadata for canonical transport.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelCostRecord {
    pub input: f64,
    pub output: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_write: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_audio: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_audio: Option<f64>,
}

/// Owned model-limit metadata for canonical transport.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLimitRecord {
    pub context: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<u64>,
    pub output: u64,
}

/// Owned model catalog entry for canonical transport.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelInfoRecord {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasoning_efforts: Vec<String>,
    pub tool_call: bool,
    pub attachment: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structured_output: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub knowledge: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_weights: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_modalities: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_modalities: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<ModelCostRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<ModelLimitRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<ProviderCapabilities>,
}

/// Flattened provider model metadata for canonical transport.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderModelRecord {
    pub provider_name: String,
    pub model: ModelInfoRecord,
}

/// Credential record with an explicit key shape suitable for JSON transport.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CredentialRecord {
    pub provider_name: String,
    pub credential_id: String,
    pub credential: CredentialEntry,
}

/// Credential health record without credential material.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CredentialHealthRecord {
    pub provider_name: String,
    pub credential_id: String,
    pub health: CredentialHealth,
}

/// Trajectory record with its associated session identifier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrajectoryRecord {
    pub session_id: SessionId,
    pub trajectory: atif::Trajectory,
}

/// Structured server error payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorBody,
}

/// Structured server error body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stub: Option<bool>,
}

impl ErrorResponse {
    /// Creates a non-stub structured error response.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: ErrorBody {
                code: code.into(),
                message: message.into(),
                details: None,
                stub: None,
            },
        }
    }

    /// Creates a non-stub structured error response with details.
    pub fn with_details(
        code: impl Into<String>,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            error: ErrorBody {
                code: code.into(),
                message: message.into(),
                details: Some(details),
                stub: None,
            },
        }
    }

    /// Creates a stubbed structured error response.
    pub fn stub(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: ErrorBody {
                code: code.into(),
                message: message.into(),
                details: None,
                stub: Some(true),
            },
        }
    }
}

impl From<ProviderModelInfo> for ProviderModelRecord {
    fn from(value: ProviderModelInfo) -> Self {
        Self {
            provider_name: value.provider_name,
            model: ModelInfoRecord {
                id: value.model.id.into_owned(),
                name: value.model.name.into_owned(),
                family: value.model.family.map(|value| value.into_owned()),
                reasoning_efforts: value
                    .model
                    .reasoning_efforts
                    .iter()
                    .map(|value| (*value).to_owned())
                    .collect(),
                tool_call: value.model.tool_call,
                attachment: value.model.attachment,
                structured_output: value.model.structured_output,
                temperature: value.model.temperature,
                knowledge: value.model.knowledge.map(|value| value.into_owned()),
                release_date: value.model.release_date.map(|value| value.into_owned()),
                last_updated: value.model.last_updated.map(|value| value.into_owned()),
                open_weights: value.model.open_weights,
                input_modalities: value
                    .model
                    .input_modalities
                    .iter()
                    .map(|value| (*value).to_owned())
                    .collect(),
                output_modalities: value
                    .model
                    .output_modalities
                    .iter()
                    .map(|value| (*value).to_owned())
                    .collect(),
                cost: value.model.cost.map(|cost| ModelCostRecord {
                    input: cost.input,
                    output: cost.output,
                    reasoning: cost.reasoning,
                    cache_read: cost.cache_read,
                    cache_write: cost.cache_write,
                    input_audio: cost.input_audio,
                    output_audio: cost.output_audio,
                }),
                limit: value.model.limit.map(|limit| ModelLimitRecord {
                    context: limit.context,
                    input: limit.input,
                    output: limit.output,
                }),
                status: value.model.status.map(|value| value.into_owned()),
                capabilities: value.model.capabilities,
            },
        }
    }
}
