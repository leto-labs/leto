use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::common::CompatQuery;
use super::permission::PermissionRuleset;
use super::provider::{FalseConstDoc, TrueConstDoc};

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct ToolListQueryDoc {
    #[serde(flatten)]
    pub compat: CompatQuery,
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpAddRequest {
    pub name: String,
    pub config: McpAddConfigDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum McpAddConfigDoc {
    Local(McpAddLocalConfigDoc),
    Remote(McpAddRemoteConfigDoc),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpAddLocalConfigDoc {
    #[serde(rename = "type")]
    #[schemars(description = "Type of MCP server connection")]
    pub config_type: McpAddLocalConfigTypeDoc,
    #[schemars(description = "Command and arguments to run the MCP server")]
    pub command: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Environment variables to set when running the MCP server")]
    pub environment: Option<std::collections::BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Enable or disable the MCP server on startup")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1, max = 9007199254740991i64))]
    #[schemars(
        description = "Timeout in ms for MCP server requests. Defaults to 5000 (5 seconds) if not specified."
    )]
    pub timeout: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub enum McpAddLocalConfigTypeDoc {
    #[default]
    #[serde(rename = "local")]
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpAddRemoteConfigDoc {
    #[serde(rename = "type")]
    #[schemars(description = "Type of MCP server connection")]
    pub config_type: McpAddRemoteConfigTypeDoc,
    #[schemars(description = "URL of the remote MCP server")]
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Enable or disable the MCP server on startup")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Headers to send with the request")]
    pub headers: Option<std::collections::BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "OAuth authentication configuration for the MCP server. Set to false to disable OAuth auto-detection."
    )]
    pub oauth: Option<McpAddRemoteOauthDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1, max = 9007199254740991i64))]
    #[schemars(
        description = "Timeout in ms for MCP server requests. Defaults to 5000 (5 seconds) if not specified."
    )]
    pub timeout: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub enum McpAddRemoteConfigTypeDoc {
    #[default]
    #[serde(rename = "remote")]
    Remote,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum McpAddRemoteOauthDoc {
    Config(McpAddOauthConfigDoc),
    Disabled(FalseConstDoc),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpAddOauthConfigDoc {
    #[serde(default, rename = "clientId", skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "OAuth client ID. If not provided, dynamic client registration (RFC 7591) will be attempted."
    )]
    pub client_id: Option<String>,
    #[serde(
        default,
        rename = "clientSecret",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(description = "OAuth client secret (if required by the authorization server)")]
    pub client_secret: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "OAuth scopes to request during authorization")]
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpAuthCallbackRequest {
    #[schemars(description = "Authorization code from OAuth callback")]
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpAuthStartResponseDoc {
    #[serde(rename = "authorizationUrl")]
    #[schemars(description = "URL to open in browser for authorization")]
    pub authorization_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SuccessResponseDoc {
    pub success: TrueConstDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "Agent")]
pub struct AgentDoc {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub mode: AgentModeDoc,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    #[serde(default, rename = "topP", skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    pub permission: PermissionRuleset,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<ModelRefDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[schemars(with = "std::collections::BTreeMap<String, Value>")]
    pub options: std::collections::BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1, max = 9007199254740991i64))]
    pub steps: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum AgentModeDoc {
    #[serde(rename = "subagent")]
    Subagent,
    #[serde(rename = "primary")]
    Primary,
    #[serde(rename = "all")]
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModelRefDoc {
    #[serde(rename = "modelID")]
    pub model_id: String,
    #[serde(rename = "providerID")]
    pub provider_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "Command")]
pub struct CommandDoc {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<CommandSourceDoc>,
    pub template: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtask: Option<bool>,
    pub hints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum CommandSourceDoc {
    #[serde(rename = "command")]
    Command,
    #[serde(rename = "mcp")]
    Mcp,
    #[serde(rename = "skill")]
    Skill,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SkillDoc {
    pub name: String,
    pub description: String,
    pub location: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "FormatterStatus")]
pub struct FormatterStatusDoc {
    pub name: String,
    pub extensions: Vec<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "LSPStatus")]
pub struct LspStatusDoc {
    pub id: String,
    pub name: String,
    pub root: String,
    pub status: LspStatusStateDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum LspStatusStateDoc {
    Connected(LspConnectedStateDoc),
    Error(LspErrorStateDoc),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum LspConnectedStateDoc {
    #[serde(rename = "connected")]
    Connected,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum LspErrorStateDoc {
    #[serde(rename = "error")]
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "ToolIDs")]
pub struct ToolIdsDoc(pub Vec<String>);

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "ToolList")]
pub struct ToolListDoc(pub Vec<ToolListItemDoc>);

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "ToolListItem")]
pub struct ToolListItemDoc {
    pub id: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "McpResource")]
pub struct McpResourceDoc {
    pub name: String,
    pub uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    pub client: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "MCPStatusConnected")]
pub struct McpStatusConnectedDoc {
    pub status: McpStatusConnectedKindDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum McpStatusConnectedKindDoc {
    #[serde(rename = "connected")]
    Connected,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "MCPStatusDisabled")]
pub struct McpStatusDisabledDoc {
    pub status: McpStatusDisabledKindDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum McpStatusDisabledKindDoc {
    #[serde(rename = "disabled")]
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "MCPStatusFailed")]
pub struct McpStatusFailedDoc {
    pub status: McpStatusFailedKindDoc,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum McpStatusFailedKindDoc {
    #[serde(rename = "failed")]
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "MCPStatusNeedsAuth")]
pub struct McpStatusNeedsAuthDoc {
    pub status: McpStatusNeedsAuthKindDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum McpStatusNeedsAuthKindDoc {
    #[serde(rename = "needs_auth")]
    NeedsAuth,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "MCPStatusNeedsClientRegistration")]
pub struct McpStatusNeedsClientRegistrationDoc {
    pub status: McpStatusNeedsClientRegistrationKindDoc,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum McpStatusNeedsClientRegistrationKindDoc {
    #[serde(rename = "needs_client_registration")]
    NeedsClientRegistration,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "MCPStatus")]
#[serde(untagged)]
pub enum McpStatusDoc {
    Connected(McpStatusConnectedDoc),
    Disabled(McpStatusDisabledDoc),
    Failed(McpStatusFailedDoc),
    NeedsAuth(McpStatusNeedsAuthDoc),
    NeedsClientRegistration(McpStatusNeedsClientRegistrationDoc),
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        McpAddConfigDoc, McpAddLocalConfigDoc, McpAddOauthConfigDoc, McpAddRemoteConfigDoc,
        McpAddRemoteOauthDoc,
    };

    #[test]
    fn mcp_add_local_config_deserializes() {
        let config: McpAddConfigDoc = serde_json::from_value(json!({
            "type": "local",
            "command": ["uvx", "example-server"],
            "environment": { "DEBUG": "1" },
            "enabled": true,
            "timeout": 5000
        }))
        .expect("local MCP add config should deserialize");

        assert!(matches!(
            config,
            McpAddConfigDoc::Local(McpAddLocalConfigDoc { .. })
        ));
    }

    #[test]
    fn mcp_add_remote_config_deserializes() {
        let config: McpAddConfigDoc = serde_json::from_value(json!({
            "type": "remote",
            "url": "https://example.invalid/mcp",
            "headers": { "Authorization": "Bearer token" },
            "enabled": true,
            "timeout": 5000
        }))
        .expect("remote MCP add config should deserialize");

        assert!(matches!(
            config,
            McpAddConfigDoc::Remote(McpAddRemoteConfigDoc { .. })
        ));
    }

    #[test]
    fn mcp_add_remote_oauth_false_deserializes() {
        let oauth: McpAddRemoteOauthDoc =
            serde_json::from_value(json!(false)).expect("oauth=false should deserialize");

        assert!(matches!(oauth, McpAddRemoteOauthDoc::Disabled(_)));
    }

    #[test]
    fn mcp_add_remote_oauth_config_deserializes() {
        let oauth: McpAddRemoteOauthDoc = serde_json::from_value(json!({
            "clientId": "client-id",
            "clientSecret": "client-secret",
            "scope": "openid profile"
        }))
        .expect("oauth object should deserialize");

        assert!(matches!(
            oauth,
            McpAddRemoteOauthDoc::Config(McpAddOauthConfigDoc { .. })
        ));
    }
}
