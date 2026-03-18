# Design: add-acp-client-surface

## Decision: ACP And BrainServer Are Parallel Primary Surfaces

`brain` will support two first-class interactive client surfaces:

1. **ACP over stdio**
   Best for existing ACP-compatible editors and other clients that already know
   how to launch ACP agents as subprocesses.

2. **BrainServer over HTTP/SSE**
   Best for `brain`'s own TUI, web UI, automation, service-style deployment,
   and any consumer that benefits from a native `brain` API rather than ACP.

This is not an either/or decision.

## Decision: ACP Is Not Bridge-Only

The old "tiny bridge later" framing is too weak for the current ecosystem.

`brain` should expose ACP directly from its runtime model rather than treating
ACP purely as an HTTP translation shim. A bridge may still exist later, but it
is not the defining architecture.

This means the ACP surface should be implemented against the same core runtime
concepts as `BrainServer`:

- sessions
- message history
- prompt turns
- event streaming
- cancellation
- approvals
- tool/file/terminal interaction

## Decision: ACP Uses Client-Owned Capabilities When Present

ACP is most valuable when `brain` uses the client environment instead of
pretending ACP is only a chat transport.

The ACP surface should:

- use `fs/read_text_file` and `fs/write_text_file` when the client advertises
  those capabilities
- use ACP terminal methods when the client advertises terminal support
- use ACP permission requests for approval UX
- degrade cleanly when those capabilities are absent

This allows editor clients to provide:

- unsaved-buffer reads
- file writes reflected directly in the editor
- visible terminal output
- native permission prompts

## Decision: Stdio First, Streamable HTTP Later

ACP support should begin with stdio because:

- it is the defined production transport today
- all serious editor ACP clients use it
- it delivers immediate value in Zed, JetBrains, VS Code ACP, and Neovim ACP
  paths

Streamable HTTP should be treated as a design target to watch, not an MVP
requirement. The runtime mapping should be transport-agnostic enough that
adopting it later does not force a redesign.

## Session Mapping

ACP sessions will map onto `brain` sessions.

- `session/new` creates a new `brain` session for the requested working
  directory and session context
- `session/load` reopens an existing `brain` session and replays conversation
  history through ACP `session/update` notifications
- `session/prompt` maps to a `brain` turn
- `session/cancel` maps to turn cancellation

The ACP layer should own only protocol-specific concerns such as JSON-RPC
marshaling, capability negotiation, and ACP-specific update shapes.

## Capability Negotiation

The ACP layer should only advertise optional ACP capabilities that `brain`
actually supports.

That includes:

- loadable sessions
- session modes
- session config options
- model selection
- list/fork/resume/close behavior

If a feature is not backed by the runtime yet, the ACP surface should omit it or
return method-not-found rather than pretending support exists.

## Relationship To Event Model Work

ACP strengthens the case for the `enrich-event-model` direction.

`brain` needs richer runtime events for:

- plan updates
- streaming tool call state
- permission requests and outcomes
- progress and status
- terminal output references
- session metadata updates

These should exist as runtime concepts first and then be mapped to:

- ACP `session/update` notifications
- `BrainServer` events and streams

## Non-Editor Clients

ACP should be treated as:

- a primary editor/IDE integration surface today
- a plausible broader client abstraction for some future shells

But `brain` should not assume ACP replaces every native client need. The
`BrainServer` surface remains strategically important for first-party clients
and for use cases where ACP is not the best fit.
