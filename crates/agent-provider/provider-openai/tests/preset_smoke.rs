//! Env-gated live smoke coverage across the OpenAI-compatible preset matrix.

use provider_openai::{
    ChatCompletionRequest, Client, OpenAiApiMode, OpenAiApiSurface, OpenAiConfigPreset,
    ResponseInputContentPart, ResponseInputItem, ResponseInputRole, ResponseRequest,
};

fn smoke_request(prompt: &str) -> ResponseRequest {
    ResponseRequest {
        input: vec![ResponseInputItem::message(
            ResponseInputRole::User,
            vec![ResponseInputContentPart::input_text(prompt)],
        )],
        ..ResponseRequest::default()
    }
}

fn is_skippable_live_error(err: &provider_openai::Error) -> bool {
    let message = err.to_string();
    message.contains("401")
        || message.contains("403")
        || message.contains("404")
        || message.contains("429")
        || message.contains("quota")
        || message.contains("rate limit")
}

fn api_mode(surface: OpenAiApiSurface) -> OpenAiApiMode {
    match surface {
        OpenAiApiSurface::Responses => OpenAiApiMode::Responses,
        OpenAiApiSurface::ChatCompletions => OpenAiApiMode::ChatCompletions,
    }
}

#[tokio::test]
async fn smoke_presets_over_supported_surfaces() {
    dotenvy::dotenv().ok();

    for preset in OpenAiConfigPreset::ALL {
        let Ok(api_key) = std::env::var(preset.env_key) else {
            eprintln!(
                "  SKIP provider-openai preset {}: {} not set",
                preset.name, preset.env_key
            );
            continue;
        };

        for &surface in preset.supported_api_surfaces {
            let client = Client::new(
                preset
                    .into_config(api_key.clone())
                    .with_api_surface_mode(api_mode(surface)),
            );

            match surface {
                OpenAiApiSurface::Responses => {
                    match client
                        .responses()
                        .create(&smoke_request(
                            "Say just the word 'hello' and nothing else.",
                        ))
                        .await
                    {
                        Ok(response) => {
                            assert!(
                                response.error.is_none(),
                                "{} returned a response error",
                                preset.name
                            );
                        }
                        Err(err) if is_skippable_live_error(&err) => {
                            eprintln!(
                                "  SKIP provider-openai preset {} {:?} auth/billing: {}",
                                preset.name, surface, err
                            );
                        }
                        Err(err) => panic!(
                            "provider-openai preset {} {:?} responses failed: {}",
                            preset.name, surface, err
                        ),
                    }
                }
                OpenAiApiSurface::ChatCompletions => {
                    match client
                        .chat_completions()
                        .create(&ChatCompletionRequest::user_text(
                            "Say just the word 'hello' and nothing else.",
                        ))
                        .await
                    {
                        Ok(response) => assert!(
                            !response.choices.is_empty(),
                            "{} returned no choices",
                            preset.name
                        ),
                        Err(err) if is_skippable_live_error(&err) => {
                            eprintln!(
                                "  SKIP provider-openai preset {} {:?} auth/billing: {}",
                                preset.name, surface, err
                            );
                        }
                        Err(err) => panic!(
                            "provider-openai preset {} {:?} chat completions failed: {}",
                            preset.name, surface, err
                        ),
                    }
                }
            }
        }
    }
}
