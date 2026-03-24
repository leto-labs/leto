//! Env-gated live smoke tests for the shared OpenAI provider adapter.

use futures::StreamExt;
use provider::{Event, Provider as _, Request};
use provider_openai::{Config, OpenAiProvider};

fn provider_client() -> Option<OpenAiProvider> {
    dotenvy::dotenv().ok();
    std::env::var("OPENAI_API_KEY")
        .ok()
        .map(|api_key| OpenAiProvider::new(Config::new(api_key).with_model("gpt-5.4-mini")))
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
async fn shared_openai_provider_streams_block_events() {
    let Some(provider) = provider_client() else {
        eprintln!("  SKIP provider-openai shared adapter: OPENAI_API_KEY not set");
        return;
    };

    let request = Request::user_text("Say just the word 'hello' and nothing else.");
    let mut stream = match provider.stream(&request).await {
        Ok(stream) => stream,
        Err(err) if is_skippable_live_error(&err) => {
            eprintln!("  SKIP provider-openai shared adapter auth/billing: {err}");
            return;
        }
        Err(err) => panic!("provider-openai shared adapter failed to start: {err}"),
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

    assert!(saw_block);
    assert!(saw_completed);
}
