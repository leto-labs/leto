use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::mcp::AgentModeDoc;
use super::provider::{FalseConstDoc, TrueConstDoc};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HealthDoc {
    pub healthy: TrueConstDoc,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AppLogRequestDoc {
    #[schemars(description = "Service name for the log entry")]
    pub service: String,
    #[schemars(description = "Log level")]
    pub level: AppLogLevelDoc,
    #[schemars(description = "Log message")]
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Additional metadata for the log entry")]
    #[schemars(with = "std::collections::BTreeMap<String, Value>")]
    pub extra: Option<std::collections::BTreeMap<String, Value>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct UpgradeRequestDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub target: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum UpgradeResultDoc {
    Success(UpgradeSuccessDoc),
    Failure(UpgradeFailureDoc),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UpgradeSuccessDoc {
    pub success: TrueConstDoc,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UpgradeFailureDoc {
    pub success: FalseConstDoc,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "LogLevel", description = "Log level")]
pub enum LogLevelDoc {
    #[serde(rename = "DEBUG")]
    Debug,
    #[serde(rename = "INFO")]
    Info,
    #[serde(rename = "WARN")]
    Warn,
    #[serde(rename = "ERROR")]
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum AppLogLevelDoc {
    #[serde(rename = "debug")]
    Debug,
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "warn")]
    Warn,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "PermissionActionConfig")]
pub enum PermissionActionConfigDoc {
    #[serde(rename = "ask")]
    Ask,
    #[serde(rename = "allow")]
    Allow,
    #[serde(rename = "deny")]
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "PermissionObjectConfig")]
pub struct PermissionObjectConfigDoc {
    #[serde(flatten)]
    pub permissions: std::collections::BTreeMap<String, PermissionActionConfigDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "PermissionRuleConfig")]
#[serde(untagged)]
pub enum PermissionRuleConfigDoc {
    Action(PermissionActionConfigDoc),
    Object(PermissionObjectConfigDoc),
}

// Keep this untagged shape stable because the compat OpenAPI output is pinned.
#[expect(
    clippy::large_enum_variant,
    reason = "compat untagged schema shape must stay inline for pinned OpenAPI parity"
)]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "PermissionConfig")]
#[serde(untagged)]
pub enum PermissionConfigDoc {
    Object(PermissionConfigObjectDoc),
    Action(PermissionActionConfigDoc),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct PermissionConfigObjectDoc {
    #[serde(
        default,
        rename = "__originalKeys",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub original_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read: Option<PermissionRuleConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edit: Option<PermissionRuleConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glob: Option<PermissionRuleConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grep: Option<PermissionRuleConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list: Option<PermissionRuleConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bash: Option<PermissionRuleConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<PermissionRuleConfigDoc>,
    #[serde(
        default,
        rename = "external_directory",
        skip_serializing_if = "Option::is_none"
    )]
    pub external_directory: Option<PermissionRuleConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub todowrite: Option<PermissionActionConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub todoread: Option<PermissionActionConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question: Option<PermissionActionConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub webfetch: Option<PermissionActionConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub websearch: Option<PermissionActionConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codesearch: Option<PermissionActionConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lsp: Option<PermissionRuleConfigDoc>,
    #[serde(default, rename = "doom_loop", skip_serializing_if = "Option::is_none")]
    pub doom_loop: Option<PermissionActionConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill: Option<PermissionRuleConfigDoc>,
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, PermissionRuleConfigDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum AgentColorDoc {
    Hex(HexColorDoc),
    Theme(ThemeColorDoc),
}

fn agent_color_schema(generator: &mut SchemaGenerator) -> Schema {
    let mut schema = generator.subschema_for::<AgentColorDoc>();
    if let Some(object) = schema.as_object_mut() {
        object.insert(
            "description".to_owned(),
            Value::String(
                "Hex color code (e.g., #FF5733) or theme color (e.g., primary)".to_owned(),
            ),
        );
    }
    schema
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HexColorDoc(#[schemars(regex(pattern = "^#[0-9a-fA-F]{6}$"))] pub String);

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ThemeColorDoc {
    #[serde(rename = "primary")]
    Primary,
    #[serde(rename = "secondary")]
    Secondary,
    #[serde(rename = "accent")]
    Accent,
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "info")]
    Info,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "AgentConfig")]
pub struct AgentConfigDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Default model variant for this agent (applies only when using the agent's configured model)."
    )]
    pub variant: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(default, rename = "top_p", skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "@deprecated Use 'permission' field instead")]
    pub tools: Option<std::collections::BTreeMap<String, bool>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disable: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Description of when to use the agent")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<AgentModeDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Hide this subagent from the @ autocomplete menu (default: false, only applies to mode: subagent)"
    )]
    pub hidden: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<std::collections::BTreeMap<String, Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(schema_with = "agent_color_schema")]
    pub color: Option<AgentColorDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1, max = 9007199254740991i64))]
    #[schemars(
        description = "Maximum number of agentic iterations before forcing text-only response"
    )]
    pub steps: Option<u64>,
    #[serde(default, rename = "maxSteps", skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1, max = 9007199254740991i64))]
    #[schemars(description = "@deprecated Use 'steps' field instead.")]
    pub max_steps: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission: Option<PermissionConfigDoc>,
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "ServerConfig")]
#[schemars(description = "Server configuration for opencode serve and web commands")]
#[serde(deny_unknown_fields)]
pub struct ServerConfigDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1, max = 9007199254740991i64))]
    #[schemars(description = "Port to listen on")]
    pub port: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Hostname to listen on")]
    pub hostname: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Enable mDNS service discovery")]
    pub mdns: Option<bool>,
    #[serde(
        default,
        rename = "mdnsDomain",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(description = "Custom domain name for mDNS service (default: opencode.local)")]
    pub mdns_domain: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Additional domains to allow for CORS")]
    pub cors: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "LayoutConfig")]
#[schemars(description = "@deprecated Always uses stretch layout.")]
pub enum LayoutConfigDoc {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "stretch")]
    Stretch,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "McpOAuthConfig")]
#[serde(deny_unknown_fields)]
pub struct McpOauthConfigDoc {
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "McpLocalConfig")]
#[serde(deny_unknown_fields)]
pub struct McpLocalConfigDoc {
    #[serde(rename = "type")]
    #[schemars(description = "Type of MCP server connection")]
    pub config_type: McpLocalConfigTypeDoc,
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
pub enum McpLocalConfigTypeDoc {
    #[default]
    #[serde(rename = "local")]
    Local,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "McpRemoteConfig")]
#[serde(deny_unknown_fields)]
pub struct McpRemoteConfigDoc {
    #[serde(rename = "type")]
    #[schemars(description = "Type of MCP server connection")]
    pub config_type: McpRemoteConfigTypeDoc,
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
    pub oauth: Option<McpRemoteOauthDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1, max = 9007199254740991i64))]
    #[schemars(
        description = "Timeout in ms for MCP server requests. Defaults to 5000 (5 seconds) if not specified."
    )]
    pub timeout: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub enum McpRemoteConfigTypeDoc {
    #[default]
    #[serde(rename = "remote")]
    Remote,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum McpRemoteOauthDoc {
    Config(McpOauthConfigDoc),
    Disabled(FalseConstDoc),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CommandConfigDoc {
    pub template: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtask: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct SkillsConfigDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Additional paths to skill folders")]
    pub paths: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "URLs to fetch skills from (e.g., https://example.com/.well-known/skills/)"
    )]
    pub urls: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct WatcherConfigDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ShareConfigDoc {
    #[serde(rename = "manual")]
    Manual,
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "disabled")]
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum AutoUpdateConfigDoc {
    Enabled(bool),
    Notify(AutoUpdateNotifyDoc),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum AutoUpdateNotifyDoc {
    #[serde(rename = "notify")]
    Notify,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct ModeConfigDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<AgentConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<AgentConfigDoc>,
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, AgentConfigDoc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct AgentMapConfigDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<AgentConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<AgentConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub general: Option<AgentConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explore: Option<AgentConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<AgentConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<AgentConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compaction: Option<AgentConfigDoc>,
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, AgentConfigDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum McpConfigEntryDoc {
    Server(McpServerConfigDoc),
    Enabled(McpEnabledOnlyDoc),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum McpServerConfigDoc {
    Local(McpLocalConfigDoc),
    Remote(McpRemoteConfigDoc),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpEnabledOnlyDoc {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum FormatterConfigDoc {
    Disabled(FalseConstDoc),
    Rules(std::collections::BTreeMap<String, FormatterRuleDoc>),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FormatterRuleDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<std::collections::BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum LspConfigDoc {
    Disabled(FalseConstDoc),
    Rules(std::collections::BTreeMap<String, LspRuleDoc>),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum LspRuleDoc {
    Disabled(LspDisabledRuleDoc),
    Enabled(LspEnabledRuleDoc),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LspDisabledRuleDoc {
    pub disabled: TrueConstDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LspEnabledRuleDoc {
    pub command: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<std::collections::BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initialization: Option<std::collections::BTreeMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EnterpriseConfigDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Enterprise URL")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CompactionConfigDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Enable automatic compaction when context is full (default: true)")]
    pub auto: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Enable pruning of old tool outputs (default: true)")]
    pub prune: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Token buffer for compaction. Leaves enough window to avoid overflow during compaction."
    )]
    #[schemars(range(min = 0, max = 9007199254740991i64))]
    pub reserved: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExperimentalConfigDoc {
    #[serde(
        default,
        rename = "disable_paste_summary",
        skip_serializing_if = "Option::is_none"
    )]
    pub disable_paste_summary: Option<bool>,
    #[serde(
        default,
        rename = "batch_tool",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(description = "Enable the batch tool")]
    pub batch_tool: Option<bool>,
    #[serde(
        default,
        rename = "openTelemetry",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(
        description = "Enable OpenTelemetry spans for AI SDK calls (using the 'experimental_telemetry' flag)"
    )]
    pub open_telemetry: Option<bool>,
    #[serde(
        default,
        rename = "primary_tools",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(description = "Tools that should only be available to primary agents.")]
    pub primary_tools: Option<Vec<String>>,
    #[serde(
        default,
        rename = "continue_loop_on_deny",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(description = "Continue the agent loop when a tool call is denied")]
    pub continue_loop_on_deny: Option<bool>,
    #[serde(
        default,
        rename = "mcp_timeout",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(description = "Timeout in milliseconds for model context protocol (MCP) requests")]
    #[schemars(range(min = 1, max = 9007199254740991i64))]
    pub mcp_timeout: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "Config")]
#[serde(deny_unknown_fields)]
pub struct CompatConfigDoc {
    #[serde(default, rename = "$schema", skip_serializing_if = "Option::is_none")]
    #[schemars(description = "JSON schema reference for configuration validation")]
    pub schema_url: Option<String>,
    #[serde(default, rename = "logLevel", skip_serializing_if = "Option::is_none")]
    pub log_level: Option<LogLevelDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server: Option<ServerConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Command configuration, see https://opencode.ai/docs/commands")]
    pub command: Option<std::collections::BTreeMap<String, CommandConfigDoc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Additional skill folder paths")]
    pub skills: Option<SkillsConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watcher: Option<WatcherConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Enable or disable snapshot tracking. When false, filesystem snapshots are not recorded and undoing or reverting will not undo/redo file changes. Defaults to true."
    )]
    pub snapshot: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Control sharing behavior:'manual' allows manual sharing via commands, 'auto' enables automatic sharing, 'disabled' disables all sharing"
    )]
    pub share: Option<ShareConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "@deprecated Use 'share' field instead. Share newly created sessions automatically"
    )]
    pub autoshare: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Automatically update to the latest version. Set to true to auto-update, false to disable, or 'notify' to show update notifications"
    )]
    pub autoupdate: Option<AutoUpdateConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Disable providers that are loaded automatically")]
    pub disabled_providers: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "When set, ONLY these providers will be enabled. All other providers will be ignored"
    )]
    pub enabled_providers: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Model to use in the format of provider/model, eg anthropic/claude-2"
    )]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Small model to use for tasks like title generation in the format of provider/model"
    )]
    pub small_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Default agent to use when none is specified. Must be a primary agent. Falls back to 'build' if not set or if the specified agent is invalid."
    )]
    pub default_agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Custom username to display in conversations instead of system username"
    )]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "@deprecated Use `agent` field instead.")]
    pub mode: Option<ModeConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Agent configuration, see https://opencode.ai/docs/agents")]
    pub agent: Option<AgentMapConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Custom provider configurations and model overrides")]
    pub provider: Option<std::collections::BTreeMap<String, super::provider::ProviderConfigDoc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "MCP (Model Context Protocol) server configurations")]
    pub mcp: Option<std::collections::BTreeMap<String, McpConfigEntryDoc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formatter: Option<FormatterConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lsp: Option<LspConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Additional instruction files or patterns to include")]
    pub instructions: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission: Option<PermissionConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<LayoutConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<std::collections::BTreeMap<String, bool>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enterprise: Option<EnterpriseConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compaction: Option<CompactionConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experimental: Option<ExperimentalConfigDoc>,
}

fn empty_object_schema(_: &mut SchemaGenerator) -> Schema {
    serde_json::from_value(json!({
        "type": "object",
        "properties": {}
    }))
    .expect("empty object schema is valid")
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[schemars(schema_with = "empty_object_schema")]
pub struct EmptyPropertiesDoc {}
