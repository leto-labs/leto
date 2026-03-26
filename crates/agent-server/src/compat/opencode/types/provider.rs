use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProviderIdPath {
    #[serde(rename = "providerID")]
    pub provider_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct ProviderOAuthAuthorizeRequest {
    #[schemars(required)]
    #[schemars(description = "Auth method index")]
    pub method: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Prompt inputs")]
    #[schemars(with = "Option<std::collections::BTreeMap<String, String>>")]
    pub inputs: Option<std::collections::BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct ProviderOAuthCallbackRequest {
    #[schemars(required)]
    #[schemars(description = "Auth method index")]
    pub method: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "OAuth authorization code")]
    pub code: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "ApiAuth")]
pub struct ApiAuthRequest {
    #[serde(rename = "type")]
    pub auth_type: ApiAuthTypeDoc,
    pub key: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "OAuth")]
pub struct OAuthAuthRequest {
    #[serde(rename = "type")]
    pub auth_type: OAuthAuthTypeDoc,
    pub refresh: String,
    pub access: String,
    pub expires: f64,
    #[serde(default, rename = "accountId", skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    #[serde(
        default,
        rename = "enterpriseUrl",
        skip_serializing_if = "Option::is_none"
    )]
    pub enterprise_url: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "WellKnownAuth")]
pub struct WellKnownAuthRequest {
    #[serde(rename = "type")]
    pub auth_type: WellKnownAuthTypeDoc,
    pub key: String,
    pub token: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub enum ApiAuthTypeDoc {
    #[default]
    #[serde(rename = "api")]
    Api,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub enum OAuthAuthTypeDoc {
    #[default]
    #[serde(rename = "oauth")]
    Oauth,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub enum WellKnownAuthTypeDoc {
    #[default]
    #[serde(rename = "wellknown")]
    Wellknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "Auth")]
#[serde(untagged)]
pub enum AuthSetRequest {
    Oauth(OAuthAuthRequest),
    Api(ApiAuthRequest),
    Wellknown(WellKnownAuthRequest),
}

impl Default for AuthSetRequest {
    fn default() -> Self {
        Self::Api(ApiAuthRequest::default())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "ProviderAuthMethod")]
pub struct ProviderAuthMethodDoc {
    #[serde(rename = "type")]
    pub method_type: ProviderAuthMethodTypeDoc,
    pub label: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prompts: Vec<ProviderAuthPromptDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ProviderAuthMethodTypeDoc {
    Oauth(ProviderAuthMethodOauthTypeDoc),
    Api(ProviderAuthMethodApiTypeDoc),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub enum ProviderAuthMethodOauthTypeDoc {
    #[default]
    #[serde(rename = "oauth")]
    Oauth,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub enum ProviderAuthMethodApiTypeDoc {
    #[default]
    #[serde(rename = "api")]
    Api,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ProviderAuthPromptDoc {
    Text(ProviderAuthTextPromptDoc),
    Select(ProviderAuthSelectPromptDoc),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProviderAuthTextPromptDoc {
    #[serde(rename = "type")]
    pub prompt_type: ProviderAuthTextPromptTypeDoc,
    pub key: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<ProviderAuthWhenDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProviderAuthSelectPromptDoc {
    #[serde(rename = "type")]
    pub prompt_type: ProviderAuthSelectPromptTypeDoc,
    pub key: String,
    pub message: String,
    pub options: Vec<ProviderAuthSelectOptionDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<ProviderAuthWhenDoc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub enum ProviderAuthTextPromptTypeDoc {
    #[default]
    #[serde(rename = "text")]
    Text,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub enum ProviderAuthSelectPromptTypeDoc {
    #[default]
    #[serde(rename = "select")]
    Select,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProviderAuthWhenDoc {
    pub key: String,
    pub op: ProviderAuthWhenOpDoc,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ProviderAuthWhenOpDoc {
    Eq(ProviderAuthWhenEqTypeDoc),
    Neq(ProviderAuthWhenNeqTypeDoc),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub enum ProviderAuthWhenEqTypeDoc {
    #[default]
    #[serde(rename = "eq")]
    Eq,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub enum ProviderAuthWhenNeqTypeDoc {
    #[default]
    #[serde(rename = "neq")]
    Neq,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProviderAuthSelectOptionDoc {
    pub label: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "ProviderAuthAuthorization")]
pub struct ProviderAuthAuthorizationDoc {
    pub url: String,
    pub method: ProviderAuthAuthorizationMethodDoc,
    pub instructions: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ProviderAuthAuthorizationMethodDoc {
    Auto(ProviderAuthAuthorizationAutoTypeDoc),
    Code(ProviderAuthAuthorizationCodeTypeDoc),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub enum ProviderAuthAuthorizationAutoTypeDoc {
    #[default]
    #[serde(rename = "auto")]
    Auto,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub enum ProviderAuthAuthorizationCodeTypeDoc {
    #[default]
    #[serde(rename = "code")]
    Code,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProviderListResponseDoc {
    pub all: Vec<ProviderListItemDoc>,
    #[schemars(with = "std::collections::BTreeMap<String, String>")]
    pub default: std::collections::BTreeMap<String, String>,
    pub connected: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "ProviderListItem", inline)]
pub struct ProviderListItemDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api: Option<String>,
    pub name: String,
    pub env: Vec<String>,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub npm: Option<String>,
    #[schemars(with = "std::collections::BTreeMap<String, ProviderListModelDoc>")]
    pub models: std::collections::BTreeMap<String, ProviderListModelDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "ProviderListModel", inline)]
pub struct ProviderListModelDoc {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    pub release_date: String,
    pub attachment: bool,
    pub reasoning: bool,
    pub temperature: bool,
    #[serde(rename = "tool_call")]
    pub tool_call: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interleaved: Option<ProviderInterleavedConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<ProviderCostConfigDoc>,
    pub limit: ProviderLimitConfigDoc,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modalities: Option<ProviderModalitiesConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experimental: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<ProviderStatusConfigDoc>,
    #[schemars(with = "std::collections::BTreeMap<String, Value>")]
    pub options: std::collections::BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<std::collections::BTreeMap<String, String>>")]
    pub headers: Option<std::collections::BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderSourceConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        with = "Option<std::collections::BTreeMap<String, std::collections::BTreeMap<String, Value>>>"
    )]
    pub variants:
        Option<std::collections::BTreeMap<String, std::collections::BTreeMap<String, Value>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConfigProvidersResponseDoc {
    pub providers: Vec<ProviderDoc>,
    #[schemars(with = "std::collections::BTreeMap<String, String>")]
    pub default: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ProviderSourceDoc {
    Env,
    Config,
    Custom,
    Api,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "Provider")]
pub struct ProviderDoc {
    pub id: String,
    pub name: String,
    pub source: ProviderSourceDoc,
    pub env: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[schemars(with = "std::collections::BTreeMap<String, Value>")]
    pub options: std::collections::BTreeMap<String, Value>,
    #[schemars(with = "std::collections::BTreeMap<String, ModelDoc>")]
    pub models: std::collections::BTreeMap<String, ModelDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModelApiDoc {
    pub id: String,
    pub url: String,
    pub npm: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModelIoDoc {
    pub text: bool,
    pub audio: bool,
    pub image: bool,
    pub video: bool,
    pub pdf: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModelInterleavedFieldDoc {
    pub field: ModelInterleavedFieldValueDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ModelInterleavedFieldValueDoc {
    #[serde(rename = "reasoning_content")]
    ReasoningContent,
    #[serde(rename = "reasoning_details")]
    ReasoningDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ModelInterleavedDoc {
    Bool(bool),
    Field(ModelInterleavedFieldDoc),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModelCapabilitiesDoc {
    pub temperature: bool,
    pub reasoning: bool,
    pub attachment: bool,
    pub toolcall: bool,
    pub input: ModelIoDoc,
    pub output: ModelIoDoc,
    pub interleaved: ModelInterleavedDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MessageTokensCacheDoc {
    pub read: f64,
    pub write: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModelCostTierDoc {
    pub input: f64,
    pub output: f64,
    pub cache: MessageTokensCacheDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModelCostDoc {
    pub input: f64,
    pub output: f64,
    pub cache: MessageTokensCacheDoc,
    #[serde(
        default,
        rename = "experimentalOver200K",
        skip_serializing_if = "Option::is_none"
    )]
    pub experimental_over_200k: Option<ModelCostTierDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModelLimitDoc {
    pub context: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<f64>,
    pub output: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ModelStatusDoc {
    #[serde(rename = "alpha")]
    Alpha,
    #[serde(rename = "beta")]
    Beta,
    #[serde(rename = "deprecated")]
    Deprecated,
    #[serde(rename = "active")]
    Active,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "Model")]
pub struct ModelDoc {
    pub id: String,
    #[serde(rename = "providerID")]
    pub provider_id: String,
    pub api: ModelApiDoc,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    pub capabilities: ModelCapabilitiesDoc,
    pub cost: ModelCostDoc,
    pub limit: ModelLimitDoc,
    pub status: ModelStatusDoc,
    #[schemars(with = "std::collections::BTreeMap<String, Value>")]
    pub options: std::collections::BTreeMap<String, Value>,
    #[schemars(with = "std::collections::BTreeMap<String, String>")]
    pub headers: std::collections::BTreeMap<String, String>,
    pub release_date: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        with = "Option<std::collections::BTreeMap<String, std::collections::BTreeMap<String, Value>>>"
    )]
    pub variants:
        Option<std::collections::BTreeMap<String, std::collections::BTreeMap<String, Value>>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "ProviderConfig")]
#[serde(deny_unknown_fields)]
pub struct ProviderConfigDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub npm: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub models: Option<std::collections::BTreeMap<String, ProviderModelConfigDoc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub whitelist: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blacklist: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<ProviderOptionsDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProviderModelConfigDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(
        default,
        rename = "release_date",
        skip_serializing_if = "Option::is_none"
    )]
    pub release_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachment: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<bool>,
    #[serde(default, rename = "tool_call", skip_serializing_if = "Option::is_none")]
    pub tool_call: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interleaved: Option<ProviderInterleavedConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<ProviderCostConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<ProviderLimitConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modalities: Option<ProviderModalitiesConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experimental: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<ProviderStatusConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<std::collections::BTreeMap<String, Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<std::collections::BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderSourceConfigDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Variant-specific configuration")]
    pub variants: Option<std::collections::BTreeMap<String, ProviderVariantConfigDoc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ProviderInterleavedConfigDoc {
    Enabled(TrueConstDoc),
    Field(ProviderInterleavedFieldDoc),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderInterleavedFieldDoc {
    pub field: ProviderInterleavedFieldValueDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ProviderInterleavedFieldValueDoc {
    #[serde(rename = "reasoning_content")]
    ReasoningContent,
    #[serde(rename = "reasoning_details")]
    ReasoningDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProviderCostConfigDoc {
    pub input: f64,
    pub output: f64,
    #[serde(
        default,
        rename = "cache_read",
        skip_serializing_if = "Option::is_none"
    )]
    pub cache_read: Option<f64>,
    #[serde(
        default,
        rename = "cache_write",
        skip_serializing_if = "Option::is_none"
    )]
    pub cache_write: Option<f64>,
    #[serde(
        default,
        rename = "context_over_200k",
        skip_serializing_if = "Option::is_none"
    )]
    pub context_over_200k: Option<ProviderCostTierConfigDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProviderCostTierConfigDoc {
    pub input: f64,
    pub output: f64,
    #[serde(
        default,
        rename = "cache_read",
        skip_serializing_if = "Option::is_none"
    )]
    pub cache_read: Option<f64>,
    #[serde(
        default,
        rename = "cache_write",
        skip_serializing_if = "Option::is_none"
    )]
    pub cache_write: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProviderLimitConfigDoc {
    pub context: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<f64>,
    pub output: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProviderModalitiesConfigDoc {
    pub input: Vec<ProviderModalityDoc>,
    pub output: Vec<ProviderModalityDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ProviderModalityDoc {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "audio")]
    Audio,
    #[serde(rename = "image")]
    Image,
    #[serde(rename = "video")]
    Video,
    #[serde(rename = "pdf")]
    Pdf,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ProviderStatusConfigDoc {
    #[serde(rename = "alpha")]
    Alpha,
    #[serde(rename = "beta")]
    Beta,
    #[serde(rename = "deprecated")]
    Deprecated,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProviderSourceConfigDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub npm: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct ProviderVariantConfigDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Disable this variant for the model")]
    pub disabled: Option<bool>,
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct ProviderOptionsDoc {
    #[serde(default, rename = "apiKey", skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default, rename = "baseURL", skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(
        default,
        rename = "enterpriseUrl",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(description = "GitHub Enterprise URL for copilot authentication")]
    pub enterprise_url: Option<String>,
    #[serde(
        default,
        rename = "setCacheKey",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(description = "Enable promptCacheKey for this provider (default false)")]
    pub set_cache_key: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Timeout in milliseconds for requests to this provider. Default is 300000 (5 minutes). Set to false to disable timeout."
    )]
    pub timeout: Option<ProviderTimeoutConfigDoc>,
    #[serde(
        default,
        rename = "chunkTimeout",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(
        description = "Timeout in milliseconds between streamed SSE chunks for this provider. If no chunk arrives within this window, the request is aborted."
    )]
    #[schemars(range(min = 1, max = 9007199254740991i64))]
    pub chunk_timeout: Option<u64>,
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ProviderTimeoutConfigDoc {
    Duration(
        #[schemars(
            description = "Timeout in milliseconds for requests to this provider. Default is 300000 (5 minutes). Set to false to disable timeout."
        )]
        #[schemars(range(min = 1, max = 9007199254740991i64))]
        u64,
    ),
    Disabled(FalseConstDoc),
}

fn false_const_schema(_generator: &mut SchemaGenerator) -> Schema {
    serde_json::from_value(json!({
        "type": "boolean",
        "const": false
    }))
    .expect("valid false const schema")
}

fn true_const_schema(_generator: &mut SchemaGenerator) -> Schema {
    serde_json::from_value(json!({
        "type": "boolean",
        "const": true
    }))
    .expect("valid true const schema")
}

#[derive(Debug, Clone, JsonSchema)]
#[schemars(schema_with = "false_const_schema")]
pub struct FalseConstDoc;

#[derive(Debug, Clone, JsonSchema)]
#[schemars(schema_with = "true_const_schema")]
pub struct TrueConstDoc;

impl Serialize for FalseConstDoc {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bool(false)
    }
}

impl<'de> Deserialize<'de> for FalseConstDoc {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match bool::deserialize(deserializer)? {
            false => Ok(Self),
            true => Err(D::Error::custom("expected false")),
        }
    }
}

impl Serialize for TrueConstDoc {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bool(true)
    }
}

impl<'de> Deserialize<'de> for TrueConstDoc {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match bool::deserialize(deserializer)? {
            true => Ok(Self),
            false => Err(D::Error::custom("expected true")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FalseConstDoc, TrueConstDoc};

    #[test]
    fn false_const_doc_serializes_and_deserializes_as_false() {
        assert_eq!(
            serde_json::to_value(FalseConstDoc).expect("false const should serialize"),
            serde_json::json!(false)
        );
        serde_json::from_value::<FalseConstDoc>(serde_json::json!(false))
            .expect("false const should deserialize from false");
        assert!(serde_json::from_value::<FalseConstDoc>(serde_json::json!(true)).is_err());
    }

    #[test]
    fn true_const_doc_serializes_and_deserializes_as_true() {
        assert_eq!(
            serde_json::to_value(TrueConstDoc).expect("true const should serialize"),
            serde_json::json!(true)
        );
        serde_json::from_value::<TrueConstDoc>(serde_json::json!(true))
            .expect("true const should deserialize from true");
        assert!(serde_json::from_value::<TrueConstDoc>(serde_json::json!(false)).is_err());
    }
}
