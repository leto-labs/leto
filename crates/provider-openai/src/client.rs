use reqwest::Url;

use crate::chat_completions::ChatCompletionsClient;
use crate::config::Config;
use crate::responses::ResponsesClient;
use crate::Error;

#[derive(Debug, Clone)]
pub struct Client {
    config: Config,
    http: reqwest::Client,
}

impl Client {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            http: reqwest::Client::new(),
        }
    }

    pub fn with_http_client(config: Config, client: reqwest::Client) -> Self {
        Self {
            config,
            http: client,
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn responses(&self) -> ResponsesClient<'_> {
        ResponsesClient::new(self)
    }

    pub fn chat_completions(&self) -> ChatCompletionsClient<'_> {
        ChatCompletionsClient::new(self)
    }

    pub(crate) fn http(&self) -> &reqwest::Client {
        &self.http
    }

    pub(crate) fn auth_header(&self) -> String {
        format!("Bearer {}", self.config.api_key)
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
        let mut url = Url::parse(&base).map_err(|e| Error::Internal(e.to_string()))?;
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
}
