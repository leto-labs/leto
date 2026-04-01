//! Env-gated live smoke tests for the shared OpenAI provider adapter across
//! every declared preset/surface pair.

use futures::StreamExt;
use provider::{Event, Provider as _, Request};
use provider_openai::{OpenAiApiMode, OpenAiApiSurface, OpenAiConfigPreset, OpenAiProvider};

fn api_mode(surface: OpenAiApiSurface) -> OpenAiApiMode {
    match surface {
        OpenAiApiSurface::Responses => OpenAiApiMode::Responses,
        OpenAiApiSurface::ChatCompletions => OpenAiApiMode::ChatCompletions,
    }
}

fn is_skippable_live_error(err: &provider::Error) -> bool {
    let message = err.to_string();
    message.contains("401")
        || message.contains("403")
        || message.contains("404")
        || message.contains("429")
        || message.contains("quota")
        || message.contains("rate limit")
}

#[tokio::test]
async fn shared_adapter_streams_for_all_supported_preset_surfaces() {
    dotenvy::dotenv().ok();

    for preset in OpenAiConfigPreset::ALL {
        let Ok(api_key) = std::env::var(preset.env_key) else {
            eprintln!(
                "  SKIP provider-openai shared adapter {}: {} not set",
                preset.name, preset.env_key
            );
            continue;
        };

        for &surface in preset.supported_api_surfaces {
            let provider = OpenAiProvider::new(
                preset
                    .into_config(api_key.clone())
                    .with_api_surface_mode(api_mode(surface)),
            );

            let request = Request::user_text("Say just the word 'hello' and nothing else.");
            let mut stream = match provider.stream(&request).await {
                Ok(stream) => stream,
                Err(err) if is_skippable_live_error(&err) => {
                    eprintln!(
                        "  SKIP provider-openai shared adapter {} {:?} auth/billing: {}",
                        preset.name, surface, err
                    );
                    continue;
                }
                Err(err) => {
                    panic!(
                        "provider-openai shared adapter {} {:?} failed to start: {}",
                        preset.name, surface, err
                    )
                }
            };

            let mut saw_block = false;
            let mut saw_completed = false;

            while let Some(event) = stream.next().await {
                match event.unwrap() {
                    Event::BlockStart { .. } => saw_block = true,
                    Event::Completed { .. } => {
                        saw_completed = true;
                        break;
                    }
                    Event::ResponseStart { .. }
                    | Event::BlockDelta { .. }
                    | Event::BlockStop { .. }
                    | Event::Usage { .. } => {}
                }
            }

            assert!(
                saw_block,
                "provider-openai shared adapter {} {:?} emitted no block",
                preset.name, surface
            );
            assert!(
                saw_completed,
                "provider-openai shared adapter {} {:?} did not complete",
                preset.name, surface
            );
        }
    }
}
