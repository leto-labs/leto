//! Env-gated live smoke tests for the OpenAI OAuth-backed provider path.

use std::sync::Arc;

use futures::StreamExt;
use provider::{CredentialEntry, CredentialPool, Provider as _, Request, StickyRoundRobin};
use provider_openai::{
    OpenAiOAuthCredentials, OpenAiOAuthPreset, OpenAiOAuthProvider, refresh_access_token,
};

fn is_skippable_live_error(message: &str) -> bool {
    message.contains("401")
        || message.contains("403")
        || message.contains("404")
        || message.contains("429")
        || message.contains("rate limit")
        || message.contains("quota")
}

fn openai_oauth_credentials() -> Option<OpenAiOAuthCredentials> {
    dotenvy::dotenv().ok();
    let raw = std::env::var("OPENAI_OAUTH").ok()?;
    serde_json::from_str(&raw)
        .or_else(|_| {
            let stripped = raw
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
                .unwrap_or(raw.as_str());
            serde_json::from_str(stripped)
        })
        .ok()
}

#[tokio::test]
async fn oauth_pool_provider_streams_with_env_credentials() {
    let Some(credentials) = openai_oauth_credentials() else {
        eprintln!("  SKIP provider-openai oauth smoke: OPENAI_OAUTH not set");
        return;
    };

    let refreshed = match refresh_access_token(&reqwest::Client::new(), &credentials).await {
        Ok(credentials) => credentials,
        Err(err) if is_skippable_live_error(&err.to_string()) => {
            eprintln!("  SKIP provider-openai oauth refresh auth/billing: {err}");
            return;
        }
        Err(err) => panic!("provider-openai oauth refresh failed: {err}"),
    };

    let pool = Arc::new(CredentialPool::new(Arc::new(StickyRoundRobin::new())));
    pool.insert(
        OpenAiOAuthPreset::OPENAI.name,
        CredentialEntry::openai_oauth(
            "oauth-env",
            refreshed.access_token,
            refreshed.account_id.clone(),
        ),
    )
    .await;

    let provider = OpenAiOAuthProvider::from_pool(OpenAiOAuthPreset::OPENAI, pool);
    let request = Request::user_text("Say just the word 'hello' and nothing else.");
    let mut stream = match provider.stream(&request).await {
        Ok(stream) => stream,
        Err(err) if is_skippable_live_error(&err.to_string()) => {
            eprintln!("  SKIP provider-openai oauth stream auth/billing: {err}");
            return;
        }
        Err(err) => panic!("provider-openai oauth stream failed to start: {err}"),
    };

    let mut saw_completed = false;
    while let Some(event) = stream.next().await {
        match event.unwrap() {
            provider::Event::Completed { .. } => saw_completed = true,
            provider::Event::ResponseStart { .. }
            | provider::Event::BlockStart { .. }
            | provider::Event::BlockDelta { .. }
            | provider::Event::BlockStop { .. }
            | provider::Event::Usage { .. } => {}
        }
    }

    assert!(
        saw_completed,
        "provider-openai oauth stream did not complete"
    );
}
