use std::sync::Arc;
use std::time::Duration;

use futures::future::BoxFuture;
use tokio::sync::Mutex;

use brain_types::*;

use crate::codex_sse::*;
use crate::oauth::browser_flow::{self, BrowserFlowConfig, BrowserFlowPrompt};
use crate::oauth::device_flow::{self, DeviceFlowConfig, DeviceUserPrompt};
use crate::oauth::refresh;
use crate::pool::CredentialPool;

use super::preset::OpenAiOAuthPreset;

enum CredentialSource {
    Direct {
        creds: Arc<Mutex<OAuthCredentials>>,
        store: Arc<dyn CredentialStore>,
        provider_name: String,
    },
    Pool(Arc<CredentialPool>),
}

pub struct OpenAiOAuthProvider {
    source: CredentialSource,
    provider_name: String,
    api_base_url: String,
    default_model: String,
    models: Vec<ProviderModelInfo>,
    client: reqwest::Client,
}

impl OpenAiOAuthProvider {
    /// Create from existing OAuth credentials and a credential store (legacy mode).
    pub fn new(
        creds: OAuthCredentials,
        store: Arc<dyn CredentialStore>,
        preset: &OpenAiOAuthPreset,
    ) -> Self {
        let provider_name = preset.name.to_owned();
        Self {
            source: CredentialSource::Direct {
                creds: Arc::new(Mutex::new(creds)),
                store,
                provider_name: provider_name.clone(),
            },
            provider_name,
            api_base_url: preset.api_base_url.to_owned(),
            default_model: preset.default_model.to_owned(),
            models: preset.models.iter().map(ProviderModelInfo::from).collect(),
            client: reqwest::Client::new(),
        }
    }

    /// Create from a CredentialPool (pool-based mode).
    pub fn with_pool(pool: Arc<CredentialPool>, preset: &OpenAiOAuthPreset) -> Self {
        Self {
            source: CredentialSource::Pool(pool),
            provider_name: preset.name.to_owned(),
            api_base_url: preset.api_base_url.to_owned(),
            default_model: preset.default_model.to_owned(),
            models: preset.models.iter().map(ProviderModelInfo::from).collect(),
            client: reqwest::Client::new(),
        }
    }

    /// Run the browser-based OAuth login flow and return a ready provider.
    #[deprecated(note = "Use OAuthFlow::login_browser instead")]
    pub async fn login_browser<F>(
        store: Arc<dyn CredentialStore>,
        preset: &OpenAiOAuthPreset,
        on_prompt: F,
    ) -> Result<Self, BrainError>
    where
        F: FnOnce(BrowserFlowPrompt),
    {
        let entry = OAuthFlow::login_browser(&*store, preset, on_prompt).await?;
        let ProviderCredential::OAuth(creds) = entry.credential else {
            return Err(BrainError::Internal(
                "OAuthFlow returned non-OAuth credential".into(),
            ));
        };
        Ok(Self::new(creds, store, preset))
    }

    /// Run the device code OAuth login flow and return a ready provider.
    #[deprecated(note = "Use OAuthFlow::login_device instead")]
    pub async fn login_device<F>(
        store: Arc<dyn CredentialStore>,
        preset: &OpenAiOAuthPreset,
        on_user_prompt: F,
    ) -> Result<Self, BrainError>
    where
        F: FnOnce(DeviceUserPrompt),
    {
        let entry = OAuthFlow::login_device(&*store, preset, on_user_prompt).await?;
        let ProviderCredential::OAuth(creds) = entry.credential else {
            return Err(BrainError::Internal(
                "OAuthFlow returned non-OAuth credential".into(),
            ));
        };
        Ok(Self::new(creds, store, preset))
    }

    /// Load stored OAuth credentials and return a ready provider.
    pub async fn from_stored(
        store: Arc<dyn CredentialStore>,
        preset: &OpenAiOAuthPreset,
    ) -> Result<Option<Self>, BrainError> {
        let entries = store.credential_load_all(preset.name).await?;
        for entry in entries {
            if let ProviderCredential::OAuth(creds) = entry.credential {
                return Ok(Some(Self::new(creds, store, preset)));
            }
        }
        Ok(None)
    }

    async fn resolve_token(
        &self,
        session_id: Option<ulid::Ulid>,
    ) -> Result<(String, Option<String>), BrainError> {
        match &self.source {
            CredentialSource::Pool(pool) => {
                let entry = pool.resolve(&self.provider_name, session_id).await?;
                let ProviderCredential::OAuth(oauth) = &entry.credential else {
                    return Err(BrainError::Internal("expected OAuth credential".into()));
                };
                Ok((oauth.access_token.clone(), oauth.account_id.clone()))
            }
            CredentialSource::Direct {
                creds,
                store,
                provider_name,
            } => {
                let needs_refresh;
                let snapshot;
                {
                    let guard = creds.lock().await;
                    needs_refresh = guard.needs_refresh();
                    snapshot = guard.clone();
                }

                if needs_refresh {
                    let new_creds = refresh::refresh_token(&self.client, &snapshot).await?;
                    let entry = CredentialEntry::oauth(new_creds.clone());
                    store.credential_save(provider_name, &entry).await?;

                    let token = new_creds.access_token.clone();
                    let account_id = new_creds.account_id.clone();
                    *creds.lock().await = new_creds;
                    Ok((token, account_id))
                } else {
                    Ok((snapshot.access_token, snapshot.account_id))
                }
            }
        }
    }
}

impl Provider for OpenAiOAuthProvider {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: self.provider_name.clone(),
            default_model: Some(self.default_model.clone()),
            models: self.models.clone(),
        }
    }

    fn chat<'a>(
        &'a self,
        messages: &'a [Message],
        tools: &'a [ToolDef],
        config: &'a InferenceConfig,
        session_id: Option<ulid::Ulid>,
    ) -> BoxFuture<'a, Result<ChatStream, BrainError>> {
        Box::pin(async move {
            let (access_token, account_id) = self.resolve_token(session_id).await?;

            let model = config
                .model
                .as_deref()
                .unwrap_or(&self.default_model)
                .to_string();

            let (instructions, input) = build_responses_input(messages);
            let resp_tools = to_responses_tools(tools);

            let body = ResponsesRequest {
                model,
                input,
                instructions,
                store: false,
                stream: true,
                text: TextOptions {
                    verbosity: "medium".to_owned(),
                },
                reasoning: ReasoningOptions {
                    effort: "high".to_owned(),
                    summary: "auto".to_owned(),
                },
                include: vec!["reasoning.encrypted_content".to_owned()],
                tools: resp_tools,
                tool_choice: if tools.is_empty() {
                    None
                } else {
                    Some("auto".to_owned())
                },
                parallel_tool_calls: if tools.is_empty() { None } else { Some(true) },
            };

            let url = format!("{}/responses", self.api_base_url.trim_end_matches('/'));

            let mut req = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {access_token}"))
                .header("OpenAI-Beta", "responses=experimental")
                .header("originator", "brain")
                .header("accept", "text/event-stream")
                .json(&body);

            if let Some(ref acct_id) = account_id {
                req = req.header("chatgpt-account-id", acct_id);
            }

            let response = req
                .send()
                .await
                .map_err(|e| BrainError::Inference(e.to_string()))?;

            if !response.status().is_success() {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                return Err(BrainError::Inference(format!("{status}: {text}")));
            }

            Ok(stream_from_response(response))
        })
    }
}

/// Standalone credential factory for OAuth login flows.
///
/// Produces `CredentialEntry` values and saves them to the store.
/// Not a Provider — callers add the credential to a `CredentialPool`.
pub struct OAuthFlow;

impl OAuthFlow {
    /// Run the browser-based OAuth login flow.
    pub async fn login_browser<F>(
        store: &dyn CredentialStore,
        preset: &OpenAiOAuthPreset,
        on_prompt: F,
    ) -> Result<CredentialEntry, BrainError>
    where
        F: FnOnce(BrowserFlowPrompt),
    {
        let client = reqwest::Client::new();
        let flow_config = BrowserFlowConfig {
            authorize_url: preset.authorize_url.to_owned(),
            token_url: preset.token_url.to_owned(),
            client_id: preset.client_id.to_owned(),
            scopes: preset.scopes.to_owned(),
            callback_port: preset.callback_port,
            timeout: Duration::from_secs(300),
        };

        let creds = browser_flow::run(&client, &flow_config, on_prompt).await?;
        let entry = CredentialEntry::oauth(creds);
        store.credential_save(preset.name, &entry).await?;
        Ok(entry)
    }

    /// Run the device code OAuth login flow.
    pub async fn login_device<F>(
        store: &dyn CredentialStore,
        preset: &OpenAiOAuthPreset,
        on_user_prompt: F,
    ) -> Result<CredentialEntry, BrainError>
    where
        F: FnOnce(DeviceUserPrompt),
    {
        let client = reqwest::Client::new();
        let flow_config = DeviceFlowConfig {
            device_code_url: preset.device_code_url.to_owned(),
            device_token_url: preset.device_token_url.to_owned(),
            token_url: preset.token_url.to_owned(),
            client_id: preset.client_id.to_owned(),
            redirect_uri: preset.device_redirect_uri.to_owned(),
        };

        let creds = device_flow::run(
            &client,
            &flow_config,
            preset.device_verification_url,
            on_user_prompt,
        )
        .await?;
        let entry = CredentialEntry::oauth(creds);
        store.credential_save(preset.name, &entry).await?;
        Ok(entry)
    }
}
