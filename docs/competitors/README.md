# Competitor Research

Source-first internal notes for projects adjacent to `brain`.

This folder focuses on the dimensions that matter most for `brain`'s architecture:

- tool call method
- provider / model / mode method
- agent loop method
- permissions / sandbox method
- platform support
- technical architecture

All requested projects are present in `repocache/repocache.json`, and the analysis below is based on the local clones in `repocache/`.

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

## Executive Summary

- `OpenCode` and `OpenClaw` are product platforms first. They solve far more than a reusable engine, which gives them depth in UX, integrations, and control-plane concerns, but also makes them less cleanly composable than `brain`.
- `Gastown` is best understood as an orchestration layer above coding agents, not as a direct engine peer. It is strongest where `brain` may eventually care about persistent coordination, work tracking, and agent operations.
- `Rig` is a strong Rust SDK/library comparison for provider, tool, and RAG composition, but it is not a full coding-agent runtime or control-plane platform.
- `async-openai` is best treated as an upstream OpenAI protocol and streaming reference for `brain-providers`, not as a full engine competitor.
- `Vercel AI SDK` is the strongest comparison for provider abstraction, streaming semantics, tool-call protocol design, and UI-facing transports. It is narrower than `brain` because it is not centered on durable session orchestration or a transport/store core.
- `pi-mono` is the closest TypeScript runtime analogue to `brain`: it has clear package layering from provider layer to agent loop to coding-agent product, but its abstractions are still package- and hook-centric rather than a small trait kernel.
- `ZeroClaw` is the closest Rust comparison in architectural spirit. It has real traits for core subsystems and a real iterative tool loop, even though most of the implementation still lives in one large root crate.
- `IronClaw` is strongest as a security-policy reference, but weaker as an architecture benchmark because some important seams appear more aspirational than fully wired in the current source.
- `brain` can differentiate by staying small and explicit around `Provider`, `Tool`, `Store`, `AgentLoop`, and `Transport`, while still borrowing good ideas from these larger systems.

## Feature Matrix

| Project | Vertical | Tool Call Method | Provider / Model / Mode | Agent Loop | Permissions / Sandbox | Platform Support | Architecture | Relevance To `brain` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `OpenCode` | Full AI coding product | Central registry builds built-ins, config tools, plugin tools, and MCP tools | Catalog-driven providers plus agent profiles that override model, tools, permissions, and params | Persistent session loop with streaming, subagents, compaction, retry, and doom-loop guards | Rule-based `allow` / `ask` / `deny`; host exec; no strong built-in sandbox | CLI/TUI, desktop, VS Code, HTTP server, JS SDK, web app | Monorepo modular monolith with one large runtime package | Great product benchmark; weak match for a minimal engine core |
| `OpenClaw` | Personal assistant and gateway platform | Typed tool definitions, delegated client tools, policy pipeline, plugin tools | Config-first model registry, fallback chains, plugin providers, per-agent model control | Serialized session run built around embedded `pi-*` runtime | Docker sandboxing, exec approvals, pairing, tool policies; plugins unsandboxed | Linux/macOS gateway, WSL, macOS app, iOS/Android nodes | Modular monolith centered on a long-running gateway | Strong control-plane benchmark; broader than a coding engine |
| `Gastown` | Advanced orchestration layer for external coding agents | Delegates coding tools to managed runtimes; adds hooks, mail, priming, nudges, and coordination around them | Runtime preset registry for agent CLIs such as Claude, Codex, Cursor, OpenCode, and pi; not a model-provider abstraction | Mayor / Convoy / Polecat orchestration with persistent work tracking, hooks, and handoffs | Mostly vendor yolo / bypass modes plus guard hooks and command allowlists; stronger sandboxing is future-oriented | macOS, Linux, Windows binaries; tmux-centric operational model | Go control-plane product with Beads, Dolt, git worktrees, tmux sessions, dashboard/feed | High relevance for orchestration, lower relevance for core provider/tool architecture |
| `Rig` | Rust LLM and agent SDK | First-class tool plumbing, tool traits, tool servers, and MCP adapters, but few built-in local coding tools | Strong provider/client abstraction plus vector-store and RAG composition | Concrete multi-turn agent implementation with hooks and max-turn control | Hook-based control, not a strong sandbox/runtime policy layer | Rust-native library workspace with CLI/Discord integrations and partial WASM story | Library-first Rust workspace centered on `rig-core` | Strong secondary benchmark for provider/tool design, weaker as a full five-trait peer |
| `async-openai` | OpenAI-specific Rust SDK | Typed OpenAI tool and hosted-tool schemas, but no local coding-tool runtime | Strong typed OpenAI config/client layer for HTTP, SSE, and realtime APIs | Not a reusable local agent loop; exposes primitives and examples | Delegates approvals, shell, search, and sandbox semantics to OpenAI APIs | Rust SDK with partial WASM support and Azure/OpenAI-compatible config | Protocol SDK workspace, not an engine | High relevance for `brain-providers/openai`, low relevance for full engine architecture |
| `Vercel AI SDK` | Provider and agent SDK | `tool()` / `dynamicTool()` plus local execution and provider-executed tool protocols | Strongest provider interface of the set; unified model factories and streaming APIs | Focused tool loop around `generateText()` / `streamText()` and `ToolLoopAgent` | Approval protocol exists; sandboxing delegated to external systems | Node plus React, Vue, Svelte, Angular, Next.js, LangChain integrations | Highly composable TS monorepo SDK | Best provider/tool API benchmark; narrower runtime scope |
| `pi-mono` | Layered agent toolkit and product family | Serializable tool schemas, auto tool execution, concrete coding-agent tools | Package-based provider registry and model registry with overrides and OAuth | Explicit loop in `pi-agent-core`, then session management and compaction in coding agent | Mostly hook- and app-layer approval; sandboxing varies by package | CLI, RPC, SDK, web UI, Slack bot, browser-capable packages | Clear package layering from SDK to runtime to products | Closest TypeScript runtime comparison |
| `IronClaw` | Security-first AI agent product | Intended provider-native function calling, but concrete registry wiring looks incomplete | Single provider trait plus preset aliases and a broad but uneven provider factory | Simple conversation loop; tool use appears single-pass | Strong policy design and sandbox backends, but integration is not always obvious | Mostly Unix-first from current source and docs | Monolithic Rust binary | Good security reference, weaker engine reference |
| `ZeroClaw` | Rust agent runtime OS / platform | Real `Tool` trait with native or prompt/XML dispatch depending on provider capability | Rich provider trait, capability flags, routing, aliases, and fallbacks | Real iterative tool loop with memory, routing, and configurable tool iterations | Strong command/path policy; runtime adapters; Docker isolation stronger than native | Linux, macOS, Windows, Termux | Large root crate with real traits and some workspace split | Closest Rust architecture benchmark today |

## `brain` Baseline

Current local baseline for `brain`'s native tools:

- `FileRead`: native `tokio::fs::read_to_string` with line-number formatting
- `FileWrite`: native `tokio::fs::create_dir_all` plus `tokio::fs::write`
- `FileEdit`: native read / unique-match replace / write flow
- `Glob`: Rust `glob` crate
- `Grep`: native `walkdir` plus Rust `regex`; does not shell out to `rg`
- `Shell`: `tokio::process::Command` using `sh -c`

This matters because several competitors expose a similar tool surface but implement it very differently:

- some use native filesystem code
- some shell out to `rg`, `fd`, or the system shell
- some expose only provider-defined remote tools rather than local runtime tools

## Core Trait Matrix Against `brain`

Legend:

- `first-class`: clear top-level boundary in the repo
- `implicit`: real subsystem, but product-level rather than a clean reusable interface
- `missing`: not central or not evidenced as a real boundary

| Project | `Provider` | `Tool` | `Store` | `AgentLoop` | `Transport` | Read Against `brain` |
| --- | --- | --- | --- | --- | --- | --- |
| `brain` | first-class trait | first-class trait | first-class trait | first-class trait | first-class trait | Baseline target architecture |
| `OpenCode` | first-class | first-class | first-class, but concrete storage modules | implicit | implicit | Strong runtime package, weaker trait seams |
| `OpenClaw` | first-class | first-class | implicit | implicit | first-class | Strong gateway and tool boundaries, but loop/store are distributed |
| `Gastown` | implicit | missing | first-class | first-class | first-class | Excellent control-plane reference, weak direct match for provider/tool core |
| `Rig` | first-class | first-class | implicit | implicit | implicit | Strong Rust SDK benchmark for provider/tool composition |
| `async-openai` | first-class | first-class, but protocol-level | implicit | missing | missing | Best read as a provider SDK reference, not an engine peer |
| `Vercel AI SDK` | first-class | first-class | implicit | first-class | first-class | Excellent SDK seams, but store is not a core runtime concept |
| `pi-mono` | first-class | first-class | first-class, but concrete classes | first-class | first-class | Closest TS runtime analogue to `brain` |
| `IronClaw` | first-class | first-class, but skeletal | first-class | implicit | first-class via channels | Architecture vocabulary exists, but some seams are thinly realized |
| `ZeroClaw` | first-class | first-class | first-class | implicit | first-class via channels | Closest Rust trait-driven comparison |

## Tool Implementation Matrix

Legend:

- `native`: implemented in repo code using local runtime or language APIs
- `shell-out`: implemented by spawning external tools such as `rg`, `fd`, or a shell
- `provider-defined`: repo defines the tool schema or adapter, but execution happens in the external provider runtime
- `declared only`: mentioned or modeled, but not clearly implemented as a concrete tool in the repo snapshot

| Project | `FileRead` | `FileWrite` | `FileEdit` | `Glob` / `Find` | `Grep` / content search | `Shell` / `Bash` |
| --- | --- | --- | --- | --- | --- | --- |
| `brain` | native Rust fs | native Rust fs | native Rust replace/write | native `glob` crate | native `walkdir` + `regex` | shell-out via `sh -c` |
| `OpenCode` | native Node/Bun fs | native Node/Bun fs | native edit plus diff helpers | shell-out to `rg` | shell-out to `rg` | shell-out via spawned shell |
| `OpenClaw` | native wrappers over upstream tool plus fs / sandbox bridge | native wrappers over upstream tool plus fs / sandbox bridge | native wrappers over upstream tool plus fs / sandbox bridge | not evidenced as local concrete tool | not evidenced as local concrete tool | native process control plus PTY and optional Docker exec |
| `Gastown` | delegated to external runtimes | delegated to external runtimes | delegated to external runtimes | not centralized as a GT tool | not centralized as a GT tool | delegated to external runtimes with GT guardrails around session behavior |
| `Rig` | delegated or app-authored | delegated or app-authored | delegated or app-authored | not centralized as agent tool; ingestion helpers exist | delegated or provider-hosted | delegated or app-authored |
| `async-openai` | not centralized as local tool | delegated to hosted APIs | delegated to hosted APIs | missing as generic built-in | delegated to hosted search APIs | delegated to hosted shell/container APIs |
| `Vercel AI SDK` | provider-defined | provider-defined | provider-defined | missing as generic built-in | missing as generic built-in | provider-defined |
| `pi-mono` | native Node fs | native Node fs | native Node edit logic | shell-out to `fd` in coding agent | shell-out to `rg` in coding agent | shell-out via spawned shell |
| `IronClaw` | declared only | declared only | missing or not central | missing as agent tool | missing as agent tool | declared only as tool; shell spawning exists elsewhere |
| `ZeroClaw` | native Rust fs | native Rust fs | native Rust replace/write | native `glob` crate | shell-out to `rg`, fallback to `grep` | shell-out via runtime adapter |

## Tool Takeaways

- `OpenCode` and `pi-mono` expose a very similar user-facing coding-agent tool surface, but both rely on external command-line tools for search-heavy primitives. In both cases, `grep` is effectively `rg` orchestration rather than a pure in-process search engine.
- `ZeroClaw` is the closest external match to `brain`'s current tool philosophy for file operations: read, write, edit, and glob are native runtime tools, while full-text search still shells out to `rg`.
- `Rig` is useful for comparing tool abstractions and MCP/vector-store integration, but it does not try to ship a built-in local coding-tool suite like `brain`, `pi-mono`, or `ZeroClaw`.
- `async-openai` is useful one layer lower than the rest: it models hosted tool schemas and streaming events very well, but it is not a local tool runtime at all.
- `Vercel AI SDK` is not a strong reference for local file/search tool implementation because most comparable tools are provider-defined remote tools rather than local runtime tools.
- `OpenClaw` is somewhat deceptive if looked at only from prompts and docs: the product exposes coding-style tools, but not all of those tools are implemented locally in the repo. Some are inherited from the embedded `pi-*` runtime.
- `Gastown` should be read separately from the others: it is not trying to own file/search/shell tools at all. It orchestrates runtimes that already have those tools, then adds persistent tracking, mail, nudges, hooks, and role-based control on top.
- `IronClaw` currently looks more like a declared tool architecture than a fully realized concrete tool stack.

## Suggested Next Analyses

- `mastra`: useful if you want another SDK-first comparison with stronger workflow/app-builder positioning.
- `langgraph`: useful if you want a graph-oriented agent-loop benchmark rather than a coding-agent benchmark.

## Working Conclusion

If the goal is to sharpen `brain`'s engine boundaries, the most useful primary comparisons are:

- `ZeroClaw` for Rust trait design and runtime composition
- `pi-mono` for layered agent-loop packaging
- `Vercel AI SDK` for provider and tool-call API design

Secondary but still important references are:

- `Rig` for Rust provider/tool/RAG library design
- `async-openai` for OpenAI wire formats, streaming events, and hosted tool schemas

If the goal is to pressure-test product direction beyond the core engine, the most useful primary comparisons are:

- `OpenCode` for coding-agent UX and session orchestration
- `OpenClaw` for gateway, approvals, nodes, and multi-surface deployment
- `Gastown` for persistent multi-agent orchestration and work tracking
- `IronClaw` for security policy ambitions
