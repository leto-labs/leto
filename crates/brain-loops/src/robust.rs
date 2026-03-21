use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::CancellationToken;
use ulid::Ulid;

use brain_types::*;

use crate::output::truncate_tool_result;

pub struct RobustLoop;

impl AgentLoop for RobustLoop {
    fn run(
        &self,
        provider: Arc<dyn Provider>,
        tools: Vec<Arc<dyn Tool>>,
        messages: Vec<Message>,
        config: AgentConfig,
        cancel: CancellationToken,
        session_id: Option<Ulid>,
    ) -> EventStream {
        let (tx, rx) = mpsc::channel(256);

        tokio::spawn(async move {
            if let Err(error) = run_inner(
                provider,
                tools,
                messages,
                config,
                cancel,
                session_id,
                tx.clone(),
            )
            .await
            {
                let _ = tx
                    .send(Event::Error {
                        code: error.code(),
                        message: error.to_string(),
                        recoverable: error.recoverable(),
                    })
                    .await;
            }
        });

        Box::pin(ReceiverStream::new(rx))
    }
}

struct ProviderTurn {
    text: String,
    tool_calls: Vec<ToolCall>,
    total_tokens: u32,
}

async fn run_inner(
    provider: Arc<dyn Provider>,
    tools: Vec<Arc<dyn Tool>>,
    mut messages: Vec<Message>,
    config: AgentConfig,
    cancel: CancellationToken,
    session_id: Option<Ulid>,
    tx: mpsc::Sender<Event>,
) -> Result<(), BrainError> {
    if let Some(ref prompt) = config.system_prompt {
        messages.insert(0, Message::system(prompt));
    }

    let tool_defs: Vec<ToolDef> = tools.iter().map(|tool| tool.definition()).collect();

    let mut iterations = 0u32;
    let mut total_tokens = 0u32;
    let mut doom_tracker = DoomTracker::default();

    loop {
        if cancel.is_cancelled() {
            return Err(BrainError::Cancelled);
        }
        if iterations >= config.max_iterations {
            return Err(BrainError::MaxIterations(iterations));
        }

        maybe_compact_history(
            provider.as_ref(),
            &mut messages,
            &config,
            session_id,
            &tx,
            &cancel,
        )
        .await?;

        iterations += 1;

        let turn = run_provider_turn(
            provider.as_ref(),
            &messages,
            &tool_defs,
            &config.inference,
            config.max_retries,
            session_id,
            &tx,
            &cancel,
        )
        .await?;
        total_tokens += turn.total_tokens;

        let mut assistant = Message::assistant(&turn.text);
        assistant.tool_calls = turn.tool_calls.clone();
        let _ = tx
            .send(Event::MessageDone {
                message: assistant.clone(),
            })
            .await;
        messages.push(assistant);

        if turn.tool_calls.is_empty() {
            break;
        }

        for call in &turn.tool_calls {
            emit_tool_lifecycle(call, &tx).await;

            let tool = tools
                .iter()
                .find(|tool| tool.definition().name == call.name);
            let (result, is_error) = match tool {
                Some(tool) => match tool.execute(call.arguments.clone()).await {
                    Ok(result) => (result, false),
                    Err(error) => (error.to_string(), true),
                },
                None => (format!("tool not found: {}", call.name), true),
            };
            let result = truncate_tool_result(&result, &config);

            let _ = tx
                .send(Event::ToolCallDone {
                    id: call.id.clone(),
                    result: result.clone(),
                    is_error,
                })
                .await;

            messages.push(Message::tool_result(&call.id, &result));

            let repetitions = doom_tracker.observe(call);
            let threshold = config.doom_loop_threshold.max(1);
            if repetitions >= threshold {
                let _ = tx
                    .send(Event::DoomLoopWarning {
                        tool_name: call.name.clone(),
                        repetitions,
                    })
                    .await;

                match config.doom_loop_strategy {
                    DoomLoopStrategy::Error => {
                        return Err(BrainError::Internal(format!(
                            "doom loop detected for tool '{}' after {repetitions} repetitions",
                            call.name
                        )));
                    }
                    DoomLoopStrategy::Steer => {
                        if repetitions > threshold {
                            return Err(BrainError::Internal(format!(
                                "doom loop persisted for tool '{}' after steering",
                                call.name
                            )));
                        }

                        messages.push(Message::system(format!(
                            "You are repeatedly calling `{}` with the same arguments. Stop repeating the same tool call. Either choose a different tool, refine the arguments, or explain why no further progress can be made.",
                            call.name
                        )));
                    }
                }
            }
        }
    }

    let _ = tx
        .send(Event::TurnDone {
            iterations,
            total_tokens,
        })
        .await;
    Ok(())
}

async fn emit_tool_lifecycle(call: &ToolCall, tx: &mpsc::Sender<Event>) {
    let _ = tx
        .send(Event::ToolCallPending {
            id: call.id.clone(),
            name: call.name.clone(),
            arguments: call.arguments.clone(),
        })
        .await;
    let _ = tx
        .send(Event::ToolCallApproved {
            id: call.id.clone(),
        })
        .await;
    let _ = tx
        .send(Event::ToolCallStart {
            id: call.id.clone(),
            name: call.name.clone(),
            arguments: call.arguments.clone(),
        })
        .await;
}

async fn run_provider_turn(
    provider: &dyn Provider,
    messages: &[Message],
    tool_defs: &[ToolDef],
    inference: &InferenceConfig,
    max_retries: u32,
    session_id: Option<Ulid>,
    tx: &mpsc::Sender<Event>,
    cancel: &CancellationToken,
) -> Result<ProviderTurn, BrainError> {
    let max_attempts = max_retries.saturating_add(1);

    'attempt: for attempt in 1..=max_attempts {
        if cancel.is_cancelled() {
            return Err(BrainError::Cancelled);
        }

        let mut stream = match provider
            .chat(messages, tool_defs, inference, session_id)
            .await
        {
            Ok(stream) => stream,
            Err(error) if attempt < max_attempts && is_transient_error(&error) => {
                emit_retry(attempt, max_attempts, &error, tx).await;
                sleep(backoff_duration(attempt)).await;
                continue;
            }
            Err(error) => return Err(error),
        };

        let mut text = String::new();
        let mut tool_calls = Vec::new();
        let mut total_tokens = 0u32;
        let mut saw_output = false;
        let mut tool_call_deltas: HashMap<String, String> = HashMap::new();

        while let Some(chunk) = stream.next().await {
            if cancel.is_cancelled() {
                return Err(BrainError::Cancelled);
            }

            match chunk {
                Ok(ChatChunk::Delta { content }) => {
                    saw_output = true;
                    text.push_str(&content);
                    let _ = tx.send(Event::Token { delta: content }).await;
                }
                Ok(ChatChunk::ToolCallDelta {
                    id,
                    name,
                    arguments_delta,
                }) => {
                    saw_output = true;
                    tool_call_deltas
                        .entry(id.clone())
                        .or_default()
                        .push_str(&arguments_delta);
                    let _ = tx
                        .send(Event::ToolCallDelta {
                            id,
                            name,
                            arguments_delta,
                        })
                        .await;
                }
                Ok(ChatChunk::ToolCall {
                    id,
                    name,
                    arguments,
                }) => {
                    saw_output = true;
                    let _ = tool_call_deltas.remove(&id);
                    tool_calls.push(ToolCall {
                        id,
                        name,
                        arguments,
                    });
                }
                Ok(ChatChunk::Done { usage }) => {
                    saw_output = true;
                    if let Some(usage) = usage {
                        total_tokens += usage.total;
                    }
                }
                Ok(_) => {
                    saw_output = true;
                }
                Err(error)
                    if !saw_output && attempt < max_attempts && is_transient_error(&error) =>
                {
                    emit_retry(attempt, max_attempts, &error, tx).await;
                    sleep(backoff_duration(attempt)).await;
                    continue 'attempt;
                }
                Err(error) => return Err(error),
            }
        }

        return Ok(ProviderTurn {
            text,
            tool_calls,
            total_tokens,
        });
    }

    Err(BrainError::Internal(
        "provider retry loop exhausted unexpectedly".into(),
    ))
}

async fn emit_retry(attempt: u32, max_attempts: u32, error: &BrainError, tx: &mpsc::Sender<Event>) {
    let _ = tx
        .send(Event::Retry {
            attempt,
            max: max_attempts,
            error: error.to_string(),
        })
        .await;
}

fn backoff_duration(attempt: u32) -> Duration {
    let capped_attempt = attempt.min(6);
    let base_ms = 100u64.saturating_mul(1u64 << (capped_attempt.saturating_sub(1)));
    let jitter_ms = ((attempt as u64 * 37) % 53) + 7;
    Duration::from_millis(base_ms.saturating_add(jitter_ms))
}

fn is_transient_error(error: &BrainError) -> bool {
    match error {
        BrainError::Inference(message) | BrainError::Internal(message) => {
            let message = message.to_ascii_lowercase();
            [
                "429",
                "500",
                "502",
                "503",
                "504",
                "rate limit",
                "timeout",
                "timed out",
                "connection reset",
                "temporarily unavailable",
                "network",
            ]
            .iter()
            .any(|needle| message.contains(needle))
        }
        BrainError::Cancelled => true,
        BrainError::Auth(_) => false,
        BrainError::ToolFailed { .. } => false,
        BrainError::ToolNotFound(_) => false,
        BrainError::MaxIterations(_) => false,
        BrainError::TurnActive(_) => false,
        BrainError::Storage(_) => false,
        BrainError::Json(_) => false,
    }
}

async fn maybe_compact_history(
    provider: &dyn Provider,
    messages: &mut Vec<Message>,
    config: &AgentConfig,
    session_id: Option<Ulid>,
    tx: &mpsc::Sender<Event>,
    cancel: &CancellationToken,
) -> Result<(), BrainError> {
    let Some(threshold) = config.compaction_threshold else {
        return Ok(());
    };
    let Some(limit) = current_context_limit(provider, config.inference.model.as_deref()) else {
        return Ok(());
    };

    let estimated_tokens = messages.iter().map(estimate_message_tokens).sum::<usize>() as f32;
    if estimated_tokens <= (limit as f32 * threshold) {
        return Ok(());
    }

    if cancel.is_cancelled() {
        return Err(BrainError::Cancelled);
    }

    let leading_systems = messages
        .iter()
        .take_while(|message| matches!(message.role, Role::System))
        .count();
    let trailing = 6usize.min(messages.len().saturating_sub(leading_systems));
    if messages.len() <= leading_systems + trailing + 1 {
        return Ok(());
    }

    let compact_end = messages.len() - trailing;
    let to_compact = messages[leading_systems..compact_end].to_vec();
    if to_compact.is_empty() {
        return Ok(());
    }

    let summary = summarize_messages(provider, &to_compact, config, session_id, cancel).await?;
    let summary_tokens = estimate_text_tokens(&summary);

    let mut compacted = Vec::with_capacity(leading_systems + trailing + 1);
    compacted.extend(messages[..leading_systems].iter().cloned());
    compacted.push(Message::system(format!("Conversation summary:\n{summary}")));
    compacted.extend(messages[compact_end..].iter().cloned());
    *messages = compacted;

    let _ = tx
        .send(Event::Compaction {
            original_messages: to_compact.len(),
            summary_tokens,
        })
        .await;
    Ok(())
}

async fn summarize_messages(
    provider: &dyn Provider,
    messages: &[Message],
    config: &AgentConfig,
    session_id: Option<Ulid>,
    cancel: &CancellationToken,
) -> Result<String, BrainError> {
    let mut inference = config.inference.clone();
    if let Some(model) = config.compaction_model.clone() {
        inference.model = Some(model);
    }

    let transcript = messages
        .iter()
        .map(render_message_for_summary)
        .collect::<Vec<_>>()
        .join("\n");

    let prompt_messages = vec![
        Message::system(
            "Summarize the following conversation history for future continuation. Preserve goals, files touched, commands attempted, tool outcomes, and unresolved next steps. Keep it concise and factual.",
        ),
        Message::user(transcript),
    ];

    let mut stream = provider
        .chat(&prompt_messages, &[], &inference, session_id)
        .await?;
    let mut summary = String::new();

    while let Some(chunk) = stream.next().await {
        if cancel.is_cancelled() {
            return Err(BrainError::Cancelled);
        }

        match chunk? {
            ChatChunk::Delta { content } => summary.push_str(&content),
            ChatChunk::Done { .. } => {}
            _ => {}
        }
    }

    if summary.trim().is_empty() {
        return Err(BrainError::Inference(
            "compaction summary generation returned empty output".into(),
        ));
    }

    Ok(summary.trim().to_owned())
}

fn render_message_for_summary(message: &Message) -> String {
    let role = match message.role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool => "tool",
    };

    if message.tool_calls.is_empty() {
        format!("{role}: {}", message.content)
    } else {
        let tool_calls = serde_json::to_string(&message.tool_calls).unwrap_or_default();
        format!("{role}: {}\ntool_calls: {tool_calls}", message.content)
    }
}

fn current_context_limit(provider: &dyn Provider, model_id: Option<&str>) -> Option<u64> {
    let info = provider.info();
    let model_id = model_id.or(info.default_model.as_deref())?;
    info.models
        .iter()
        .find(|model| model.id == model_id)
        .and_then(|model| model.limit.map(|limit| limit.context))
}

fn estimate_message_tokens(message: &Message) -> usize {
    let mut content = message.content.clone();
    if !message.tool_calls.is_empty() {
        let tool_calls = serde_json::to_string(&message.tool_calls).unwrap_or_default();
        content.push_str(&tool_calls);
    }
    estimate_text_tokens(&content)
}

fn estimate_text_tokens(text: &str) -> usize {
    (text.chars().count() / 4).max(1)
}

#[derive(Default)]
struct DoomTracker {
    recent: VecDeque<String>,
}

impl DoomTracker {
    fn observe(&mut self, call: &ToolCall) -> u32 {
        let signature = format!(
            "{}:{}",
            call.name,
            serde_json::to_string(&call.arguments).unwrap_or_default()
        );
        self.recent.push_back(signature.clone());
        while self.recent.len() > 8 {
            self.recent.pop_front();
        }

        self.recent
            .iter()
            .rev()
            .take_while(|entry| *entry == &signature)
            .count() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_stream::stream;
    use futures::future::BoxFuture;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    struct RetryProvider {
        attempts: AtomicUsize,
    }

    impl RetryProvider {
        fn new() -> Self {
            Self {
                attempts: AtomicUsize::new(0),
            }
        }
    }

    impl Provider for RetryProvider {
        fn chat<'a>(
            &'a self,
            _messages: &'a [Message],
            _tools: &'a [ToolDef],
            _config: &'a InferenceConfig,
            _session_id: Option<Ulid>,
        ) -> BoxFuture<'a, Result<ChatStream, BrainError>> {
            Box::pin(async move {
                let attempt = self.attempts.fetch_add(1, Ordering::SeqCst);
                if attempt == 0 {
                    return Err(BrainError::Inference("429 rate limit".into()));
                }

                let s = stream! {
                    yield Ok(ChatChunk::Delta { content: "done".into() });
                    yield Ok(ChatChunk::Done {
                        usage: Some(TokenUsage {
                            prompt: 3,
                            completion: 1,
                            total: 4,
                        }),
                    });
                };
                Ok(Box::pin(s) as ChatStream)
            })
        }

        fn info(&self) -> ProviderInfo {
            ProviderInfo {
                name: "retry".into(),
                default_model: Some("retry-model".into()),
                models: vec![ModelInfo {
                    id: "retry-model",
                    name: "Retry Model",
                    family: None,
                    reasoning: None,
                    tool_call: true,
                    attachment: false,
                    structured_output: None,
                    temperature: None,
                    knowledge: None,
                    release_date: None,
                    last_updated: None,
                    open_weights: None,
                    input_modalities: &["text"],
                    output_modalities: &["text"],
                    cost: None,
                    limit: Some(ModelLimit {
                        context: 4096,
                        input: None,
                        output: 1024,
                    }),
                    status: None,
                }],
            }
        }
    }

    struct RepeatingToolProvider;

    impl Provider for RepeatingToolProvider {
        fn chat<'a>(
            &'a self,
            _messages: &'a [Message],
            _tools: &'a [ToolDef],
            _config: &'a InferenceConfig,
            _session_id: Option<Ulid>,
        ) -> BoxFuture<'a, Result<ChatStream, BrainError>> {
            Box::pin(async move {
                let s = stream! {
                    yield Ok(ChatChunk::ToolCall {
                        id: "repeat-call".into(),
                        name: "dummy".into(),
                        arguments: serde_json::json!({ "path": "src/lib.rs" }),
                    });
                    yield Ok(ChatChunk::Done { usage: None });
                };
                Ok(Box::pin(s) as ChatStream)
            })
        }

        fn info(&self) -> ProviderInfo {
            ProviderInfo {
                name: "repeat".into(),
                default_model: Some("repeat-model".into()),
                models: vec![],
            }
        }
    }

    struct SummaryProvider {
        summary_called: AtomicBool,
    }

    impl SummaryProvider {
        fn new() -> Self {
            Self {
                summary_called: AtomicBool::new(false),
            }
        }
    }

    impl Provider for SummaryProvider {
        fn chat<'a>(
            &'a self,
            messages: &'a [Message],
            _tools: &'a [ToolDef],
            _config: &'a InferenceConfig,
            _session_id: Option<Ulid>,
        ) -> BoxFuture<'a, Result<ChatStream, BrainError>> {
            Box::pin(async move {
                let is_summary = messages.first().is_some_and(|message| {
                    message
                        .content
                        .contains("Summarize the following conversation history")
                });
                if is_summary {
                    self.summary_called.store(true, Ordering::SeqCst);
                }

                let text = if is_summary { "summary" } else { "ok" };
                let s = stream! {
                    yield Ok(ChatChunk::Delta { content: text.into() });
                    yield Ok(ChatChunk::Done {
                        usage: Some(TokenUsage {
                            prompt: 20,
                            completion: 5,
                            total: 25,
                        }),
                    });
                };
                Ok(Box::pin(s) as ChatStream)
            })
        }

        fn info(&self) -> ProviderInfo {
            ProviderInfo {
                name: "summary".into(),
                default_model: Some("compact-model".into()),
                models: vec![ModelInfo {
                    id: "compact-model",
                    name: "Compact Model",
                    family: None,
                    reasoning: None,
                    tool_call: true,
                    attachment: false,
                    structured_output: None,
                    temperature: None,
                    knowledge: None,
                    release_date: None,
                    last_updated: None,
                    open_weights: None,
                    input_modalities: &["text"],
                    output_modalities: &["text"],
                    cost: None,
                    limit: Some(ModelLimit {
                        context: 20,
                        input: None,
                        output: 10,
                    }),
                    status: None,
                }],
            }
        }
    }

    struct DummyTool;

    impl Tool for DummyTool {
        fn definition(&self) -> ToolDef {
            ToolDef {
                name: "dummy".into(),
                description: "dummy".into(),
                parameters: serde_json::json!({}),
            }
        }

        fn execute(&self, _args: serde_json::Value) -> BoxFuture<'_, Result<String, BrainError>> {
            Box::pin(async { Ok("ok".into()) })
        }
    }

    #[tokio::test]
    async fn retries_transient_provider_failures_before_output() {
        let provider: Arc<dyn Provider> = Arc::new(RetryProvider::new());
        let loop_impl = RobustLoop;
        let config = AgentConfig {
            max_retries: 2,
            ..AgentConfig::default()
        };

        let events: Vec<Event> = loop_impl
            .run(
                provider,
                vec![],
                vec![Message::user("hello")],
                config,
                CancellationToken::new(),
                None,
            )
            .collect()
            .await;

        assert!(
            events
                .iter()
                .any(|event| matches!(event, Event::Retry { .. }))
        );
        assert!(
            events
                .iter()
                .any(|event| matches!(event, Event::TurnDone { .. }))
        );
        assert!(
            events
                .iter()
                .any(|event| matches!(event, Event::Token { delta } if delta == "done"))
        );
    }

    #[tokio::test]
    async fn doom_loop_detection_can_error_immediately() {
        let provider: Arc<dyn Provider> = Arc::new(RepeatingToolProvider);
        let loop_impl = RobustLoop;
        let config = AgentConfig {
            doom_loop_threshold: 2,
            doom_loop_strategy: DoomLoopStrategy::Error,
            max_iterations: 10,
            ..AgentConfig::default()
        };

        let events: Vec<Event> = loop_impl
            .run(
                provider,
                vec![Arc::new(DummyTool)],
                vec![Message::user("repeat")],
                config,
                CancellationToken::new(),
                None,
            )
            .collect()
            .await;

        assert!(events.iter().any(
            |event| matches!(event, Event::DoomLoopWarning { repetitions, .. } if *repetitions == 2)
        ));
        assert!(
            events
                .iter()
                .any(|event| matches!(event, Event::Error { .. }))
        );
    }

    #[tokio::test]
    async fn compaction_runs_before_large_turns() {
        let provider = Arc::new(SummaryProvider::new());
        let loop_impl = RobustLoop;
        let config = AgentConfig {
            compaction_threshold: Some(0.2),
            ..AgentConfig::default()
        };
        let mut messages = vec![Message::system("System prompt")];
        for idx in 0..8 {
            messages.push(Message::user(format!(
                "long user message number {idx} with a lot of repeated words words words words"
            )));
            messages.push(Message::assistant(format!(
                "assistant reply number {idx} with more repeated words words words words"
            )));
        }

        let events: Vec<Event> = loop_impl
            .run(
                provider.clone(),
                vec![],
                messages,
                config,
                CancellationToken::new(),
                None,
            )
            .collect()
            .await;

        assert!(provider.summary_called.load(Ordering::SeqCst));
        assert!(
            events
                .iter()
                .any(|event| matches!(event, Event::Compaction { .. }))
        );
        assert!(
            events
                .iter()
                .any(|event| matches!(event, Event::TurnDone { .. }))
        );
    }
}
