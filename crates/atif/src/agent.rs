use serde::{Deserialize, Serialize};

use crate::{JsonObject, SchemaVersion, ValidationError, require_supported};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Agent {
    pub name: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_definitions: Option<Vec<JsonObject>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra: Option<JsonObject>,
}

impl Agent {
    pub(crate) fn validate(&self, schema_version: SchemaVersion) -> Result<(), ValidationError> {
        if self.tool_definitions.is_some() {
            require_supported(
                schema_version.supports_tool_definitions(),
                "agent.tool_definitions",
                SchemaVersion::V1_5,
                schema_version,
            )?;
        }

        Ok(())
    }
}
