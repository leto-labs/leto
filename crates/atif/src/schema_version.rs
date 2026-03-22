use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum SchemaVersion {
    #[serde(rename = "ATIF-v1.0")]
    V1_0,
    #[serde(rename = "ATIF-v1.1")]
    V1_1,
    #[serde(rename = "ATIF-v1.2")]
    V1_2,
    #[serde(rename = "ATIF-v1.3")]
    V1_3,
    #[serde(rename = "ATIF-v1.4")]
    V1_4,
    #[serde(rename = "ATIF-v1.5")]
    V1_5,
    #[default]
    #[serde(rename = "ATIF-v1.6")]
    V1_6,
}

impl SchemaVersion {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V1_0 => "ATIF-v1.0",
            Self::V1_1 => "ATIF-v1.1",
            Self::V1_2 => "ATIF-v1.2",
            Self::V1_3 => "ATIF-v1.3",
            Self::V1_4 => "ATIF-v1.4",
            Self::V1_5 => "ATIF-v1.5",
            Self::V1_6 => "ATIF-v1.6",
        }
    }

    pub fn supports_trajectory_extra(self) -> bool {
        self >= Self::V1_1
    }

    pub fn supports_system_step_observation(self) -> bool {
        self >= Self::V1_2
    }

    pub fn supports_completion_token_ids(self) -> bool {
        self >= Self::V1_3
    }

    pub fn supports_prompt_token_ids(self) -> bool {
        self >= Self::V1_4
    }

    pub fn supports_tool_definitions(self) -> bool {
        self >= Self::V1_5
    }

    pub fn supports_copied_context(self) -> bool {
        self >= Self::V1_5
    }

    pub fn supports_multimodal_content(self) -> bool {
        self >= Self::V1_6
    }
}

impl fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
