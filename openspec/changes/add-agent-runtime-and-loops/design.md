# Design

## Core decision

Add two new greenfield crates:

- `agent-runtime` as the reusable in-memory session engine
- `agent-loops` as strategy implementations over that runtime

This preserves the same architectural style as the standalone `provider` crate:
shared reusable subsystem first, top-level composition later.

## Runtime boundary

`agent-runtime` depends on `provider` directly and stays storeless.

It owns:

- live session state
- command-driven session execution
- a flat runtime registry internal to `agent-runtime`
- typed runtime actions for spawn, wait, input, pause/resume, interrupt,
  listing, and messaging
- transcript mutation
- provider event normalization
- tool execution lifecycle
- steering and interruption queues
- approval waiting/resume state
- child-runtime bookkeeping
- parent-child lineage metadata
- registry-routed cross-agent messaging
- mailbox and direct-message storage
- provider-visible child report injection at safe boundaries
- safe-boundary handling
- runtime event emission

It does not own:

- persistence
- transports
- top-level application composition

## Loop boundary

The loop trait lives in `agent-runtime` so that concrete loop crates can depend
on it without a cycle.

`agent-loops` initially ships one loop only:

- `SimpleLoop`

The runtime APIs are explicitly shaped for future reuse by more complex loops
such as robust or terminal-oriented strategies.

Loops no longer own the full async turn body. They return decisions over engine
state instead:

- run another provider step
- execute pending tools
- wait for approval
- wait for input
- finish the current turn
- spawn or await child runtimes
- send runtime input or cross-agent messages
- compact context later

This keeps loops focused on policy while the runtime owns the mechanics.

## Transcript model

The canonical model-visible transcript uses `provider::Message` directly.

Assistant provider output is committed as assistant messages assembled from
provider block events. Tool results are committed as transcript messages
containing `ContentBlock::ToolResult`.

## Event model

`agent-runtime` exposes its own runtime events rather than raw `provider::Event`
as the main public interface, while still preserving provider block semantics.

The runtime event model is bidirectional-session aware. In addition to output,
tool, and transcript events, it includes:

- queued input and control events
- phase and boundary transitions
- approval requested/resolved events
- steering queued/applied events
- child-session lifecycle events

## Child sessions

Delegation is modeled as child-runtime lifecycle owned by the runtime rather
than as an ad-hoc convention inside the generic tool layer.

Agent management is exposed to the model through runtime-owned native tools, not
through the generic application `ToolExecutor` surface. The model may call tools
such as `spawn_agent`, `message_agent`, `read_agent_mail`, `interrupt_agent`,
`list_agents`, and `wait_agent`, but those tools are thin facades over runtime
actions and registry operations.

The runtime keeps a flat live structure:

- each runtime has one id
- each child runtime stores an optional parent reference
- the registry stores live runtime handles and parent-child indexes

This avoids recursive runtime ownership while preserving lineage and allows
registry-mediated routing by runtime id.

Cross-agent communication is split into:

- runtime input/steering for safe-boundary transcript injection
- direct messages that notify at the next safe boundary
- mailbox messages that are explicitly pulled later
- child reports that may be promoted into parent model context

Loops may still surface delegation through a provider-visible tool facade later,
but the runtime remains the source of truth for parent/child bookkeeping.

## Channel readiness

The runtime is source-aware without implementing a full routing subsystem.

Session commands may carry optional source metadata such as an origin id and
channel label. Outer layers remain free to multiplex CLI, TUI, HTTP, or other
surfaces into one authoritative serialized session lane.

## Defaults

The runtime defaults intentionally favor non-destructive and non-blocking
behavior:

- child spawns default to background execution
- child spawns inherit the parent runtime’s effective provider/model/runtime
  settings unless explicitly overridden
- waits default to releasing the hold on timeout rather than interrupting the
  child
- agent messages default to mailbox-style delivery unless the caller explicitly
  asks for a direct message
