//! Tests for the built-in OpenAI-compatible preset catalog.

use provider_openai::{OpenAiApiMode, OpenAiApiSurface, OpenAiConfigPreset};

#[test]
fn preset_lookup_is_case_insensitive() {
    let preset = OpenAiConfigPreset::by_name("GrOq").expect("expected groq preset");
    assert_eq!(preset.name, "groq");
    assert_eq!(preset.base_url, "https://api.groq.com/openai/v1");
}

#[test]
fn built_in_presets_cover_expected_provider_set() {
    let names = OpenAiConfigPreset::ALL
        .iter()
        .map(|preset| preset.name)
        .collect::<Vec<_>>();

    assert_eq!(
        names,
        vec![
            "openai",
            "groq",
            "deepseek",
            "together",
            "xai",
            "fireworks",
            "mistral",
            "openrouter",
            "ollama",
            "gemini",
            "minimax",
        ]
    );
}

#[test]
fn openai_catalog_preserves_rich_model_metadata() {
    let gpt_54 = OpenAiConfigPreset::OPENAI
        .models
        .iter()
        .find(|model| model.id == "gpt-5.4")
        .expect("gpt-5.4 should exist");

    assert_eq!(gpt_54.name.as_ref(), "GPT-5.4");
    assert!(gpt_54.supports_reasoning());
    assert_eq!(gpt_54.cost.as_ref().map(|cost| cost.input), Some(2.5));
    assert_eq!(
        gpt_54.limit.as_ref().map(|limit| limit.context),
        Some(1_050_000)
    );
    assert!(
        gpt_54
            .input_modalities
            .iter()
            .any(|modality| *modality == "image")
    );
}

#[test]
fn ollama_preset_keeps_local_base_url() {
    assert_eq!(
        OpenAiConfigPreset::OLLAMA.base_url,
        "http://localhost:11434/v1"
    );
}

#[test]
fn responses_capability_is_declared_per_preset() {
    assert!(
        OpenAiConfigPreset::OPENAI
            .supported_api_surfaces
            .contains(&OpenAiApiSurface::Responses)
    );
    assert!(
        OpenAiConfigPreset::GROQ
            .supported_api_surfaces
            .contains(&OpenAiApiSurface::Responses)
    );
    assert!(
        !OpenAiConfigPreset::GEMINI
            .supported_api_surfaces
            .contains(&OpenAiApiSurface::Responses)
    );
    assert!(
        !OpenAiConfigPreset::MISTRAL
            .supported_api_surfaces
            .contains(&OpenAiApiSurface::Responses)
    );
}

#[test]
fn auto_prefers_responses_when_supported() {
    let openai = OpenAiConfigPreset::OPENAI.into_config("test-key");
    assert_eq!(
        openai.resolved_api_surface(),
        Some(OpenAiApiSurface::Responses)
    );

    let gemini = OpenAiConfigPreset::GEMINI.into_config("test-key");
    assert_eq!(
        gemini.resolved_api_surface(),
        Some(OpenAiApiSurface::ChatCompletions)
    );

    let forced_chat = openai.with_api_surface_mode(OpenAiApiMode::ChatCompletions);
    assert_eq!(
        forced_chat.resolved_api_surface(),
        Some(OpenAiApiSurface::ChatCompletions)
    );
}
