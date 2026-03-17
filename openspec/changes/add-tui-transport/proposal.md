# Proposal: add-tui-transport

## Why

`brain` already has the runtime boundary needed for a modern terminal client:
`BrainServer`, `BrainApi`, in-process clients via `server.client()`, and
HTTP/SSE for remote consumers. What it lacks is a terminal experience that
feels like a real coding agent rather than a line-based debug shell.

The research on Codex and OpenCode points to a clear MVP shape:

- a transcript-first full-screen chat UI
- streaming assistant output that stays readable while incomplete
- inline tool activity inside the conversation flow
- a multiline composer with clear state transitions
- a footer/status row that changes meaning based on runtime state
- interruption that preserves partial output instead of treating cancel as a failure

The current `add-tui-transport` change is too broad. It includes planning,
subagents, approval UX, diff rendering, rich markdown polish, sidebars, leader
keys, themes, and other features that are useful later but distract from the
core product question: does chatting with the agent feel good?

## What Changes

This change narrows the MVP to a state-driven TUI client over `BrainApi`.

### MVP scope

- `brain` launches a full-screen TUI in local in-process mode
- `brain serve` runs the existing server surface for trusted local/dev use
- `brain attach <url>` launches the same TUI against a remote server
- the UI is built around four regions:
  - header
  - transcript
  - multiline composer
  - footer/status row
- assistant text streams directly into the transcript
- tool activity renders inline in the transcript
- users can cancel active turns without losing partial output
- users can switch sessions inside the TUI
- users can compose and queue the next message while a turn is still running

### Explicit non-goals for MVP

- planning or review modes
- subagents or child-session navigation
- tool approval / permission UX
- diff rendering
- rich syntax-highlighted markdown
- sidebar, command palette, leader key system, themes, mouse support
- secure remote deployment, auth, or policy enforcement

## Supporting runtime changes

The TUI itself remains the center of gravity, but the UX needs one small
runtime contract improvement:

- user cancellation should produce an `Interrupted` terminal turn outcome, not a
  generic `Error`

This keeps the chat UI honest: cancelled work is not the same as a provider
failure, and the transcript/footer should be able to reflect that cleanly.

## UX direction

The MVP should borrow structure from Codex and OpenCode without copying their
full feature sets:

- stable transcript, not a pane-heavy layout
- footer as a small state-specific controller, not a static status bar
- composer remains useful in idle, drafting, and running states
- inline tool activity, no dedicated tool pane
- streaming and cancellation stay visible in place

## Testing expectations

This change SHALL treat TUI testing as part of the product definition, not as
follow-up polish.

The MVP should only be considered ready when it has:

- deterministic state-transition tests for the TUI controller/state model
- deterministic rendering or snapshot tests for transcript, footer, composer,
  tool, and narrow-width states
- deterministic integration tests for local in-process and remote HTTP/SSE flows
- an opt-in live-provider validation suite that runs when `OPENAI_API_KEY` is
  available via the existing `.env` / `dotenvy` pattern already used in provider
  smoke tests

This is necessary to avoid an implementation that satisfies the event contract
but still feels broken or unstable in real use.

## Change Dependencies

- **Requires**: `add-server-architecture`
- **Requires**: `add-brain-cli`
- **Coordinates with**: `enrich-event-model` and `harden-agent-loop`, but MVP
  does not depend on planning, approval, compaction, or other non-core flows

## Impact

- **Modifies**: `brain-cli` spec to make the TUI the default interactive UX
- **Modifies**: `brain-server` spec to clarify interactive turn terminal-event behavior
- **Modifies**: `brain-loops` spec to distinguish interruption from generic error
- **Adds runtime deps**: `ratatui`, `crossterm`, `tui-textarea`
- **Adds validation work**: state, snapshot, integration, and opt-in live TUI tests
- **Defers**: markdown polish and richer terminal affordances to later changes
