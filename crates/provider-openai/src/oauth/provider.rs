use std::sync::Arc;
use std::time::Duration;

use futures::{StreamExt, future::BoxFuture};
use provider::{
    CredentialFailure, CredentialPool as SharedCredentialPool, Error as ProviderError, Event,
    EventStream, Provider, ProviderInfo, Request, ResolvedCredential,
};

use crate::{Client, Config, Error, OpenAiApiMode, OpenAiApiSurface, OpenAiProvider};

use super::browser_flow::{BrowserFlowPrompt, BrowserOAuthConfig, run_browser_flow};
use super::device_flow::{DeviceFlowConfig, DeviceUserPrompt, run_device_flow};
use super::preset::OpenAiOAuthPreset;
use super::refresh::OpenAiOAuthCredentials;

pub(crate) async fn resolve_credential(
    pool: &SharedCredentialPool,
    provider_name: &str,
    session_id: Option<&str>,
) -> Result<ResolvedCredential, Error> {
    pool.resolve(provider_name, session_id)
        .await
        .map_err(|err| Error::Auth(err.to_string()))
}

pub(crate) async fn mark_credential_ok(
    pool: &SharedCredentialPool,
    provider_name: &str,
    credential_id: &str,
) {
    pool.mark_ok(provider_name, credential_id).await;
}

pub(crate) async fn mark_credential_error(
    pool: &SharedCredentialPool,
    provider_name: &str,
    credential_id: &str,
    failure: CredentialFailure,
) {
    pool.mark_error(provider_name, credential_id, failure).await;
}

pub(crate) fn config_with_resolved_credential(
    mut config: Config,
    credential: &ResolvedCredential,
) -> Result<Config, Error> {
    let authorization = credential
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("authorization"))
        .map(|(_, value)| value.clone())
        .ok_or_else(|| Error::Auth("resolved credential missing Authorization header".into()))?;

    let bearer = authorization
        .strip_prefix("Bearer ")
        .unwrap_or(authorization.as_str())
        .to_owned();
    config.api_key = bearer;

    for (name, value) in &credential.headers {
        if !name.eq_ignore_ascii_case("authorization") {
            config.default_headers.insert(name.clone(), value.clone());
        }
    }

    Ok(config)
}

pub struct OAuthFlow;

impl OAuthFlow {
    pub async fn login_browser<F>(
        preset: &OpenAiOAuthPreset,
        on_prompt: F,
    ) -> Result<OpenAiOAuthCredentials, Error>
    where
        F: FnOnce(BrowserFlowPrompt),
    {
        let client = reqwest::Client::new();
        run_browser_flow(
            &client,
            &BrowserOAuthConfig {
                authorize_url: preset.authorize_url.to_owned(),
                token_url: preset.token_url.to_owned(),
                client_id: preset.client_id.to_owned(),
                scopes: preset.scopes.to_owned(),
                callback_port: preset.callback_port,
                timeout: Duration::from_secs(300),
            },
            on_prompt,
        )
        .await
    }

    pub async fn login_device<F>(
        preset: &OpenAiOAuthPreset,
        on_prompt: F,
    ) -> Result<OpenAiOAuthCredentials, Error>
    where
        F: FnOnce(DeviceUserPrompt),
    {
        let client = reqwest::Client::new();
        run_device_flow(
            &client,
            &DeviceFlowConfig {
                device_code_url: preset.device_code_url.to_owned(),
                device_token_url: preset.device_token_url.to_owned(),
                token_url: preset.token_url.to_owned(),
                client_id: preset.client_id.to_owned(),
                redirect_uri: preset.device_redirect_uri.to_owned(),
            },
            preset.device_verification_url,
            on_prompt,
        )
        .await
    }
}

#[derive(Clone)]
pub struct OpenAiOAuthProvider {
    preset: OpenAiOAuthPreset,
    credential_pool: Arc<SharedCredentialPool>,
    http: reqwest::Client,
}

impl OpenAiOAuthProvider {
    /// Creates an OAuth-backed provider that resolves credentials from the
    /// shared standalone credential pool.
    pub fn from_pool(preset: OpenAiOAuthPreset, pool: Arc<SharedCredentialPool>) -> Self {
        Self {
            preset,
            credential_pool: pool,
            http: reqwest::Client::new(),
        }
    }

    fn session_id_for_request(request: &Request) -> Option<&str> {
        request
            .options
            .metadata
            .get("session_id")
            .and_then(|value| value.as_str())
    }

    fn temporary_client(&self, credential: &ResolvedCredential) -> Result<Client, Error> {
        let mut config = Config {
            name: self.preset.name.to_owned(),
            api_key: String::new(),
            base_url: self.preset.api_base_url.to_owned(),
            default_model: self.preset.default_model.to_owned(),
            models: self.preset.models,
            supported_api_surfaces: &[OpenAiApiSurface::Responses],
            api_surface_mode: OpenAiApiMode::Responses,
            default_headers: Default::default(),
        }
        .with_default_header("OpenAI-Beta", "responses=experimental")
        .with_default_header("originator", "provider-openai");
        if let Some(account_id) = credential
            .metadata
            .get("account_id")
            .and_then(|value| value.as_str())
        {
            config = config.with_default_header("chatgpt-account-id", account_id);
        }
        Ok(Client::with_http_client(
            config_with_resolved_credential(config, credential)?,
            self.http.clone(),
        ))
    }
}

impl Provider for OpenAiOAuthProvider {
    fn stream<'a>(
        &'a self,
        request: &'a Request,
    ) -> BoxFuture<'a, Result<EventStream<'a>, ProviderError>> {
        Box::pin(async move {
            let credential = resolve_credential(
                &self.credential_pool,
                self.preset.name,
                Self::session_id_for_request(request),
            )
            .await
            .map_err(|err| ProviderError::Configuration(err.to_string()))?;
            let credential_id = credential.credential_id.clone();
            let provider_name = self.preset.name.to_owned();
            let credential_pool = self.credential_pool.clone();
            let client = self
                .temporary_client(&credential)
                .map_err(map_openai_error)?;
            let mut stream = OpenAiProvider::stream_for_surface(
                &client,
                crate::responses::ResponseStreamTransport::Sse,
                request,
            )
            .await?;
            Ok(Box::pin(async_stream::stream! {
                let mut saw_completed = false;
                while let Some(event) = stream.next().await {
                    match event {
                        Ok(event) => {
                            if matches!(event, Event::Completed { .. }) {
                                saw_completed = true;
                            }
                            yield Ok(event);
                        }
                        Err(err) => {
                            mark_credential_error(
                                credential_pool.as_ref(),
                                &provider_name,
                                &credential_id,
                                CredentialFailure::new(err.to_string()),
                            ).await;
                            yield Err(err);
                            return;
                        }
                    }
                }
                if saw_completed {
                    mark_credential_ok(credential_pool.as_ref(), &provider_name, &credential_id).await;
                } else {
                    mark_credential_error(
                        credential_pool.as_ref(),
                        &provider_name,
                        &credential_id,
                        CredentialFailure::new("stream closed before terminal event"),
                    ).await;
                }
            }) as EventStream<'a>)
        })
    }

    fn info(&self) -> ProviderInfo {
        let config = Config {
            name: self.preset.name.to_owned(),
            api_key: String::new(),
            base_url: self.preset.api_base_url.to_owned(),
            default_model: self.preset.default_model.to_owned(),
            models: self.preset.models,
            supported_api_surfaces: &[OpenAiApiSurface::Responses],
            api_surface_mode: OpenAiApiMode::Responses,
            default_headers: Default::default(),
        };
        OpenAiProvider::from_client(Client::with_http_client(config, self.http.clone())).info()
    }
}

fn map_openai_error(err: Error) -> ProviderError {
    match err {
        Error::Auth(message) => ProviderError::Remote(message),
        Error::Inference(message) => ProviderError::Inference(message),
        Error::Internal(message) => ProviderError::Configuration(message),
        Error::Json(err) => ProviderError::Json(err),
    }
}
