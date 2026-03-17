# Competitor Research

Source-first internal notes for projects adjacent to `brain`.

This folder focuses on the dimensions that matter most for `brain`'s architecture:

- tool system
- provider and model abstraction
- agent loop implementation
- compaction and context management
- system prompt and prompt building
- planning, modes, and interactivity
- permissions and sandboxing
- storage and sessions
- server/client transport shape
- CLI/TUI ergonomics

All requested projects are present in `repocache/repocache.json`, and the implementation analysis below is based on the local clones in `repocache/`. CLI/TUI cross-product comparisons additionally draw from `openspec/changes/add-tui-transport/competitor-analysis.md`.

## Table of Contents

- [Index](#index)
- [Executive Summary](#executive-summary)
- [Feature Matrix](#feature-matrix)
- [`brain` Baseline](#brain-baseline)
- [Core Trait Matrix Against `brain`](#core-trait-matrix-against-brain)
- [Planning & Mode Matrix](#planning--mode-matrix)
- [CLI & TUI Matrix](#cli--tui-matrix)
- [Server/Client Architecture Matrix](#serverclient-architecture-matrix)
- [Tool Implementation Matrix](#tool-implementation-matrix)
- [Storage Layout Matrix](#storage-layout-matrix)
- [Data Model Matrix](#data-model-matrix)
- [Suggested Next Analyses](#suggested-next-analyses)
- [Working Conclusion](#working-conclusion)

## Index

- [`OpenCode.md`](OpenCode.md)
- [`OpenClaw.md`](OpenClaw.md)
- [`Gastown.md`](Gastown.md)
- [`Rig.md`](Rig.md)
- [`AsyncOpenAI.md`](AsyncOpenAI.md)
- [`VercelAISDK.md`](VercelAISDK.md)
- [`PiMono.md`](PiMono.md)
- [`IronClaw.md`](IronClaw.md)
- [`ZeroClaw.md`](ZeroClaw.md)
- [`Codex.md`](Codex.md)

## Executive Summary

- `OpenCode` and `OpenClaw` are product platforms first. They solve much more than a reusable engine, which gives them depth in UX, approvals, and control-plane concerns, but also makes them less cleanly composable than `brain`.
- `Codex` is the strongest Rust product benchmark. It validates JSONL session persistence, typed tool routing, compaction, plan-mode UX, sandboxing, and client/server layering, while also showing the risks of letting one core runtime own too much.
- `ZeroClaw` is the closest Rust comparison in architectural spirit. It has real traits, a real iterative tool loop, explicit compaction/context trimming, prompt-section composition, and concrete approval/provider routing.
- `pi-mono` is the closest TypeScript runtime analogue. It preserves meaningful layering from provider SDK to generic loop package to coding-agent shell, with strong compaction, prompt-template, and interactive-shell ideas.
- `Vercel AI SDK` is the strongest benchmark for provider API quality, streaming semantics, tool-call ergonomics, and app-owned interactivity, but it is thinner on durable runtime concerns.
- `Rig` is a strong Rust SDK/library comparison for provider, tool, and retrieval composition, but it is not a full coding-agent runtime.
- `async-openai` is best treated as an upstream protocol and streaming reference for `brain-providers/openai`, not a full engine competitor.
- `Gastown` is an orchestration layer above coding agents, not a direct engine peer. It is strongest where `brain` may eventually care about persistent coordination and operator tooling.
- `IronClaw` is strongest as a security-policy reference, but weaker as an architecture benchmark because some important seams appear more aspirational than fully wired.
- `brain` can differentiate by keeping `Provider`, `Tool`, `Store`, `AgentLoop`, and `Transport` explicit and small while still borrowing strong ideas from the bigger systems.

## Feature Matrix

All matrices below use the same product order: `brain`, `Codex`, `OpenCode`, `pi-mono`, `ZeroClaw`, `OpenClaw`, `IronClaw`, `Gastown`, `Rig`, `async-openai`, `Vercel AI SDK`.

| Dimension | `brain` | `Codex` | `OpenCode` | `pi-mono` | `ZeroClaw` | `OpenClaw` | `IronClaw` | `Gastown` | `Rig` | `async-openai` | `Vercel AI SDK` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| **Vertical** | Trait-first agent engine | Full Rust coding product | Full AI coding product | Layered agent toolkit | Rust agent runtime platform | Assistant + gateway platform | Security-first agent product | Multi-agent orchestration layer | Rust LLM/agent SDK | OpenAI Rust SDK | Provider + agent SDK |
| **Architecture Shape** | Rust workspace + 5 traits | Large workspace centered on `codex-core` | Modular Bun monolith | Package-layered runtime family | Large root crate + subsystems | Gateway daemon + embedded pi runtime | Monolithic Rust binary | Go control plane + managed runtimes | Library-first workspace | Protocol SDK workspace | Composable TS monorepo |
| **Tool System** | Native tools + driver adapters | `ToolHandler` + registry/router + MCP | Registry + built-ins + plugins + MCP | Serializable schemas + coding tools | Real trait + native/prompt dispatch | Typed tools + delegated clients + plugins | Provider-native concept, uneven tooling | Delegated to wrapped CLIs | Tool traits + MCP adapters | Hosted tool schemas | `tool()` / `dynamicTool()` + provider tools |
| **Provider / Model** | Feature-gated providers + router | Registry-driven providers + `ModelClient` | Catalog + agent-profile overrides | Package/model registry | Rich trait + aliases + failover | Config registry + embedded pi behavior | Single provider trait + presets | Runtime presets for external agents | Strong client/provider abstraction | Typed OpenAI client/config | Strongest provider interface in set |
| **Agent Loop** | Simple iterative loop | Session-driven iterative loop | Iterative session loop | Generic loop + coding shell | Real iterative tool loop | Serialized session runs around queues | Mostly single-pass | Workflow orchestration | Multi-turn agent flow | No local loop | Text/tool continuation loop |
| **Streaming** | Provider chunks + events | Rich event stream with fallback transport | Rich stream events via session processor | Provider streams + interactive shell | Provider stream + CLI/gateway channels | Gateway streams over SSE/WS | Light abstraction; CLI mostly request/response | Delegated to runtimes | Provider/request streaming | SSE protocol support | First-class stream APIs |
| **Tool Dispatch** | Sequential looped execution | Structured tools + MCP + child agents | Serialized event stream; multi-tool capable | Iterative tool turns + shell tools | Parallel when safe; ordered results | Runtime + policy + delegated execution | Provider tool calls through security pipeline | Delegated | Agent/toolset orchestration | Hosted/provider executed | Parse, validate, execute, continue |
| **Compaction** | Not productized yet | Inline or remote auto-compaction | First-class auto-compaction tasks | Pure compaction module + session-managed | Auto-compact + trim + memory snapshots | Session lifecycle concern; retry/reset aware | Configured, not central | Delegated | Not central | None local | Caller-owned |
| **Context Management** | Basic history + system prompt | Strong `ContextManager` + token accounting | Overflow checks + tool-output spill files | Token budget + conversation serialization | Bootstrap caps + history trim | Bootstrap caps + collapsed history strings | Thin, partly config-based | Wrapper/context injection | Prompt/request level only | Caller-owned | Caller-managed |
| **Prompt Building** | Static system prompt input | Base prompt + AGENTS + custom prompts | Static fragments + dynamic env/skills | Code-built + context files + skills + templates | Composable prompt sections | Custom builder + hook stages + bootstrap files | Config-seeded system prompt | Runtime wrapper injection | Agent/request level only | Protocol only | Caller-driven prompt APIs |
| **Planning / Modes** | No dedicated plan mode | Explicit plan/review/fast/collab modes | Agent-profile modes: `build`, `plan`, `explore` | Transport/session modes, no formal plan mode | Infrastructural modes, no branded plan mode | `promptMode` + queue/runtime toggles | Presets, no polished plan UX | Operator workflows, not agent modes | Little formal mode system | None local | App-owned only |
| **Interactivity** | Minimal CLI chat + loop events | Plan/review/permissions prompts + slash commands | Approval-heavy + slash/TUI workflows | Slash commands + ask-before-risky actions | Approvals + delegate/gateway flows | Approvals, pairing, client/gateway prompts | Approvals + partial UI | Operator nudges + dashboard flows | App-authored | Hosted API dependent | App-driven approvals/prompts |
| **Subagents** | None | First-class child threads | First-class subtasks/explore agents | Branch/fork workflows more than child agents | Delegate tool reuses loop | Lighter embedded subagents | Separate multi-agent orchestration | Convoys and managed agents | Not central | Hosted only | Not central |
| **Retry / Recovery** | Mostly provider-level | Strong retries + WS to HTTPS fallback | Processor retries + doom-loop guard | Session/compaction controls | Reliable provider wrapper + retries | Queue handling + compaction resets | Thin loop-level retries | Workflow-level resilience | Provider/client level | Client-level only | App/provider driven |
| **Permissions / Sandbox** | Host-local, no sandbox layer | Strongest cross-platform sandboxing | `allow` / `ask` / `deny`; light boundary | Hook/app-layer approvals | Strong command/path policy; Docker strongest | Docker + policy + approvals | Strongest policy vocabulary | Wrapper guardrails | Hooks only | Hosted semantics | External to SDK |
| **CLI / TUI** | Basic CLI; server examples | Mature TUI + app-server + remote attach | Strong TUI + `serve` / `attach` | Strong TUI + RPC mode | CLI + gateway/dashboard | Gateway-centric multi-client UX | CLI + partial web UI | CLI + dashboard | No defining CLI/TUI | N/A | Not terminal-first |
| **Relevance to `brain`** | Baseline | Strongest Rust product benchmark | Great UX benchmark | Closest TS runtime analogue | Closest Rust architecture benchmark | Strong control-plane benchmark | Strong security reference | Strong orchestration benchmark | Strong provider/tool benchmark | Strong OpenAI wire reference | Strong provider/tool API benchmark |

## `brain` Baseline

Current local baseline for `brain`'s native tools:

- `FileRead`: native `tokio::fs::read_to_string` with line-number formatting
- `FileWrite`: native `tokio::fs::create_dir_all` plus `tokio::fs::write`
- `FileEdit`: native read / unique-match replace / write flow
- `Glob`: Rust `glob` crate
- `Grep`: native `walkdir` plus Rust `regex`; does not shell out to `rg`
- `Shell`: `tokio::process::Command` using `sh -c`

This matters because several competitors expose a similar user-facing tool surface but implement it very differently:

- some use native filesystem code
- some shell out to `rg`, `fd`, or the system shell
- some expose only provider-defined remote tools rather than local runtime tools

## Core Trait Matrix Against `brain`

Symbol legend:

- `FULL` = explicit / first-class / clearly supported
- `PART` = present but implicit / partial / product-owned
- `WEAK` = present but notably uneven / skeletal
- `NONE` = missing / not central / not evidenced

| Dimension | `brain` | `Codex` | `OpenCode` | `pi-mono` | `ZeroClaw` | `OpenClaw` | `IronClaw` | `Gastown` | `Rig` | `async-openai` | `Vercel AI SDK` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| **`Provider`** | `FULL` | `PART` concrete client | `FULL` | `FULL` | `FULL` | `FULL` | `FULL` | `PART` | `FULL` | `FULL` | `FULL` |
| **`Tool`** | `FULL` | `FULL` | `FULL` | `FULL` | `FULL` | `FULL` | `WEAK` uneven | `NONE` | `FULL` | `FULL` protocol-only | `FULL` |
| **`Store`** | `FULL` | `PART` SQLite + JSONL only | `PART` concrete modules | `PART` concrete classes | `FULL` | `PART` distributed | `FULL` | `FULL` control-plane | `PART` | `PART` | `PART` |
| **`AgentLoop`** | `FULL` | `PART` embedded in runtime | `PART` product-owned | `FULL` | `PART` | `PART` embedded/runtime split | `PART` | `FULL` orchestration loop | `PART` | `NONE` | `FULL` |
| **`Transport`** | `FULL` | `PART` app-server/TUI owned | `PART` runtime-owned | `FULL` | `FULL` via channels | `FULL` | `FULL` via channels | `FULL` | `PART` | `NONE` | `FULL` |
| **Read Against `brain`** | Baseline target architecture | Strong product, few clean reusable seams | Strong runtime package, weaker trait seams | Closest TS runtime analogue | Closest Rust trait-driven comparison | Strong gateway/tool seams, loop/store distributed | Good vocabulary, uneven realization | Excellent control-plane reference | Strong SDK benchmark for provider/tool composition | Provider SDK reference, not engine peer | Excellent SDK seams, store is secondary |

## Planning & Mode Matrix

How these products handle planning artifacts, read-only or plan-oriented behavior, and general operational modes.

| Dimension | `brain` | `Codex` | `OpenCode` | `pi-mono` | `ZeroClaw` | `OpenClaw` | `IronClaw` | `Gastown` | `Rig` | `async-openai` | `Vercel AI SDK` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| **Explicit Planning Mode** | `NONE` | `FULL` `/plan` in TUI | `FULL` built-in `plan` agent | `NONE` | `NONE` | `PART` approval-plan artifacts, not chat plan mode | `NONE` | `NONE` | `NONE` | `NONE` | `NONE` app-owned only |
| **Planning / Intermediate Artifacts** | `PART` session summaries only | `FULL` plan-mode shaping + compaction + forks | `FULL` plan files + title/summary/compaction | `FULL` branch + compaction summaries | `FULL` compaction summaries + delegated results | `FULL` `system.run` plans + compaction artifacts | `PART` memory compaction + agent aggregation | `PART` work items and nudges | `NONE` | `NONE` | `PART` app-owned |
| **General Mode System** | `PART` simple loop + config choices | `FULL` plan/review/fast/collaboration modes | `FULL` profile-driven modes | `FULL` interactive/print/RPC + session controls | `PART` autonomy/scenario/provider modes | `FULL` prompt/queue/runtime toggles | `PART` provider/sandbox presets | `PART` workflow/operator modes | `NONE` | `NONE` | `PART` app-defined |
| **Interactive Planning Flow** | `NONE` | `FULL` plan mode changes output and approval flow | `FULL` plan entry/exit with read-only behavior | `PART` slash-command and branch-driven | `PART` delegated-agent planning artifacts | `PART` approval-oriented planning | `NONE` | `PART` operator orchestration checkpoints | `NONE` | `NONE` | `PART` app-defined review flows |

### Planning and Mode Takeaways

- `Codex` and `OpenCode` are the clearest references if `brain` wants a true read-only planning mode rather than just “tell the model not to edit”.
- `OpenCode` is the clearest example of mode-as-agent-profile: permissions, prompt, tools, and model all move together.
- `Codex` is the clearest example of mode-as-collaboration-state: plan mode, review mode, fast mode, and collaboration settings alter runtime behavior across the UI and loop.
- `OpenClaw` shows a third pattern where “mode” is spread across prompt rendering, queueing, reasoning visibility, execution policy, and approvals.
- `pi-mono` shows that many useful planning artifacts can emerge without a single formal planning mode, via branching, summaries, and prompt templates.

## CLI & TUI Matrix

This section condenses the richer CLI/TUI source pass in `openspec/changes/add-tui-transport/competitor-analysis.md`.

### Top-Level UX Matrix

| Dimension | `brain` | `Codex` | `OpenCode` | `pi-mono` | `ZeroClaw` | `OpenClaw` | `IronClaw` | `Gastown` | `Rig` | `async-openai` | `Vercel AI SDK` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| **Default Entry** | `brain` stdin chat | `codex` TUI | `opencode` TUI | `pi` TUI | `zeroclaw agent` | Gateway/client model | `ironclaw run` | `gt` CLI | N/A | N/A | N/A |
| **Remote / Serve Model** | Example server only; no attach UX yet | `app-server` + `--remote ws://...` | `serve` + `attach <url>` | `--mode rpc` over stdio JSONL | `gateway start` + daemon/web clients | `openclaw gateway` + many clients | Partial gateway/web UI | Dashboard/feed; not agent attach | N/A | N/A | App-owned |
| **Notable Commands** | sessions, credentials | `exec`, `review`, `fork`, `mcp`, `sandbox`, `debug` | `run`, `session`, `models`, `mcp`, `export` | `/model`, `/compact`, `/fork`, `/tree`, `/share` | `daemon`, `models`, `auth`, `status`, `doctor` | `setup`, `models`, `sessions`, `plugins`, `sandbox` | `onboard`, `ui`, `doctor`, `models` | convoy/work-item oriented commands | N/A | N/A | App-level only |
| **Terminal UX Notes** | Minimal CLI today | Mature TUI + app-server boundary | Strongest terminal-first product after Codex | Strong TUI + session-tree ergonomics | CLI + dashboard, not ratatui TUI | Gateway-centric more than TUI-centric | Weak terminal story | CLI + dashboard, not coding-agent TUI | No product shell | Not terminal-first | Not terminal-first |

### Interactive Controls Matrix

| Dimension | `brain` | `Codex` | `OpenCode` | `pi-mono` | `ZeroClaw` | `OpenClaw` | `IronClaw` | `Gastown` | `Rig` | `async-openai` | `Vercel AI SDK` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| **Slash Command Strength** | `NONE` | `FULL` operational slash commands | `FULL` session/model/help/MCP/agents | `FULL` prompt/session commands | `NONE` | `PART` gateway/client commands matter more | `NONE` | `NONE` | `NONE` | `NONE` | `NONE` app-owned |
| **Keybinding / TUI Highlights** | `NONE` | Mature TUI shared across local/remote | `Ctrl+X` leader, palette, sidebar | External editor, tree UX, tool/thinking toggles | Dashboard/gateway emphasis | Client surface matters more than keyboarding | Partial web UI only | Dashboard/feed over TUI | N/A | N/A | N/A |
| **Interactive Prompts** | Minimal event-driven flow | Plan/review/permissions prompts | Frequent approval and permission prompts | Ask-before-risky + slash-command workflows | Approval and gateway-driven prompts | Pairing, approvals, delegated-client prompts | Approval prompts + partial UI | Operator nudges and coordination prompts | App-authored | Hosted API dependent | App-authored |
| **Approval / Confirmation UX** | Not productized yet | Strong built-in approval + sandbox UX | Strong `allow` / `ask` / `deny` UX | App/hook-layer confirmations | Interactive approvals in loop/gateway | Strong gateway approvals + policy | Security pipeline approvals | Wrapper-level guardrails | App-authored | Hosted semantics only | SDK protocol only |

### CLI/TUI Takeaways

- `serve` plus `attach` is a recurring product pattern: OpenCode, Codex, and OpenClaw all validate a client/server split for richer clients.
- `pi-mono` is the clearest reference for stdin/stdout RPC mode and session-tree style UX.
- `Codex` is the strongest example of a TUI that remains a client of an app-server boundary even in-process.
- `OpenCode` is the strongest reference for terminal-first product ergonomics and approval-heavy interaction.
- `brain`'s most obvious transport gap is still `brain serve` plus `brain attach`, followed by a richer TUI command and confirmation surface.

## Server/Client Architecture Matrix

How each framework structures the relationship between the UI, the agent runtime, and any server process. This matters for `brain`'s decision on in-process vs. server architecture and how the TUI connects to `BrainApi`.

| Dimension | `brain` | `Codex` | `OpenCode` | `pi-mono` | `ZeroClaw` | `OpenClaw` | `IronClaw` | `Gastown` | `Rig` | `async-openai` | `Vercel AI SDK` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| **Default Mode** | CLI + in-process `BrainServer` | TUI + embedded app server | TUI + Worker thread | Interactive TUI, in-process | CLI agent, in-process | Gateway daemon + clients | CLI + optional web UI | CLI orchestration | Library-only | Client SDK | SDK library |
| **Server Process** | In-process only today | In-process by default; optional separate app server | In-process by default; optional HTTP server | No HTTP server | Optional gateway/daemon | Persistent gateway daemon | No strong default server | No central agent server | N/A | N/A | App-owned |
| **TUI / Client to Agent Transport** | In-process trait calls | In-memory JSON-RPC channels or WebSocket | Worker RPC or HTTP + SSE | Direct calls or stdio JSONL RPC | In-process CLI channel; WS/HTTP for gateway | WebSocket JSON-RPC to gateway | stdin/stdout; partial WS/web UI | N/A; tmux/worktree coordination | N/A | N/A | App-owned |
| **Headless Server** | `PART` example server only | `FULL` `codex app-server` | `FULL` `opencode serve` | `NONE` | `FULL` gateway/daemon modes | `FULL` `openclaw gateway` | `WEAK` partial | `PART` dashboard only | `NONE` | `NONE` | `PART` app-owned |
| **Client Attachment** | `NONE` planned only | `FULL` `--remote ws://...` | `FULL` `attach <url>` | `PART` RPC client spawns subprocess | `FULL` web dashboard + API clients | `FULL` CLI/web/mobile/macOS clients | `WEAK` weakly evidenced | `NONE` | `NONE` | `NONE` | `PART` app-owned |
| **Server Persists After Exit?** | `NONE` | `NONE` | `NONE` | `NONE` | `FULL` gateway only | `FULL` | `NONE` | `NONE` | N/A | N/A | N/A |

### Architecture Takeaways

- In-process is still the default almost everywhere. Only OpenClaw is daemon-first.
- The TUI is usually a client of an API boundary, even when the boundary stays in-process.
- HTTP plus SSE, WebSocket, and stdio JSONL RPC are the dominant remote-client shapes.
- `brain`'s existing `BrainApi` and in-process `server.client()` shape are directionally correct, but the attach story is still missing.

## Tool Implementation Matrix

Legend:

- `native`: implemented in repo code using local runtime or language APIs
- `shell-out`: implemented by spawning external tools such as `rg`, `fd`, or a shell
- `provider-defined`: repo defines the tool schema or adapter, but execution happens in the external provider runtime
- `declared only`: mentioned or modeled, but not clearly implemented as a concrete tool in the repo snapshot

| Dimension | `brain` | `Codex` | `OpenCode` | `pi-mono` | `ZeroClaw` | `OpenClaw` | `IronClaw` | `Gastown` | `Rig` | `async-openai` | `Vercel AI SDK` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| **`FileRead`** | native Rust fs | native Rust fs | native Node/Bun fs | native Node fs | native Rust fs | native wrappers + fs/sandbox bridge | declared only | delegated | delegated/app-authored | hosted APIs | provider-defined |
| **`FileWrite`** | native Rust fs | native `apply_patch` flow | native Node/Bun fs | native Node fs | native Rust fs | native wrappers + fs/sandbox bridge | declared only | delegated | delegated/app-authored | hosted APIs | provider-defined |
| **`FileEdit`** | native Rust replace/write | native `apply_patch` crate | native edit + diff helpers | native Node edit logic | native Rust replace/write | native wrappers + recovery logic | missing / not central | delegated | delegated/app-authored | hosted APIs | provider-defined |
| **`Glob` / `Find`** | native `glob` crate | native `list_dir` | shell-out `rg`-style list/search | shell-out to `fd` | native `glob` crate | not clearly evidenced local built-in | missing | not centralized | not centralized | missing generic local built-in | missing generic local built-in |
| **`Grep` / Content Search** | native `walkdir` + `regex` | shell-out to `rg` | shell-out to `rg` | shell-out to `rg` | shell-out to `rg`, fallback `grep` | not clearly evidenced local built-in | missing | not centralized | not centralized | hosted search APIs | retrieval/provider-oriented |
| **`Shell` / `Bash`** | shell-out via `sh -c` | sandboxed shell + PTY | shell-out via spawned shell | shell-out via spawned shell | shell-out via runtime adapter | native process control + PTY + optional Docker exec | declared only as tool; shell spawning exists elsewhere | delegated with GT guardrails | delegated/app-authored | hosted shell/container APIs | provider-defined |

### Tool Takeaways

- `ZeroClaw` is the closest external match to `brain`'s current file-tool philosophy.
- `Codex` is the closest Rust product match for native file tools plus `rg`-backed search and serious sandboxing.
- `OpenCode` and `pi-mono` both keep file mutation local while using battle-tested external CLIs for search.
- `Vercel AI SDK` and `async-openai` are best read as protocol/tool-schema references, not local tool-runtime references.

## Storage Layout Matrix

How each framework persists data to disk. This matters for `brain`'s decision on global vs. project-local storage and the nesting of sessions, messages, and credentials.

| Dimension | `brain` | `Codex` | `OpenCode` | `pi-mono` | `ZeroClaw` | `OpenClaw` | `IronClaw` | `Gastown` | `Rig` | `async-openai` | `Vercel AI SDK` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| **Global Root** | `~/.brain/` proposed | `~/.codex/` or `$CODEX_HOME` | XDG data + config dirs | `~/.pi/` or project-local | `~/.zeroclaw/` config + workspace data | `~/.openclaw/` | XDG via `directories` | Product-managed state | App-owned | App-owned | App-owned |
| **Root Method** | Dotfile root | Dotfile root or env override | XDG | Dotfile root | Env/config-selected workspace | Dotfile root | XDG | Product-specific | App-specific | Caller-owned | Caller-owned |
| **Credentials** | Per-provider JSON files | `auth.json`, keyring, or age secrets | `auth.json` in data dir | `auth.json` | `auth-profiles.json`, optional encryption | `.env` + secret providers | config/env + auth tokens | Managed runtime creds | App-owned | Client credentials | App/provider owned |
| **Sessions Format** | JSON metadata + JSONL messages | JSONL rollouts + SQLite metadata | SQLite | JSONL session files | SQLite memories, not full transcript sessions | JSON index + JSONL transcripts | Mostly in-memory sessions | Work items / orchestration state | App-owned | Caller-owned | Caller-owned |
| **Messages Location** | `messages.jsonl` per session | Inline in rollout JSONL | In SQLite | Inline in session files | In memory stores / snapshots | Per-agent session JSONL + memory stores | Mostly in-memory transcript | Runtime-specific | App-owned | Caller-owned | Caller-owned |
| **Project-Local** | `.agents/config.toml` | `.codex/config.toml`, `.codex/skills`, `.codex/rules` | `.opencode/` + `opencode.json` | `.pi/` | Workspace dir | Agent workspace dirs | Not clearly documented | Worktree-oriented | App-owned | Caller-owned | Caller-owned |
| **Config Location** | Global + project config | System, user, and project config | Global + project config files | `.pi/` | Global + workspace config | `~/.openclaw/openclaw.json` | XDG config dirs | Product config | App-owned | Caller-owned | Caller-owned |

### Storage Takeaways

- The dominant ecosystem pattern is still a single dotfile root, not strict XDG.
- SQLite and JSONL are the two common persistence shapes for coding-agent products.
- Messages are almost always co-located with session state, either in one DB or one rollout file.
- `Codex` has the most layered config precedence; most others use simpler user-plus-project layering.

## Data Model Matrix

Actual data structures each framework uses for projects, sessions, messages, and credentials. Based on source code inspection of repocache clones.

### Project Entity

| Dimension | `brain` | `Codex` | `OpenCode` | `pi-mono` | `ZeroClaw` | `OpenClaw` | `IronClaw` | `Gastown` | `Rig` | `async-openai` | `Vercel AI SDK` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| **Has Project?** | `FULL` | `NONE` | `FULL` | `NONE` | `PART` workspace scope only | `PART` agent config stands in | `NONE` | `NONE` | `NONE` | `NONE` | `NONE` |
| **Key Fields** | id, name, root, config, timestamps | `cwd` lives on thread metadata | id, worktree, vcs, name, icon, sandboxes, commands, timestamps | N/A | path plus memory/session IDs | id, name, workspace, model, skills, sandbox, tools | N/A | N/A | N/A | N/A | N/A |
| **Identification** | ULID | `cwd` on each thread | String ID | `cwd`-scoped | Path | Agent ID | N/A | N/A | N/A | N/A | N/A |
| **Notes** | Clean entity linking sessions to working dir | Threads store cwd + git info directly | Richest project model in set | Sessions keyed by encoded cwd path | No first-class project struct | Agent is organizational unit, not project | Config-only | No first-class project entity | No first-class project entity | No local project entity | SDK leaves it to app |

### Session Entity

| Dimension | `brain` | `Codex` | `OpenCode` | `pi-mono` | `ZeroClaw` | `OpenClaw` | `IronClaw` | `Gastown` | `Rig` | `async-openai` | `Vercel AI SDK` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| **Key Fields** | id, project_id, title, timestamps | id, rollout_path, cwd, source, provider, title, sandbox policy, approval mode, tokens, git info | id, project/workspace link, parent_id, slug, title, summary, permissions, compaction/archive timestamps | id, cwd, parentSession, name, timestamps | implicit `session_id` in memory entries | sessionId, agentId, model/provider overrides, token counts, compaction count, label, queueMode, channel metadata | id, turn_count, max_turns | Work items and convoy state | App-owned | Caller-owned | Caller-owned |
| **Persistence** | JSON file + JSONL messages | SQLite metadata + JSONL rollout | SQLite | JSONL header line + entries | No dedicated session store | JSON index + JSONL | In-memory only | Product state | App-owned | Caller-owned | Caller-owned |
| **Notable Extras** | Minimal and clean | Thread metadata mirrors rollout headers | Forks, summaries, permissions, diffs | Parent sessions for forks | Session is mainly memory scope | Extremely rich metadata | No durable transcript session store | Coordination-first, not chat-first | App-defined | N/A | N/A |

### Message Entity

| Dimension | `brain` | `Codex` | `OpenCode` | `pi-mono` | `ZeroClaw` | `OpenClaw` | `IronClaw` | `Gastown` | `Rig` | `async-openai` | `Vercel AI SDK` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| **Key Fields** | id, role, content, tool_calls, tool_call_id, created_at | `RolloutItem` / `ResponseItem` variants | Message row + Part rows | User / Assistant / ToolResult unions | `ConversationMessage` enum | pi-style transcript entries | `Message` with tool_calls, tool_results, content_blocks | Runtime-specific | App-defined | OpenAI request/response items | SDK message/tool abstractions |
| **Persistence** | JSONL | JSONL | SQLite | JSONL | In-memory | JSONL | In-memory | Runtime-owned | App-owned | Caller-owned | Caller-owned |
| **Tool Call Representation** | Assistant tool calls + separate tool results | Function, shell, MCP, and custom calls as typed response items | Parts for text, reasoning, tool, patch, snapshot, subtask, compaction | Tool calls inside assistant content blocks | Separate assistant-tool-call and tool-result variants | Same as pi-mono | Tool data stored directly on message | Delegated to runtimes | App-authored | Hosted tool and function items | Built-in tool protocol objects |

### Credential Entity

| Dimension | `brain` | `Codex` | `OpenCode` | `pi-mono` | `ZeroClaw` | `OpenClaw` | `IronClaw` | `Gastown` | `Rig` | `async-openai` | `Vercel AI SDK` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| **Key Fields** | id, credential, health, enabled, created_at | auth mode + tokens/API key | access/refresh tokens + account metadata | API-key or OAuth credential | auth profile, token set, metadata, timestamps | auth profiles + secret refs | provider API key in config | Managed runtime creds | App-owned | client credential config | app/provider-owned |
| **Persistence** | JSON per provider dir | `auth.json`, keyring, or age secrets | SQLite/data dir | `auth.json` | `auth-profiles.json`, optionally encrypted | config + `.env` / secret providers | config/env | Product-managed | App-owned | Caller-owned | Caller-owned |
| **Multi-Provider** | `FULL` | `NONE` single-provider runtime | `NONE` single active account | `FULL` | `FULL` | `FULL` | `PART` minimal | `PART` runtime-dependent | `FULL` app-authored | `NONE` | `FULL` app-authored |
| **Health Tracking** | `FULL` | `NONE` | `NONE` | `NONE` | `NONE` | `PART` cooldowns instead | `NONE` | `NONE` | app-authored | `NONE` | app-authored |

### Data Model Takeaways

- `brain` has the cleanest first-class Project/Session/Message/Credential shape in the set.
- `brain` is the only framework in the comparison with explicit per-credential health tracking.
- Session metadata should stay lean; OpenCode and OpenClaw show how quickly it can balloon.
- JSONL message persistence remains a strong, validated choice across multiple competitors.

## Suggested Next Analyses

- `mastra`: useful for another SDK-first comparison with stronger workflow/app-builder positioning.
- `langgraph`: useful for a graph-oriented agent-loop benchmark rather than a coding-agent benchmark.
- `ACP` and editor protocol implementations: useful if `brain` wants to sharpen its attach/editor story after the TUI transport work.

## Working Conclusion

If the goal is to sharpen `brain`'s engine boundaries, the most useful primary comparisons are:

- `ZeroClaw` for Rust trait design, provider routing, iterative loop behavior, compaction, and interactive approvals.
- `Codex` for Rust product-level tool routing, session persistence, plan/review UX, context management, app-server layering, and sandboxing.
- `pi-mono` for layered runtime packaging, compaction, prompt templates, interactive shell design, and RPC mode.
- `Vercel AI SDK` for provider and tool-call API design quality when the host app owns prompt-building, interactivity, and workflow semantics.

Secondary but still important references are:

- `Rig` for Rust provider/tool/RAG library design.
- `async-openai` for OpenAI wire formats, streaming events, and hosted tool schemas.
- `OpenCode` for product-grade coding-agent UX, approval flows, agent profiles, and session orchestration.
- `OpenClaw` for gateway architecture, approvals, delegated execution, and multi-surface deployment.
- `Gastown` for persistent multi-agent orchestration above the engine layer.
- `IronClaw` for security-policy vocabulary and approval thinking.

The emerging picture is stable: `brain` is directionally right to keep `Provider`, `Tool`, `Store`, `AgentLoop`, and `Transport` explicit and small. The biggest competitors win on UX and operations by layering product-specific systems around those concerns; the best opportunity for `brain` is to preserve the clean kernel while borrowing the strongest surrounding ideas selectively.
