#![cfg(feature = "mistralrs")]

use futures::StreamExt;
use provider::{BlockKind, Event, Provider as _, ReasoningConfig, Request};
use provider_mistralrs::{MistralRsModelPreset, MistralRsProvider};
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::Mutex;

fn mistral_smoke_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

async fn collect_events(request: Request) -> Vec<Event> {
    let _guard = mistral_smoke_lock().lock().await;
    tokio::time::timeout(Duration::from_secs(120), async move {
        let preset = MistralRsModelPreset::QWEN3_0_6B;
        let preset_id = preset.id().to_owned();
        let provider =
            MistralRsProvider::new(vec![(preset_id.clone(), preset.into_config())], &preset_id);
        provider.preload(&preset_id).await.unwrap();

        let mut stream = provider.stream(&request).await.unwrap();
        let mut events = Vec::new();
        while let Some(event) = stream.next().await {
            events.push(event.unwrap());
        }
        events
    })
    .await
    .expect("local mistral.rs smoke timed out")
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
async fn smoke_qwen3_0_6b_streams_events() {
    let mut request = Request::user_text("Say just the word 'hello' and nothing else.");
    request.options.max_output_tokens = Some(8);
    let events = collect_events(request).await;

    assert!(
        saw_text(&events) || saw_reasoning(&events),
        "events: {events:#?}"
    );
    assert!(saw_completed(&events));
}

#[tokio::test]
#[ignore = "downloads local GGUF models and depends on host capabilities"]
async fn smoke_qwen3_0_6b_streams_reasoning_events() {
    let mut request = Request::user_text("Think briefly, then answer with hello.");
    request.options.max_output_tokens = Some(8);
    request.options.reasoning = Some(ReasoningConfig::default());
    let events = collect_events(request).await;

    assert!(saw_reasoning(&events), "events: {events:#?}");
    assert!(saw_completed(&events));
}
