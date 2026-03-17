# Tasks: add-tui-transport

## Implementation Checklist

### 1. Spec and contract updates
- [ ] Update `add-tui-transport` proposal to describe a state-driven TUI MVP
- [ ] Update `add-tui-transport` design to define the TUI layout and runtime states
- [ ] Update `brain-cli` delta spec to make the TUI the default interactive UX
- [ ] Add `brain-server` delta spec covering interactive terminal-event behavior
- [ ] Add `brain-loops` delta spec covering interruption as a terminal outcome
- [ ] Run `openspec validate add-tui-transport --strict`

### 2. TUI app skeleton
- [ ] Add a `tui` feature to `brain-cli`
- [ ] Add dependencies: `ratatui`, `crossterm`, `tui-textarea`
- [ ] Implement `TuiApp` as a `BrainApi` client
- [ ] Implement terminal setup/teardown with panic-safe restore
- [ ] Implement the main event loop over terminal events and engine events

### 3. MVP layout and state model
- [ ] Implement header, transcript, composer, and footer regions
- [ ] Implement explicit UI states: idle, drafting, running, running-with-queued-input, tool-running, interrupted, loading
- [ ] Implement resize handling and narrow-terminal fallbacks
- [ ] Keep the header lightweight and move stateful hints into the footer

### 4. Transcript and rendering
- [ ] Implement scrollable transcript with bottom-stick behavior
- [ ] Implement “new content below” behavior when the user is scrolled up
- [ ] Render streamed assistant text incrementally
- [ ] Render basic markdown structure without syntax highlighting
- [ ] Render tool activity inline in transcript
- [ ] Use compact rendering for light tools and block rendering for verbose tools

### 5. Composer and interaction flow
- [ ] Integrate `tui-textarea` as the multiline composer
- [ ] Implement submit vs newline behavior
- [ ] Implement input history
- [ ] Implement local queueing of follow-up user messages while a turn is active
- [ ] Surface queued-message state in the footer or composer area
- [ ] Implement slash commands: `/help`, `/new`, `/sessions`, `/model`, `/quit`

### 6. Session and connection flow
- [ ] Implement local mode using `BrainServer::client()`
- [ ] Implement `brain serve` using the existing server surface
- [ ] Implement `brain attach <url>` using the HTTP/SSE client path
- [ ] Implement loading and reconnect states for attach/session hydration
- [ ] Implement session create/list/switch/resume inside the TUI

### 7. Turn lifecycle and interruption
- [ ] Add an `Interrupted` terminal event to the loop event contract
- [ ] Ensure `cancel_turn()` causes `Interrupted`, not a generic `Error`
- [ ] Preserve partial assistant and tool output after cancellation
- [ ] Render interrupted turns distinctly from failed turns
- [ ] Keep footer state and transcript state consistent across success, interruption, and failure

### 8. Tests
- [ ] Unit test the TUI state transitions
- [ ] Unit test footer/composer behavior for each major runtime state
- [ ] Unit test transcript auto-scroll and “new content below” behavior
- [ ] Unit test queued-input behavior while a turn is active
- [ ] Unit test inline and block tool rendering states
- [ ] Unit test cancellation and interrupted terminal-event handling
- [ ] Add snapshot or golden tests for idle, drafting, running, queued, tool-running, interrupted, and narrow-width screens
- [ ] Integration test local in-process TUI state updates with a mock provider
- [ ] Integration test attach-mode event flow against HTTP/SSE server
- [ ] Integration test session hydration and switching across local and attached modes
- [ ] Add an opt-in live TUI validation suite gated by `OPENAI_API_KEY`
- [ ] Document the live-validation command path and `.env`-based setup

## Deferred

- [ ] Planning/review modes
- [ ] Subagent UX
- [ ] Permission / approval UI
- [ ] Diff rendering
- [ ] Syntax-highlighted markdown
- [ ] Sidebar, command palette, leader-key system
- [ ] Theme system
- [ ] Mouse, clipboard, and file autocomplete
