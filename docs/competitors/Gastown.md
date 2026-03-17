# Gastown

## Overview

`Gastown` is a sophisticated orchestration and control-plane layer for external coding agents, not a direct engine peer to `brain`.

| Item | Value |
| --- | --- |
| Vertical | Multi-agent workspace manager and orchestration layer |
| Best comparison inside `brain` | Persistent coordination, work tracking, runtime lifecycle, and transport-heavy agent operations |
| Main lesson | This is what the stack above a core engine can look like once durable multi-agent operations become first-class |

## Architecture

`Gastown` is a Go control-plane product built around persistent work state, git worktrees, tmux session management, dashboard/feed surfaces, and wrappers/hooks for external runtimes such as Claude Code, Codex, Cursor, OpenCode, and pi. The strongest architectural emphasis is on `Store`, orchestration, and operator visibility rather than on a GT-owned local tool runtime.

## Agent Loop

The core loop is orchestration-oriented rather than model-centric. Roles such as Mayor, Convoy, Polecat, Witness, and Deacon coordinate external agent sessions over time. The system is strongest where work tracking, mail, nudges, hooks, and runtime lifecycle matter.

For `brain`, this is most relevant as an example of what can sit above `AgentLoop`, not as a direct benchmark for the kernel loop itself.

## System Prompt & Prompt Building

Prompt construction is delegated to the managed runtimes. `Gastown` injects context through wrapper scripts, environment variables, hooks, and runtime-specific startup behavior, but it does not own a universal system-prompt builder comparable to OpenClaw or pi-mono.

## Provider & Model

The closest analogue to `brain`'s `Provider` trait is a runtime preset registry for external agent CLIs. `Gastown` abstracts over executables and operational wrappers, not over model-provider APIs.

## Tool System

`Gastown` does not primarily own a local coding-tool runtime. Tool execution is delegated to the managed runtimes, while Gastown adds coordination around them: startup priming, work assignment, mail, nudges, hooks, runtime wrappers, and session management.

## Storage & Sessions

Persistent work state is a core strength. The system uses beads/Dolt-backed state, git worktrees, and tmux session tracking to make agent coordination survive process restarts. This is exactly the kind of durable coordination layer `brain` might want above its core traits later.

## Server/Client Architecture

There is no central agent server. Each `gt` CLI invocation is separate, while external agents run in tmux and persistent state lives in Dolt and the workspace. `gt dashboard` serves HTTP, and `gt feed` provides a TUI-like operational surface, but the control model is file/process based rather than RPC-based.

## Security & Permissions

The current security model is pragmatic rather than hardened. Much of the behavior relies on vendor bypass or yolo modes plus additional hooks and allowlists. Stronger sandboxed execution is discussed, but it is not the dominant present-day model.

## CLI & TUI

Gastown is CLI-first with dashboard/feed operator surfaces rather than a coding-agent TUI. `gt dashboard` and `gt feed` are the relevant UX references here. For the broader command matrix and how it differs from coding-agent TUIs, see `openspec/changes/add-tui-transport/competitor-analysis.md`.

## Mapping to brain Traits

- `Provider`: implicit through runtime presets and agent wrappers.
- `Tool`: missing as a GT-owned core abstraction.
- `Store`: first-class and central.
- `AgentLoop`: first-class as orchestration workflows.
- `Transport`: first-class through tmux, mail, nudges, dashboard, and runtime lifecycle control.

## Key Takeaways

- `Steal:` persistent work tracking, handoff/nudge concepts, and control-plane visibility.
- `Differentiate:` keep `brain` strong as a local engine core instead of collapsing into pure CLI orchestration or wrapper scripts.

## Key Evidence

- `repocache/steveyegge/gastown/cmd/gt/main.go`
- `repocache/steveyegge/gastown/internal/cmd/root.go`
- `repocache/steveyegge/gastown/internal/tmux/tmux.go`
- `repocache/steveyegge/gastown/internal/web/commands.go`
- `repocache/steveyegge/gastown/internal/config/agents.go`
- `repocache/steveyegge/gastown/internal/runtime/runtime.go`
- `repocache/steveyegge/gastown/internal/hooks/templates/opencode/gastown.js`
- `repocache/steveyegge/gastown/internal/wrappers/scripts/gt-codex`
- `repocache/steveyegge/gastown/docs/design/architecture.md`
- `repocache/steveyegge/gastown/docs/agent-provider-integration.md`
- `repocache/steveyegge/gastown/docs/design/agent-api-inventory.md`
- `repocache/steveyegge/gastown/docs/design/sandboxed-polecat-execution.md`
