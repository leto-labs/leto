//! Top-level Anthropic client facade.
//!
//! Official references:
//! - API overview: <https://platform.claude.com/docs/en/api/overview>
//! - Messages create: <https://platform.claude.com/docs/en/api/messages/create>

use crate::config::Config;
use crate::messages::MessagesClient;

/// Top-level Anthropic wire client exposing the Messages API surface.
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

    /// Returns the Messages API surface.
    ///
    /// Official reference:
    /// <https://platform.claude.com/docs/en/api/messages/create>
    pub fn messages(&self) -> MessagesClient<'_> {
        MessagesClient::new(self)
    }

    pub(crate) fn http(&self) -> &reqwest::Client {
        &self.http
    }

    pub(crate) fn endpoint_url(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.config.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_endpoints_cleanly() {
        let client = Client::new(Config::new("test"));
        assert_eq!(
            client.endpoint_url("/v1/messages"),
            "https://api.anthropic.com/v1/messages"
        );
    }
}
