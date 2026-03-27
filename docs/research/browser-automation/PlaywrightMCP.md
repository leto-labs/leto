# Playwright MCP

## Summary

`microsoft/playwright-mcp` is the strongest structured MCP browser-control
surface in the current research set.

Its role is different from Chrome DevTools MCP. It is more focused on browser
automation through Playwright and structured page state than on deep DevTools
inspection.

## Confirmed Capabilities

From the README:

- browser automation through Playwright
- structured accessibility snapshots instead of a screenshot-first model
- support for persistent profiles and isolated sessions
- support for storage-state loading and saving
- support for connecting to an existing browser via extension or CDP-like modes
- broad client installation guidance including explicit Codex setup

The README also contains a very important recommendation:

- CLI + skills is generally the better fit for coding agents
- MCP is still useful for persistent exploratory loops, self-healing tests, and
  long-running autonomous workflows

## Why It Matters For This Repo

This tool matters mainly as the structured-MCP counterpart to Playwright CLI.

It is attractive when we want:

- persistent browser context inside an MCP client
- structured page-state interaction
- a stronger long-running exploratory loop than one-shot CLI commands

It is less compelling when the goal is simply "give Codex a good browser tool
tomorrow," because upstream itself argues that the CLI path is usually the
better default for coding agents.

## Tradeoffs

- For coding-agent workflows, upstream explicitly prefers CLI + skills.
- It appears more automation- and structure-oriented than deep-debug oriented.
- As with other MCP servers, there is some token/context overhead relative to a
  narrow CLI command surface.

## Working Recommendation

Use `playwright-mcp` if we decide we specifically want:

- a persistent MCP browser surface
- structured exploratory automation
- self-healing or long-running page loops

Otherwise, prefer `@playwright/cli` first and treat this as the more stateful
MCP counterpart rather than the default adoption path.

## Key Sources

- GitHub repo:
  <https://github.com/microsoft/playwright-mcp>
- README:
  <https://raw.githubusercontent.com/microsoft/playwright-mcp/main/README.md>
