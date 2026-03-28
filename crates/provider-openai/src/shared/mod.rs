//! Shared helpers used by the OpenAI wire client.

use serde_json::Value;

use crate::Error;

/// Token-usage structure shared by Responses and Chat Completions parsing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct TokenUsage {
    pub prompt: u32,
    pub completion: u32,
    pub total: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_write: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<u32>,
}

pub(crate) async fn ensure_success(
    response: reqwest::Response,
) -> Result<reqwest::Response, Error> {
    if response.status().is_success() {
        Ok(response)
    } else {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        let detail = serde_json::from_str::<Value>(&text)
            .ok()
            .and_then(|value| {
                value
                    .get("error")
                    .and_then(|error| error.get("message"))
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .filter(|message| !message.is_empty())
            .unwrap_or(text);
        Err(Error::Inference(format!("{status}: {detail}")))
    }
}

pub(crate) async fn json_value(response: reqwest::Response) -> Result<Value, Error> {
    response
        .json()
        .await
        .map_err(|e| Error::Inference(e.to_string()))
}
