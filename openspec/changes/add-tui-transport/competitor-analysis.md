# Competitor Analysis: AI Coding Agent CLI/TUI

Deep source-level analysis of 6 competitors to inform brain's TUI and CLI design.
All analysis based on repocache clones (direct source inspection, not marketing docs).

---

## 1. Product Overview

| Product | Language | Stars | TUI Framework | Architecture |
|---------|----------|-------|---------------|-------------|
| **OpenCode** | TypeScript/Bun | 120K+ | OpenTUI + Solid.js | Worker + HTTP/SSE (in-process or remote) |
| **Codex RS** | Rust | 65K+ | ratatui + crossterm (forked) | App-server + in-process/WebSocket client |
| **ZeroClaw** | Rust | 26K+ | None (CLI + web dashboard) | Gateway daemon + REST/SSE/WebSocket |
| **Pi-mono** | TypeScript | 1.9K+ | Custom TUI (pi-tui, differential rendering) | RPC over stdin/stdout JSON lines |
| **OpenClaw** | TypeScript | 312K+ | pi-tui (borrowed from pi-mono) | Gateway WebSocket + HTTP |
| **IronClaw** | Rust | — | None (terminal readline + web UI) | Engine in-process, gateway (scaffold) |

---

## 2. CLI Commands Comparison

### Top-Level Commands

| Command | OpenCode | Codex RS | ZeroClaw | Pi-mono | OpenClaw | IronClaw | brain (current) |
|---------|----------|----------|----------|---------|----------|----------|-----------------|
| Default (chat) | `opencode` → TUI | `codex` → TUI | `zeroclaw agent` | `pi` → TUI | `openclaw` (no default) | `ironclaw run` | `brain` → stdin chat |
| Serve/daemon | `opencode serve` | `codex app-server` | `zeroclaw daemon` | `pi --mode rpc` | `openclaw gateway` | `ironclaw ui` | `examples/server` |
| Attach to server | `opencode attach <url>` | `codex --remote ws://` | N/A (API clients) | N/A | `openclaw tui --url` | N/A | **MISSING** |
| Run (non-interactive) | `opencode run [msg]` | `codex exec [msg]` | N/A | `pi --mode text` | `openclaw message send` | N/A | **MISSING** |
| Login/auth | `opencode account` | `codex login` | `zeroclaw auth login` | `pi-ai login` | `openclaw configure` | `ironclaw onboard` | `brain credentials login` |
| Logout | N/A | `codex logout` | `zeroclaw auth logout` | `pi-ai logout` | N/A | N/A | `brain credentials remove` |
| Session list | `opencode session list` | `codex resume --all` | N/A | `pi --resume` (picker) | `openclaw sessions` | N/A | `brain sessions list` |
| Session resume | `opencode run -c` | `codex resume` | N/A | `pi -c` / `pi -r` | N/A | N/A | `brain sessions resume` |
| Session fork | `opencode run --fork` | `codex fork` | N/A | `/fork` (TUI only) | N/A | N/A | **MISSING** |
| Models/providers | `opencode models` | N/A | `zeroclaw models` | `pi --list-models` | `openclaw models` | `ironclaw models` | **MISSING** |
| Config init | N/A | N/A | `zeroclaw onboard` | `pi config` | `openclaw setup` | `ironclaw onboard` | **MISSING** |
| MCP management | `opencode mcp` | `codex mcp` | N/A | N/A (extensions) | `openclaw plugins` | N/A | **MISSING** |
| Export | `opencode export` | N/A | N/A | `pi --export` | N/A | N/A | **MISSING** |
| Health/status | N/A | N/A | `zeroclaw status` | N/A | `openclaw health` | `ironclaw doctor` | **MISSING** |
| PR/GitHub | `opencode pr` | N/A | N/A | N/A | N/A | N/A | **MISSING** |
| Code review | N/A | `codex review` | N/A | N/A | N/A | N/A | **MISSING** |
| Sandbox | N/A | `codex sandbox` | N/A | N/A | `openclaw sandbox` | N/A | **MISSING** |
| Debug | `opencode debug` | `codex debug` | `zeroclaw doctor` | N/A | `openclaw doctor` | `ironclaw doctor` | **MISSING** |
| Completions | N/A | `codex completion` | `zeroclaw completions` | N/A | `openclaw completion` | N/A | **MISSING** |
| Upgrade/update | `opencode upgrade` | N/A | N/A | N/A | `openclaw update` | N/A | **MISSING** |

### Key CLI Flags

| Flag | OpenCode | Codex RS | Pi-mono | brain (current) |
|------|----------|----------|---------|-----------------|
| `--model` / `-m` | `run --model` | `-m` | `--model` / `--provider` | **MISSING** |
| `--continue` / `-c` | `run -c` | N/A | `-c` | **MISSING** |
| `--session` / `-s` | `run -s <id>` | N/A | `--session <id>` | **MISSING** |
| `--format json` | `run --format json` | N/A | `--mode json` | **MISSING** |
| `@file` references | Yes (TUI) | N/A | `@files...` positional | **MISSING** |
| `!bash` shortcut | N/A | N/A | `!command` / `!!command` | **MISSING** |

---

## 3. Serve / Attach Architecture

This is a **critical architectural gap** in our current spec. Multiple competitors support a client-server split where:
1. A server/daemon runs the agent engine
2. Multiple clients (TUI, web, IDE) can attach to it

| Feature | OpenCode | Codex RS | ZeroClaw | OpenClaw | brain (current) |
|---------|----------|----------|----------|----------|-----------------|
| **Headless server** | `opencode serve` | `codex app-server` | `zeroclaw daemon` | `openclaw gateway` | `examples/server` (demo only) |
| **TUI attaches to server** | `opencode attach <url>` | `codex --remote ws://` | N/A (web clients) | `openclaw tui --url` | **MISSING** |
| **Protocol** | HTTP + SSE | JSON-RPC (stdio or WebSocket) | REST + SSE + WebSocket | WebSocket RPC | REST + SSE (exists but not exposed via CLI) |
| **Auth** | Basic auth (password) | N/A (local only) | Pairing code → bearer token | Token/password | **MISSING** |
| **In-process mode** | Worker thread, same process | `InProcessAppServerClient` | N/A | N/A | `BrainServer::new() + server.client()` |
| **Multi-client** | Yes (serve + multiple attach) | Yes (app-server + clients) | Yes (gateway + channels) | Yes (gateway + TUI/web/channels) | **No** |

### Implications for brain

Our `BrainServer` already has the server architecture (REST + SSE), and `brain-cli` already uses `server.client()` in-process. The key missing pieces are:

1. **`brain serve`** — run headless HTTP server (we have `examples/server` but not a CLI subcommand)
2. **`brain attach <url>`** — connect TUI to a remote `brain serve` instance
3. **Remote `BrainApi` client** — HTTP client implementing `BrainApi` trait
4. **Auth** — at minimum password/token auth for remote connections

---

## 4. TUI Features Comparison

### Layout

| Feature | OpenCode | Codex RS | Pi-mono | brain (spec) |
|---------|----------|----------|---------|--------------|
| Header bar | Session + model + tokens + cost | Model + thread info | Logo + instructions | Session + model + tokens + cost |
| Message area | Scrollable, markdown | Scrollable, markdown | Scrollable, markdown | Scrollable, markdown |
| Input area | Multi-line, dynamic height | Multi-line (composer) | Multi-line (editor) | Multi-line (tui-textarea) |
| Footer/status bar | CWD + model + help hints | Status line | Footer with hints | CWD + model + help |
| Sidebar | Toggle, 42 cols (context/MCP/diff/todo) | N/A | N/A | Phase 3 |

### Slash Commands

| Command | OpenCode | Codex RS | Pi-mono | brain (spec) |
|---------|----------|----------|---------|--------------|
| `/help` | Yes | N/A (command palette) | `/hotkeys` | Yes |
| `/new` | Yes | `/new` | `/new` | Yes |
| `/sessions` | Yes | `/resume` | `/resume` | Yes |
| `/model` | `/models` | `/model` | `/model` | Yes |
| `/compact` | N/A | `/compact` | `/compact` | Yes |
| `/thinking` | N/A | N/A | N/A | Yes |
| `/quit` | `/exit`, `/quit`, `/q` | `/quit`, `/exit` | `/quit` | Yes |
| `/review` | N/A | `/review` | N/A | **MISSING** |
| `/diff` | N/A | `/diff` | N/A | **MISSING** |
| `/copy` | N/A | `/copy` | `/copy` | **MISSING** |
| `/fork` | N/A | `/fork` | `/fork` | **MISSING** |
| `/theme` | `/themes` | `/theme` | N/A (settings) | **MISSING** (Phase 3) |
| `/plan` | N/A | `/plan` | N/A | **MISSING** |
| `/settings` | `/status` | `/status` | `/settings` | **MISSING** |
| `/mcp` | `/mcps` | `/mcp` | N/A | **MISSING** |
| `/export` | N/A | N/A | `/export` | **MISSING** |
| `/share` | N/A | N/A | `/share` | **MISSING** |
| `/skills` | N/A | `/skills` | N/A | **MISSING** |
| `/agents` | `/agents` | `/agent`, `/subagents` | N/A | **MISSING** |
| `/permissions` | N/A | `/approvals`, `/permissions` | N/A | **MISSING** |
| `/connect` | `/connect` | `/login` | `/login` | **MISSING** |
| `/rename` | N/A | `/rename` | `/name` | **MISSING** |
| `/init` | N/A | `/init` (AGENTS.md) | N/A | **MISSING** |
| `/ps` | N/A | `/ps` (background terminals) | N/A | **MISSING** |
| `/stop` | N/A | `/stop` (stop terminals) | N/A | **MISSING** |

### Keybindings

| Binding | OpenCode | Codex RS | Pi-mono | brain (spec) |
|---------|----------|----------|---------|--------------|
| Leader key | `Ctrl+X` | N/A | N/A | `Ctrl+X` |
| Submit | Enter | Enter | Enter | Enter |
| Newline | Shift+Enter, Ctrl+Enter | Shift+Enter | Shift+Enter | Shift+Enter, Alt+Enter |
| Cancel/interrupt | Escape | Escape | Escape | Escape |
| Clear input | Ctrl+C | Ctrl+C | Ctrl+C | Ctrl+C |
| Exit | Ctrl+C, Ctrl+D | N/A | Ctrl+D | Ctrl+C (empty) |
| Command palette | Ctrl+P | N/A | N/A | Ctrl+P |
| Cycle model | F2 | N/A | Ctrl+P | **MISSING** |
| Cycle agent | Tab | Alt+Left/Right | N/A | **MISSING** |
| Toggle thinking | N/A | N/A | Ctrl+T | **MISSING** |
| Toggle tool output | N/A | N/A | Ctrl+O | **MISSING** |
| External editor | `<leader>e` | N/A | Ctrl+G | **MISSING** |
| Copy message | `<leader>y` | N/A | N/A | **MISSING** |
| Undo/redo message | `<leader>u`/`<leader>r` | N/A | N/A | **MISSING** |
| Suspend | Ctrl+Z | Ctrl+Z | Ctrl+Z | **MISSING** |
| Session export | `<leader>x` | N/A | N/A | **MISSING** |
| Page up/down | PageUp/PageDown | N/A | PageUp/PageDown | PageUp/PageDown |
| First/last message | Ctrl+G/Ctrl+Alt+G | N/A | N/A | Home/End |

### Tool Display

| Feature | OpenCode | Codex RS | Pi-mono | brain (spec) |
|---------|----------|----------|---------|--------------|
| Inline tools | Yes (single line) | Yes | Yes | Yes |
| Block tools | Yes (bordered) | Yes | Yes | Yes |
| Collapse/expand | Yes | Yes | Yes (Ctrl+O) | Yes |
| Diff view | Yes (unified + split) | Yes | Yes | Yes |
| Spinner | Yes (Knight Rider) | Yes (braille) | Yes | Yes |
| Tool approval | Inline prompt | Popup | `beforeToolCall` hook | Inline prompt |
| Duration shown | Yes | Yes | N/A | **MISSING** |

### Session Management

| Feature | OpenCode | Codex RS | Pi-mono | brain (spec) |
|---------|----------|----------|---------|--------------|
| Session list overlay | Yes (fuzzy, grouped) | Yes (resume picker) | Yes (picker) | Yes |
| Date grouping | Today/Yesterday/etc. | N/A | N/A | Yes |
| Fuzzy search | Yes | N/A | N/A | **MISSING** |
| Rename | Yes | `/rename` | `/name` | Yes |
| Delete | Yes | N/A | N/A | Yes |
| Fork | Yes | `/fork` | `/fork` | **MISSING** |
| Share | N/A | N/A | `/share` (gist) | **MISSING** |
| Session tree | N/A | N/A | `/tree` | **MISSING** |

---

## 5. Configuration Comparison

| Feature | OpenCode | Codex RS | ZeroClaw | Pi-mono | brain (current) |
|---------|----------|----------|----------|---------|-----------------|
| Format | JSON/JSON5 | TOML | TOML | JSON | TOML |
| Project config | `opencode.json` | `AGENTS.md` | `zeroclaw.toml` | `.pi/settings.json` | `.agents/config.toml` |
| Global config | `~/.config/opencode/` | `~/.codex/config.toml` | `~/.zeroclaw/config.toml` | `~/.pi/agent/settings.json` | `~/.brain/config.toml` |
| TUI config | Separate `tui.json` | In main config | N/A | In main settings | **MISSING** |
| Keybinding config | Yes (`tui.json`) | N/A | N/A | `keybindings.json` | **MISSING** |
| Theme config | Yes | N/A | N/A | In settings | **MISSING** |
| Custom models | Via providers | In config | In config | `models.json` | **MISSING** |
| MCP config | In config | In config | N/A | Extensions | **MISSING** |

---

## 6. Server Protocol Comparison

| Feature | OpenCode | Codex RS | ZeroClaw | brain (current) |
|---------|----------|----------|----------|-----------------|
| Transport | HTTP + SSE | JSON-RPC (stdio/WebSocket) | REST + SSE + WebSocket | REST + SSE |
| API style | REST-ish | JSON-RPC methods | REST + webhook | REST |
| Streaming | SSE events | JSON-RPC notifications | SSE + WebSocket | SSE |
| Auth | Basic auth (password) | N/A | Pairing code → bearer | **MISSING** |
| Multi-client | Yes | Yes | Yes | **No** |
| WebSocket | N/A | Yes | Yes | **MISSING** |
| mDNS | Yes (0.0.0.0 mode) | N/A | N/A | **MISSING** |
| Metrics | N/A | N/A | Prometheus | **MISSING** |

---

## 7. Security & Permissions Comparison

| Feature | OpenCode | Codex RS | ZeroClaw | IronClaw | brain (current) |
|---------|----------|----------|----------|----------|-----------------|
| Tool approval | Permission system (ask/allow/deny) | Approval presets (OnRequest/Never) | ApprovalManager | 13-step security pipeline | **MISSING** |
| Sandbox | N/A | Seatbelt (macOS), Landlock (Linux), restricted token (Windows) | Docker, Bubblewrap, Firejail, Landlock | Docker, Bubblewrap, native | **MISSING** |
| Filesystem scoping | Permission rules + patterns | Sandbox profiles | SecurityPolicy (allowed_roots) | Permissions config (read/write/deny paths) | **MISSING** |
| Command blocking | N/A | Exec policy (Starlark rules) | Guardian (45+ blocklist patterns) | Guardian (blocked_patterns, block_pipes, block_redirects) | **MISSING** |
| Credential encryption | N/A | N/A | ChaCha20-Poly1305 SecretStore | AES-256-GCM memory encryption | **MISSING** |
| DLP/output scanning | N/A | N/A | Credential scrubbing | DLP engine, SIEM export | **MISSING** |
| E-stop | N/A | N/A | EstopManager (kill-all, network-kill) | N/A | **MISSING** |

---

## 8. Notable Patterns Worth Adopting

### From OpenCode
- **`serve` + `attach`** — the killer workflow. Run `opencode serve` on a remote machine, `opencode attach` from laptop
- **Command palette** (`Ctrl+P`) — unified entry point for all actions, fuzzy searchable
- **Layered config** — project → global → remote → managed
- **Agent profiles** — build/plan/general/explore agents switchable via Tab
- **Session fork** — branch a conversation without losing the original

### From Codex RS
- **Dual TUI architecture** — legacy ratatui TUI + new app-server TUI (shows TUI can be evolved)
- **In-process vs remote client** — same `AppServerClient` trait, different implementations
- **`/plan` mode** — read-only planning mode before executing
- **`/review`** — review git changes with the AI
- **`/ps` + `/stop`** — background terminal management
- **Shell completion** (`codex completion`) — bash/zsh/fish
- **Exec policy** — Starlark-based policy rules

### From Pi-mono
- **RPC mode** (`pi --mode rpc`) — JSON lines on stdin/stdout for IDE integration
- **Extensions/plugins** — installable packages that add tools, commands, providers
- **Session tree** (`/tree`) — navigate branching conversation history
- **Thinking level cycle** (Shift+Tab) — quick toggle between thinking levels
- **External editor** (Ctrl+G) — open full editor for long prompts
- **OAuth for consumer plans** — Claude Pro, ChatGPT Plus, Copilot, Gemini

### From ZeroClaw
- **Channel abstraction** — trait-based, 20+ channels (could inform future Transport expansion)
- **Gateway + pairing** — secure remote access with one-time pairing codes
- **Memory system** — SQLite + hybrid keyword/vector search
- **Cron/scheduled tasks** — `zeroclaw cron` for recurring agent tasks
- **Hardware peripherals** — USB discovery, peripheral configuration (niche but unique)

### From OpenClaw
- **Lazy CLI loading** — fast-path routing for common commands before full parse
- **Hot config reload** — config changes detected and applied without restart
- **Multiplexed gateway** — single port for WebSocket + HTTP
- **Plugin SDK** — full SDK for channel plugins, tool plugins, CLI command plugins

### From IronClaw
- **Security pipeline** — 13-step tool execution security pipeline
- **Skill verification** — Ed25519 signatures for installed skills
- **Static scanner** — 27 rules scanning skill code for dangerous patterns
- **Audit log** — structured JSON audit trail with SIEM export

---

## 9. Gap Analysis: What brain's TUI Spec Is Missing

### Critical Gaps (should address before implementation)

1. **`brain serve` subcommand** — our server exists but isn't exposed as a CLI command
2. **`brain attach <url>` subcommand** — connect TUI to remote server (OpenCode's killer feature)
3. **Remote BrainApi client** — HTTP client implementing `BrainApi` for attach mode
4. **Non-interactive mode** — `brain run "message"` for scripting/piping (OpenCode `run`, Codex `exec`)
5. **`--model` / `--provider` flags** — override model at startup
6. **`--continue` / `--session` flags** — resume last/specific session without subcommand
7. **`--format json` flag** — JSON output for scripting
8. **Shell completions** — `brain completions bash/zsh/fish`
9. **`/fork` slash command** — branch conversation
10. **`/copy` slash command** — copy last response to clipboard
11. **`/diff` slash command** — review git diff
12. **`/rename` slash command** — rename current session
13. **`/settings` or `/status` slash command** — view current config

### Important Gaps (Phase 2 priority)

14. **Tool approval system** — all competitors have this; our spec mentions it but depends on `enrich-event-model`
15. **`@file` completion** — reference files in prompts
16. **`!bash` shortcut** — quick shell command execution
17. **External editor** (Ctrl+G) — open $EDITOR for long prompts
18. **Thinking level toggle** — cycle thinking/reasoning effort
19. **Model cycle keybinding** — quick model switching (F2 or Ctrl+P)
20. **Agent/profile switching** — Tab to switch between agent profiles
21. **Suspend** (Ctrl+Z) — return to shell, resume later
22. **Session fork** — branch conversation from any point
23. **`/plan` mode** — read-only planning before execution
24. **`/review` command** — review staged/unstaged git changes
25. **MCP management** — `/mcp` slash command for MCP tool management

### Nice-to-Have Gaps (Phase 3+)

26. **Plugin/extension system** — installable packages
27. **RPC/stdio mode** — for IDE integration (ACP covers some of this)
28. **Config init wizard** — `brain init` interactive setup
29. **Health/doctor** — `brain doctor` diagnostics
30. **Export** — `/export` to HTML/markdown
31. **Share** — `/share` session as gist/link
32. **Session tree navigation** — branching history viewer
33. **Cron/scheduled tasks** — recurring agent tasks
34. **OAuth for consumer plans** — ChatGPT Plus, Claude Pro, etc.
35. **Metrics** — Prometheus-style metrics endpoint
36. **Hot config reload** — detect and apply config changes
37. **`/ps` + `/stop`** — background terminal management

---

## 10. Recommended Spec Revisions

### To `add-brain-cli` (CLI subcommands)

Add these subcommands to the CLI spec:
```
brain                            # interactive TUI (default)
brain serve [--port PORT]        # headless HTTP server
brain attach <url> [--password]  # connect TUI to remote server
brain run "message" [--model M]  # non-interactive single turn
brain run --continue             # non-interactive, continue last session
brain sessions list              # (exists)
brain sessions resume <id>       # (exists)
brain sessions fork <id>         # fork a session
brain credentials add/login/...  # (exists)
brain models list                # list available models
brain completions bash|zsh|fish  # shell completions
brain doctor                     # diagnostic check
brain init                       # initialize project config
```

Add these root-level flags:
```
--model, -m <model>              # override model
--provider <provider>            # override provider
--continue, -c                   # continue last session
--session, -s <id>               # resume specific session
--format json|text               # output format
--no-tui                         # force plain CLI mode
```

### To `add-tui-transport` (TUI spec)

Add these slash commands:
```
/fork              # fork current conversation
/copy              # copy last response to clipboard
/diff              # show git diff
/rename [name]     # rename current session
/settings          # view/edit current settings
/plan              # toggle plan mode (read-only)
/review            # review git changes
/mcp               # manage MCP tools
/export [path]     # export session
```

Add these keybindings:
```
Ctrl+Z             # suspend (return to shell)
Ctrl+G / <leader>e # open external editor for prompt
F2                 # cycle recent models
Ctrl+T             # toggle thinking blocks
Ctrl+O             # expand/collapse tool outputs
```

Add these requirements:
- Non-interactive mode rendering (for `brain run`)
- Remote server connection (for `brain attach`)
- Duration display on completed tool calls
- Fuzzy search in session list
- `@file` autocomplete in input
- `!bash` shortcut for quick shell commands
