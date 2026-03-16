use brain_providers::{OpenAiConfigPreset, OpenAiProvider};
use brain_types::*;
use futures::StreamExt;

async fn smoke_test_preset(preset: OpenAiConfigPreset) {
    let config = match preset.from_env() {
        Some(c) => c,
        None => {
            eprintln!("  SKIP {}: {} not set", preset.name, preset.env_key);
            return;
        }
    };

    eprintln!(
        "  TEST {}: {} @ {}",
        preset.name, config.default_model, config.base_url
    );

    let provider = OpenAiProvider::new(config);
    let messages = vec![Message::user("Say just the word 'hello' and nothing else.")];
    let inference = InferenceConfig::default();

    let result = provider.chat(&messages, &[], &inference, None).await;
    match result {
        Ok(mut stream) => {
            let mut text = String::new();
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(ChatChunk::Delta { content }) => text.push_str(&content),
                    Ok(ChatChunk::Done { usage }) => {
                        let tokens = usage.map(|u| u.total).unwrap_or(0);
                        eprintln!("    OK: {:?} ({} tokens)", text.trim(), tokens);
                    }
                    Ok(ChatChunk::ToolCall { .. }) => {}
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("    STREAM ERROR: {e}");
                        break;
                    }
                }
            }
            assert!(!text.is_empty(), "{}: got empty response", preset.name);
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("401") || msg.contains("403") || msg.contains("429") {
                eprintln!("    SKIP (auth/billing): {msg}");
                return;
            }
            panic!("{}: chat() failed: {e}", preset.name);
        }
    }
}

#[tokio::test]
async fn smoke_openai() {
    dotenvy::dotenv().ok();
    smoke_test_preset(OpenAiConfigPreset::OPENAI).await;
}

#[tokio::test]
async fn smoke_gemini() {
    dotenvy::dotenv().ok();
    smoke_test_preset(OpenAiConfigPreset::GEMINI).await;
}

#[tokio::test]
async fn smoke_openrouter() {
    dotenvy::dotenv().ok();
    smoke_test_preset(OpenAiConfigPreset::OPENROUTER).await;
}

#[tokio::test]
async fn smoke_xai() {
    dotenvy::dotenv().ok();
    smoke_test_preset(OpenAiConfigPreset::XAI).await;
}

#[tokio::test]
async fn smoke_mistral() {
    dotenvy::dotenv().ok();
    smoke_test_preset(OpenAiConfigPreset::MISTRAL).await;
}

#[tokio::test]
async fn smoke_minimax() {
    dotenvy::dotenv().ok();
    smoke_test_preset(OpenAiConfigPreset::MINIMAX).await;
}

#[tokio::test]
async fn smoke_groq() {
    dotenvy::dotenv().ok();
    smoke_test_preset(OpenAiConfigPreset::GROQ).await;
}

#[tokio::test]
async fn smoke_deepseek() {
    dotenvy::dotenv().ok();
    smoke_test_preset(OpenAiConfigPreset::DEEPSEEK).await;
}

#[tokio::test]
async fn smoke_together() {
    dotenvy::dotenv().ok();
    smoke_test_preset(OpenAiConfigPreset::TOGETHER).await;
}

#[tokio::test]
async fn smoke_fireworks() {
    dotenvy::dotenv().ok();
    smoke_test_preset(OpenAiConfigPreset::FIREWORKS).await;
}
