use brain_providers::OpenAiConfigPreset;

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
