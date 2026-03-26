use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "BadRequestError")]
pub struct BadRequestErrorDoc {
    pub data: Value,
    #[schemars(with = "Vec<std::collections::BTreeMap<String, Value>>")]
    pub errors: Vec<std::collections::BTreeMap<String, Value>>,
    #[schemars(schema_with = "false_const_schema")]
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "NotFoundError")]
pub struct NotFoundErrorDoc {
    pub name: NotFoundErrorNameDoc,
    pub data: NotFoundDataDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum NotFoundErrorNameDoc {
    #[serde(rename = "NotFoundError")]
    NotFoundError,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NotFoundDataDoc {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "APIError")]
pub struct ApiErrorDoc {
    pub name: ApiErrorNameDoc,
    pub data: ApiErrorDataDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ApiErrorNameDoc {
    #[serde(rename = "APIError")]
    ApiError,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ApiErrorDataDoc {
    pub message: String,
    #[serde(
        default,
        rename = "statusCode",
        skip_serializing_if = "Option::is_none"
    )]
    pub status_code: Option<f64>,
    #[serde(rename = "isRetryable")]
    pub is_retryable: bool,
    #[serde(
        default,
        rename = "responseHeaders",
        skip_serializing_if = "Option::is_none"
    )]
    pub response_headers: Option<std::collections::BTreeMap<String, String>>,
    #[serde(
        default,
        rename = "responseBody",
        skip_serializing_if = "Option::is_none"
    )]
    pub response_body: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "ProviderAuthError")]
pub struct ProviderAuthErrorDoc {
    pub name: ProviderAuthErrorNameDoc,
    pub data: ProviderAuthErrorDataDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ProviderAuthErrorNameDoc {
    #[serde(rename = "ProviderAuthError")]
    ProviderAuthError,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProviderAuthErrorDataDoc {
    #[serde(rename = "providerID")]
    pub provider_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "UnknownError")]
pub struct UnknownErrorDoc {
    pub name: UnknownErrorNameDoc,
    pub data: UnknownErrorDataDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum UnknownErrorNameDoc {
    #[serde(rename = "UnknownError")]
    UnknownError,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UnknownErrorDataDoc {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "MessageAbortedError")]
pub struct MessageAbortedErrorDoc {
    pub name: MessageAbortedErrorNameDoc,
    pub data: MessageAbortedErrorDataDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum MessageAbortedErrorNameDoc {
    #[serde(rename = "MessageAbortedError")]
    MessageAbortedError,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MessageAbortedErrorDataDoc {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "MessageOutputLengthError")]
pub struct MessageOutputLengthErrorDoc {
    pub name: MessageOutputLengthErrorNameDoc,
    pub data: MessageOutputLengthErrorDataDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum MessageOutputLengthErrorNameDoc {
    #[serde(rename = "MessageOutputLengthError")]
    MessageOutputLengthError,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[schemars(schema_with = "empty_object_schema")]
pub struct MessageOutputLengthErrorDataDoc {}

fn empty_object_schema(_: &mut SchemaGenerator) -> Schema {
    serde_json::from_value(json!({
        "type": "object",
        "properties": {}
    }))
    .expect("empty object schema is valid")
}

fn false_const_schema(_: &mut SchemaGenerator) -> Schema {
    serde_json::from_value(json!({
        "type": "boolean",
        "const": false
    }))
    .expect("false const schema is valid")
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "StructuredOutputError")]
pub struct StructuredOutputErrorDoc {
    pub name: StructuredOutputErrorNameDoc,
    pub data: StructuredOutputErrorDataDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum StructuredOutputErrorNameDoc {
    #[serde(rename = "StructuredOutputError")]
    StructuredOutputError,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StructuredOutputErrorDataDoc {
    pub message: String,
    pub retries: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "ContextOverflowError")]
pub struct ContextOverflowErrorDoc {
    pub name: ContextOverflowErrorNameDoc,
    pub data: ContextOverflowErrorDataDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ContextOverflowErrorNameDoc {
    #[serde(rename = "ContextOverflowError")]
    ContextOverflowError,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ContextOverflowErrorDataDoc {
    pub message: String,
    #[serde(
        default,
        rename = "responseBody",
        skip_serializing_if = "Option::is_none"
    )]
    pub response_body: Option<String>,
}
