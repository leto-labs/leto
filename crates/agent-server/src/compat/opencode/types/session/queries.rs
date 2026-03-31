#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct MessageListQuery {
    #[serde(default)]
    #[schemars(with = "String")]
    pub directory: Option<String>,
    #[serde(default)]
    #[schemars(with = "String")]
    pub workspace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 0, max = 9007199254740991i64))]
    pub limit: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct SessionListQuery {
    #[serde(default)]
    #[schemars(with = "String")]
    pub directory: Option<String>,
    #[serde(default)]
    #[schemars(with = "String")]
    pub workspace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roots: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct SessionDiffQuery {
    #[serde(default)]
    #[schemars(with = "String")]
    pub directory: Option<String>,
    #[serde(default)]
    #[schemars(with = "String")]
    pub workspace: Option<String>,
    #[serde(default, rename = "messageID", skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = "^msg.*"))]
    pub message_id: Option<String>,
}
