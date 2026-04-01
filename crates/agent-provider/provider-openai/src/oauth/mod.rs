mod browser_flow;
mod device_flow;
mod jwt;
mod pkce;
mod preset;
mod provider;
mod refresh;

pub use browser_flow::{BrowserFlowPrompt, BrowserOAuthConfig};
pub use device_flow::{DeviceFlowConfig, DeviceUserPrompt};
pub use preset::OpenAiOAuthPreset;
pub use provider::{OAuthFlow, OpenAiOAuthProvider};
pub(crate) use provider::{
    config_with_resolved_credential, mark_credential_error, mark_credential_ok, resolve_credential,
};
pub use refresh::{OpenAiOAuthCredentials, refresh_access_token};
