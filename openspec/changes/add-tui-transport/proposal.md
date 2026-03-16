# Proposal: add-tui-transport

## Why

brain's only transport is `CliTransport` — a line-based stdin/stdout reader.
It works for testing but the experience is nothing like modern AI coding
agents (OpenCode, Codex, Claude Code). Users expect:

- Streaming token output with proper formatting
- Markdown rendering with syntax-highlighted code blocks
- Visual tool call status (spinner → name → result)
- Scrollable message history
- Multi-line input with editing
- Session info, model info, token counts
- Keyboard shortcuts for common actions
- The ability to interrupt/cancel a running turn

To build a TUI that competes with these products, we need a proper `TuiTransport`
backed by a real terminal UI framework.

## Library Selection

### Core: ratatui + crossterm

**ratatui** (19K+ stars) is the dominant Rust TUI library. Immediate-mode
rendering, rich widget ecosystem, cross-platform via crossterm backend.
No real alternative in Rust — it's the clear choice.

**crossterm** is ratatui's default terminal backend. Cross-platform
(Linux, macOS, Windows), handles raw mode, alternate screen, mouse events,
and keyboard input.

Added to `repocache/repocache.json` for source-first reference.

### Text input: tui-textarea

**tui-textarea** (489 stars) provides a multiline text editor widget for
ratatui with Emacs keybindings, undo/redo, search, selection, and yank.
Exactly what we need for the prompt input area.

Added to `repocache/repocache.json` for source-first reference.

### Markdown rendering: tui-markdown

**tui-markdown** converts markdown to ratatui `Text` widgets using
`pulldown-cmark`. Supports syntax-highlighted code blocks via `syntect`.
This handles rendering assistant responses with proper formatting.

### Syntax highlighting: syntect

**syntect** is the standard Rust syntax highlighting library (used by bat,
delta, etc.). `tui-markdown` uses it under the hood for code blocks. We
may also use it directly for highlighting diffs and tool outputs.

### Full dependency list

| Crate | Purpose | Notes |
|-------|---------|-------|
| `ratatui` | TUI framework | Core rendering, layout, widgets |
| `crossterm` | Terminal backend | Raw mode, events, alternate screen |
| `tui-textarea` | Multi-line input | Prompt editing with keybindings |
| `tui-markdown` | Markdown rendering | Assistant message formatting |
| `syntect` | Syntax highlighting | Code blocks in markdown |

## Reference: OpenCode's TUI Layout

OpenCode has the most polished coding-agent TUI. Their layout (via their
custom OpenTUI + SolidJS framework):

```
┌────────────────────────────────────────────────────┬──────────┐
│ Header: session title | model | token count | cost │ Sidebar  │
├────────────────────────────────────────────────────┤ (toggle) │
│                                                    │          │
│  Message scroll area                               │ Context  │
│    ┌ User message ─────────────────────────────┐   │ MCP      │
│    │ What files handle auth?                   │   │ Diff     │
│    └───────────────────────────────────────────┘   │ Todos    │
│    ┌ Assistant message ────────────────────────┐   │          │
│    │ Streaming tokens with markdown...         │   │          │
│    │ ```rust                                   │   │          │
│    │ fn main() { ... }                         │   │          │
│    │ ```                                       │   │          │
│    │ ┌ Tool: grep ─────────────────────────┐   │   │          │
│    │ │ ▶ searching for "auth"...           │   │   │          │
│    │ │ Found 12 matches in 4 files         │   │   │          │
│    │ └────────────────────────────────────-┘   │   │          │
│    └───────────────────────────────────────────┘   │          │
│                                                    │          │
├────────────────────────────────────────────────────┤          │
│ ┌ Input ───────────────────────────────────────┐   │          │
│ │ > _                                          │   │          │
│ └──────────────────────────────────────────────┘   │          │
├────────────────────────────────────────────────────┴──────────┤
│ Footer: cwd | provider | session id | /help                   │
└───────────────────────────────────────────────────────────────┘
```

Key UX patterns:
- Leader key (`Ctrl+X`) + letter for actions
- Slash commands (`/sessions`, `/model`, `/help`, `/new`)
- Vim-like scrolling (j/k, Ctrl+D/U, G/gg)
- Sidebar toggle for context info
- Theme system with many built-in themes

## What

### TuiApp

A TUI application (in `brain-tui` crate or `brain-cli` directly) that acts
as a thin client of `BrainApi` (from `add-server-architecture`), rendering
to the terminal using ratatui:

1. **Layout**: header + scrollable message area + input + footer
2. **Message rendering**: markdown with syntax-highlighted code blocks
3. **Streaming tokens**: real-time token display as events arrive
4. **Tool call display**: name + spinner → collapsible result
5. **Input**: multi-line editor with tui-textarea
6. **Scrolling**: vim-style keybindings for message history
7. **Status**: model, session, token count, cwd in header/footer
8. **Slash commands**: `/help`, `/sessions`, `/new`, `/model`, `/quit`
9. **Cancel**: Ctrl+C or Escape cancels current turn
10. **Resize**: responsive layout adapts to terminal size

### BrainApi client integration

The TUI is a client of `BrainApi` (from `add-server-architecture`), NOT a
`Transport` implementation. It:

- Calls `client.send_message(session_id, content)` when the user submits input
- Subscribes to `client.subscribe()` for engine events
- Calls `client.cancel_turn(session_id)` on Ctrl+C
- Calls `client.list_sessions()`, `client.create_session()`, etc. for session management

The TUI never touches Brain directly. It only knows `BrainApi`.

### Event loop architecture

The TUI multiplexes two event sources:
1. Terminal events (keyboard, mouse, resize) — from crossterm
2. Engine events (tokens, tool calls, etc.) — from `BrainApi::subscribe()`

Both flow into a single `tokio::select!` loop that updates TUI state and
re-renders. The TUI is purely reactive — it renders state, never drives
the engine.

### Phased delivery

**Phase 1: Functional MVP**
- Basic layout: message area + input + status bar
- Streaming token display
- Tool call display (name + result, no collapse)
- Single session, no sidebar
- Ctrl+C to cancel

**Phase 2: Full featured**
- Markdown rendering with syntax highlighting
- Collapsible tool call results
- Slash commands
- Session switching
- Scrollable history with vim keybindings

**Phase 3: Polish**
- Sidebar with context info
- Theme system
- Leader key shortcuts
- Mouse support
- Clipboard integration

## Change Dependencies

- **Requires**: `add-server-architecture` (TUI is a BrainApi client)
- **Requires**: `enrich-event-model` (TUI renders all enriched event variants)
- **Requires**: `add-brain-cli` (TUI lives in or alongside brain-cli)

## Impact

- **New crate or module**: `brain-tui` or inline in `brain-cli`
- **New spec**: `tui-transport` (name kept for continuity, though TUI is a
  BrainApi client, not a Transport impl)
- **Modifies**: `brain-cli` spec (TUI as default frontend)
- **Dependencies**: ratatui, crossterm, tui-textarea, tui-markdown, syntect
- **Repocache**: `ratatui` and `tui-textarea` added for source-first reference
