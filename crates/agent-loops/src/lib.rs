mod simple;

pub use simple::SimpleLoop;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use agent_runtime::{
        ApprovalDecision, ControlEvent, InterruptMode, Message, MessageRole, RuntimeConfig,
        RuntimeEvent, SessionBoundary, SessionCommand, SessionEngine, SessionPhase, SessionState,
        SteerWhen, ToolApproval, ToolCall, ToolExecutionResult, ToolExecutor,
    };
    use async_stream::stream;
    use futures::future::BoxFuture;
    use provider::{
        Block, BlockDelta, BlockKind, Event, EventStream, FinishReason, MockProvider, Provider,
        ProviderInfo, Request, ToolDefinition, Usage,
    };
    use tokio::time::{Duration, sleep, timeout};
    use ulid::Ulid;

    use crate::SimpleLoop;

    struct EchoTools;

    impl ToolExecutor for EchoTools {
        fn definitions(&self) -> Vec<ToolDefinition> {
            vec![ToolDefinition::new(
                "echo",
                "Echo tool",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "message": { "type": "string" }
                    },
                    "required": ["message"]
                }),
            )]
        }

        fn execute<'a>(
            &'a self,
            call: ToolCall,
        ) -> BoxFuture<'a, Result<ToolExecutionResult, agent_runtime::RuntimeError>> {
            Box::pin(async move {
                Ok(ToolExecutionResult::success(serde_json::json!({
                    "message": call.input["message"].clone()
                })))
            })
        }
    }

    struct ApprovalEchoTools;

    impl ToolExecutor for ApprovalEchoTools {
        fn definitions(&self) -> Vec<ToolDefinition> {
            EchoTools.definitions()
        }

        fn approval(&self, _call: &ToolCall) -> Option<ToolApproval> {
            Some(ToolApproval {
                reason: Some("operator approval required".into()),
            })
        }

        fn execute<'a>(
            &'a self,
            call: ToolCall,
        ) -> BoxFuture<'a, Result<ToolExecutionResult, agent_runtime::RuntimeError>> {
            EchoTools.execute(call)
        }
    }

    struct ToolThenTextProvider;

    impl Provider for ToolThenTextProvider {
        fn stream<'a>(
            &'a self,
            request: &'a Request,
        ) -> BoxFuture<'a, Result<EventStream<'a>, provider::Error>> {
            Box::pin(async move {
                let saw_tool_result = request.messages.iter().any(|message| {
                    message
                        .content
                        .iter()
                        .any(|block| matches!(block, provider::ContentBlock::ToolResult { .. }))
                });
                let stream = stream! {
                    yield Ok(Event::ResponseStart {
                        response_id: Some("test-response".into()),
                        model: Some("tool-then-text".into()),
                    });
                    if saw_tool_result {
                        yield Ok(Event::BlockStart {
                            block: Block {
                                id: "assistant-text".into(),
                                output_index: 0,
                                kind: BlockKind::Text,
                                item_id: None,
                            },
                        });
                        yield Ok(Event::BlockDelta {
                            id: "assistant-text".into(),
                            delta: BlockDelta::Text {
                                text: "done".into(),
                            },
                        });
                        yield Ok(Event::BlockStop {
                            id: "assistant-text".into(),
                        });
                    } else {
                        yield Ok(Event::BlockStart {
                            block: Block {
                                id: "assistant-tool".into(),
                                output_index: 0,
                                kind: BlockKind::ToolCall {
                                    name: Some("echo".into()),
                                    call_id: Some("call-1".into()),
                                },
                                item_id: None,
                            },
                        });
                        yield Ok(Event::BlockDelta {
                            id: "assistant-tool".into(),
                            delta: BlockDelta::Json {
                                partial_json: serde_json::json!({"message": "hello tool"}).to_string(),
                            },
                        });
                        yield Ok(Event::BlockStop {
                            id: "assistant-tool".into(),
                        });
                    }
                    yield Ok(Event::Usage {
                        usage: Usage::with_totals(Some(1), Some(1)),
                    });
                    yield Ok(Event::Completed {
                        response_id: Some("test-response".into()),
                        finish_reason: Some(FinishReason::Stop),
                    });
                };
                Ok(Box::pin(stream) as EventStream<'a>)
            })
        }

        fn info(&self) -> ProviderInfo {
            MockProvider::new().info()
        }
    }

    struct SlowTextProvider;

    impl Provider for SlowTextProvider {
        fn stream<'a>(
            &'a self,
            _request: &'a Request,
        ) -> BoxFuture<'a, Result<EventStream<'a>, provider::Error>> {
            Box::pin(async move {
                let stream = stream! {
                    yield Ok(Event::ResponseStart {
                        response_id: Some("slow".into()),
                        model: Some("slow-model".into()),
                    });
                    yield Ok(Event::BlockStart {
                        block: Block {
                            id: "assistant-text".into(),
                            output_index: 0,
                            kind: BlockKind::Text,
                            item_id: None,
                        },
                    });
                    sleep(Duration::from_millis(40)).await;
                    yield Ok(Event::BlockDelta {
                        id: "assistant-text".into(),
                        delta: BlockDelta::Text {
                            text: "working".into(),
                        },
                    });
                    sleep(Duration::from_millis(180)).await;
                    yield Ok(Event::BlockStop {
                        id: "assistant-text".into(),
                    });
                    yield Ok(Event::Completed {
                        response_id: Some("slow".into()),
                        finish_reason: Some(FinishReason::Stop),
                    });
                };
                Ok(Box::pin(stream) as EventStream<'a>)
            })
        }

        fn info(&self) -> ProviderInfo {
            MockProvider::new().info()
        }
    }

    #[tokio::test]
    async fn simple_loop_finishes_after_plain_assistant_output() {
        let engine = SessionEngine::new(
            Arc::new(MockProvider::new()),
            Arc::new(EchoTools),
            Arc::new(SimpleLoop),
            RuntimeConfig::default(),
            SessionState::new(Ulid::new()),
        );
        let mut events = engine.subscribe();

        engine
            .submit(SessionCommand::SubmitInput {
                input: vec![Message::user_text("plain response")],
                source: None,
            })
            .await
            .unwrap();

        loop {
            let event = timeout(Duration::from_millis(500), events.recv())
                .await
                .unwrap()
                .unwrap();
            if matches!(event, RuntimeEvent::TurnFinished { .. }) {
                break;
            }
        }

        let state = engine.snapshot().await;
        assert_eq!(state.transcript.len(), 2);
        assert_eq!(state.transcript[1].role, MessageRole::Assistant);
    }

    #[tokio::test]
    async fn simple_loop_executes_tool_and_reenters_inference() {
        let engine = SessionEngine::new(
            Arc::new(ToolThenTextProvider),
            Arc::new(EchoTools),
            Arc::new(SimpleLoop),
            RuntimeConfig::default(),
            SessionState::new(Ulid::new()),
        );
        let mut events = engine.subscribe();

        engine
            .submit(SessionCommand::SubmitInput {
                input: vec![Message::user_text("tool: hello tool")],
                source: None,
            })
            .await
            .unwrap();

        let mut saw_tool_finish = false;
        loop {
            let event = timeout(Duration::from_millis(500), events.recv())
                .await
                .unwrap()
                .unwrap();
            if matches!(event, RuntimeEvent::ToolCallFinished { .. }) {
                saw_tool_finish = true;
            }
            if matches!(event, RuntimeEvent::TurnFinished { .. }) {
                break;
            }
        }

        assert!(saw_tool_finish);
        let state = engine.snapshot().await;
        assert_eq!(state.transcript.len(), 4);
        assert_eq!(state.transcript[0].role, MessageRole::User);
        assert_eq!(state.transcript[1].role, MessageRole::Assistant);
        assert_eq!(state.transcript[2].role, MessageRole::User);
        assert_eq!(state.transcript[3].role, MessageRole::Assistant);
    }

    #[tokio::test]
    async fn simple_loop_waits_for_approval_before_running_tools() {
        let engine = SessionEngine::new(
            Arc::new(ToolThenTextProvider),
            Arc::new(ApprovalEchoTools),
            Arc::new(SimpleLoop),
            RuntimeConfig::default(),
            SessionState::new(Ulid::new()),
        );
        let mut events = engine.subscribe();

        engine
            .submit(SessionCommand::SubmitInput {
                input: vec![Message::user_text("tool: hello tool")],
                source: None,
            })
            .await
            .unwrap();

        let request_id = loop {
            let event = timeout(Duration::from_millis(500), events.recv())
                .await
                .unwrap()
                .unwrap();
            if let RuntimeEvent::ApprovalRequested { request } = event {
                break request.id;
            }
        };

        let paused_state = engine.snapshot().await;
        assert_eq!(paused_state.phase, SessionPhase::AwaitingApproval);
        assert_eq!(
            paused_state.boundary,
            Some(SessionBoundary::AwaitingApproval)
        );

        engine
            .submit(SessionCommand::Approve(ApprovalDecision::Allow {
                request_id,
            }))
            .await
            .unwrap();

        loop {
            let event = timeout(Duration::from_millis(500), events.recv())
                .await
                .unwrap()
                .unwrap();
            if matches!(event, RuntimeEvent::TurnFinished { .. }) {
                break;
            }
        }

        let state = engine.snapshot().await;
        assert_eq!(state.transcript.len(), 4);
    }

    #[tokio::test]
    async fn simple_loop_pauses_after_current_boundary_when_interrupted() {
        let engine = SessionEngine::new(
            Arc::new(SlowTextProvider),
            Arc::new(EchoTools),
            Arc::new(SimpleLoop),
            RuntimeConfig::default(),
            SessionState::new(Ulid::new()),
        );
        let mut events = engine.subscribe();

        engine
            .submit(SessionCommand::SubmitInput {
                input: vec![Message::user_text("wait")],
                source: None,
            })
            .await
            .unwrap();
        sleep(Duration::from_millis(80)).await;
        engine
            .submit(SessionCommand::Control(ControlEvent::Interrupt {
                mode: InterruptMode::ImmediateCancel,
            }))
            .await
            .unwrap();

        loop {
            let event = timeout(Duration::from_millis(500), events.recv())
                .await
                .unwrap()
                .unwrap();
            if matches!(
                event,
                RuntimeEvent::BoundaryReached {
                    boundary: SessionBoundary::Paused
                }
            ) {
                break;
            }
        }

        let state = engine.snapshot().await;
        assert_eq!(state.phase, SessionPhase::Paused);
        assert_eq!(state.boundary, Some(SessionBoundary::Paused));
    }

    #[tokio::test]
    async fn simple_loop_applies_steering_at_boundary() {
        let engine = SessionEngine::new(
            Arc::new(SlowTextProvider),
            Arc::new(EchoTools),
            Arc::new(SimpleLoop),
            RuntimeConfig::default(),
            SessionState::new(Ulid::new()),
        );
        let mut events = engine.subscribe();

        engine
            .submit(SessionCommand::SubmitInput {
                input: vec![Message::user_text("start")],
                source: None,
            })
            .await
            .unwrap();
        sleep(Duration::from_millis(80)).await;
        engine
            .submit(SessionCommand::Control(ControlEvent::Steer {
                message: Message::developer_text("stay concise"),
                when: SteerWhen::NextSafeBoundary,
            }))
            .await
            .unwrap();

        let mut saw_steering = false;
        loop {
            let event = timeout(Duration::from_millis(500), events.recv())
                .await
                .unwrap()
                .unwrap();
            if matches!(event, RuntimeEvent::SteeringApplied { .. }) {
                saw_steering = true;
            }
            if matches!(event, RuntimeEvent::TurnFinished { .. }) {
                break;
            }
        }

        assert!(saw_steering);
        let state = engine.snapshot().await;
        assert!(
            state
                .transcript
                .iter()
                .any(|message| message.plain_text_lossy().contains("stay concise"))
        );
    }
}
