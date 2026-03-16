# OpenClaw

## One-Line Take

`OpenClaw` is broader than a coding agent: it is a multi-surface personal assistant and gateway platform with strong operational controls.

## Snapshot

- Vertical: assistant platform spanning gateway, channels, nodes, voice, browser/canvas tools, UI, and agent runtime.
- Best comparison inside `brain`: transport breadth, gateway design, approval flow, and deployment surfaces.
- Main lesson: `OpenClaw` is excellent for control-plane ideas, but it is not shaped like a small reusable engine.

## Tool Call Method

`OpenClaw` treats tools as a typed first-class system. The runtime sends both structured schemas to the model API and human-readable instructions in prompts. Tool availability is filtered by policy, profile, provider, and session context.

A notable design choice is delegated client tools. Some tools can return a pending result and be executed outside the gateway runtime by a connected client or host. That is useful for multi-surface deployments and is more transport-aware than many coding-agent repos.

## Provider / Model / Mode Method

Model selection is config-first and normalized around `provider/model` references. The system supports default models, per-agent overrides, allowlists, fallback chains, and extension points for custom providers. It is a practical product architecture, but it is registry- and plugin-centric rather than trait-centric.

`OpenClaw` also inherits part of its runtime model behavior from the `pi-*` ecosystem, which means some core loop decisions are embedded from a broader runtime stack rather than defined by a narrow local interface boundary.

## Agent Loop Method

The loop is explicitly documented as a serialized per-session run: intake, context assembly, inference, tool execution, streaming, and persistence. It uses session and global lanes to coordinate execution and then bridges internal runtime events into lifecycle, tool, and assistant streams.

This is robust from an operations perspective, but the architecture is tightly integrated with gateway concerns. `brain` can learn from the serialization model without adopting the same degree of product coupling.

## Permissions / Sandbox Method

This is one of `OpenClaw`'s strongest areas. The system combines:

- gateway auth and device pairing
- tool policy
- Docker sandboxing
- execution approvals
- elevated execution modes

Sandboxing is configurable by scope and workspace access, and default container networking is restrictive. The main caveat is that plugins run in-process and unsandboxed, which weakens the extension trust boundary.

## Platform Support

`OpenClaw` has unusually broad deployment intent:

- Linux and macOS gateway runtime
- Windows via WSL2 recommendation
- macOS companion app
- iOS and Android node surfaces
- web UI and remote control plane

This makes it much more of an assistant platform than a terminal-first coding tool.

## Technical Architecture

The repo is best described as a modular monolith centered on a long-running gateway. Package boundaries exist for UI, native apps, extensions, and plugin surfaces, but the core value comes from a persistent control plane rather than a small reusable kernel.

Compared with `brain`, the key difference is where extensibility lives. `OpenClaw` leans on registries, config, and in-process plugins. `brain` aims for explicit swappable traits and a smaller orchestration center.

## Server/Client Architecture

### Process Model

OpenClaw uses a **long-lived Gateway daemon** as its central server. All clients (CLI, macOS app, web UI, mobile nodes, channel adapters) connect to the Gateway over WebSocket. This is the only framework in the comparison set that uses a persistent daemon as the default architecture.

| Component | Role | Transport |
|-----------|------|-----------|
| `openclaw gateway` | Central daemon | WebSocket + HTTP on configured port (default 18789) |
| `openclaw` (CLI) | Client | WebSocket to Gateway |
| macOS app / web UI | Client | WebSocket to Gateway |
| iOS/Android nodes | Client | WebSocket to Gateway (role: node) |
| Channel adapters | In-Gateway | WhatsApp (Baileys), Telegram (grammY), Slack (Bolt), Discord, Signal, etc. |

### Gateway

The Gateway is the only process that owns messaging surfaces and agent execution:

- One Gateway per host.
- Owns all channel connections (WhatsApp session, Telegram bot, etc.).
- Runs the embedded Pi agent runtime for inference.
- Binds to `127.0.0.1:18789` by default.
- Can be installed as a launchd/systemd service via `openclaw onboard --install-daemon`.

### Wire Protocol (WebSocket)

All client/server communication uses JSON over WebSocket:

- First frame must be `connect` (handshake with capabilities, version).
- **Requests**: `{type:"req", id, method, params}` → `{type:"res", id, ok, payload|error}`.
- **Events**: `{type:"event", event, payload, seq?, stateVersion?}`.
- Node connections declare `role: node` with explicit capabilities and commands.

### CLI as Client

The CLI is a thin client. Commands like `openclaw agent` call `callGateway()` / `GatewayClient` which connect over WebSocket to the running Gateway:

```typescript
// src/gateway/client.ts
export type GatewayClientOptions = {
  url?: string; // ws://127.0.0.1:18789
  connectDelayMs?: number;
```

### HTTP Surface

Same port serves static content:

- `/__openclaw__/canvas/` -- agent-editable HTML/CSS/JS
- `/__openclaw__/a2ui/` -- A2UI host

No REST API for agent operations -- everything goes through WebSocket.

### DM Pairing / Node Pairing

- Unknown senders are paired via DM pairing flow.
- Device nodes (iOS/Android/macOS/headless) pair with explicit capabilities.

### Server Lifecycle

The Gateway is a **persistent daemon**. It survives CLI exits and continues serving channels and nodes. It can be stopped manually or via service management. This is fundamentally different from all other frameworks in this set where the server dies with the CLI.

### Client/Server Boundary

The boundary is the WebSocket JSON-RPC protocol. `GatewayClient` in `src/gateway/client.ts` is the client-side implementation. The Gateway handles requests by dispatching to the embedded Pi agent runtime, channel adapters, and plugin system.

## Mapping To `brain` Core Traits

- `Provider`: first-class boundary through provider plugins and provider-loading infrastructure.
- `Tool`: first-class boundary through tool registration, policy, and gateway-facing assembly.
- `Store`: implicit boundary; persistence exists across sessions, transcripts, and memory systems, but not as one obvious repo-wide store interface.
- `AgentLoop`: implicit boundary; the loop is real, but much of it lives in the embedded `pi-*` runtime and gateway orchestration.
- `Transport`: first-class boundary through channels, gateway methods, and remote control surfaces.

Compared with `brain`, `OpenClaw` is strongest on `Tool` and `Transport`, while `Store` and `AgentLoop` are more distributed than trait-like.

## Concrete Tool Implementation Notes

- `FileRead`: locally implemented as OpenClaw wrappers around upstream coding-agent tools, with extra workspace and sandbox logic.
- `FileWrite`: same pattern as read; local wrappers plus native fs or sandbox bridge behavior.
- `FileEdit`: same pattern as write, with local recovery logic layered on top of upstream tool behavior.
- `Glob` / `Find`: visible in prompts and system behavior, but not clearly implemented as a local concrete tool in this repo snapshot.
- `Grep`: also visible at the product level, but not clearly implemented as a local concrete tool in this repo snapshot.
- `Shell` / `Bash`: locally implemented through `exec`-style tools, process supervision, PTY support, and optional Docker-backed execution.

The important nuance is that `OpenClaw`'s visible tool surface is broader than the set of tools it clearly implements locally; some coding tools appear inherited from the embedded `pi-*` stack.

## What To Steal

- Clear separation between gateway-level policy and agent-level execution.
- Session serialization and execution-lane discipline.
- Rich multi-surface deployment thinking, especially around delegated execution.

## What To Differentiate

- Keep extension boundaries safer than in-process unsandboxed plugins.
- Preserve trait-based engine seams instead of central registries as the main abstraction.
- Avoid making the core engine depend on a long-running gateway daemon.

## Data Model

### Sessions

`SessionEntry` with ~50 fields: sessionId, agentId, updatedAt, model/provider overrides, token counts (input/output/total, cache read/write), compactionCount, label, displayName, channel, groupId, subject, space, origin, queueMode (steer/followup/collect/queue/interrupt), sandbox/exec settings, ACP metadata. Session index stored as `sessions.json` at `~/.openclaw/agents/<agentId>/sessions/sessions.json`.

### Messages

Uses pi-coding-agent's `SessionManager` and `AgentMessage` types (same as pi-mono). Transcripts are JSONL files at `~/.openclaw/agents/<agentId>/sessions/<sessionId>.jsonl`. Message structure matches pi-mono: UserMessage, AssistantMessage (with ToolCall content blocks), ToolResultMessage.

### Agents (instead of Projects)

`AgentConfig { id, default, name, workspace, agentDir, model, skills, memorySearch, humanDelay, heartbeat, identity, groupChat, subagents, sandbox, params, tools, runtime }`. Agent is the organizational unit. Session keys are `agent:<agentId>:<mainKey>` or `agent:<agentId>:<channel>:group:<groupId>`.

### Credentials

Auth profiles: `AuthProfileConfig { provider, mode (api_key/oauth/token), email }` with ordering and cooldowns per provider. Secrets via `SecretRef { source (env/file/exec), provider, id }` and `SecretProviderConfig`. API keys from env vars or `.env` files (cwd first, then `~/.openclaw/.env`).

### Storage

Global root: `~/.openclaw/`. Per-agent sessions: `~/.openclaw/agents/<agentId>/sessions/`. Config: `~/.openclaw/openclaw.json` (JSON5). Workspace at `~/clawd/` or `AgentConfig.workspace`.

## Key Evidence

- `repocache/openclaw/openclaw/docs/concepts/architecture.md`
- `repocache/openclaw/openclaw/docs/gateway/protocol.md`
- `repocache/openclaw/openclaw/src/gateway/call.ts`
- `repocache/openclaw/openclaw/src/gateway/client.ts`
- `repocache/openclaw/openclaw/README.md`
- `repocache/openclaw/openclaw/docs/concepts/architecture.md`
- `repocache/openclaw/openclaw/docs/concepts/agent-loop.md`
- `repocache/openclaw/openclaw/docs/concepts/models.md`
- `repocache/openclaw/openclaw/docs/tools/index.md`
- `repocache/openclaw/openclaw/docs/tools/plugin.md`
- `repocache/openclaw/openclaw/docs/gateway/sandboxing.md`
- `repocache/openclaw/openclaw/docs/tools/exec-approvals.md`
- `repocache/openclaw/openclaw/src/commands/agent.ts`
- `repocache/openclaw/openclaw/src/agents/pi-embedded-runner/run.ts`
- `repocache/openclaw/openclaw/src/agents/pi-tools.ts`
- `repocache/openclaw/openclaw/src/agents/pi-tools.read.ts`
- `repocache/openclaw/openclaw/src/agents/pi-tools.host-edit.ts`
- `repocache/openclaw/openclaw/src/agents/bash-tools.exec.ts`
- `repocache/openclaw/openclaw/src/agents/bash-tools.exec-runtime.ts`
- `repocache/openclaw/openclaw/src/agents/tool-policy-shared.ts`
- `repocache/openclaw/openclaw/src/plugins/loader.ts`
