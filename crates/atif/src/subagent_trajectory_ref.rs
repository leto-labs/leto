use serde::{Deserialize, Serialize};

use crate::JsonObject;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubagentTrajectoryRef {
    pub session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trajectory_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra: Option<JsonObject>,
}
