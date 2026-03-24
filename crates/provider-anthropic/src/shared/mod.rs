//! Shared helpers used by the Anthropic wire client.

use serde_json::Value;

use crate::Error;

pub(crate) async fn ensure_success(
    response: reqwest::Response,
) -> Result<reqwest::Response, Error> {
    if response.status().is_success() {
        Ok(response)
    } else {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        Err(Error::Inference(format!("{status}: {text}")))
    }
}

pub(crate) async fn json_value(response: reqwest::Response) -> Result<Value, Error> {
    response
        .json()
        .await
        .map_err(|e| Error::Inference(e.to_string()))
}
