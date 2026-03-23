# JetBrains

## Summary

JetBrains is now a serious ACP client, not a speculative future one. The official
AI Assistant docs and registry announcement confirm that JetBrains IDEs support:

- ACP agent installation from a built-in registry
- manual custom-agent configuration via `~/.jetbrains/acp.json`
- MCP exposure to ACP agents
- logging and troubleshooting flows
- use without a JetBrains AI subscription

For `brain`, this makes ACP far more attractive than a one-editor bet.

## What JetBrains Exposes

The ACP docs show two main paths.

### Registry install

The user can:

- open the AI Assistant agent picker
- choose "Install From ACP Registry"
- install an ACP agent
- optionally configure MCP exposure
- use it immediately in AI Chat

JetBrains says the IDE handles download, preparation, update, and uninstall
workflow. That is the strongest available evidence that ACP is becoming a
productized agent distribution path rather than only a developer protocol.

### Manual custom agent

JetBrains also documents `~/.jetbrains/acp.json` with an `agent_servers` object
containing:

- `command`
- `args`
- `env`

The IDE launches the agent as a subprocess. This is the same core shape as Zed,
which is good news for `brain`: a single stable ACP CLI entry point can cover
both ecosystems.

## MCP Exposure

JetBrains adds an interesting extra:

- user-configured MCP servers can be passed through to ACP agents
- the integrated IntelliJ MCP server can also be exposed
- tool exposure can be narrowed for the IntelliJ MCP server

This matters because it suggests JetBrains ACP is not just a text chat surface.
It is explicitly designed to let an external agent reach richer IDE-owned
context and tools.

## Operational Notes

JetBrains also documents:

- detectable local ACP agents that can be added automatically
- ACP logs available from the UI
- an extended logging mode that captures request and response traffic
- no support for ACP-compatible agents in WSL at the moment

These are small but important product signals. They indicate real support and
real operational expectations around ACP, not a thin proof of concept.

## What The Registry Means Here

The January 2026 JetBrains post frames the registry as:

- direct distribution to JetBrains and Zed from one listing
- a way to avoid lock-in
- a way for users to choose different agents for different tasks
- a route for agent builders to reach large IDE audiences quickly

For `brain`, that means JetBrains should be treated as a primary ACP target from
the beginning, not as an afterthought after Zed.

## What This Means For `brain`

- A `brain` ACP surface should be tested against JetBrains assumptions, not only
  Zed assumptions.
- `brain` should keep its launch config simple enough to fit naturally into
  `acp.json`.
- If `brain` wants registry adoption later, JetBrains is already part of the
  payoff, not just a speculative future consumer.

## Key Sources

- JetBrains ACP docs: <https://www.jetbrains.com/help/ai-assistant/acp.html>
- JetBrains registry announcement: <https://blog.jetbrains.com/ai/2026/01/acp-agent-registry/>
