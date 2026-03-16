# Design: add-tui-transport

## UX Reference Analysis

Based on deep analysis of OpenCode and pi-mono TUIs — the two most polished
AI coding agent terminal interfaces.

---

## Message Rendering

### Assistant Messages (the hard part)

**Streaming markdown is the core challenge.** The model streams tokens one at
a time. At any moment, the partial text might be mid-word, mid-code-block, or
mid-bold. The renderer must handle all these states gracefully.

**OpenCode approach**: passes `streaming={true}` to both `<markdown>` and
`<code>` components. Partial markdown is rendered as-is; no special buffering.
When a code fence opens but hasn't closed, it's rendered as an open code block.

**pi-mono approach**: same — incremental updates to the streaming component,
markdown rendered as it arrives.

**Our approach**: render partial markdown on every token. Accept that mid-fence
states look slightly rough. This is what users expect from ChatGPT, Claude, etc.

### Message Types

| Type | Visual Treatment |
|------|-----------------|
| **User** | Colored left border, distinct background, show attached files as badges |
| **Assistant** | Markdown rendered, code blocks syntax-highlighted, thinking/reasoning blocks in muted style |
| **Tool call** | Inline or block depending on tool type (see below) |
| **Error** | Error-colored border, error message text |
| **System** | Muted text, not always visible |

### Thinking/Reasoning Blocks

Models increasingly emit "thinking" content. Both OpenCode and pi-mono render
it in a muted/italic style, toggleable via a keybinding. We should support this.

---

## Tool Call UX

### Two display modes (following OpenCode)

**Inline tools** (single line, minimal): for read-only or simple tools.
Show tool name + key args + status icon (spinner → checkmark/error).

```
  ⠋ Reading src/main.rs
  ✓ Read src/main.rs (45 lines)
  ✓ Grep "auth" in src/ → 12 matches in 4 files
  ✓ Glob **/*.rs → 23 files
```

**Block tools** (bordered block, expandable): for tools with significant output.
Show tool name + args, then collapsible result.

```
  ┌ Shell ─────────────────────────────────────
  │ $ cargo test --workspace
  │ ────────────────────────────────────────────
  │   running 42 tests
  │   test foo ... ok
  │   ... (click to expand)
  └─────────────────────────────────────────────
```

### Tool categorization

| Category | Tools | Display |
|----------|-------|---------|
| Inline | file_read, glob_search, grep, list_dir, web_fetch | Single line |
| Block | shell, file_write, file_edit, apply_patch | Bordered block |
| Block (diff) | file_edit, apply_patch | Show diff view |

### Tool approval flow

When tool approval is enabled (from `enrich-event-model`):

1. `ToolCallPending` event arrives
2. Tool block appears with pending state and "Allow / Reject" prompt
3. User presses A (allow), R (reject), or Enter (allow)
4. Tool executes or is rejected
5. Block updates with result

OpenCode renders this inline (not modal) above the prompt. pi-mono has no
approval flow. We follow OpenCode — inline, non-modal.

### Tool progress

- Spinner animation while tool is running (Knight Rider style from OpenCode,
  or simple braille spinner)
- Tool name + args visible during execution
- Duration shown after completion

---

## Interruption / Cancellation

This is critical UX. Users need to be able to interrupt the AI at any point.

### OpenCode's model

1. **Escape once**: sets interrupt flag, shows "esc again to interrupt" in footer
2. **Escape twice (within 5s)**: sends abort to server, kills the turn
3. **Ctrl+C when input empty**: exit app
4. **Ctrl+C when input has text**: clear input

### pi-mono's model

1. **Escape**: abort immediately
2. **Ctrl+C once**: clear input
3. **Ctrl+C twice (within 500ms)**: exit app

### Our model (combining both)

1. **Escape during turn**: cancel the turn via `client.cancel_turn()`
2. **Escape when idle**: no-op (or show help hint)
3. **Ctrl+C when input has text**: clear input
4. **Ctrl+C when input empty**: exit app
5. **Ctrl+C during turn**: cancel the turn (same as Escape)

### What happens to partial content

- Partial assistant message stays visible
- Tool calls in progress show error/cancelled state
- Footer shows "interrupted" indicator on the message
- The turn's messages are still persisted (partial content + cancellation)

---

## Subagents / Child Sessions

OpenCode has first-class subagent support:

- Child sessions are separate sessions with `parentID`
- Task tool shows inline summary + link to child session
- Navigation: `<leader>down` to child, `up` to parent, `left`/`right` to cycle
- Header in child view shows "Subagent session" with Parent/Prev/Next nav

### Our approach

For MVP, we don't need subagents. But the message rendering and session
model should be designed to support them later:

- Messages can reference child sessions
- The session model supports parentID
- Navigation can be added without restructuring the UI

---

## Session Management

### Session list (overlay)

Both OpenCode and pi-mono use an overlay/dialog for session list:

- Fuzzy search
- Grouped by date (Today, Yesterday, This Week, etc.)
- Shows title, date, message count
- Delete with double-confirm
- Rename
- Keybindings: Enter to open, Ctrl+D to delete, Ctrl+R to rename

### Session switching

- Selecting a session loads its history and scrolls to bottom
- Current session title shown in header
- New session via `/new` or keybinding

---

## Input UX

### Submit vs newline

Both tools use the same convention:
- **Enter**: submit message
- **Shift+Enter / Alt+Enter / Ctrl+Enter**: insert newline

### Slash commands

Triggered by `/` at start of input:
- Fuzzy-matched autocomplete dropdown
- Commands: `/help`, `/new`, `/sessions`, `/model`, `/quit`, `/compact`
- Arguments after command name

### Autocomplete

- `@` for file/path completion (with fuzzy search)
- `/` for commands
- Tab to complete, Escape to dismiss

### Input history

- Up/Down arrows recall previous messages
- Only when cursor is at start/end of input

### Input stash (OpenCode)

Save current draft, type something else, restore later. Nice for context
switching mid-thought. OpenCode has stash/pop/list.

### Textarea resize

- Grows with content up to a max height (6 lines in OpenCode)
- Scrolls internally beyond max height

---

## Scrolling

### Auto-scroll behavior

- When user is at the bottom: auto-scroll as new content arrives
- When user has scrolled up: freeze scroll position, show indicator
- Scroll-to-bottom on Enter (submitting message)

### Navigation keybindings (OpenCode)

| Key | Action |
|-----|--------|
| Page Up / Page Down | Page scroll |
| Ctrl+G / Home | First message |
| Ctrl+Alt+G / End | Last message |
| Message navigation | Jump to next/prev message boundary |

### Scroll acceleration

Configurable speed multiplier for scroll wheel / page scroll.

---

## Diff Display

When the agent modifies files, show the diff:

- **Unified diff** (default, narrow terminals)
- **Split diff** (wide terminals, >120 cols) — OpenCode auto-switches
- Syntax highlighted
- Line numbers
- File path header
- Collapsible (large diffs collapsed by default)

For `file_edit`: show before/after diff of the change.
For `apply_patch`: show per-file diffs with created/deleted/modified labels.

---

## Error Handling

### Error types and display

| Error | Display |
|-------|---------|
| API/network error | Error message in assistant message block, error-colored border |
| Rate limit | "Retrying in Xs, attempt #N" with countdown (OpenCode) |
| Token limit exceeded | Warning in header (context % indicator) |
| Tool error | Error in tool block, error-colored background |
| Auth error | Prompt to login / re-authenticate |

### Toast notifications

For transient errors (network hiccups, MCP server disconnect):
brief notification that auto-dismisses. OpenCode uses toasts.

---

## Sidebar

OpenCode's sidebar (42 cols, toggleable):

| Section | Content |
|---------|---------|
| Context | Token count, % used, cost estimate |
| MCP | Connected servers, status |
| Diff | Modified files with +/- line counts |
| Todo | Non-completed items from agent |
| CWD | Current working directory |

Toggle via keybinding. Auto-show when terminal is wide enough (>120 cols).
Hidden in child sessions.

### Our approach

Phase 3. The sidebar is useful but not essential for MVP. The header/footer
can carry the most critical info (model, tokens, session) initially.

---

## Keybinding System

### Leader key (OpenCode)

Ctrl+X as leader, then a letter within 2s timeout. Blurs input focus while
waiting for the second key. Avoids conflicts with standard editor bindings.

### Discoverability

- Help overlay (all keybindings listed)
- Command palette (Ctrl+P) shows keybindings per command
- Footer hints for contextual actions
- Inline hints in tool output ("press X to expand")

### Configuration

Keybindings should be overridable via config (both OpenCode and pi-mono
support this). Low priority but the architecture should support it.

---

## Theme System

OpenCode has 30+ themes. pi-mono has a simpler MarkdownTheme.

### Minimum viable theme

Define colors for:
- User message (border, background)
- Assistant message (text, code background)
- Tool call (pending, success, error backgrounds)
- Header / footer (background, text)
- Input area (border, background)
- Diff (added, removed, context)
- Error / warning / info / success
- Muted / secondary text

### Phase 3 concern

Full theme system (switchable, custom themes) is Phase 3. For MVP, one
good dark theme is sufficient. But the color system should be centralized
(not hardcoded in widgets) so themes can be added later.

---

## Status Indicators

### Footer content (combining OpenCode + pi-mono)

```
 ~/projects/brain  main  openai/gpt-4o  ↑1.2k ↓3.4k  $0.02  │ /help
```

Left: cwd, git branch, model name
Right: token stats (in/out), cost, help hint

### During turn

```
 ⠋ Generating...  esc to interrupt  │ openai/gpt-4o
```

Spinner + "esc to interrupt" replaces normal footer during active turn.

### Retry state

```
 ⚠ Rate limited, retrying in 5s (attempt 2/3)  │ openai/gpt-4o
```
