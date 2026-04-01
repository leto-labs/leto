#![cfg(feature = "llamacpp")]

use futures::StreamExt;
use provider::{
    BlockKind, ContentBlock, Event, Message, MessageRole, Provider as _, ReasoningConfig, Request,
};
use provider_llamacpp::{LlamaCppModelPreset, LlamaCppProvider};
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::Mutex;

const TINY_PNG_DATA_URL: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVQIHWP4z8DwHwAFAAH/e+m+7wAAAABJRU5ErkJggg==";

fn llama_smoke_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

async fn collect_events(request: Request) -> Vec<Event> {
    let _guard = llama_smoke_lock().lock().await;
    tokio::time::timeout(Duration::from_secs(120), async move {
        let preset = LlamaCppModelPreset::QWEN35_0_8B;
        let preset_id = preset.id().to_owned();
        let provider =
            LlamaCppProvider::new(vec![(preset_id.clone(), preset.into_config())], &preset_id)
                .unwrap();
        provider.preload(&preset_id).await.unwrap();

        let mut stream = provider.stream(&request).await.unwrap();
        let mut events = Vec::new();
        while let Some(event) = stream.next().await {
            events.push(event.unwrap());
        }
        events
    })
    .await
    .expect("local llama.cpp smoke timed out")
}

fn saw_completed(events: &[Event]) -> bool {
    events
        .iter()
        .any(|event| matches!(event, Event::Completed { .. }))
}

fn saw_text(events: &[Event]) -> bool {
    events.iter().any(|event| match event {
        Event::BlockStart { block } => matches!(block.kind, BlockKind::Text),
        Event::BlockDelta { delta, .. } => matches!(delta, provider::BlockDelta::Text { .. }),
        _ => false,
    })
}

fn saw_reasoning(events: &[Event]) -> bool {
    events.iter().any(|event| match event {
        Event::BlockStart { block } => matches!(block.kind, BlockKind::Reasoning),
        Event::BlockDelta { delta, .. } => match delta {
            provider::BlockDelta::Reasoning { .. } => true,
            provider::BlockDelta::Text { text } => {
                text.contains("<think>") || text.contains("</think>")
            }
            _ => false,
        },
        _ => false,
    })
}

#[tokio::test]
#[ignore = "downloads local GGUF models and depends on host capabilities"]
async fn smoke_qwen35_0_8b_streams_events() {
    let mut request = Request::user_text("Say just the word 'hello' and nothing else.");
    request.options.max_output_tokens = Some(8);
    let events = collect_events(request).await;

    assert!(saw_text(&events));
    assert!(saw_completed(&events));
}

#[tokio::test]
#[ignore = "downloads local GGUF models and depends on host capabilities"]
async fn smoke_qwen35_0_8b_streams_multimodal_events() {
    let request = Request {
        messages: vec![Message::new(
            MessageRole::User,
            vec![
                ContentBlock::text("Describe this image in a few words."),
                ContentBlock::image_url(TINY_PNG_DATA_URL),
            ],
        )],
        options: provider::RequestOptions {
            max_output_tokens: Some(16),
            ..Default::default()
        },
        ..Default::default()
    };
    let events = collect_events(request).await;

    assert!(saw_completed(&events));
}

#[tokio::test]
#[ignore = "downloads local GGUF models and depends on host capabilities"]
async fn smoke_qwen35_0_8b_streams_reasoning_events() {
    let mut request = Request::user_text("Think briefly, then answer with hello.");
    request.options.max_output_tokens = Some(64);
    request.options.reasoning = Some(ReasoningConfig::default());
    let events = collect_events(request).await;

    assert!(saw_reasoning(&events));
    assert!(saw_completed(&events));
}
