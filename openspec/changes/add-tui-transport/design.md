# Design: add-tui-transport

## Summary

The MVP TUI is a thin `BrainApi` client with a strong state model. It should
feel responsive and readable before it feels feature-rich.

The design is driven by Codex and OpenCode:

- the transcript is the primary surface
- partial assistant output is visible immediately
- tool activity stays inline with the conversation
- the footer changes meaning based on state
- user cancellation preserves context rather than collapsing into a generic error

## Architecture

The TUI is not a `Transport` implementation.

- Local mode: `brain` boots `BrainServer` and uses `server.client()`
- Server mode: `brain serve` exposes the existing HTTP/SSE API
- Remote mode: `brain attach <url>` uses the same `BrainApi` contract over HTTP/SSE

This change does not introduce a new protocol. It reuses the existing
REST + SSE server model.

## Layout

The MVP layout has four persistent regions:

1. `Header`
2. `Transcript`
3. `Composer`
4. `Footer / Status Row`

The transcript remains stable across most state transitions. The composer and
footer do most of the visible state switching.

### Header

Keep the header light:

- session title or fallback name
- active model
- local vs attached mode

The header is ambient context, not a control surface.

### Transcript

The transcript is the main interaction surface.

- append-only from the user's perspective
- scrollable
- auto-scrolls only while the user is already at the bottom
- when scrolled away from bottom, new content does not force-jump
- a small “new content below” indicator is enough

### Composer

The composer is multiline and always visible.

- `Enter` submits
- modified enter inserts newline
- composer supports input history
- while a turn is active, the user can keep typing
- submitting while busy queues the next message instead of rejecting it

### Footer / Status Row

The footer is intentionally stateful. It should render short, high-value hints
in a fixed location.

Typical footer roles:

- idle hint surface
- draft / submit hint surface
- running spinner + active work summary
- queued-message indicator
- cancel / interrupt hint
- retry / reconnect / error summary

This follows the best Codex/OpenCode pattern: the footer becomes the compact,
stable “what can I do now?” surface.

## UI State Model

The MVP state model should be explicit in the implementation and in tests.

### Idle

- no active turn
- composer focused and editable
- footer shows help / session / model hints

### Drafting

- no active turn
- composer contains text
- footer shifts to submit/edit hints

### Running

- assistant output is streaming
- transcript updates incrementally
- footer shows spinner + summary + cancel hint

### Running With Queued Input

- a turn is active
- user continues typing
- submitted follow-up messages are queued locally
- footer or composer area shows pending queue state clearly

### Tool Running

- transcript shows inline tool activity
- the active assistant turn remains visible around it

### Cancelling / Interrupted

- cancellation request is in-flight or complete
- partial assistant and tool content remains visible
- transcript marks the turn interrupted
- footer clears busy state and returns to idle/draft hints

### Retry / Error

- retry and provider failure states do not replace transcript history
- the footer can summarize transient issues
- hard failures are shown in context without collapsing the session UI

### Loading / Attaching / Reconnecting

- used for startup, attach, session hydration, and dropped connections
- distinct from idle so the user knows the client is not ready yet

## Rendering Decisions

### Assistant text

Readable-first markdown only:

- paragraphs
- lists
- inline code
- fenced code blocks rendered as styled text blocks

No syntax highlighting or diff rendering in MVP.

The renderer updates on every token or chunk. Partial markdown is accepted as a
normal state.

### Tool rendering

Tool activity stays inline in the transcript.

- light tools: compact single-line status
- verbose or long-running tools: bordered block with summarized output

There is no separate tool panel in MVP.

### Cancellation rendering

Cancellation is a first-class UX state, not an error skin.

- streamed assistant text stays visible
- started tools remain visible with interrupted/cancelled styling
- the turn ends with `Interrupted`, not a generic `Error`

## Input Scenarios

### Waiting for input

- composer focused
- footer shows operational hints

### Thinking / streaming

- footer shows spinner and interrupt hint
- composer remains available for drafting and queueing

### Executing tools

- transcript shows tool progress inline
- footer remains focused on turn-level state, not per-tool detail

### Session switching

- session list opens as an overlay
- switching reloads transcript/history and resets per-session state

## Runtime Contract Changes

The TUI should derive as much as possible client-side, but one event-level
change is worth making now:

- `Interrupted` becomes a terminal turn outcome emitted on user cancellation

This gives the TUI a clean way to differentiate:

- success: `TurnDone`
- user cancel: `Interrupted`
- failure: `Error`

That distinction matters for transcript styling, footer messaging, and tests.

## Testing Strategy

The TUI should be designed for deep automated validation from the start.

### State-level tests

Keep the main interaction model in testable logic rather than burying behavior
entirely inside widget code.

Required state-transition coverage:

- `idle` -> `drafting`
- `drafting` -> `running`
- `running` -> `running_with_queued_input`
- `running` -> `tool_running`
- `running` -> `interrupted`
- `loading` -> `idle`
- `error` recovery back to usable input state

These tests should verify footer mode, composer behavior, queue state, and
terminal turn outcome handling.

### Rendering and snapshot tests

Use a headless terminal backend to render the major UI surfaces and capture
stable snapshots.

Required snapshot-style coverage:

- idle screen
- drafting screen
- running stream with partial assistant text
- running with queued input
- inline tool activity
- verbose/block tool activity
- interrupted turn with partial content preserved
- scrolled-up transcript with “new content below”
- narrow terminal layout

These are UX regression tests, not just visual niceties.

### Deterministic integration tests

The implementation should support scripted end-to-end tests without a real LLM.

Required deterministic integration coverage:

- local mode via `BrainServer::client()`
- remote mode against HTTP/SSE server transport
- session creation and switching
- history hydration on attach
- ordered event handling
- cancellation preserving partial output

### Opt-in live-provider validation

The repo already uses `.env` + `dotenvy` for provider smoke tests. The TUI
should follow the same pattern for an opt-in live validation lane.

Required live coverage:

- one real streaming assistant response
- one interrupted live response
- at least one attach or remote-flow validation if practical

These tests should be opt-in and skipped automatically when `OPENAI_API_KEY`
is unavailable. They should not make normal offline development noisy or
fragile.

## Deferred Work

These are intentionally out of scope for MVP:

- planning/review modes
- subagent UI
- approval prompts
- diff rendering
- syntax-highlighted markdown
- sidebars and palettes
- theme system
- command leader UX
- mouse and clipboard features
- auth-secure remote operation
