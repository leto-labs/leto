# Playwright CLI

## Summary

`@playwright/cli` currently looks like the strongest non-MCP browser-control
surface for coding agents.

The important signal is upstream's own positioning: the README explicitly says
CLI + skills is the best fit for coding agents, while MCP is better reserved
for more persistent exploratory loops.

For this repo, that matters because the main goal is not "generate more tests."
It is "let the agent open the UI, click around, inspect state, and debug local
integration issues quickly."

## Confirmed Capabilities

From the README:

- installable as `npm install -g @playwright/cli@latest`
- optional skills install via `playwright-cli install --skills`
- headless by default, headed mode available with `--headed`
- supports persistent and named sessions
- supports a visual dashboard via `playwright-cli show`
- supports:
  - `open`, `goto`, `click`, `type`, `fill`, `hover`, `drag`
  - `screenshot`, `pdf`, `snapshot`
  - tab management
  - cookie, localStorage, and sessionStorage inspection/mutation
  - `route`, `console`, `network`
  - tracing and video recording

That is already enough to cover most local OpenCode validation/debug loops.

## Why It Matters For This Repo

The best immediate use would be:

- start local `agent-server`
- start the OpenCode web app
- point Playwright CLI at `http://127.0.0.1:3000`
- let Codex drive the UI through shell commands instead of manually juggling a
  browser

The session support is especially useful. The README says the browser profile
is preserved between CLI calls within a session, and `--persistent` can save it
to disk across restarts. That maps well to repeated local OpenCode debugging
where we want to keep auth/localStorage/default-server state around.

The dashboard is also relevant. `playwright-cli show` opens a live visual
monitor for active sessions, which is exactly the kind of "watch what the agent
is doing" surface that improves debugging confidence.

## Tradeoffs

- It is still Playwright-shaped rather than high-level-agent-shaped. You get a
  powerful direct command surface, not a single magical "solve the whole
  workflow" abstraction.
- It is oriented around CLI + skills rather than an MCP server, so persistent
  state exists at the CLI session layer, not through an MCP client contract.
- The README is strong on commands and operational ergonomics, but less focused
  on AI-native exploration patterns than Stagehand.

## Working Recommendation

If we want the first non-MCP browser tool that Codex should actually use during
development, this is the best current candidate.

It has the best combination of:

- upstream support for coding-agent use
- broad browser control surface
- observability through snapshots, screenshots, console, network, tracing, and
  dashboard views
- local operation without cloud dependencies

## Key Sources

- README:
  <https://raw.githubusercontent.com/microsoft/playwright-cli/main/README.md>
- Playwright Test Agents:
  <https://playwright.dev/docs/test-agents>
