# Browser Automation Research

Source-first internal notes for local-first browser automation tools that could
improve development and debugging loops in this repo.

This folder is intended to answer one concrete workflow question for `brain`:

> Which browser-automation surfaces should we use during local UI development
> and debugging, especially when validating OpenCode against `agent-server`,
> without depending on cloud-only infrastructure?

Short answer, as of 2026-03-27: there is no single winner for every use case.
The practical split is:

- non-MCP command surfaces for fast coding-agent workflows
- MCP browser/debug surfaces for persistent interactive inspection
- local SDKs when we want to build our own reusable agent harnesses

## Why This Matters For `brain`

The repo now has a pinned OpenCode UI validation workspace under
`submodules/opencode`, but the workflow is still mostly manual:

- start `agent-server`
- start the OpenCode web app
- attach the UI to the compat path
- click around until a mismatch appears

That is enough for smoke checks, but not enough for a strong day-to-day loop.
We want tools that help an agent:

- open and drive the browser locally
- click, type, navigate, and switch tabs
- capture screenshots and page state
- inspect console, network, and storage state when debugging
- preserve sessions when needed
- avoid pushing the entire interaction through brittle test generation first

## Focus Areas

This research concentrates on the dimensions that matter most for this repo:

- local-first operation
- no cloud requirement for the core loop
- direct browser control during development/debugging
- compatibility with coding-agent workflows
- persistence/session reuse when helpful
- debugging surfaces such as screenshots, console, network, and traces
- realistic setup burden for Codex on a development machine

## Index

- [`PlaywrightCLI.md`](PlaywrightCLI.md)
- [`BrowserUse.md`](BrowserUse.md)
- [`AgentBrowser.md`](AgentBrowser.md)
- [`DevBrowser.md`](DevBrowser.md)
- [`Stagehand.md`](Stagehand.md)
- [`ChromeDevToolsMCP.md`](ChromeDevToolsMCP.md)
- [`PlaywrightMCP.md`](PlaywrightMCP.md)
- [`BrowserMCP.md`](BrowserMCP.md)

## Executive Summary

- `@playwright/cli` is the strongest non-MCP candidate for coding-agent-driven
  browser work. Its own README explicitly says CLI + skills is the best fit for
  coding agents, and it exposes a broad command surface for open/click/type,
  tabs, storage, network, tracing, screenshots, and a visual dashboard.
- Vercel's `agent-browser` is the strongest newly found local CLI contender
  beyond Playwright CLI and Browser Use. It has an unusually deep direct
  command surface for AI agents, including sessions, profiles, state save/load,
  storage inspection, HAR recording, traces, diffing, and console/network
  debugging, all from a native Rust CLI. It also ships an official skill via
  `npx skills add vercel-labs/agent-browser`, plus user/project config files.
  In the pinned `v0.22.3` source, the observability surface is runtime
  streaming, not the newer `dashboard` command.
- `dev-browser` is the most interesting skill-shaped local option. It gives the
  agent a sandboxed JavaScript runtime with persistent pages and the full
  Playwright `Page` API, and it explicitly documents Codex/skill installation
  paths rather than only generic automation usage.
- `browser-use` is the strongest non-MCP alternative if we want a direct,
  persistent browser-control CLI outside the Playwright ecosystem. It supports
  local direct control, screenshots, real Chrome profiles, CDP connections, and
  a background daemon for fast command-to-command latency.
- `Stagehand` is the strongest local SDK option when we want to embed AI-native
  browser control into repo-owned scripts. Its local mode, Playwright
  integration, and workflow caching make it especially attractive for
  converting exploratory automation into deterministic local flows.
- `chrome-devtools-mcp` is the strongest MCP candidate for debugging. It adds
  direct control over a live Chrome browser plus screenshots, console access,
  network inspection, performance traces, and memory snapshots.
- `microsoft/playwright-mcp` is the strongest structured MCP browser control
  option. It is more browser-automation-oriented than DevTools-oriented and is
  well suited to persistent exploratory loops, but its own README still argues
  that CLI + skills is usually the better default for coding agents.
- `BrowserMCP` is interesting when we specifically want to control a real,
  logged-in browser tab/profile through an extension rather than launch a fresh
  automation browser. It looks promising but less mature than the Microsoft and
  ChromeDevTools MCP surfaces.

## Tool Matrix

| Tool | Primary surface | Local-first | Best use in this repo | Main tradeoff |
| --- | --- | --- | --- | --- |
| `@playwright/cli` | CLI + skills | Yes | best first non-MCP tool for Codex-driven browser work | less autonomous than Stagehand-style agents |
| `agent-browser` | native Rust CLI | Yes | best deep direct-control CLI beyond Playwright, especially for stateful debugging | newer project and less ecosystem history than Playwright |
| `dev-browser` | CLI + sandboxed JS skill | Yes | best if we want the agent to drive a browser through reusable sandboxed scripts | less standardized than Playwright CLI and more script-oriented |
| `browser-use` CLI / SDK | Python CLI + agent SDK | Yes, but docs mix in cloud heavily | strong direct-control alternative with persistent sessions and profile reuse | Python-centric, cloud platform framing leaks into docs |
| `Stagehand` | TypeScript SDK | Yes | best when we want repo-owned AI browser scripts with caching and Playwright reuse | more of a library than a ready-to-drive Codex tool |
| `chrome-devtools-mcp` | MCP + DevTools | Yes | best debugging surface for network, console, traces, and screenshots | Chrome-specific and MCP-centric |
| `playwright-mcp` | MCP + Playwright | Yes | best structured MCP browser control surface | upstream says coding agents often do better with CLI + skills |
| `BrowserMCP` | MCP + browser extension | Yes | best if we need the agent to drive a real logged-in tab/profile | extension setup and thinner documentation/tooling evidence |

## Recommended Adoption Order

### 1. First non-MCP tool to try: `@playwright/cli`

This is the strongest direct fit for Codex itself:

- upstream explicitly positions CLI + skills as better than MCP for coding
  agents
- the command surface is broad enough for click/type/screenshot/state/storage
- it supports persistent sessions and a visual dashboard

### 2. Second non-MCP tool to evaluate: `agent-browser`

This is the strongest newly found "pure CLI" alternative:

- native Rust binary rather than a Node or Python-first wrapper
- explicitly built for AI agents
- deep command coverage for clicks, tabs, storage, console, network, HAR,
  traces, diffs, and state persistence
- supports persistent profiles, named sessions, and auth state save/load
- ships an official skill rather than requiring us to invent one

### 3. Third non-MCP tool to evaluate: `browser-use` CLI

This is the best alternative if we prefer:

- a persistent daemon model
- real Chrome profile reuse
- Python-based automation and direct browser control

### 4. Skill-oriented local tool to keep in reserve: `dev-browser`

This is worth evaluating if we specifically want a CLI/skill surface that lets
the agent run sandboxed Playwright-flavored scripts rather than issuing many
small browser commands.

It is attractive for:

- persistent named pages across script invocations
- direct attachment to an already running Chrome instance
- Codex/skill-oriented installation guidance
- low host-risk due to the QuickJS sandbox model

### 5. First local SDK to consider for repo-owned harnesses: `Stagehand`

If we want to build browser-driving scripts inside this repo, `Stagehand` is
the most interesting library:

- AI-native `act()`, `observe()`, and `extract()`
- local browser mode
- Playwright integration
- deterministic caching for repeat runs

### 6. MCP tools only after we know we want persistent interactive inspection

Start with:

- `chrome-devtools-mcp` for debugging
- `playwright-mcp` for structured browser control

Keep `BrowserMCP` as a niche option for real-profile automation.

## Working Conclusion

For this repo, the local browser-automation stack should not be one tool.

The clean separation is:

- `@playwright/cli`, `agent-browser`, or `browser-use` CLI for agent-driven
  browser work during development
- `dev-browser` when we specifically want sandboxed script-style control as a
  skill surface
- `Stagehand` for repo-owned reusable AI browser scripts
- MCP browser tools only when we need persistent interactive inspection or a
  richer debugging surface than plain CLI automation provides

That gives us a path from:

1. ad hoc local clicking
2. to agent-driven browser control
3. to reusable scripted flows
4. to deterministic regression coverage

without forcing every UI investigation through Playwright test generation first.

## Lower-Confidence Options

The broader search did turn up a few additional projects, but they currently
look less compelling for this repo than the shortlist above:

- `ai-agent-browser` has a CLI and MCP story, but the public materials read
  more like a broad capability catalog than a proven local dev/debugging
  surface we should standardize on first.
- Browser-extension-centric options are interesting when we specifically want
  to drive a real logged-in browser profile, but they add more setup and are
  not the cleanest default for repeatable local repo workflows.
- Cloud-framed agent browsers may still have useful ideas, but they are not a
  fit for the current "local-first, no cloud dependency" requirement.

## Primary Sources

- Playwright CLI README:
  <https://raw.githubusercontent.com/microsoft/playwright-cli/main/README.md>
- Playwright MCP README:
  <https://raw.githubusercontent.com/microsoft/playwright-mcp/main/README.md>
- Playwright Test Agents:
  <https://playwright.dev/docs/test-agents>
- Browser Use quickstart:
  <https://docs.browser-use.com/open-source/quickstart>
- Browser Use coding-agent quickstart:
  <https://docs.browser-use.com/open-source/coding-agent-quickstart>
- Browser Use CLI:
  <https://docs.browser-use.com/open-source/browser-use-cli>
- Vercel agent-browser README:
  <https://raw.githubusercontent.com/vercel-labs/agent-browser/main/README.md>
- Dev Browser README:
  <https://raw.githubusercontent.com/SawyerHood/dev-browser/main/README.md>
- Browser Use MCP:
  <https://docs.browser-use.com/open-source/customize/integrations/mcp-server>
- Stagehand introduction:
  <https://docs.stagehand.dev/v3/first-steps/introduction>
- Stagehand quickstart:
  <https://docs.stagehand.dev/v3/first-steps/quickstart>
- Stagehand browser configuration:
  <https://docs.stagehand.dev/v3/configuration/browser>
- Stagehand Playwright integration:
  <https://docs.stagehand.dev/v3/integrations/playwright>
- Stagehand deterministic agent scripts:
  <https://docs.stagehand.dev/v3/best-practices/deterministic-agent>
- Stagehand observability:
  <https://docs.stagehand.dev/v3/configuration/observability>
- Chrome DevTools MCP repo:
  <https://github.com/ChromeDevTools/chrome-devtools-mcp>
- Chrome DevTools MCP README:
  <https://raw.githubusercontent.com/ChromeDevTools/chrome-devtools-mcp/main/README.md>
- BrowserMCP docs:
  <https://docs.browsermcp.io/>
- BrowserMCP repo:
  <https://github.com/browsermcp/mcp>
