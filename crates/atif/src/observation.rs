use serde::{Deserialize, Serialize};

use crate::{ObservationResult, SchemaVersion, ValidationError};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub results: Vec<ObservationResult>,
}

impl Observation {
    pub(crate) fn validate(&self, schema_version: SchemaVersion) -> Result<(), ValidationError> {
        for result in &self.results {
            if let Some(content) = &result.content {
                content.validate(schema_version, "steps[].observation.results[].content")?;
            }
        }
        Ok(())
    }

    pub fn has_multimodal_content(&self) -> bool {
        self.results.iter().any(|result| {
            result
                .content
                .as_ref()
                .is_some_and(crate::MessageContent::has_multimodal_content)
        })
    }
}
