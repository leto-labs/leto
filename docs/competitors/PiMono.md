# pi-mono

## One-Line Take

`pi-mono` is the closest TypeScript analogue to `brain`: a layered stack from provider SDK to agent loop to coding-agent product.

## Snapshot

- Vertical: agent toolkit family with reusable runtime packages and multiple end-user shells.
- Best comparison inside `brain`: package layering around provider APIs, loop orchestration, product shells, and transport modes.
- Main lesson: `pi-mono` shows how far a layered runtime can go before product assumptions start to dominate the core.

## Tool Call Method

`pi-mono` separates tool schema description from tool execution. The lower layers expose serializable tool definitions and streaming tool-call deltas, while `pi-agent-core` adds automatic execution, argument validation, and sequential or parallel tool handling. Higher layers such as `pi-coding-agent` then provide concrete coding tools like file operations, shell, and search.

This is one of the clearest examples of tool orchestration being layered instead of dumped into a single app package. It is still more package- and hook-driven than trait-driven, but the structure is close in spirit to `brain`.

## Provider / Model / Mode Method

The provider layer is strong. `pi-ai` separates model metadata, provider wiring, and streaming APIs, while `pi-coding-agent` adds a model registry for overrides, custom models, and OAuth-aware provider registration.

This is a practical pattern for `brain-providers`: keep the provider layer reusable and typed, then let higher layers own model selection policy. The main difference is that `pi-mono` expresses this through package contracts rather than a narrow trait vocabulary.

## Agent Loop Method

This is the closest direct competitor area. `pi-agent-core` exposes an explicit loop with turns, streaming assistant output, tool execution, steering interrupts, and follow-up queues. `pi-coding-agent` then wraps that loop with session management, compaction, retry, and extension hooks.

The layering is useful because it preserves a meaningful "runtime core" even while allowing a full product shell above it. `brain` can learn from this without adopting all of the coding-agent-specific assumptions.

## Permissions / Sandbox Method

Permissions are mostly enforced through hooks and extensions rather than a single engine-wide runtime boundary. The coding agent can implement confirmation gates for risky actions, while package-specific components such as `pi-mom` add stronger sandbox options for their own deployment model.

That makes the system flexible, but also somewhat inconsistent across packages. `brain` can differentiate by making permission and execution boundaries clearer and more uniform at the engine level.

## Platform Support

`pi-mono` supports a broad set of product surfaces:

- CLI and terminal workflows
- RPC mode over stdin/stdout JSONL
- SDK usage
- web UI packages
- Slack bot deployment
- browser-capable provider tooling in lower layers

This breadth makes it a valuable benchmark for transport thinking, especially if `brain` wants to support multiple frontends without bloating its orchestration core.

## Technical Architecture

The repo has clear package layering:

- provider and model layer
- agent runtime layer
- coding-agent product layer
- UI and deployment shells

This is more composable than the product-heavy monoliths in this comparison set, but still more opinionated than `brain`'s explicit five-trait design. Persistence, compaction, and transport are concrete implementations before they are abstract engine concepts.

## Mapping To `brain` Core Traits

- `Provider`: first-class boundary through `pi-ai` and the model registry layer.
- `Tool`: first-class boundary through `AgentTool` and concrete built-in tool packages.
- `Store`: first-class in practice through concrete session-manager and store components rather than a minimal generic trait.
- `AgentLoop`: first-class boundary through `pi-agent-core`.
- `Transport`: first-class boundary through provider transport choices, proxy/RPC layers, and multiple runtime modes.

This is the cleanest TypeScript comparison for `brain` because all five concerns exist clearly, even if some are package contracts rather than narrow engine traits.

## Concrete Tool Implementation Notes

- `FileRead`: native Node filesystem implementation.
- `FileWrite`: native Node filesystem implementation.
- `FileEdit`: native Node read/replace/write flow with local diff and fuzzy-match helpers.
- `Glob` / `Find`: implemented as `find`, primarily by shelling out to `fd`; local glob logic mainly supports ignore discovery.
- `Grep`: implemented by shelling out to `rg`, then reconstructing context and truncation behavior in Node.
- `Shell` / `Bash`: implemented locally by spawning the configured shell.

`pi-mono` is especially useful because it mixes both styles `brain` may care about: native in-process file tools and shell-out search tools.

## What To Steal

- Layered progression from provider package to loop package to product shell.
- Explicit loop orchestration with streaming and tool continuation.
- RPC mode and multi-shell thinking that does not require one UI to dominate the runtime.

## What To Differentiate

- Keep the engine traits more explicit and central than package contracts and hooks.
- Make permission and sandbox behavior more consistent across surfaces.
- Avoid letting coding-agent assumptions become the default engine shape.

## Data Model

### Sessions

`SessionHeader { type: "session", version, id, timestamp, cwd, parentSession }`. Sessions stored as JSONL files at `~/.pi/agent/sessions/<encoded-cwd>/<timestamp>_<uuid>.jsonl`. First line is header; rest are `SessionEntry` records. `SessionInfo` (for listing): path, id, cwd, name, parentSessionPath, created, modified, messageCount, firstMessage.

### Messages

Union type: `UserMessage { role: "user", content, timestamp }`, `AssistantMessage { role: "assistant", content: (TextContent | ThinkingContent | ToolCall)[], api, provider, model, usage, stopReason, timestamp }`, `ToolResultMessage { role: "toolResult", toolCallId, toolName, content, details, isError, timestamp }`. Tool calls are content blocks inside AssistantMessage, not separate messages. Coding-agent adds custom roles: `BashExecutionMessage`, `CompactionSummaryMessage`, `BranchSummaryMessage`, `CustomMessage`.

### Session Entries

JSONL entries are a tree structure via parentId: `SessionMessageEntry`, `ThinkingLevelChangeEntry`, `ModelChangeEntry`, `CompactionEntry` (summary, firstKeptEntryId, tokensBefore), `BranchSummaryEntry`, `LabelEntry`, `SessionInfoEntry`, `CustomEntry`, `CustomMessageEntry`.

### Credentials

`AuthCredential = ApiKeyCredential { type: "api_key", key } | OAuthCredential { type: "oauth", ...OAuthCredentials }`. Stored in `~/.pi/agent/auth.json` (chmod 0600) as flat dict keyed by provider. Resolution: runtime override -> auth.json -> env vars -> fallback resolver.

### Compaction

`CompactionSettings { enabled, reserveTokens: 16384, keepRecentTokens: 20000 }`. Flow: `prepareCompaction()` -> `findCutPoint()` -> `generateSummary()` -> `compact()` -> append `CompactionEntry` to JSONL.

### Storage

Sessions at `~/.pi/agent/sessions/<encoded-cwd>/`. Credentials at `~/.pi/agent/auth.json`. Config in `.pi/` project-local. No dedicated project entity; sessions keyed by encoded cwd.

## Key Evidence

- `repocache/badlogic/pi-mono/package.json`
- `repocache/badlogic/pi-mono/packages/ai/README.md`
- `repocache/badlogic/pi-mono/packages/ai/src/types.ts`
- `repocache/badlogic/pi-mono/packages/ai/src/stream.ts`
- `repocache/badlogic/pi-mono/packages/agent/README.md`
- `repocache/badlogic/pi-mono/packages/agent/src/agent-loop.ts`
- `repocache/badlogic/pi-mono/packages/agent/src/types.ts`
- `repocache/badlogic/pi-mono/packages/coding-agent/README.md`
- `repocache/badlogic/pi-mono/packages/coding-agent/src/core/model-registry.ts`
- `repocache/badlogic/pi-mono/packages/coding-agent/src/core/tools/read.ts`
- `repocache/badlogic/pi-mono/packages/coding-agent/src/core/tools/write.ts`
- `repocache/badlogic/pi-mono/packages/coding-agent/src/core/tools/edit.ts`
- `repocache/badlogic/pi-mono/packages/coding-agent/src/core/tools/find.ts`
- `repocache/badlogic/pi-mono/packages/coding-agent/src/core/tools/grep.ts`
- `repocache/badlogic/pi-mono/packages/coding-agent/src/core/tools/bash.ts`
- `repocache/badlogic/pi-mono/packages/coding-agent/docs/rpc.md`
- `repocache/badlogic/pi-mono/packages/mom/docs/sandbox.md`
