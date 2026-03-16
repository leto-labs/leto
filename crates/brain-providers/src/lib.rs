pub mod mock;
pub mod strategy;

#[cfg(any(feature = "openai", feature = "openai-oauth"))]
pub(crate) mod openai_sse;

#[cfg(feature = "openai-oauth")]
pub(crate) mod codex_sse;

#[cfg(feature = "openai")]
pub mod openai;

#[cfg(feature = "openai-oauth")]
pub mod oauth;
#[cfg(feature = "openai-oauth")]
pub mod openai_oauth;

pub mod pool;

#[cfg(feature = "mistralrs")]
pub mod mistralrs;

#[cfg(feature = "llamacpp")]
pub mod llamacpp;

pub use mock::MockProvider;
pub use strategy::{StickyRoundRobin, Fallback};
#[cfg(feature = "openai")]
pub use openai::{OpenAiConfig, OpenAiConfigPreset, OpenAiProvider};
#[cfg(feature = "openai-oauth")]
pub use openai_oauth::{OAuthFlow, OpenAiOAuthPreset, OpenAiOAuthProvider};
#[cfg(feature = "openai-oauth")]
pub use oauth::{
    OAuthCredentials, ProviderCredential, CredentialStore,
    browser_flow, device_flow, pkce, refresh,
};
pub use pool::CredentialPool;
#[cfg(feature = "mistralrs")]
pub use self::mistralrs::{DevicePreference, MistralRsConfig, MistralRsModelPreset, MistralRsProvider};
#[cfg(feature = "llamacpp")]
pub use self::llamacpp::{LlamaCppConfig, LlamaCppModelPreset, LlamaCppProvider};
