use schemars::{JsonSchema, json_schema};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::common::CompatQuery;
use super::global::McpServerConfigDoc;
use super::permission::PermissionRuleset;
use super::provider::TrueConstDoc;

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
    // schemars expands this untagged enum inline by default, but OpenCode's
    // published contract reuses the top-level MCP config components here.
    // Keep the contract shape at the DTO boundary rather than patching the
    // final document in doc.rs.
    #[schemars(schema_with = "mcp_add_config_schema")]
    pub config: McpServerConfigDoc,
}

fn mcp_add_config_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    json_schema!({
        "anyOf": [
            {
                "type": "object",
                "properties": {
                    "type": {
                        "type": "string",
                        "const": "local",
                        "description": "Type of MCP server connection"
                    },
                    "command": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Command and arguments to run the MCP server"
                    },
                    "environment": {
                        "type": "object",
                        "description": "Environment variables to set when running the MCP server"
                    },
                    "enabled": {
                        "type": "boolean",
                        "description": "Enable or disable the MCP server on startup"
                    },
                    "timeout": {
                        "type": "integer",
                        "maximum": 9007199254740991u64,
                        "exclusiveMinimum": 0,
                        "description": "Timeout in ms for MCP server requests. Defaults to 5000 (5 seconds) if not specified."
                    }
                },
                "required": ["type", "command"]
            },
            {
                "type": "object",
                "properties": {
                    "type": {
                        "type": "string",
                        "const": "remote",
                        "description": "Type of MCP server connection"
                    },
                    "url": {
                        "type": "string",
                        "description": "URL of the remote MCP server"
                    },
                    "enabled": {
                        "type": "boolean",
                        "description": "Enable or disable the MCP server on startup"
                    },
                    "headers": {
                        "type": "object",
                        "description": "Headers to send with the request"
                    },
                    "oauth": {
                        "description": "OAuth authentication configuration for the MCP server. Set to false to disable OAuth auto-detection.",
                        "anyOf": [
                            {
                                "type": "object",
                                "properties": {
                                    "clientId": {
                                        "type": "string",
                                        "description": "OAuth client ID. If not provided, dynamic client registration (RFC 7591) will be attempted."
                                    },
                                    "clientSecret": {
                                        "type": "string",
                                        "description": "OAuth client secret (if required by the authorization server)"
                                    },
                                    "scope": {
                                        "type": "string",
                                        "description": "OAuth scopes to request during authorization"
                                    }
                                }
                            },
                            {
                                "type": "boolean",
                                "const": false
                            }
                        ]
                    },
                    "timeout": {
                        "type": "integer",
                        "maximum": 9007199254740991u64,
                        "exclusiveMinimum": 0,
                        "description": "Timeout in ms for MCP server requests. Defaults to 5000 (5 seconds) if not specified."
                    }
                },
                "required": ["type", "url"]
            }
        ]
    })
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
