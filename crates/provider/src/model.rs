//! Shared model catalog metadata for provider plugins and preset systems.

use std::borrow::Cow;

use serde::Serialize;

use crate::ProviderCapabilities;

/// Per-million-token pricing in USD for a model.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ModelCost {
    /// Input token price in USD per million tokens.
    pub input: f64,
    /// Output token price in USD per million tokens.
    pub output: f64,
    /// Optional reasoning token price in USD per million tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<f64>,
    /// Optional cached-input read price in USD per million tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read: Option<f64>,
    /// Optional cached-input write price in USD per million tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_write: Option<f64>,
    /// Optional audio-input price in USD per million tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_audio: Option<f64>,
    /// Optional audio-output price in USD per million tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_audio: Option<f64>,
}

/// Token limits advertised for a model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModelLimit {
    /// Total context window in tokens.
    pub context: u64,
    /// Optional maximum input tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<u64>,
    /// Maximum output tokens.
    pub output: u64,
}

/// Shared model catalog entry used by the provider SDK.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ModelInfo {
    /// Stable model identifier as understood by the provider.
    pub id: Cow<'static, str>,
    /// Human-friendly display name.
    pub name: Cow<'static, str>,
    /// Optional provider-specific model family.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<Cow<'static, str>>,
    /// Supported reasoning effort levels, if any.
    #[serde(default, skip_serializing_if = "str_slice_is_empty")]
    pub reasoning_efforts: Cow<'static, [&'static str]>,
    /// Whether the model supports tool calling.
    pub tool_call: bool,
    /// Whether the model supports file attachments or analogous rich inputs.
    pub attachment: bool,
    /// Whether the model supports structured output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structured_output: Option<bool>,
    /// Whether the model supports user-configurable temperature.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<bool>,
    /// Optional knowledge cutoff or snapshot string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub knowledge: Option<Cow<'static, str>>,
    /// Optional release date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_date: Option<Cow<'static, str>>,
    /// Optional last-updated date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<Cow<'static, str>>,
    /// Whether the model has open weights, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_weights: Option<bool>,
    /// Supported input modalities.
    #[serde(default, skip_serializing_if = "str_slice_is_empty")]
    pub input_modalities: Cow<'static, [&'static str]>,
    /// Supported output modalities.
    #[serde(default, skip_serializing_if = "str_slice_is_empty")]
    pub output_modalities: Cow<'static, [&'static str]>,
    /// Optional static pricing metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<ModelCost>,
    /// Optional model limits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<ModelLimit>,
    /// Optional lifecycle status such as `alpha`, `beta`, or `deprecated`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<Cow<'static, str>>,
    /// Optional model-specific runtime capability overrides.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<ProviderCapabilities>,
}

impl ModelInfo {
    /// Returns true when the model exposes any reasoning effort levels.
    pub fn supports_reasoning(&self) -> bool {
        !self.reasoning_efforts.is_empty()
    }
}

fn str_slice_is_empty(value: &Cow<'static, [&'static str]>) -> bool {
    value.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_info_tracks_reasoning_support() {
        let model = ModelInfo {
            id: Cow::Borrowed("gpt-5.4"),
            name: Cow::Borrowed("GPT-5.4"),
            family: Some(Cow::Borrowed("gpt")),
            reasoning_efforts: Cow::Borrowed(&["low", "medium", "high"]),
            tool_call: true,
            attachment: true,
            structured_output: Some(true),
            temperature: Some(false),
            knowledge: Some(Cow::Borrowed("2025-08-31")),
            release_date: Some(Cow::Borrowed("2026-03-05")),
            last_updated: Some(Cow::Borrowed("2026-03-05")),
            open_weights: Some(false),
            input_modalities: Cow::Borrowed(&["text", "image"]),
            output_modalities: Cow::Borrowed(&["text"]),
            cost: Some(ModelCost {
                input: 2.5,
                output: 15.0,
                reasoning: None,
                cache_read: Some(0.25),
                cache_write: None,
                input_audio: None,
                output_audio: None,
            }),
            limit: Some(ModelLimit {
                context: 1_050_000,
                input: Some(922_000),
                output: 128_000,
            }),
            status: None,
            capabilities: None,
        };

        assert!(model.supports_reasoning());

        let json = serde_json::to_string(&model).unwrap();
        assert!(json.contains("\"id\":\"gpt-5.4\""));
        assert!(json.contains("\"reasoning_efforts\":[\"low\",\"medium\",\"high\"]"));
    }
}
