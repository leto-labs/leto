pub mod mock;
pub mod strategy;

#[cfg(any(feature = "openai", feature = "openai-oauth"))]
pub(crate) mod openai_sse;

#[cfg(any(feature = "openai", feature = "openai-oauth"))]
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

#[cfg(feature = "llamacpp")]
pub use self::llamacpp::{LlamaCppConfig, LlamaCppModelPreset, LlamaCppProvider};
#[cfg(feature = "mistralrs")]
pub use self::mistralrs::{
    DevicePreference, MistralRsConfig, MistralRsModelPreset, MistralRsProvider,
};
pub use mock::MockProvider;
#[cfg(feature = "openai-oauth")]
pub use oauth::{
    CredentialStore, OAuthCredentials, ProviderCredential, browser_flow, device_flow, pkce, refresh,
};
#[cfg(feature = "openai")]
pub use openai::{
    OpenAiApiMode, OpenAiApiSurface, OpenAiConfig, OpenAiConfigPreset, OpenAiProvider,
};
#[cfg(feature = "openai-oauth")]
pub use openai_oauth::{OAuthFlow, OpenAiOAuthPreset, OpenAiOAuthProvider};
pub use pool::CredentialPool;
pub use strategy::{Fallback, StickyRoundRobin};
