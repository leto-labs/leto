# IronClaw

## One-Line Take

`IronClaw` is most useful as a security-policy reference, but less convincing as a clean engine architecture benchmark.

## Snapshot

- Vertical: security-first AI agent product with broad platform ambitions.
- Best comparison inside `brain`: permission policy, approval rules, and sandbox ambition.
- Main lesson: strong security claims are only as useful as their wiring into the actual execution path.

## Tool Call Method

The intended tool model is provider-native function calling backed by JSON schemas from a registry. On paper, that is a solid direction. In the current source, though, the architectural tool layer appears stronger than the concrete runtime wiring. The engine builds a registry and works with tool schemas, but the repo evidence suggests the concrete tool implementation path is not as obviously connected as in stronger competitors.

This makes `IronClaw` a cautionary comparison: defining the seam is not enough if the operational path is unclear.

## Provider / Model / Mode Method

The provider layer revolves around a single `Provider` trait and a large provider factory with presets like `fast`, `smart`, `cheap`, and `local`. That is conceptually close to `brain`'s interest in swappable providers.

The limitation is completeness. Some provider branches are clearly stubs, which weakens the repo as a direct provider-quality benchmark even if the abstraction direction is reasonable.

## Agent Loop Method

The loop is relatively simple: send the conversation, receive tool calls, execute them through the security pipeline, append outputs, and end the turn. It does not look like a rich iterative loop that repeatedly re-queries the model inside the same turn after each tool execution.

Compared with `brain`, this is less mature as an `AgentLoop` reference. It is closer to a conversation shell around provider interaction than to a strong standalone orchestration core.

## Permissions / Sandbox Method

This is the strongest part of the repo. `IronClaw` encodes:

- RBAC
- filesystem and network policy
- approval gates
- DLP and anti-stealer checks
- SSRF protections
- audit logging
- sandbox backends for Docker and Bubblewrap

The caution is that the runtime path does not always make sandbox integration as concrete as the policy surface suggests. The design ambition is strong; the current wiring is less persuasive.

## Platform Support

The repo looks mostly Unix-first from the current code and dependencies. The docs mention generic Rust setup and Docker/Ollama workflows, but the implementation evidence is much stronger for Linux-like environments than for a broad cross-platform story.

## Technical Architecture

`IronClaw` is a monolithic Rust application rather than a clearly separated set of crates. That keeps everything in one place, but it also means the boundaries between provider, loop, permissions, and transport are weaker than `brain`'s intended structure.

As a result, it is more useful for policy ideas than for package or trait architecture.

## Mapping To `brain` Core Traits

- `Provider`: first-class boundary through an explicit trait and provider factory.
- `Tool`: first-class on paper, but skeletal in the current snapshot.
- `Store`: first-class via memory-store abstractions, though backend completeness varies.
- `AgentLoop`: implicit boundary living inside the main engine and workflow orchestration.
- `Transport`: first-class through channel abstractions, though some implementations appear thin or stub-like.

This means `IronClaw` is strongest as an architecture vocabulary reference and weaker as proof that those boundaries are deeply realized.

## Concrete Tool Implementation Notes

- `FileRead`: declared in the broader tool architecture, but not clearly implemented as a concrete local tool in the current snapshot.
- `FileWrite`: same as file read.
- `FileEdit`: not evidenced as a concrete built-in tool.
- `Glob` / `Find`: used internally for policy-style matching, not evidenced as an agent tool.
- `Grep`: not evidenced as a concrete agent tool.
- `Shell` / `Bash`: conceptually present in policy and security flow, but not clearly implemented as a concrete shell tool in the main tool stack.

The key warning for `brain` is that a repo can look rich at the architecture-document level while still lacking a convincingly realized concrete tool layer.

## What To Steal

- Config invariants that prevent obviously unsafe combinations.
- Security policy vocabulary that is richer than simple allow/deny prompts.
- Defense-in-depth thinking around audit, DLP, and network controls.

## What To Differentiate

- Ensure the tool registry and sandbox boundary are obviously wired into the real runtime path.
- Prefer smaller crate boundaries over one large binary crate.
- Build a stronger iterative loop than single-pass tool handling.

## Data Model

### Sessions

`Session { id: String (UUID v4), turn_count: u32, max_turns: u32 }`. In-memory only; no persistence. Conversation is `Vec<Message>` passed to `process_message()`.

### Messages

`Message { role: MessageRole, content: String, tool_calls: Vec<ToolCall>, tool_results: Vec<ToolResult>, timestamp, id: String (UUID v4), content_blocks: Vec<ContentBlock> }`. `MessageRole`: System, User, Assistant, Tool. `ContentBlock` enum supports multimodal: Text, Image, Audio, Video, File. In-memory only.

### Memory

`MemoryEntry { key, content, context (session/user/global), timestamp, category (System/User/Instruction/Observation/Conversation) }`. `EncryptedSqliteStore` with AES-256-GCM per entry. Key derived from `ironclaw-memory-{path}-{USER}`. History config (`HistoryConfig`) exists with SQLite/file backend options, max_conversations, max_messages, compress_old -- but wiring into the engine is not fully evident.

### Credentials

No dedicated vault. API keys come from `ProviderConfig.api_key` in config or env vars. `SessionAuthenticator` produces HMAC-SHA256 signed `SessionToken` for HTTP auth (provider, model, session_id, issued_at, expires_at) but this is session-level, not credential storage.

### Storage

XDG dirs via `directories` crate. Memory DB at `~/.ironclaw/memory.db`. No project or workspace concept.

## Key Evidence

- `repocache/JoasASantos/ironclaw/README.md`
- `repocache/JoasASantos/ironclaw/Cargo.toml`
- `repocache/JoasASantos/ironclaw/src/main.rs`
- `repocache/JoasASantos/ironclaw/src/core/engine.rs`
- `repocache/JoasASantos/ironclaw/src/core/tool.rs`
- `repocache/JoasASantos/ironclaw/src/providers/mod.rs`
- `repocache/JoasASantos/ironclaw/src/rbac/mod.rs`
- `repocache/JoasASantos/ironclaw/src/memory/mod.rs`
- `repocache/JoasASantos/ironclaw/src/channels/mod.rs`
- `repocache/JoasASantos/ironclaw/src/gateway/mod.rs`
- `repocache/JoasASantos/ironclaw/src/sandbox/mod.rs`
- `repocache/JoasASantos/ironclaw/src/core/config.rs`
- `repocache/JoasASantos/ironclaw/tests/security_tests.rs`
