# VS Code

## Summary

There is now a credible generic ACP client for VS Code:

- GitHub repo: `formulahendry/vscode-acp`
- Marketplace extension: `formulahendry.acp-client`

This matters because it answers one of the biggest open ecosystem questions:

> Is there a VS Code ACP path that looks like a real chat interface instead of a
> theoretical protocol adapter?

As of 2026-03-18, the answer is yes.

## What The Extension Actually Provides

The repository and README show that the extension is more than a process
launcher. It includes:

- a chat webview
- a session tree panel
- mode and model pickers
- permission management
- filesystem handlers
- terminal handlers
- ACP traffic logging
- registry browsing
- restart and reconnect flows

That is enough to count as a real ACP client shell for coding-agent use.

## Architecture Notes

The extension code is useful because it makes the client-side ACP expectations
very concrete.

### Connection model

`ConnectionManager`:

- spawns a child process
- bridges stdio into an ACP `ClientSideConnection`
- initializes with protocol version and client capabilities
- advertises filesystem read/write and terminal support

This is exactly the environment a `brain` ACP agent would need to work well in.

### Filesystem behavior

The `FileSystemHandler`:

- reads unsaved document state from open editors when possible
- falls back to workspace filesystem reads otherwise
- writes files through the VS Code workspace API
- opens written files in the editor

This is one of the strongest arguments for ACP over a separate server API in
editor scenarios. A generic ACP client can provide editor-native file semantics
that a remote HTTP server cannot get for free.

### Terminal behavior

The `TerminalHandler`:

- spawns real child processes
- captures output
- exposes `terminal/output`, `wait_for_exit`, `kill`, and `release`
- mirrors output into a visible VS Code pseudo-terminal

Again, this is not just chat. ACP is being used as a shell for integrated
command execution and visible tool output.

### Permission behavior

The `PermissionHandler`:

- maps ACP permission requests into VS Code quick-pick UI
- supports ask vs allow-all policy

That lines up cleanly with the kind of approval boundary `brain` will need
anyway.

## Preconfigured Agent Story

The extension ships defaults for a growing set of agents, including:

- GitHub Copilot
- Claude Code
- Gemini CLI
- Qwen Code
- Codex
- OpenCode
- OpenClaw

That matters because it suggests ACP is already becoming a cross-vendor launch
surface inside VS Code.

## Limits

- This is not first-party VS Code ACP support from Microsoft.
- The ecosystem maturity is therefore still different from Zed and JetBrains.
- The extension is still only as good as the ACP support quality of each agent.

But it is still enough to invalidate the claim that ACP is "editor support for
everyone except VS Code."

## What This Means For `brain`

- ACP can plausibly get `brain` into VS Code without a custom extension as a
  first step.
- If `brain` exposes ACP client capabilities cleanly, this extension can supply
  a serious chat/edit/terminal shell immediately.
- A dedicated `brain` VS Code extension may still be worthwhile later, but ACP
  is already a credible bridge to that future.

## Key Sources

- VS Code ACP repo: [`README.md`](../../repocache/formulahendry/vscode-acp/README.md)
- Connection manager: [`src/core/ConnectionManager.ts`](../../repocache/formulahendry/vscode-acp/src/core/ConnectionManager.ts)
- Filesystem handler: [`src/handlers/FileSystemHandler.ts`](../../repocache/formulahendry/vscode-acp/src/handlers/FileSystemHandler.ts)
- Terminal handler: [`src/handlers/TerminalHandler.ts`](../../repocache/formulahendry/vscode-acp/src/handlers/TerminalHandler.ts)
- Permission handler: [`src/handlers/PermissionHandler.ts`](../../repocache/formulahendry/vscode-acp/src/handlers/PermissionHandler.ts)
- Marketplace listing: <https://marketplace.visualstudio.com/items?itemName=formulahendry.acp-client>
