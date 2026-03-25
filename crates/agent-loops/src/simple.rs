use agent_runtime::{LoopContext, LoopDecision, LoopStrategy, RuntimeError};
use futures::future::BoxFuture;

/// Reference loop strategy that alternates provider steps and tool execution.
pub struct SimpleLoop;

impl LoopStrategy for SimpleLoop {
    fn name(&self) -> &'static str {
        "simple"
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

            if state.active_turn {
                if state.iteration_count >= ctx.config().max_iterations {
                    return Err(RuntimeError::MaxIterations(state.iteration_count));
                }
                return Ok(LoopDecision::RunProvider);
            }

            Ok(LoopDecision::WaitForInput)
        })
    }
}
