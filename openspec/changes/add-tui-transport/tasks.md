# Tasks: add-tui-transport

## Implementation Checklist

### Phase 1: Core Infrastructure

#### App skeleton
- [ ] Add `tui` feature gate to `brain-cli`
- [ ] Add dependencies: ratatui, crossterm, tui-textarea
- [ ] Implement `TuiApp` struct holding BrainApi client + TUI state
- [ ] Implement alternate screen setup/teardown (raw mode, mouse capture, panic cleanup)
- [ ] Implement event loop: `tokio::select!` over crossterm events + BrainApi::subscribe()
- [ ] Implement centralized theme/color struct (no hardcoded colors in widgets)
- [ ] Implement terminal capability detection (true color vs 256 color)

#### Layout
- [ ] Implement main layout: header (1 row) + messages (flex) + input (dynamic) + footer (1 row)
- [ ] Implement responsive resize handling
- [ ] Implement header widget: session title, model name, token count, cost
- [ ] Implement footer widget: cwd, model, help hint (idle) / spinner + interrupt hint (during turn) / retry countdown

#### Message area
- [ ] Implement scrollable message list (ratatui List or custom widget)
- [ ] Implement user message widget: colored left border, text content
- [ ] Implement assistant message widget: streaming text accumulator
- [ ] Implement auto-scroll: stick to bottom when at bottom, freeze when scrolled up
- [ ] Implement "new content below" indicator when scrolled up
- [ ] Implement scroll-to-bottom on message submit
- [ ] Page Up / Page Down scrolling

#### Tool call widgets
- [ ] Implement inline tool display: icon + name + args + status (single line)
- [ ] Implement block tool display: bordered block with header + collapsible result
- [ ] Implement spinner animation for running tools
- [ ] Implement tool categorization (inline vs block based on tool name)
- [ ] Implement collapsed/expanded toggle for block tool results

#### Input area
- [ ] Integrate tui-textarea as input widget
- [ ] Implement Enter to submit, Shift+Enter / Alt+Enter for newline
- [ ] Implement dynamic height (grow to 6 lines, scroll internally beyond)
- [ ] Implement input clearing after submit
- [ ] Implement input history (up/down arrow recalls previous messages)

#### BrainApi integration
- [ ] On Enter: call `client.send_message(session_id, text)`
- [ ] Subscribe to `client.subscribe()` for ServerEvent stream
- [ ] Handle Token events: append to streaming message, re-render
- [ ] Handle ToolCallStart/Done: create/update tool widgets
- [ ] Handle MessageDone: finalize message, add model/duration footer
- [ ] Handle TurnDone: update header stats, restore idle footer
- [ ] Handle Error: show in message area or footer

#### Cancellation
- [ ] Escape during turn: call `client.cancel_turn(session_id)`
- [ ] Ctrl+C with text in input: clear input
- [ ] Ctrl+C with empty input, no turn: exit app
- [ ] Visual feedback: "Cancelling..." in footer
- [ ] Partial content preserved after cancel with "interrupted" indicator
- [ ] In-progress tool calls show cancelled state

### Phase 2: Rich Rendering

#### Markdown rendering
- [ ] Add tui-markdown and syntect dependencies
- [ ] Render assistant messages as markdown (headers, bold, italic, lists, links, blockquotes)
- [ ] Render fenced code blocks with syntax highlighting
- [ ] Handle streaming partial markdown (unclosed fences, partial bold)
- [ ] Render inline code with distinct styling

#### Thinking/reasoning blocks
- [ ] Detect thinking/reasoning content in assistant messages
- [ ] Render in muted/italic style
- [ ] Toggle visibility via keybinding or `/thinking` command

#### Diff view for edit tools
- [ ] Implement unified diff renderer (syntax highlighted, line numbers)
- [ ] Implement split diff renderer (when terminal > 120 cols)
- [ ] Show diff for file_edit tool results
- [ ] Show per-file diffs for apply_patch results with Modified/Created/Deleted labels

#### Tool approval flow
- [ ] Handle ToolCallPending events: show tool with inline approval prompt
- [ ] Render [A]llow / [R]eject prompt within tool block
- [ ] Send approval/rejection via appropriate mechanism
- [ ] Auto-approve flow (ToolCallPending → immediate ToolCallApproved)

#### Slash commands
- [ ] Parse `/` prefix in input as command
- [ ] Implement autocomplete dropdown for commands
- [ ] Implement `/help` — show help overlay
- [ ] Implement `/new` — create new session
- [ ] Implement `/sessions` — open session list overlay
- [ ] Implement `/model <name>` — switch model
- [ ] Implement `/compact` — trigger compaction
- [ ] Implement `/thinking` — toggle thinking visibility
- [ ] Implement `/quit` — exit
- [ ] Handle unknown commands with error message

#### Session management
- [ ] Implement session list overlay (grouped by date, searchable)
- [ ] Implement session switching (load history, scroll to bottom)
- [ ] Implement session rename in list
- [ ] Implement session delete with confirmation
- [ ] Implement `/new` session creation
- [ ] Show session title in header

#### Scrolling enhancements
- [ ] Message-boundary navigation (jump to next/prev message)
- [ ] Home/End for first/last message
- [ ] Scroll speed configuration

### Phase 3: Polish

#### Overlay system
- [ ] Implement overlay rendering (dialog on top of message area)
- [ ] Implement overlay stacking (LIFO)
- [ ] Escape closes top overlay
- [ ] Streaming continues behind overlays

#### Leader key system
- [ ] Implement leader key (Ctrl+X) with 2s timeout
- [ ] Visual indicator of command mode (muted prompt)
- [ ] `<leader>n` new session, `<leader>l` session list, `<leader>m` model selector, `<leader>q` quit

#### Command palette
- [ ] Implement Ctrl+P command palette overlay
- [ ] List all commands with keybindings
- [ ] Fuzzy search
- [ ] Execute selected command

#### Sidebar
- [ ] Implement sidebar (42 cols, right side)
- [ ] Context section: token count, % used, cost
- [ ] Modified files section: file list with +/- counts
- [ ] Toggle via `<leader>b`
- [ ] Auto-show when terminal > 120 cols

#### Theme system
- [ ] Define default dark theme
- [ ] Define default light theme
- [ ] Implement theme switching via `/theme` or `<leader>t`
- [ ] Support custom themes from config

#### Input stash
- [ ] Implement stash: save current input to stack
- [ ] Implement stash pop: restore last saved input
- [ ] Implement stash list: pick from saved inputs

#### Advanced interactions
- [ ] Mouse support: click to select message, scroll wheel
- [ ] Clipboard: copy message content or code block
- [ ] Input autocomplete: `@` for file paths
- [ ] Message footer: model, duration, token count per message

### Tests
- [ ] Unit test: message rendering with sample markdown
- [ ] Unit test: tool call widget states (pending, running, success, error, cancelled)
- [ ] Unit test: slash command parsing
- [ ] Unit test: scroll behavior (auto-scroll, manual scroll, indicators)
- [ ] Unit test: cancellation state transitions
- [ ] Unit test: input history
- [ ] Integration test: TuiApp with MockProvider (headless state verification)
