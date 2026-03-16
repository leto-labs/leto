# OpenCode

## One-Line Take

`OpenCode` is a product-complete AI coding platform with a large integrated runtime, not a minimal reusable engine.

## Snapshot

- Vertical: end-user coding agent product spanning CLI, desktop, IDE, HTTP server, SDK, and web surfaces.
- Best comparison inside `brain`: product pressure on top of `AgentLoop`, `Transport`, and permission orchestration.
- Main lesson: rich product behavior comes from a deeply integrated runtime, but that same integration makes the core less cleanly composable.

## Tool Call Method

`OpenCode` builds tools from a central runtime registry. The model-facing tool surface can include built-in tools, config-defined JS/TS tools, plugin tools, and MCP tools. The set is adaptive: some models get `apply_patch`, others get `edit` and `write`, and some search tools are gated behind provider-specific capabilities.

Execution is heavily productized. Tool runs pass through plugin hooks, schema validation, result truncation, and context-management logic before their outputs are fed back into the session. This is much closer to a product shell around tool use than to `brain`'s smaller `Tool` trait boundary.

## Provider / Model / Mode Method

Provider support is catalog-driven and adapter-heavy. `OpenCode` merges provider metadata, auth state, config, and plugin state, then instantiates providers through its runtime and bundled adapters. The abstraction is flexible, but it is configuration-first rather than a small language-level interface like `brain`'s `Provider` trait.

`OpenCode` also treats "mode" as "agent". Built-in agents such as `build`, `plan`, `general`, and `explore` can each override model, prompt, permissions, tools, and generation settings. That makes agent behavior a first-class product construct instead of a narrow loop primitive.

## Agent Loop Method

The loop is persistent and session-centric. It rehydrates history, creates a new assistant turn, streams model output, records structured session parts, executes tools, handles retries, and can trigger compaction or child-session delegation. It also includes doom-loop protection when the same tool call repeats too often.

This is operationally mature and user-friendly, but it is tightly coupled to product concerns such as session snapshots, UI streaming, compaction workers, and child task sessions. `brain` can stay cleaner by keeping those concerns outside the smallest orchestration core.

## Permissions / Sandbox Method

The safety model is primarily policy-based: `allow`, `ask`, and `deny` rules can be scoped by tool or wildcard pattern and overridden per agent. File access can detect external directories and protect certain OS paths, but command execution is still host execution through spawned processes.

The key implication is that `OpenCode` has a strong approval system but not a strong built-in sandbox boundary. That is useful as a UX reference, but less compelling as a hard-isolation architecture.

## Platform Support

`OpenCode` has unusually broad first-party surface area:

- terminal UI
- desktop applications
- VS Code integration
- HTTP/OpenAPI server
- JS SDK
- web client

This is one of the clearest cases where a higher-level product repo will naturally outperform a focused engine repo on user-facing breadth.

## Technical Architecture

The repo is a monorepo, but the important architectural fact is that a large amount of core behavior lives inside one dominant runtime package. Provider wiring, sessions, tools, permissions, MCP, server behavior, and project management all live close together.

That makes `OpenCode` highly capable and extensible through plugins and config, but less aligned with `brain`'s goal of a cleanly swappable engine core. It is best described as a modular monolith rather than a small composable substrate.

## Server/Client Architecture

### Process Model

OpenCode uses a **Worker thread** model. The default `opencode` command runs both the TUI and the server logic in one process, with the server running on a Worker thread. There is no persistent daemon.

| Command | Server Location | Process Model |
|---------|-----------------|---------------|
| `opencode` (default TUI) | Worker thread, same process | In-process by default; Worker starts HTTP with `--port` |
| `opencode run` | Same process | No HTTP; direct `Server.Default().fetch()` |
| `opencode serve` | Same process | Starts `Bun.serve()` on configured port and blocks |
| `opencode attach <url>` | External (connect to running server) | TUI-only client over HTTP |
| `opencode run --attach <url>` | External (connect to running server) | Headless client over HTTP |

### Default Flow (`opencode`)

1. Main thread spawns a Worker (`worker.ts`).
2. Worker starts an event stream via `Server.Default().fetch(request)` -- **no HTTP server**, just in-process calls.
3. TUI connects to Worker via `createWorkerFetch(client)` -- RPC to Worker, which calls `Server.Default().fetch()`.
4. If `--port` / `--hostname` / `--mdns` flags are set, Worker starts `Bun.serve()` and TUI connects over HTTP instead.

### Headless Server Mode

`opencode serve` starts a standalone HTTP server using `Bun.serve()`:

- Binds to configured port (tries 4096 first, then falls back to 0).
- Same REST + SSE API as the embedded server.
- Password protection via `OPENCODE_SERVER_PASSWORD` (HTTP Basic auth, username `opencode`).
- Optional mDNS advertisement with `--mdns`.

### Client Attachment

`opencode attach <url>` connects a TUI to a running server:

- Client creates `OpencodeClient` via `createOpencodeClient({ baseUrl, headers })`.
- SDK is generated from an OpenAPI 3.1 spec (`@opencode-ai/sdk/v2`).
- Events are received via **SSE** (`GET /event`) with 10-second heartbeat.
- REST calls for actions (`POST /session`, `POST /session/:id/message`, etc.).
- Authentication is HTTP Basic from `--password` or `$OPENCODE_SERVER_PASSWORD`.
- URL is always provided manually; no auto-discovery despite optional mDNS publish.

`opencode run --attach <url>` is the same but with stdout output instead of TUI rendering.

### Server Lifecycle

The server does **not** persist after the CLI exits. Worker shutdown calls `Instance.disposeAll()` and `server.stop()`. No lock files, no PID detection, no socket-based reconnection.

### API Surface

The server exposes a Hono-based HTTP API:

- `GET /event` -- SSE event stream (all bus events)
- `POST /session` -- create session
- `POST /session/:id/message` -- send message
- `GET /session` -- list sessions
- `GET /config`, `PATCH /config` -- config management
- `GET /provider` -- list providers
- Plus: MCP, permissions, file, and TUI endpoints

### Client/Server Boundary

The boundary is the HTTP API (REST + SSE), exposed as a typed SDK generated from OpenAPI. In-process mode bypasses HTTP entirely via `Server.Default().fetch()`. The TUI never talks to the agent runtime directly -- always through the server API, whether in-process or remote.

## Mapping To `brain` Core Traits

- `Provider`: first-class boundary via dedicated provider modules and runtime model resolution.
- `Tool`: first-class boundary via a common tool contract and central registry.
- `Store`: first-class in practice, but implemented as concrete storage modules instead of a small swappable trait.
- `AgentLoop`: implicit runtime boundary; the loop is central, but it is not exposed as a clean standalone interface.
- `Transport`: implicit runtime boundary; HTTP server, SDK, and TUI clients exist, but not behind one `Transport` abstraction.

The short version is that `OpenCode` is strongest where `brain` cares about `Provider` and `Tool`, but it is more product-runtime oriented than trait-oriented for `Store`, `AgentLoop`, and `Transport`.

## Concrete Tool Implementation Notes

- `FileRead`: implemented locally with native Node/Bun file APIs. No shell-out.
- `FileWrite`: implemented locally with native file writes. No shell-out.
- `FileEdit`: implemented locally with native fs plus diff-oriented helper logic. `apply_patch` is a separate structured tool for some model families.
- `Glob`: implemented as a ripgrep-backed file listing flow rather than a pure in-process glob implementation.
- `Grep`: explicitly shells out to `rg` and post-processes results in TypeScript.
- `Shell` / `Bash`: implemented by spawning the configured shell and layering permission analysis on top.

This makes `OpenCode` a good example of a product that keeps file mutation local but treats search primitives as wrappers around battle-tested external CLI tools.

## What To Steal

- First-class session streaming and structured event persistence.
- Strong agent-profile concept where prompts, permissions, tools, and model choice move together.
- Good operational features around compaction, retry, and child-task delegation.

## What To Differentiate

- Keep provider, tool, store, loop, and transport boundaries explicit instead of burying them in one product runtime.
- Prefer a harder execution boundary than approval rules alone.
- Avoid coupling the engine core to desktop, TUI, SDK, and server concerns.

## Data Model

### Project

`ProjectTable` (Drizzle SQLite): id, worktree, vcs, name, icon_url, icon_color, sandboxes (JSON array), commands (JSON), timestamps. `worktree` is the git worktree root. A separate `WorkspaceTable` links workspaces (branches) to projects.

### Session

`SessionTable` (SQLite): id, project_id, workspace_id, parent_id, slug, directory, title, version, share_url, summary_additions, summary_deletions, summary_files, summary_diffs (JSON), revert (JSON), permission (JSON ruleset), timestamps, time_compacting, time_archived. Supports fork (via parent_id), compaction, and archiving.

### Message + Part

Two-table design. `MessageTable`: id, session_id, timestamps, data (JSON blob). `PartTable`: id, message_id, session_id, timestamps, data (JSON blob). Part types include: text, reasoning, file, tool, snapshot, patch, agent, compaction, subtask, retry, step-start, step-finish. Tool calls are `ToolPart` with callID, tool name, and state (pending/running/completed/error).

### Credentials

`AccountTable` (SQLite): id, email, url, access_token, refresh_token, token_expiry, timestamps. `AccountStateTable` tracks the active account. Single active account model, not multi-credential-per-provider.

### Storage

Global data in `~/.local/share/opencode/` (XDG data dir), config in `~/.config/opencode/`. All persistence is SQLite (`opencode.db`). Project-local: `.opencode/` directory and `opencode.json`.

## Key Evidence

- `repocache/anomalyco/opencode/packages/opencode/src/cli/cmd/tui/thread.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/cli/cmd/tui/worker.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/cli/cmd/tui/attach.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/cli/cmd/tui/app.tsx`
- `repocache/anomalyco/opencode/packages/opencode/src/cli/cmd/tui/context/sdk.tsx`
- `repocache/anomalyco/opencode/packages/opencode/src/cli/cmd/serve.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/cli/cmd/run.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/server/server.ts`
- `repocache/anomalyco/opencode/packages/sdk/js/src/v2/client.ts`
- `repocache/anomalyco/opencode/README.md`
- `repocache/anomalyco/opencode/packages/web/src/content/docs/agents.mdx`
- `repocache/anomalyco/opencode/packages/web/src/content/docs/permissions.mdx`
- `repocache/anomalyco/opencode/packages/web/src/content/docs/server.mdx`
- `repocache/anomalyco/opencode/packages/opencode/src/agent/agent.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/session/prompt.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/session/processor.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/provider/provider.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/tool/registry.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/tool/read.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/tool/write.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/tool/edit.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/tool/apply_patch.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/tool/glob.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/tool/grep.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/file/ripgrep.ts`
- `repocache/anomalyco/opencode/packages/opencode/src/tool/bash.ts`
