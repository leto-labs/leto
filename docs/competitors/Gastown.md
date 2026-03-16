# Gastown

## One-Line Take

`Gastown` is a very advanced orchestration and control-plane layer for external coding agents, not a direct engine peer to `brain`.

## Snapshot

- Vertical: multi-agent workspace manager and orchestration layer.
- Best comparison inside `brain`: persistent coordination, work tracking, runtime lifecycle, and transport-heavy agent operations.
- Main lesson: `Gastown` is what the stack above a core engine can look like once multi-agent dispatch, recovery, and visibility become first-class.

## Tool Call Method

`Gastown` does not primarily own a local coding-tool runtime. Instead, it delegates tool execution to managed runtimes such as Claude Code, Codex, Cursor, OpenCode, Pi, and related agent CLIs. The repo is explicit that it prefers loose coupling and CLI-level integration instead of importing agent libraries directly.

What `Gastown` does add is orchestration around those tool calls: startup priming, work assignment, mail, nudges, hooks, runtime wrapper scripts, and session management. This makes it much more of a control plane than a tool-runtime implementation.

## Provider / Runtime Method

The closest analogue to `brain`'s `Provider` trait is a runtime preset registry for agent CLIs. `Gastown` knows how to launch and manage different external runtimes, inject environment/config, and apply agent-specific startup behavior.

That means its "provider" layer is really a runtime adapter layer. It abstracts over agent executables rather than model providers or inference backends.

## Agent Loop Method

This is where `Gastown` is strongest. The system centers on orchestration roles and flows such as Mayor, Polecats, Convoys, Witness, Deacon, hooks, and persistent work tracking. The loop is not a single reasoning kernel; it is a coordination workflow spanning multiple sessions, roles, and work items over time.

Relative to `brain`, `Gastown` is a useful benchmark for what can sit above `AgentLoop` once durable multi-agent operations become part of the product.

## Permissions / Sandbox Method

The current security model is practical but not idealized. Much of the runtime behavior relies on vendor-specific bypass or yolo modes for the external agents, with additional guardrails layered on top through hooks and command allowlists.

The repo also discusses stronger sandboxed polecat execution, but the current default model is still host-oriented. So `Gastown` is a useful coordination and policy reference, but not a strong reference for hard local sandbox boundaries.

## Platform Support

`Gastown` ships platform binaries and supports macOS, Linux, and Windows. Operationally, though, it is still very tmux-centric, which makes the workflow feel most natural on Unix-like systems even if Windows is supported.

It also includes dashboard and feed surfaces, which reinforces the point that this is a control-plane product rather than a minimal runtime library.

## Technical Architecture

`Gastown` is a Go product organized around:

- Beads and Dolt-backed persistent work state
- git worktrees and hooks
- tmux session management
- convoy-style work distribution
- dashboard and live-feed surfaces
- runtime wrapper scripts for external agents

The strongest architectural emphasis is on `Store`, `AgentLoop`, and `Transport`, not on a first-class local `Tool` layer.

## Server/Client Architecture

### Process Model

Gastown is a **CLI-only** tool with no central server. Each `gt` command is a separate process invocation. External agents are managed via tmux sessions, not client/server IPC.

| Component | Role | Transport |
|-----------|------|-----------|
| `gt` (CLI) | Short-lived command | Direct execution |
| `gt dashboard` | Long-lived HTTP server | HTTP + htmx (port 8080) |
| `gt feed` | Long-lived TUI | Reads Dolt/beads/agent state |
| `gt dolt start` | Background daemon | Dolt SQL Server for beads storage |
| External agents | Managed via tmux | `send-keys`, `pane_current_command` |

### No Central Server

Gastown does not run a server process for agent orchestration. Instead:

- Each `gt` CLI invocation is independent and exits when done.
- Agent coordination happens through Dolt (persistent SQL database), git worktrees, and tmux sessions.
- External agents (Claude Code, Codex, Cursor, OpenCode, etc.) run in their own tmux panes.
- Gastown injects context via environment variables, wrapper scripts, and Claude Code hooks.

### Dashboard

`gt dashboard` starts an HTTP server (default port 8080):

- Single-page overview: agents, convoys, hooks, queues, issues, escalations.
- Auto-refreshes via htmx (no WebSocket).
- Command palette runs `gt` subprocesses via allowed command whitelist.
- Purely read + command execution -- no bidirectional agent communication.

### Feed TUI

`gt feed` runs an interactive terminal dashboard:

- Agent Tree: hierarchical view of agents grouped by rig and role.
- Convoy Panel: in-progress and recently-landed convoys.
- Event Stream: chronological feed of beads activity.
- Reads from Dolt and agent state; does not drive agents.

### Agent Integration

Loose coupling via tmux:

- Launch agents in tmux: `NudgeSession`, `send-keys`.
- Detect liveness: `pane_current_command`.
- Inject context: wrapper scripts (`gt-codex`, etc.), hooks (`gastown.js`), environment variables.
- No direct imports of agent code; agents are treated as external CLIs.

### Server Lifecycle

- CLI: each invocation exits immediately.
- Dashboard: long-lived while running, no daemon behavior.
- Dolt: long-lived SQL server, managed via `gt dolt start/stop`.
- Agents: persist in tmux until killed.

### Client/Server Boundary

No formal client/server boundary. The coordination model is file-based (git, Dolt) and process-based (tmux) rather than network-based. The dashboard is read-only HTTP, not a control API.

## Mapping To `brain` Core Traits

- `Provider`: implicit boundary through runtime presets and agent CLI wrappers.
- `Tool`: missing as a central GT-owned abstraction; tool behavior is delegated to external runtimes.
- `Store`: first-class and central to the product.
- `AgentLoop`: first-class, but as orchestration workflows rather than a compact reasoning loop.
- `Transport`: first-class through tmux sessions, mail, nudges, dashboards, and runtime lifecycle control.

The simplest read is that `Gastown` is highly relevant to the upper half of the stack above `brain`, especially where persistence and multi-agent coordination matter.

## Concrete Tool Implementation Notes

- `FileRead`: delegated to the external runtime.
- `FileWrite`: delegated to the external runtime.
- `FileEdit`: delegated to the external runtime.
- `Glob` / `Find`: not centralized as a GT-owned agent tool.
- `Grep` / content search: not centralized as a GT-owned agent tool.
- `Shell` / `Bash`: delegated to the external runtime, though Gastown adds wrapper behavior and guardrails around session execution.

This is the main reason `Gastown` belongs in the matrix as an orchestration-layer competitor rather than a local tool-runtime benchmark.

## What To Steal

- Persistent work tracking that survives agent restarts.
- Multi-agent orchestration concepts such as convoys, handoffs, nudges, and role-specific coordination.
- Control-plane visibility through dashboards, feeds, and durable session metadata.

## What To Differentiate

- Keep `brain` strong as a local engine core instead of collapsing into pure CLI orchestration.
- Preserve first-class `Provider` and `Tool` architecture rather than delegating all capabilities to external runtimes.
- Build a cleaner sandbox and permission story than vendor bypass modes plus wrappers.

## Key Evidence

- `repocache/steveyegge/gastown/cmd/gt/main.go`
- `repocache/steveyegge/gastown/internal/cmd/root.go`
- `repocache/steveyegge/gastown/internal/tmux/tmux.go`
- `repocache/steveyegge/gastown/internal/web/commands.go`
- `repocache/steveyegge/gastown/docs/design/architecture.md`
- `repocache/steveyegge/gastown/docs/agent-provider-integration.md`
- `repocache/steveyegge/gastown/README.md`
- `repocache/steveyegge/gastown/docs/agent-provider-integration.md`
- `repocache/steveyegge/gastown/docs/design/architecture.md`
- `repocache/steveyegge/gastown/docs/design/agent-api-inventory.md`
- `repocache/steveyegge/gastown/docs/design/sandboxed-polecat-execution.md`
- `repocache/steveyegge/gastown/internal/config/agents.go`
- `repocache/steveyegge/gastown/internal/runtime/runtime.go`
- `repocache/steveyegge/gastown/internal/hooks/templates/opencode/gastown.js`
- `repocache/steveyegge/gastown/internal/wrappers/scripts/gt-codex`
- `repocache/steveyegge/gastown/internal/web/commands.go`
