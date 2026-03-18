use std::collections::HashMap;
use std::sync::Arc;

use futures::StreamExt;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::CancellationToken;
use ulid::Ulid;

use brain_types::*;

pub struct SimpleLoop;

impl AgentLoop for SimpleLoop {
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
            if let Err(e) = run_inner(
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
                        code: e.code(),
                        message: e.to_string(),
                        recoverable: e.recoverable(),
                    })
                    .await;
            }
        });

        Box::pin(ReceiverStream::new(rx))
    }
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

    let tool_defs: Vec<ToolDef> = tools.iter().map(|t| t.definition()).collect();

    let mut iterations = 0u32;
    let mut total_tokens = 0u32;

    loop {
        if cancel.is_cancelled() {
            return Err(BrainError::Cancelled);
        }
        if iterations >= config.max_iterations {
            return Err(BrainError::MaxIterations(iterations));
        }
        iterations += 1;

        let mut stream = provider
            .chat(&messages, &tool_defs, &config.inference, session_id)
            .await?;

        let mut text = String::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();
        let mut tool_call_deltas: HashMap<String, String> = HashMap::new();

        while let Some(chunk) = stream.next().await {
            if cancel.is_cancelled() {
                return Err(BrainError::Cancelled);
            }
            match chunk? {
                ChatChunk::Delta { content } => {
                    text.push_str(&content);
                    let _ = tx.send(Event::Token { delta: content }).await;
                }
                ChatChunk::ToolCallDelta {
                    id,
                    name,
                    arguments_delta,
                } => {
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
                ChatChunk::ToolCall {
                    id,
                    name,
                    arguments,
                } => {
                    let _ = tool_call_deltas.remove(&id);
                    tool_calls.push(ToolCall {
                        id,
                        name,
                        arguments,
                    });
                }
                ChatChunk::Done { usage } => {
                    if let Some(u) = usage {
                        total_tokens += u.total;
                    }
                }
                _ => {}
            }
        }

        let mut asst = Message::assistant(&text);
        asst.tool_calls = tool_calls.clone();
        let _ = tx
            .send(Event::MessageDone {
                message: asst.clone(),
            })
            .await;
        messages.push(asst);

        if tool_calls.is_empty() {
            break;
        }

        for call in &tool_calls {
            let _ = tx
                .send(Event::ToolCallPending {
                    id: call.id.clone(),
                    name: call.name.clone(),
                    arguments: call.arguments.clone(),
                })
                .await;

            // Auto-approve (no approval gate configured yet)
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

            let tool = tools.iter().find(|t| t.definition().name == call.name);

            let (result, is_error) = match tool {
                Some(t) => match t.execute(call.arguments.clone()).await {
                    Ok(r) => (r, false),
                    Err(e) => (e.to_string(), true),
                },
                None => (format!("tool not found: {}", call.name), true),
            };

            let _ = tx
                .send(Event::ToolCallDone {
                    id: call.id.clone(),
                    result: result.clone(),
                    is_error,
                })
                .await;

            messages.push(Message::tool_result(&call.id, &result));
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

#[cfg(test)]
mod tests {
    use super::*;
    use async_stream::stream;
    use futures::future::BoxFuture;

    struct EchoProvider;

    impl Provider for EchoProvider {
        fn info(&self) -> ProviderInfo {
            ProviderInfo {
                name: "echo".into(),
                default_model: None,
                models: vec![],
            }
        }

        fn chat<'a>(
            &'a self,
            messages: &'a [Message],
            _tools: &'a [ToolDef],
            _config: &'a InferenceConfig,
            _session_id: Option<Ulid>,
        ) -> BoxFuture<'a, Result<ChatStream, BrainError>> {
            let last = messages
                .iter()
                .rev()
                .find(|m| m.role == Role::User)
                .map(|m| m.content.clone())
                .unwrap_or_default();
            Box::pin(async move {
                let s = stream! {
                    yield Ok(ChatChunk::Delta { content: last });
                    yield Ok(ChatChunk::Done { usage: Some(TokenUsage { prompt: 10, completion: 5, total: 15 }) });
                };
                Ok(Box::pin(s) as ChatStream)
            })
        }
    }

    struct ToolCallProvider {
        remaining: std::sync::atomic::AtomicU32,
    }

    struct ToolCallDeltaProvider {
        yielded: std::sync::atomic::AtomicBool,
    }

    impl ToolCallDeltaProvider {
        fn new() -> Self {
            Self {
                yielded: std::sync::atomic::AtomicBool::new(false),
            }
        }
    }

    impl Provider for ToolCallDeltaProvider {
        fn info(&self) -> ProviderInfo {
            ProviderInfo {
                name: "tool-call-delta".into(),
                default_model: None,
                models: vec![],
            }
        }

        fn chat<'a>(
            &'a self,
            _messages: &'a [Message],
            _tools: &'a [ToolDef],
            _config: &'a InferenceConfig,
            _session_id: Option<Ulid>,
        ) -> BoxFuture<'a, Result<ChatStream, BrainError>> {
            let yielded = self.yielded.swap(true, std::sync::atomic::Ordering::AcqRel);
            Box::pin(async move {
                if yielded {
                    let s = stream! {
                        yield Ok(ChatChunk::Delta { content: "done".into() });
                        yield Ok(ChatChunk::Done { usage: None });
                    };
                    Ok(Box::pin(s) as ChatStream)
                } else {
                    let s = stream! {
                        yield Ok(ChatChunk::ToolCallDelta {
                            id: "d1".into(),
                            name: "echo".into(),
                            arguments_delta: "{\"message\":\"hello\"}".into(),
                        });
                        yield Ok(ChatChunk::ToolCall {
                            id: "d1".into(),
                            name: "echo".into(),
                            arguments: serde_json::json!({"message":"hello"}),
                        });
                        yield Ok(ChatChunk::Done { usage: None });
                    };
                    Ok(Box::pin(s) as ChatStream)
                }
            })
        }
    }

    impl ToolCallProvider {
        fn new(tool_calls: u32) -> Self {
            Self {
                remaining: std::sync::atomic::AtomicU32::new(tool_calls),
            }
        }
    }

    impl Provider for ToolCallProvider {
        fn info(&self) -> ProviderInfo {
            ProviderInfo {
                name: "tool-call".into(),
                default_model: None,
                models: vec![],
            }
        }

        fn chat<'a>(
            &'a self,
            _messages: &'a [Message],
            _tools: &'a [ToolDef],
            _config: &'a InferenceConfig,
            _session_id: Option<Ulid>,
        ) -> BoxFuture<'a, Result<ChatStream, BrainError>> {
            let remaining = self
                .remaining
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            Box::pin(async move {
                if remaining > 0 {
                    let s = stream! {
                        yield Ok(ChatChunk::ToolCall {
                            id: format!("call-{remaining}"),
                            name: "echo".into(),
                            arguments: serde_json::json!({"message": "test"}),
                        });
                        yield Ok(ChatChunk::Done { usage: None });
                    };
                    Ok(Box::pin(s) as ChatStream)
                } else {
                    let s = stream! {
                        yield Ok(ChatChunk::Delta { content: "done".into() });
                        yield Ok(ChatChunk::Done { usage: None });
                    };
                    Ok(Box::pin(s) as ChatStream)
                }
            })
        }
    }

    struct EchoTool;

    impl Tool for EchoTool {
        fn definition(&self) -> ToolDef {
            ToolDef {
                name: "echo".into(),
                description: "echoes input".into(),
                parameters: serde_json::json!({}),
            }
        }

        fn execute(&self, args: serde_json::Value) -> BoxFuture<'_, Result<String, BrainError>> {
            Box::pin(async move {
                Ok(args
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string())
            })
        }
    }

    fn default_config() -> AgentConfig {
        AgentConfig {
            max_iterations: 10,
            system_prompt: None,
            inference: InferenceConfig::default(),
        }
    }

    #[tokio::test]
    async fn simple_echo_turn() {
        let agent_loop = SimpleLoop;
        let provider: Arc<dyn Provider> = Arc::new(EchoProvider);
        let messages = vec![Message::user("hello")];

        let mut stream = agent_loop.run(
            provider,
            vec![],
            messages,
            default_config(),
            CancellationToken::new(),
            None,
        );

        let mut tokens = String::new();
        let mut saw_message_done = false;
        let mut saw_turn_done = false;

        while let Some(event) = stream.next().await {
            match event {
                Event::Token { delta } => tokens.push_str(&delta),
                Event::MessageDone { message } => {
                    assert_eq!(message.role, Role::Assistant);
                    saw_message_done = true;
                }
                Event::TurnDone { iterations, .. } => {
                    assert_eq!(iterations, 1);
                    saw_turn_done = true;
                }
                _ => {}
            }
        }

        assert_eq!(tokens, "hello");
        assert!(saw_message_done);
        assert!(saw_turn_done);
    }

    #[tokio::test]
    async fn system_prompt_prepended() {
        struct CaptureProvider;

        impl Provider for CaptureProvider {
            fn info(&self) -> ProviderInfo {
                ProviderInfo {
                    name: "capture".into(),
                    default_model: None,
                    models: vec![],
                }
            }

            fn chat<'a>(
                &'a self,
                messages: &'a [Message],
                _tools: &'a [ToolDef],
                _config: &'a InferenceConfig,
                _session_id: Option<Ulid>,
            ) -> BoxFuture<'a, Result<ChatStream, BrainError>> {
                assert_eq!(messages[0].role, Role::System);
                assert_eq!(messages[0].content, "Be helpful");
                Box::pin(async move {
                    let s = stream! {
                        yield Ok(ChatChunk::Delta { content: "ok".into() });
                        yield Ok(ChatChunk::Done { usage: None });
                    };
                    Ok(Box::pin(s) as ChatStream)
                })
            }
        }

        let config = AgentConfig {
            max_iterations: 5,
            system_prompt: Some("Be helpful".into()),
            inference: InferenceConfig::default(),
        };

        let mut stream = SimpleLoop.run(
            Arc::new(CaptureProvider),
            vec![],
            vec![Message::user("hi")],
            config,
            CancellationToken::new(),
            None,
        );

        while stream.next().await.is_some() {}
    }

    #[tokio::test]
    async fn tool_call_loop() {
        let provider: Arc<dyn Provider> = Arc::new(ToolCallProvider::new(1));
        let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(EchoTool)];

        let mut stream = SimpleLoop.run(
            provider,
            tools,
            vec![Message::user("hello")],
            default_config(),
            CancellationToken::new(),
            None,
        );

        let mut tool_starts = 0;
        let mut tool_dones = 0;
        let mut turn_iterations = 0;

        while let Some(event) = stream.next().await {
            match event {
                Event::ToolCallStart { .. } => tool_starts += 1,
                Event::ToolCallDone { is_error, .. } => {
                    assert!(!is_error);
                    tool_dones += 1;
                }
                Event::TurnDone { iterations, .. } => turn_iterations = iterations,
                _ => {}
            }
        }

        assert_eq!(tool_starts, 1);
        assert_eq!(tool_dones, 1);
        assert_eq!(turn_iterations, 2);
    }

    #[tokio::test]
    async fn tool_call_deltas_emitted_before_final_call() {
        let provider: Arc<dyn Provider> = Arc::new(ToolCallDeltaProvider::new());
        let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(EchoTool)];

        let mut stream = SimpleLoop.run(
            provider,
            tools,
            vec![Message::user("hello")],
            default_config(),
            CancellationToken::new(),
            None,
        );

        let mut saw_delta = false;
        let mut saw_pending = false;
        let mut saw_done = false;

        while let Some(event) = stream.next().await {
            match event {
                Event::ToolCallDelta { .. } => {
                    saw_delta = true;
                }
                Event::ToolCallPending { .. } => {
                    assert!(
                        saw_delta,
                        "ToolCallPending should arrive after delta stream"
                    );
                    saw_pending = true;
                }
                Event::ToolCallDone { is_error, .. } => {
                    assert!(!is_error);
                    saw_done = true;
                }
                _ => {}
            }
        }

        assert!(saw_delta, "delta events should be emitted");
        assert!(saw_pending, "pending event should be emitted");
        assert!(saw_done, "tool should be executed");
    }

    #[tokio::test]
    async fn max_iterations_error() {
        let provider: Arc<dyn Provider> = Arc::new(ToolCallProvider::new(100));
        let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(EchoTool)];

        let config = AgentConfig {
            max_iterations: 2,
            system_prompt: None,
            inference: InferenceConfig::default(),
        };

        let mut stream = SimpleLoop.run(
            provider,
            tools,
            vec![Message::user("hello")],
            config,
            CancellationToken::new(),
            None,
        );

        let mut saw_error = false;
        while let Some(event) = stream.next().await {
            if let Event::Error { code, .. } = event {
                assert_eq!(code, BrainErrorCode::MaxIterations);
                saw_error = true;
            }
        }
        assert!(saw_error);
    }

    #[tokio::test]
    async fn cancellation_stops_loop() {
        let provider: Arc<dyn Provider> = Arc::new(ToolCallProvider::new(100));
        let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(EchoTool)];
        let cancel = CancellationToken::new();
        cancel.cancel();

        let mut stream = SimpleLoop.run(
            provider,
            tools,
            vec![Message::user("hello")],
            default_config(),
            cancel,
            None,
        );

        let mut saw_cancelled = false;
        while let Some(event) = stream.next().await {
            if let Event::Error { code, .. } = event {
                assert_eq!(code, BrainErrorCode::Cancelled);
                saw_cancelled = true;
            }
        }
        assert!(saw_cancelled);
    }

    #[tokio::test]
    async fn missing_tool_returns_error_result() {
        struct OneCallProvider;

        impl Provider for OneCallProvider {
            fn info(&self) -> ProviderInfo {
                ProviderInfo {
                    name: "one-call".into(),
                    default_model: None,
                    models: vec![],
                }
            }

            fn chat<'a>(
                &'a self,
                messages: &'a [Message],
                _tools: &'a [ToolDef],
                _config: &'a InferenceConfig,
                _session_id: Option<Ulid>,
            ) -> BoxFuture<'a, Result<ChatStream, BrainError>> {
                let has_tool_result = messages.iter().any(|m| m.role == Role::Tool);
                Box::pin(async move {
                    if has_tool_result {
                        let s = stream! {
                            yield Ok(ChatChunk::Delta { content: "ok".into() });
                            yield Ok(ChatChunk::Done { usage: None });
                        };
                        Ok(Box::pin(s) as ChatStream)
                    } else {
                        let s = stream! {
                            yield Ok(ChatChunk::ToolCall {
                                id: "c1".into(),
                                name: "nonexistent".into(),
                                arguments: serde_json::json!({}),
                            });
                            yield Ok(ChatChunk::Done { usage: None });
                        };
                        Ok(Box::pin(s) as ChatStream)
                    }
                })
            }
        }

        let mut stream = SimpleLoop.run(
            Arc::new(OneCallProvider),
            vec![],
            vec![Message::user("hello")],
            default_config(),
            CancellationToken::new(),
            None,
        );

        let mut saw_error_result = false;
        while let Some(event) = stream.next().await {
            if let Event::ToolCallDone {
                is_error, result, ..
            } = event
            {
                assert!(is_error);
                assert!(result.contains("not found"));
                saw_error_result = true;
            }
        }
        assert!(saw_error_result);
    }

    #[tokio::test]
    async fn tool_call_emits_pending_and_approved() {
        let provider: Arc<dyn Provider> = Arc::new(ToolCallProvider::new(1));
        let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(EchoTool)];

        let mut stream = SimpleLoop.run(
            provider,
            tools,
            vec![Message::user("hello")],
            default_config(),
            CancellationToken::new(),
            None,
        );

        let mut saw_pending = false;
        let mut saw_approved = false;
        let mut saw_start = false;
        let mut pending_before_approved = false;
        let mut approved_before_start = false;

        while let Some(event) = stream.next().await {
            match event {
                Event::ToolCallPending { name, .. } => {
                    assert_eq!(name, "echo");
                    saw_pending = true;
                }
                Event::ToolCallApproved { .. } => {
                    pending_before_approved = saw_pending;
                    saw_approved = true;
                }
                Event::ToolCallStart { .. } => {
                    approved_before_start = saw_approved;
                    saw_start = true;
                }
                _ => {}
            }
        }

        assert!(saw_pending, "should emit ToolCallPending");
        assert!(saw_approved, "should emit ToolCallApproved");
        assert!(saw_start, "should emit ToolCallStart");
        assert!(
            pending_before_approved,
            "ToolCallPending must precede ToolCallApproved"
        );
        assert!(
            approved_before_start,
            "ToolCallApproved must precede ToolCallStart"
        );
    }
}
