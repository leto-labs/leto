use agent_runtime::{LoopContext, LoopDecision, LoopStrategy, Message, RuntimeError, SteerWhen};
use futures::future::BoxFuture;

/// Hardened reference loop strategy that layers retry/doom-loop/compaction
/// behavior on top of the runtime's provider and tool execution surfaces.
pub struct RobustLoop;

impl LoopStrategy for RobustLoop {
    fn name(&self) -> &'static str {
        "robust"
    }

    fn decide<'a>(&'a self, ctx: LoopContext) -> BoxFuture<'a, Result<LoopDecision, RuntimeError>> {
        Box::pin(async move {
            let state = ctx.state();

            if let Some(request) = state.pending_approval.clone() {
                return Ok(LoopDecision::RequestToolApproval { request });
            }

            let waiting_children = state
                .children
                .values()
                .filter(|child| {
                    child.spawn_mode == agent_runtime::SpawnMode::AwaitCompletion
                        && !child.status.is_terminal()
                })
                .map(|child| child.runtime_id)
                .collect::<Vec<_>>();
            if !waiting_children.is_empty() {
                return Ok(LoopDecision::WaitForAgents {
                    ids: waiting_children,
                    wait: agent_runtime::WaitRequest::default(),
                });
            }

            if !state.pending_tool_calls.is_empty() {
                return Ok(LoopDecision::ExecuteToolBatch);
            }

            if state.pending_completion {
                return Ok(LoopDecision::FinishTurn);
            }

            if !state.active_turn {
                return Ok(LoopDecision::WaitForInput);
            }

            if let Some(doom_loop) = state
                .doom_loop
                .as_ref()
                .filter(|doom_loop| !doom_loop.handled)
            {
                return Ok(LoopDecision::QueueSteering {
                    message: Message::developer_text(format!(
                        "You are repeatedly calling `{}` with the same input. Stop repeating the same tool call. Either choose a different tool, refine the arguments, delegate the work, or explain why no further progress can be made.",
                        doom_loop.tool_name
                    )),
                    when: SteerWhen::NextSafeBoundary,
                });
            }

            if state
                .context_pressure
                .as_ref()
                .is_some_and(|pressure| pressure.should_compact)
            {
                return Ok(LoopDecision::CompactContext);
            }

            if state.iteration_count >= ctx.config().max_iterations {
                return Err(RuntimeError::MaxIterations(state.iteration_count));
            }

            Ok(LoopDecision::RunProvider)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use agent_runtime::{ContextPressure, DoomLoopState, RuntimeConfig, SessionState};
    use provider::{MockProvider, Provider};
    use ulid::Ulid;

    use super::*;

    #[tokio::test]
    async fn robust_loop_compacts_when_runtime_advises_pressure() {
        let loop_strategy = RobustLoop;
        let provider = MockProvider::new();
        let mut state = SessionState::new(Ulid::new());
        state.active_turn = true;
        state.context_pressure = Some(ContextPressure {
            estimated_tokens: 900,
            context_limit: 1_000,
            ratio: 0.9,
            should_compact: true,
        });

        let decision = loop_strategy
            .decide(LoopContext::new(
                provider.info(),
                Arc::new(RuntimeConfig::default()),
                state,
            ))
            .await
            .unwrap();

        assert_eq!(decision, LoopDecision::CompactContext);
    }

    #[tokio::test]
    async fn robust_loop_queues_steering_for_unhandled_doom_loop() {
        let loop_strategy = RobustLoop;
        let provider = MockProvider::new();
        let mut state = SessionState::new(Ulid::new());
        state.active_turn = true;
        state.doom_loop = Some(DoomLoopState {
            tool_name: "echo".into(),
            signature: "echo:{\"message\":\"hello\"}".into(),
            repetitions: 3,
            handled: false,
        });

        let decision = loop_strategy
            .decide(LoopContext::new(
                provider.info(),
                Arc::new(RuntimeConfig::default()),
                state,
            ))
            .await
            .unwrap();

        match decision {
            LoopDecision::QueueSteering { message, when } => {
                assert_eq!(when, SteerWhen::NextSafeBoundary);
                assert_eq!(message.role, provider::MessageRole::Developer);
                assert!(
                    message
                        .plain_text_lossy()
                        .contains("repeatedly calling `echo`")
                );
            }
            other => panic!("expected QueueSteering decision, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn robust_loop_runs_provider_when_no_hardening_action_is_needed() {
        let loop_strategy = RobustLoop;
        let provider = MockProvider::new();
        let mut state = SessionState::new(Ulid::new());
        state.active_turn = true;

        let decision = loop_strategy
            .decide(LoopContext::new(
                provider.info(),
                Arc::new(RuntimeConfig::default()),
                state,
            ))
            .await
            .unwrap();

        assert_eq!(decision, LoopDecision::RunProvider);
    }
}
