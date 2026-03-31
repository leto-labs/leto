#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SessionIdPath {
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SessionMessagePath {
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    #[serde(rename = "messageID")]
    #[schemars(regex(pattern = "^msg.*"))]
    pub message_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SessionMessagePartPath {
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    #[serde(rename = "messageID")]
    #[schemars(regex(pattern = "^msg.*"))]
    pub message_id: String,
    #[serde(rename = "partID")]
    #[schemars(regex(pattern = "^prt.*"))]
    pub part_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SessionPermissionPath {
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    #[serde(rename = "permissionID")]
    #[schemars(regex(pattern = "^per.*"))]
    pub permission_id: String,
}
