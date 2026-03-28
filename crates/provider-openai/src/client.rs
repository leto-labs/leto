//! Top-level OpenAI client facade.
//!
//! Official references:
//! - Responses API overview: <https://developers.openai.com/api/reference/responses/overview>
//! - Chat Completions overview: <https://developers.openai.com/api/reference/chat-completions/overview>

use reqwest::Url;

use crate::Error;
use crate::assistants::AssistantsClient;
use crate::audit_logs::AuditLogsClient;
use crate::chat_completions::ChatCompletionsClient;
use crate::config::Config;
use crate::embeddings::EmbeddingsClient;
use crate::fine_tuning::FineTuningClient;
use crate::responses::ResponsesClient;
use crate::vector_stores::VectorStoresClient;
use crate::videos::VideosClient;

/// Top-level OpenAI wire client that exposes explicit modern API surfaces.
#[derive(Debug, Clone)]
pub struct Client {
    config: Config,
    http: reqwest::Client,
}

impl Client {
    /// Creates a new client with a default `reqwest` HTTP client.
    pub fn new(config: Config) -> Self {
        Self {
            config,
            http: reqwest::Client::new(),
        }
    }

    /// Creates a client with a caller-provided HTTP client.
    pub fn with_http_client(config: Config, client: reqwest::Client) -> Self {
        Self {
            config,
            http: client,
        }
    }

    /// Returns the immutable client configuration.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Returns the Responses API surface.
    ///
    /// Official reference:
    /// <https://developers.openai.com/api/reference/resources/responses/methods/create>
    pub fn responses(&self) -> ResponsesClient<'_> {
        ResponsesClient::new(self)
    }

    /// Returns the Audit Logs API surface.
    ///
    /// Official reference:
    /// <https://platform.openai.com/docs/api-reference/audit-logs>
    pub fn audit_logs(&self) -> AuditLogsClient<'_> {
        AuditLogsClient::new(self)
    }

    /// Returns the Assistants API surface.
    ///
    /// Official reference:
    /// <https://platform.openai.com/docs/api-reference/assistants>
    pub fn assistants(&self) -> AssistantsClient<'_> {
        AssistantsClient::new(self)
    }

    /// Returns the Chat Completions API surface.
    ///
    /// Official reference:
    /// <https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create>
    pub fn chat_completions(&self) -> ChatCompletionsClient<'_> {
        ChatCompletionsClient::new(self)
    }

    /// Returns the Fine-Tuning API surface.
    ///
    /// Official reference:
    /// <https://platform.openai.com/docs/api-reference/fine-tuning>
    pub fn fine_tuning(&self) -> FineTuningClient<'_> {
        FineTuningClient::new(self)
    }

    /// Returns the Embeddings API surface.
    ///
    /// Official reference:
    /// <https://platform.openai.com/docs/api-reference/embeddings>
    pub fn embeddings(&self) -> EmbeddingsClient<'_> {
        EmbeddingsClient::new(self)
    }

    /// Returns the Vector Stores API surface.
    ///
    /// Official reference:
    /// <https://platform.openai.com/docs/api-reference/vector-stores>
    pub fn vector_stores(&self) -> VectorStoresClient<'_> {
        VectorStoresClient::new(self)
    }

    /// Returns the Videos API surface.
    ///
    /// Official reference:
    /// <https://platform.openai.com/docs/api-reference/videos>
    pub fn videos(&self) -> VideosClient<'_> {
        VideosClient::new(self)
    }

    pub(crate) fn http(&self) -> &reqwest::Client {
        &self.http
    }

    pub(crate) fn http_client(&self) -> reqwest::Client {
        self.http.clone()
    }

    pub(crate) fn auth_header(&self) -> String {
        format!("Bearer {}", self.config.api_key)
    }

    pub(crate) fn apply_default_headers(
        &self,
        mut builder: reqwest::RequestBuilder,
    ) -> reqwest::RequestBuilder {
        for (name, value) in &self.config.default_headers {
            builder = builder.header(name, value);
        }
        builder
    }

    pub(crate) fn default_headers(&self) -> &std::collections::BTreeMap<String, String> {
        &self.config.default_headers
    }

    pub(crate) fn endpoint_url(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.config.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    pub(crate) fn endpoint_joined_url(&self, segments: &[&str]) -> Result<Url, Error> {
        let base = self.config.base_url.trim_end_matches('/');
        let mut url = Url::parse(base).map_err(|e| Error::Internal(e.to_string()))?;
        let mut path_segments = url
            .path_segments_mut()
            .map_err(|_| Error::Internal("base URL does not support path segments".into()))?;
        for segment in segments {
            path_segments.push(segment);
        }
        drop(path_segments);
        Ok(url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_endpoints_cleanly() {
        let client = Client::new(Config::new("test"));
        assert_eq!(
            client.endpoint_url("/responses"),
            "https://api.openai.com/v1/responses"
        );
        assert_eq!(
            client.endpoint_url("chat/completions"),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    #[test]
    fn joins_http_base_url_paths() {
        let client = Client::new(Config::new("test").with_base_url("http://localhost:11434/v1"));
        let url = client.endpoint_joined_url(&["responses", "abc", "input_items"]);
        assert_eq!(
            url.unwrap().as_str(),
            "http://localhost:11434/v1/responses/abc/input_items"
        );
    }

    #[test]
    fn stores_default_headers() {
        let client =
            Client::new(Config::new("test").with_default_header("x-provider", "provider-openai"));
        assert_eq!(
            client
                .default_headers()
                .get("x-provider")
                .map(String::as_str),
            Some("provider-openai")
        );
    }
}
