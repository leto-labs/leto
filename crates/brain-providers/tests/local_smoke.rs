#![cfg(feature = "mistralrs")]

use brain_providers::{MistralRsModelPreset, MistralRsProvider};
use brain_types::*;
use futures::StreamExt;

#[tokio::test]
async fn smoke_mistralrs_qwen3_0_6b() {
    let preset = MistralRsModelPreset::QWEN3_0_6B;
    eprintln!("  TEST mistralrs: {} ({})", preset.name, preset.model_id);
    eprintln!("  (first run downloads ~400 MB from HuggingFace)");

    let config = preset.into_config();
    let provider = MistralRsProvider::new(vec![(preset.name.to_owned(), config)], preset.name);
    if let Err(e) = provider.preload(preset.name).await {
        eprintln!("    SKIP: failed to load model: {e}");
        return;
    }

    let messages = vec![Message::user("Say just the word 'hello' and nothing else.")];
    let inference = InferenceConfig::default();

    match provider.chat(&messages, &[], &inference, None).await {
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
            assert!(!text.is_empty(), "mistralrs: got empty response");
        }
        Err(e) => {
            eprintln!("    ERROR: {e}");
            panic!("mistralrs chat() failed: {e}");
        }
    }
}

#[test]
fn mistralrs_preset_lookup() {
    assert!(MistralRsModelPreset::by_name("qwen3-0.6b").is_some());
    assert!(MistralRsModelPreset::by_name("qwen3-1.7b").is_some());
    assert!(MistralRsModelPreset::by_name("qwen3-4b").is_some());
    assert!(MistralRsModelPreset::by_name("nonexistent").is_none());
    assert!(MistralRsModelPreset::ALL.len() >= 3);
}
