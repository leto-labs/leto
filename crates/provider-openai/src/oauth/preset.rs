use provider::ModelInfo;

#[derive(Debug, Clone, Copy)]
pub struct OpenAiOAuthPreset {
    pub name: &'static str,
    pub authorize_url: &'static str,
    pub token_url: &'static str,
    pub device_code_url: &'static str,
    pub device_token_url: &'static str,
    pub device_verification_url: &'static str,
    pub device_redirect_uri: &'static str,
    pub client_id: &'static str,
    pub scopes: &'static str,
    pub api_base_url: &'static str,
    pub default_model: &'static str,
    pub callback_port: u16,
    pub models: &'static [ModelInfo],
}

impl OpenAiOAuthPreset {
    pub const OPENAI: Self = Self {
        name: "openai-oauth",
        authorize_url: "https://auth.openai.com/oauth/authorize",
        token_url: "https://auth.openai.com/oauth/token",
        device_code_url: "https://auth.openai.com/api/accounts/deviceauth/usercode",
        device_token_url: "https://auth.openai.com/api/accounts/deviceauth/token",
        device_verification_url: "https://auth.openai.com/codex/device",
        device_redirect_uri: "https://auth.openai.com/deviceauth/callback",
        client_id: "app_EMoamEEZ73f0CkXaXp7hrann",
        scopes: "openid profile email offline_access",
        api_base_url: "https://chatgpt.com/backend-api/codex",
        default_model: "gpt-5.3-codex",
        callback_port: 1455,
        models: &[],
    };

    pub fn by_name(name: &str) -> Option<Self> {
        match name {
            "openai-oauth" => Some(Self::OPENAI),
            _ => None,
        }
    }
}
