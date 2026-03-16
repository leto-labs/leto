# brain-cli Delta Spec

## Note
This change currently describes planned TUI behavior. The `brain-cli` crate and this transport wiring are not implemented in the current workspace.

## MODIFIED Requirements

### Requirement: Unified Binary
The `brain-cli` binary SHALL include the `tui` feature by default and use `TuiTransport` as its transport when implemented.

#### Scenario: Default is TUI
- **WHEN** `brain` is run
- **THEN** it SHALL start with the TUI interface

### Requirement: Feature Gates
The feature gate list SHALL be extended with:
- `tui` (default) — ratatui-based terminal UI

#### Scenario: TUI feature included
- **WHEN** `brain-cli` is compiled with default features
- **THEN** `TuiTransport` SHALL be the transport used

## ADDED Requirements

### Requirement: TuiApp
The system SHALL provide a `TuiApp` that acts as a thin client of `BrainApi` (from `add-server-architecture`), rendering an interactive terminal UI using ratatui + crossterm. It does NOT implement the `Transport` trait. It communicates with the engine exclusively through `BrainApi` methods and the event subscription.

#### Scenario: Interactive session
- **WHEN** `TuiApp` is started with a `BrainApi` client
- **THEN** the user SHALL see a full-screen TUI with message history, streaming output, and a prompt input area

#### Scenario: Engine independence
- **WHEN** the TUI is running
- **THEN** it SHALL only interact with the engine via `BrainApi`
- **AND** it SHALL never call `Brain` methods directly

#### Scenario: Clean terminal restore
- **WHEN** the TUI exits (normally or via panic)
- **THEN** the terminal SHALL be restored to its original state (leave alternate screen, disable raw mode)

### Requirement: Layout
The TUI SHALL render a layout with four zones:
1. **Header** (1 row): session title, model name, token usage, cost estimate
2. **Message area** (flexible): scrollable list of conversation messages
3. **Input area** (1-6 rows, dynamic): multi-line text editor
4. **Footer** (1 row): contextual information and hints

The layout SHALL adapt to terminal size changes (responsive).

#### Scenario: Terminal resize
- **WHEN** the terminal is resized
- **THEN** the layout SHALL re-render to fit the new dimensions without losing state

#### Scenario: Small terminal
- **WHEN** the terminal is fewer than 80 columns wide
- **THEN** the layout SHALL still be usable (header/footer truncate, message area gets remaining space)

### Requirement: Message Rendering — User Messages
User messages SHALL be rendered with a distinct colored left border and optional background. Attached files (if any) SHALL be shown as MIME-type badges. Messages SHALL show a "QUEUED" badge while waiting for the agent to start processing.

#### Scenario: User message display
- **WHEN** the user submits "What files handle auth?"
- **THEN** the message area SHALL show the text with a colored left border

#### Scenario: Queued state
- **WHEN** the user submits a message and the turn has not started yet
- **THEN** the message SHALL show a "QUEUED" indicator until the first Token event arrives

### Requirement: Message Rendering — Assistant Messages
Assistant messages SHALL be rendered as markdown with syntax-highlighted code blocks. Messages consist of parts: text parts, thinking/reasoning parts, and tool call parts. Each part type has distinct styling.

#### Scenario: Streaming markdown
- **WHEN** Token events arrive during a turn
- **THEN** the assistant message SHALL be re-rendered with the accumulated text on each token
- **AND** partial markdown (e.g., unclosed code fence) SHALL be rendered as-is without errors

#### Scenario: Code blocks
- **WHEN** the assistant message contains a fenced code block with a language tag
- **THEN** it SHALL be rendered with syntax highlighting for that language

#### Scenario: Thinking/reasoning blocks
- **WHEN** the model emits thinking/reasoning content
- **THEN** it SHALL be rendered in muted/italic style, visually distinct from normal text
- **AND** thinking visibility SHALL be toggleable via keybinding

#### Scenario: Completed message
- **WHEN** a `MessageDone` event arrives
- **THEN** the message SHALL show a footer with: model name, duration, and "interrupted" if the turn was cancelled

### Requirement: Tool Call Display — Inline Tools
Read-only and simple tools SHALL be displayed as a single inline line showing: status icon (spinner while running, checkmark on success, X on error), tool name, and key arguments.

Inline tools: `file_read`, `glob_search`, `grep`, `list_dir`, `web_fetch`.

#### Scenario: Inline tool running
- **WHEN** a `ToolCallStart` for `grep` with pattern "auth" arrives
- **THEN** the display SHALL show: `⠋ Grep "auth" in src/`

#### Scenario: Inline tool completed
- **WHEN** a `ToolCallDone` for `grep` arrives with results
- **THEN** the display SHALL show: `✓ Grep "auth" in src/ → 12 matches in 4 files`

### Requirement: Tool Call Display — Block Tools
Tools with significant output SHALL be displayed as bordered blocks with a header (tool name + args) and a collapsible result area. Block tools: `shell`, `file_write`, `file_edit`, `apply_patch`.

#### Scenario: Shell tool running
- **WHEN** a `ToolCallStart` for `shell` with command "cargo test" arrives
- **THEN** a bordered block SHALL appear with the command and a spinner

#### Scenario: Block tool result collapsed
- **WHEN** a `ToolCallDone` arrives for a shell command with >10 lines of output
- **THEN** the result SHALL be collapsed by default, showing the first few lines
- **AND** a hint SHALL indicate how to expand ("Enter to expand")

#### Scenario: Block tool result expanded
- **WHEN** the user expands a collapsed tool result
- **THEN** the full output SHALL be visible within the block

### Requirement: Tool Call Display — Diff View
For `file_edit` and `apply_patch`, the tool result SHALL show a diff view:
- Unified diff by default
- Split diff when terminal width exceeds 120 columns
- Syntax highlighted
- Line numbers
- File path header

#### Scenario: Edit diff
- **WHEN** a `file_edit` tool completes
- **THEN** the tool block SHALL show a diff of the change (before → after)

#### Scenario: Apply patch diff
- **WHEN** an `apply_patch` tool completes modifying multiple files
- **THEN** each file SHALL get its own diff block with a label (Modified/Created/Deleted)

### Requirement: Tool Approval Flow
When `ToolCallPending` events arrive (from `enrich-event-model`), the TUI SHALL show an inline approval prompt within the tool block. The prompt SHALL offer: Allow (A or Enter), Reject (R), and optionally "Allow always" for the tool type.

This is NOT a modal dialog — it appears inline in the message flow.

#### Scenario: Tool needs approval
- **WHEN** a `ToolCallPending` event arrives for a shell command
- **THEN** the tool block SHALL show the command and an inline prompt: `[A]llow  [R]eject`

#### Scenario: Tool approved
- **WHEN** the user presses A or Enter on a pending tool
- **THEN** the TUI SHALL send approval and the tool SHALL execute

#### Scenario: Tool rejected
- **WHEN** the user presses R on a pending tool
- **THEN** the TUI SHALL send rejection with optional reason

### Requirement: Interruption and Cancellation
The TUI SHALL support interrupting active turns with clear visual feedback.

#### Scenario: Escape during turn
- **WHEN** Escape is pressed during an active turn
- **THEN** the TUI SHALL call `client.cancel_turn(session_id)`
- **AND** the footer SHALL show "Cancelling..."
- **AND** the partial assistant message SHALL remain visible with an "interrupted" indicator

#### Scenario: Ctrl+C with input text
- **WHEN** Ctrl+C is pressed and the input area contains text
- **THEN** the input area SHALL be cleared (not cancel the turn)

#### Scenario: Ctrl+C with empty input
- **WHEN** Ctrl+C is pressed and the input area is empty and no turn is active
- **THEN** the TUI SHALL exit cleanly

#### Scenario: Partial content after cancel
- **WHEN** a turn is cancelled
- **THEN** all partial content (tokens, tool calls) SHALL remain visible
- **AND** in-progress tool calls SHALL show a cancelled/error state
- **AND** the message footer SHALL show "interrupted"

### Requirement: Input
The TUI SHALL provide a multi-line text input area using tui-textarea with:
- Standard text editing (insert, delete, cursor movement)
- Emacs-style keybindings (C-a, C-e, C-k, C-n, C-p)
- Enter to submit, Shift+Enter / Alt+Enter for newline
- Dynamic height: grows with content up to 6 lines, scrolls internally beyond
- Input history: up/down arrow recalls previous messages when cursor is at start/end
- Undo/redo

#### Scenario: Submit message
- **WHEN** the user presses Enter with non-empty input
- **THEN** the TUI SHALL call `client.send_message(session_id, text)` and clear the input area

#### Scenario: Multi-line input
- **WHEN** the user presses Shift+Enter or Alt+Enter
- **THEN** a newline SHALL be inserted without submitting

#### Scenario: Dynamic resize
- **WHEN** the user types multiple lines
- **THEN** the input area SHALL grow up to 6 lines, then scroll internally

#### Scenario: Input history
- **WHEN** the user presses Up with the cursor at line 1, column 0
- **THEN** the input SHALL be replaced with the previous submitted message

### Requirement: Slash Commands
Input starting with `/` SHALL be intercepted as commands, not sent to the agent.

| Command | Action |
|---------|--------|
| `/help` | Show help overlay with keybindings and commands |
| `/new` | Create a new session |
| `/sessions` | Open session list overlay |
| `/model <name>` | Switch model |
| `/compact` | Trigger context compaction |
| `/thinking` | Toggle thinking/reasoning block visibility |
| `/quit` or `/exit` | Exit the application |

#### Scenario: Slash command autocomplete
- **WHEN** the user types `/` at the start of input
- **THEN** an autocomplete dropdown SHALL appear with matching commands

#### Scenario: Unknown command
- **WHEN** the user types `/foobar` and presses Enter
- **THEN** the TUI SHALL show "Unknown command: /foobar" without sending to the agent

### Requirement: Scrolling
The message area SHALL be scrollable with smart auto-scroll behavior.

#### Scenario: Auto-scroll at bottom
- **WHEN** the user is viewing the latest messages and new Token events arrive
- **THEN** the view SHALL scroll to keep the latest content visible

#### Scenario: Manual scroll preserved
- **WHEN** the user scrolls up to read earlier messages
- **THEN** new events SHALL NOT change the scroll position
- **AND** an indicator SHALL show new content is available below

#### Scenario: Scroll to bottom on submit
- **WHEN** the user submits a new message
- **THEN** the view SHALL scroll to the bottom and resume auto-scrolling

#### Scenario: Message navigation
- **WHEN** the user presses the "next message" keybinding
- **THEN** the view SHALL jump to the next message boundary

### Requirement: Session Management
The TUI SHALL provide session management via an overlay dialog.

#### Scenario: Session list overlay
- **WHEN** the user opens the session list (via `/sessions` or keybinding)
- **THEN** an overlay SHALL show all sessions grouped by date (Today, Yesterday, etc.)
- **AND** the user can search, select, rename, or delete sessions

#### Scenario: Session switching
- **WHEN** the user selects a different session from the list
- **THEN** the message area SHALL load that session's history and scroll to the bottom

#### Scenario: New session
- **WHEN** the user runs `/new` or presses the new session keybinding
- **THEN** a new session SHALL be created and the message area SHALL be cleared

### Requirement: Header
The header SHALL show (left to right): session title (or "New Chat"), model name, token usage (input/output), cost estimate (if available).

#### Scenario: Header during turn
- **WHEN** a turn is active
- **THEN** the header SHALL show the current model and update token counts as `TurnDone` events arrive

### Requirement: Footer
The footer SHALL show contextual information:
- **Idle**: cwd, git branch (if available), model name, help hint
- **During turn**: spinner + "esc to interrupt" + model name
- **Retry**: warning icon + "Retrying in Xs (attempt N)" + model name

#### Scenario: Footer during turn
- **WHEN** a turn is active
- **THEN** the footer SHALL show a spinner and "esc to interrupt"

#### Scenario: Footer retry
- **WHEN** a `Retry` event arrives
- **THEN** the footer SHALL show the retry countdown and attempt number

### Requirement: Overlay System
The TUI SHALL support overlays for dialogs (session list, help, model selector) that render on top of the message area without replacing it.

#### Scenario: Overlay does not destroy state
- **WHEN** a session list overlay is opened during a streaming response
- **THEN** the response SHALL continue streaming behind the overlay
- **AND** closing the overlay SHALL reveal the updated message area

#### Scenario: Overlay stacking
- **WHEN** an overlay opens while another is visible
- **THEN** the new overlay SHALL stack on top
- **AND** Escape SHALL close the top overlay

### Requirement: Keybindings
The TUI SHALL support a leader key system (default: Ctrl+X) for actions that don't conflict with editor keybindings. The leader key activates a "command mode" with a 2-second timeout.

| Key | Action |
|-----|--------|
| Escape | Cancel turn / close overlay |
| Ctrl+C | Clear input / exit if empty |
| Ctrl+P | Command palette |
| `<leader>n` | New session |
| `<leader>l` | Session list |
| `<leader>m` | Model selector |
| `<leader>b` | Toggle sidebar (Phase 3) |
| `<leader>t` | Theme selector (Phase 3) |
| `<leader>q` | Quit |
| Page Up / Page Down | Scroll |
| Home / End | First / last message |

#### Scenario: Leader key
- **WHEN** the user presses Ctrl+X
- **THEN** the TUI SHALL enter command mode (muted prompt, waiting for second key)
- **AND** if no second key within 2 seconds, command mode SHALL be cancelled

#### Scenario: Help discoverable
- **WHEN** the user opens the help overlay (via `/help` or keybinding)
- **THEN** all available keybindings SHALL be listed with descriptions

### Requirement: Error Display
Errors SHALL be displayed contextually:
- **API/provider errors**: in the assistant message block with error-colored border
- **Rate limits**: in the footer with retry countdown
- **Tool errors**: in the tool block with error background
- **Transient errors**: as brief toast notifications that auto-dismiss

#### Scenario: Rate limit with retry
- **WHEN** a `Retry` event arrives with attempt=2, max=3
- **THEN** the footer SHALL show "Rate limited, retrying in 5s (attempt 2/3)"

#### Scenario: Provider error
- **WHEN** a provider returns a 500 error with no retry
- **THEN** the assistant message SHALL show the error text with an error-colored border

#### Scenario: Tool error
- **WHEN** a tool execution fails
- **THEN** the tool block SHALL show the error with an error-colored background

### Requirement: Event Rendering
The TUI SHALL receive `ServerEvent` instances from `BrainApi::subscribe()` and render each appropriately:

| Event | Rendering |
|-------|-----------|
| `Token { delta }` | Append to current assistant message, re-render markdown |
| `ToolCallDelta { .. }` | Update tool call argument preview (block tools) |
| `ToolCallPending { .. }` | Show tool with approval prompt |
| `ToolCallStart { .. }` | Show tool with spinner |
| `ToolCallDone { .. }` | Mark tool complete, show result (collapsed if large) |
| `MessageDone { .. }` | Finalize message, show footer (model, duration) |
| `TurnDone { .. }` | Update token counts in header, restore idle footer |
| `Error { .. }` | Show in message or footer depending on type |
| `Progress { .. }` | Show progress in footer |
| `Retry { .. }` | Show retry countdown in footer |
| `Compaction { .. }` | Show brief compaction notice |
| `SessionStart { .. }` | Update header with session info |
| `SessionResume { .. }` | Update header, load history |

#### Scenario: All events handled
- **WHEN** any ServerEvent is received
- **THEN** it SHALL be rendered appropriately or ignored silently (never crash)

### Requirement: Feature Gate
The TUI SHALL be gated behind the `tui` feature flag. Building without the feature SHALL not compile ratatui, crossterm, or any TUI-related dependencies.

#### Scenario: Feature disabled
- **WHEN** compiled without the `tui` feature
- **THEN** `TuiApp` SHALL not be available

### Requirement: Color System
All colors SHALL be defined in a centralized theme struct, never hardcoded in widgets. This enables future theme switching without refactoring.

#### Scenario: Theme struct
- **WHEN** a widget needs a color (e.g., user message border)
- **THEN** it SHALL read from the theme struct, not use a hardcoded color

### Requirement: Accessibility
The TUI SHALL work correctly in:
- 256-color terminals
- True-color terminals (24-bit)
- Terminals with light or dark backgrounds

#### Scenario: Color fallback
- **WHEN** the terminal does not support true color
- **THEN** the TUI SHALL fall back to 256-color equivalents


## ADDED Requirements

### Requirement: TuiApp
The system SHALL provide a `TuiApp` that acts as a thin client of `BrainApi` (from `add-server-architecture`), rendering an interactive terminal UI using ratatui + crossterm. It does NOT implement the `Transport` trait. It communicates with the engine exclusively through `BrainApi` methods and the event subscription.

#### Scenario: Interactive session
- **WHEN** `TuiApp` is started with a `BrainApi` client
- **THEN** the user SHALL see a full-screen TUI with message history, streaming output, and a prompt input area

#### Scenario: Engine independence
- **WHEN** the TUI is running
- **THEN** it SHALL only interact with the engine via `BrainApi`
- **AND** it SHALL never call `Brain` methods directly

#### Scenario: Clean terminal restore
- **WHEN** the TUI exits (normally or via panic)
- **THEN** the terminal SHALL be restored to its original state (leave alternate screen, disable raw mode)

### Requirement: Layout
The TUI SHALL render a layout with four zones:
1. **Header** (1 row): session title, model name, token usage, cost estimate
2. **Message area** (flexible): scrollable list of conversation messages
3. **Input area** (1-6 rows, dynamic): multi-line text editor
4. **Footer** (1 row): contextual information and hints

The layout SHALL adapt to terminal size changes (responsive).

#### Scenario: Terminal resize
- **WHEN** the terminal is resized
- **THEN** the layout SHALL re-render to fit the new dimensions without losing state

#### Scenario: Small terminal
- **WHEN** the terminal is fewer than 80 columns wide
- **THEN** the layout SHALL still be usable (header/footer truncate, message area gets remaining space)

### Requirement: Message Rendering — User Messages
User messages SHALL be rendered with a distinct colored left border and optional background. Attached files (if any) SHALL be shown as MIME-type badges. Messages SHALL show a "QUEUED" badge while waiting for the agent to start processing.

#### Scenario: User message display
- **WHEN** the user submits "What files handle auth?"
- **THEN** the message area SHALL show the text with a colored left border

#### Scenario: Queued state
- **WHEN** the user submits a message and the turn has not started yet
- **THEN** the message SHALL show a "QUEUED" indicator until the first Token event arrives

### Requirement: Message Rendering — Assistant Messages
Assistant messages SHALL be rendered as markdown with syntax-highlighted code blocks. Messages consist of parts: text parts, thinking/reasoning parts, and tool call parts. Each part type has distinct styling.

#### Scenario: Streaming markdown
- **WHEN** Token events arrive during a turn
- **THEN** the assistant message SHALL be re-rendered with the accumulated text on each token
- **AND** partial markdown (e.g., unclosed code fence) SHALL be rendered as-is without errors

#### Scenario: Code blocks
- **WHEN** the assistant message contains a fenced code block with a language tag
- **THEN** it SHALL be rendered with syntax highlighting for that language

#### Scenario: Thinking/reasoning blocks
- **WHEN** the model emits thinking/reasoning content
- **THEN** it SHALL be rendered in muted/italic style, visually distinct from normal text
- **AND** thinking visibility SHALL be toggleable via keybinding

#### Scenario: Completed message
- **WHEN** a `MessageDone` event arrives
- **THEN** the message SHALL show a footer with: model name, duration, and "interrupted" if the turn was cancelled

### Requirement: Tool Call Display — Inline Tools
Read-only and simple tools SHALL be displayed as a single inline line showing: status icon (spinner while running, checkmark on success, X on error), tool name, and key arguments.

Inline tools: `file_read`, `glob_search`, `grep`, `list_dir`, `web_fetch`.

#### Scenario: Inline tool running
- **WHEN** a `ToolCallStart` for `grep` with pattern "auth" arrives
- **THEN** the display SHALL show: `⠋ Grep "auth" in src/`

#### Scenario: Inline tool completed
- **WHEN** a `ToolCallDone` for `grep` arrives with results
- **THEN** the display SHALL show: `✓ Grep "auth" in src/ → 12 matches in 4 files`

### Requirement: Tool Call Display — Block Tools
Tools with significant output SHALL be displayed as bordered blocks with a header (tool name + args) and a collapsible result area. Block tools: `shell`, `file_write`, `file_edit`, `apply_patch`.

#### Scenario: Shell tool running
- **WHEN** a `ToolCallStart` for `shell` with command "cargo test" arrives
- **THEN** a bordered block SHALL appear with the command and a spinner

#### Scenario: Block tool result collapsed
- **WHEN** a `ToolCallDone` arrives for a shell command with >10 lines of output
- **THEN** the result SHALL be collapsed by default, showing the first few lines
- **AND** a hint SHALL indicate how to expand ("Enter to expand")

#### Scenario: Block tool result expanded
- **WHEN** the user expands a collapsed tool result
- **THEN** the full output SHALL be visible within the block

### Requirement: Tool Call Display — Diff View
For `file_edit` and `apply_patch`, the tool result SHALL show a diff view:
- Unified diff by default
- Split diff when terminal width exceeds 120 columns
- Syntax highlighted
- Line numbers
- File path header

#### Scenario: Edit diff
- **WHEN** a `file_edit` tool completes
- **THEN** the tool block SHALL show a diff of the change (before → after)

#### Scenario: Apply patch diff
- **WHEN** an `apply_patch` tool completes modifying multiple files
- **THEN** each file SHALL get its own diff block with a label (Modified/Created/Deleted)

### Requirement: Tool Approval Flow
When `ToolCallPending` events arrive (from `enrich-event-model`), the TUI SHALL show an inline approval prompt within the tool block. The prompt SHALL offer: Allow (A or Enter), Reject (R), and optionally "Allow always" for the tool type.

This is NOT a modal dialog — it appears inline in the message flow.

#### Scenario: Tool needs approval
- **WHEN** a `ToolCallPending` event arrives for a shell command
- **THEN** the tool block SHALL show the command and an inline prompt: `[A]llow  [R]eject`

#### Scenario: Tool approved
- **WHEN** the user presses A or Enter on a pending tool
- **THEN** the TUI SHALL send approval and the tool SHALL execute

#### Scenario: Tool rejected
- **WHEN** the user presses R on a pending tool
- **THEN** the TUI SHALL send rejection with optional reason

### Requirement: Interruption and Cancellation
The TUI SHALL support interrupting active turns with clear visual feedback.

#### Scenario: Escape during turn
- **WHEN** Escape is pressed during an active turn
- **THEN** the TUI SHALL call `client.cancel_turn(session_id)`
- **AND** the footer SHALL show "Cancelling..."
- **AND** the partial assistant message SHALL remain visible with an "interrupted" indicator

#### Scenario: Ctrl+C with input text
- **WHEN** Ctrl+C is pressed and the input area contains text
- **THEN** the input area SHALL be cleared (not cancel the turn)

#### Scenario: Ctrl+C with empty input
- **WHEN** Ctrl+C is pressed and the input area is empty and no turn is active
- **THEN** the TUI SHALL exit cleanly

#### Scenario: Partial content after cancel
- **WHEN** a turn is cancelled
- **THEN** all partial content (tokens, tool calls) SHALL remain visible
- **AND** in-progress tool calls SHALL show a cancelled/error state
- **AND** the message footer SHALL show "interrupted"

### Requirement: Input
The TUI SHALL provide a multi-line text input area using tui-textarea with:
- Standard text editing (insert, delete, cursor movement)
- Emacs-style keybindings (C-a, C-e, C-k, C-n, C-p)
- Enter to submit, Shift+Enter / Alt+Enter for newline
- Dynamic height: grows with content up to 6 lines, scrolls internally beyond
- Input history: up/down arrow recalls previous messages when cursor is at start/end
- Undo/redo

#### Scenario: Submit message
- **WHEN** the user presses Enter with non-empty input
- **THEN** the TUI SHALL call `client.send_message(session_id, text)` and clear the input area

#### Scenario: Multi-line input
- **WHEN** the user presses Shift+Enter or Alt+Enter
- **THEN** a newline SHALL be inserted without submitting

#### Scenario: Dynamic resize
- **WHEN** the user types multiple lines
- **THEN** the input area SHALL grow up to 6 lines, then scroll internally

#### Scenario: Input history
- **WHEN** the user presses Up with the cursor at line 1, column 0
- **THEN** the input SHALL be replaced with the previous submitted message

### Requirement: Slash Commands
Input starting with `/` SHALL be intercepted as commands, not sent to the agent.

| Command | Action |
|---------|--------|
| `/help` | Show help overlay with keybindings and commands |
| `/new` | Create a new session |
| `/sessions` | Open session list overlay |
| `/model <name>` | Switch model |
| `/compact` | Trigger context compaction |
| `/thinking` | Toggle thinking/reasoning block visibility |
| `/quit` or `/exit` | Exit the application |

#### Scenario: Slash command autocomplete
- **WHEN** the user types `/` at the start of input
- **THEN** an autocomplete dropdown SHALL appear with matching commands

#### Scenario: Unknown command
- **WHEN** the user types `/foobar` and presses Enter
- **THEN** the TUI SHALL show "Unknown command: /foobar" without sending to the agent

### Requirement: Scrolling
The message area SHALL be scrollable with smart auto-scroll behavior.

#### Scenario: Auto-scroll at bottom
- **WHEN** the user is viewing the latest messages and new Token events arrive
- **THEN** the view SHALL scroll to keep the latest content visible

#### Scenario: Manual scroll preserved
- **WHEN** the user scrolls up to read earlier messages
- **THEN** new events SHALL NOT change the scroll position
- **AND** an indicator SHALL show new content is available below

#### Scenario: Scroll to bottom on submit
- **WHEN** the user submits a new message
- **THEN** the view SHALL scroll to the bottom and resume auto-scrolling

#### Scenario: Message navigation
- **WHEN** the user presses the "next message" keybinding
- **THEN** the view SHALL jump to the next message boundary

### Requirement: Session Management
The TUI SHALL provide session management via an overlay dialog.

#### Scenario: Session list overlay
- **WHEN** the user opens the session list (via `/sessions` or keybinding)
- **THEN** an overlay SHALL show all sessions grouped by date (Today, Yesterday, etc.)
- **AND** the user can search, select, rename, or delete sessions

#### Scenario: Session switching
- **WHEN** the user selects a different session from the list
- **THEN** the message area SHALL load that session's history and scroll to the bottom

#### Scenario: New session
- **WHEN** the user runs `/new` or presses the new session keybinding
- **THEN** a new session SHALL be created and the message area SHALL be cleared

### Requirement: Header
The header SHALL show (left to right): session title (or "New Chat"), model name, token usage (input/output), cost estimate (if available).

#### Scenario: Header during turn
- **WHEN** a turn is active
- **THEN** the header SHALL show the current model and update token counts as `TurnDone` events arrive

### Requirement: Footer
The footer SHALL show contextual information:
- **Idle**: cwd, git branch (if available), model name, help hint
- **During turn**: spinner + "esc to interrupt" + model name
- **Retry**: warning icon + "Retrying in Xs (attempt N)" + model name

#### Scenario: Footer during turn
- **WHEN** a turn is active
- **THEN** the footer SHALL show a spinner and "esc to interrupt"

#### Scenario: Footer retry
- **WHEN** a `Retry` event arrives
- **THEN** the footer SHALL show the retry countdown and attempt number

### Requirement: Overlay System
The TUI SHALL support overlays for dialogs (session list, help, model selector) that render on top of the message area without replacing it.

#### Scenario: Overlay does not destroy state
- **WHEN** a session list overlay is opened during a streaming response
- **THEN** the response SHALL continue streaming behind the overlay
- **AND** closing the overlay SHALL reveal the updated message area

#### Scenario: Overlay stacking
- **WHEN** an overlay opens while another is visible
- **THEN** the new overlay SHALL stack on top
- **AND** Escape SHALL close the top overlay

### Requirement: Keybindings
The TUI SHALL support a leader key system (default: Ctrl+X) for actions that don't conflict with editor keybindings. The leader key activates a "command mode" with a 2-second timeout.

| Key | Action |
|-----|--------|
| Escape | Cancel turn / close overlay |
| Ctrl+C | Clear input / exit if empty |
| Ctrl+P | Command palette |
| `<leader>n` | New session |
| `<leader>l` | Session list |
| `<leader>m` | Model selector |
| `<leader>b` | Toggle sidebar (Phase 3) |
| `<leader>t` | Theme selector (Phase 3) |
| `<leader>q` | Quit |
| Page Up / Page Down | Scroll |
| Home / End | First / last message |

#### Scenario: Leader key
- **WHEN** the user presses Ctrl+X
- **THEN** the TUI SHALL enter command mode (muted prompt, waiting for second key)
- **AND** if no second key within 2 seconds, command mode SHALL be cancelled

#### Scenario: Help discoverable
- **WHEN** the user opens the help overlay (via `/help` or keybinding)
- **THEN** all available keybindings SHALL be listed with descriptions

### Requirement: Error Display
Errors SHALL be displayed contextually:
- **API/provider errors**: in the assistant message block with error-colored border
- **Rate limits**: in the footer with retry countdown
- **Tool errors**: in the tool block with error background
- **Transient errors**: as brief toast notifications that auto-dismiss

#### Scenario: Rate limit with retry
- **WHEN** a `Retry` event arrives with attempt=2, max=3
- **THEN** the footer SHALL show "Rate limited, retrying in 5s (attempt 2/3)"

#### Scenario: Provider error
- **WHEN** a provider returns a 500 error with no retry
- **THEN** the assistant message SHALL show the error text with an error-colored border

#### Scenario: Tool error
- **WHEN** a tool execution fails
- **THEN** the tool block SHALL show the error with an error-colored background

### Requirement: Event Rendering
The TUI SHALL receive `ServerEvent` instances from `BrainApi::subscribe()` and render each appropriately:

| Event | Rendering |
|-------|-----------|
| `Token { delta }` | Append to current assistant message, re-render markdown |
| `ToolCallDelta { .. }` | Update tool call argument preview (block tools) |
| `ToolCallPending { .. }` | Show tool with approval prompt |
| `ToolCallStart { .. }` | Show tool with spinner |
| `ToolCallDone { .. }` | Mark tool complete, show result (collapsed if large) |
| `MessageDone { .. }` | Finalize message, show footer (model, duration) |
| `TurnDone { .. }` | Update token counts in header, restore idle footer |
| `Error { .. }` | Show in message or footer depending on type |
| `Progress { .. }` | Show progress in footer |
| `Retry { .. }` | Show retry countdown in footer |
| `Compaction { .. }` | Show brief compaction notice |
| `SessionStart { .. }` | Update header with session info |
| `SessionResume { .. }` | Update header, load history |

#### Scenario: All events handled
- **WHEN** any ServerEvent is received
- **THEN** it SHALL be rendered appropriately or ignored silently (never crash)

### Requirement: Feature Gate
The TUI SHALL be gated behind the `tui` feature flag. Building without the feature SHALL not compile ratatui, crossterm, or any TUI-related dependencies.

#### Scenario: Feature disabled
- **WHEN** compiled without the `tui` feature
- **THEN** `TuiApp` SHALL not be available

### Requirement: Color System
All colors SHALL be defined in a centralized theme struct, never hardcoded in widgets. This enables future theme switching without refactoring.

#### Scenario: Theme struct
- **WHEN** a widget needs a color (e.g., user message border)
- **THEN** it SHALL read from the theme struct, not use a hardcoded color

### Requirement: Accessibility
The TUI SHALL work correctly in:
- 256-color terminals
- True-color terminals (24-bit)
- Terminals with light or dark backgrounds

#### Scenario: Color fallback
- **WHEN** the terminal does not support true color
- **THEN** the TUI SHALL fall back to 256-color equivalents
