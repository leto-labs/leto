use serde::{Deserialize, Serialize};

use crate::{JsonObject, SchemaVersion, ValidationError, require_supported};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metrics {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cached_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_token_ids: Option<Vec<i64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion_token_ids: Option<Vec<i64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<Vec<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra: Option<JsonObject>,
}

impl Metrics {
    pub(crate) fn validate(&self, schema_version: SchemaVersion) -> Result<(), ValidationError> {
        if self.completion_token_ids.is_some() {
            require_supported(
                schema_version.supports_completion_token_ids(),
                "steps[].metrics.completion_token_ids",
                SchemaVersion::V1_3,
                schema_version,
            )?;
        }
        if self.prompt_token_ids.is_some() {
            require_supported(
                schema_version.supports_prompt_token_ids(),
                "steps[].metrics.prompt_token_ids",
                SchemaVersion::V1_4,
                schema_version,
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{Metrics, SchemaVersion, ValidationError};

    #[test]
    fn prompt_token_ids_require_v1_4() {
        let metrics = Metrics {
            prompt_tokens: None,
            completion_tokens: None,
            cached_tokens: None,
            cost_usd: None,
            prompt_token_ids: Some(vec![1, 2]),
            completion_token_ids: None,
            logprobs: None,
            extra: None,
        };

        assert!(matches!(
            metrics.validate(SchemaVersion::V1_3),
            Err(ValidationError::UnsupportedFieldForSchemaVersion { .. })
        ));
    }
}
