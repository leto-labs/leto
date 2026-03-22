use brain_providers::{OpenAiApiMode, OpenAiApiSurface, OpenAiConfigPreset};

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

    assert_eq!(names.len(), 11);
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
fn openai_catalog_includes_recent_chat_models() {
    let model_ids = OpenAiConfigPreset::OPENAI
        .models
        .iter()
        .map(|model| model.id)
        .collect::<Vec<_>>();

    assert!(model_ids.contains(&"gpt-5.4"));
    assert!(model_ids.contains(&"gpt-5.2"));
    assert!(model_ids.contains(&"gpt-5.1"));
    assert!(!model_ids.iter().any(|id| id.contains("codex")));
    assert!(!model_ids.iter().any(|id| id.contains("embedding")));
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
