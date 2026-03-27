# Browser Use

## Summary

`browser-use` is the strongest local-first non-Playwright alternative in this
research set.

The project spans several surfaces:

- a Python agent SDK
- a direct browser-control CLI
- an MCP server

For this repo, the most relevant parts are the local CLI and the local SDK, not
the hosted cloud product.

## Confirmed Capabilities

From the open-source docs:

- the Python quickstart installs `browser-use` locally and runs an agent with
  `Agent(...)`
- the CLI exposes direct browser control:
  - `open`
  - `state`
  - `click`
  - `type`
  - `input`
  - `screenshot`
  - tabs, cookies, waits, JS execution, and information retrieval
- the CLI supports different browser modes:
  - default headless Chromium
  - headed mode
  - real Chrome profiles via `--profile`
  - connection to an existing browser via `--cdp-url`
- the CLI docs describe a persistent background daemon so the browser remains
  alive between commands
- the coding-agent quickstart explicitly positions the project for Cursor,
  Claude, and other coding agents

## Why It Matters For This Repo

The CLI is the key local dev/debugging surface.

The most attractive properties are:

- direct control from the shell
- screenshot capture
- a stable `state` command that returns indexed elements
- real Chrome profile reuse
- CDP attach support
- a persistent daemon model that avoids cold-starting the browser every time

That means it could be used in a loop like:

1. open the OpenCode UI
2. inspect current state
3. click/input elements by index
4. capture screenshots
5. reconnect using the same profile/session

This is a good fit for repeated local debugging where we do not necessarily want
to generate tests first.

## Tradeoffs

- The docs mix local open-source usage with the hosted cloud platform, so the
  project's public framing is noisier than Playwright CLI or Stagehand local
  mode.
- The SDK and CLI are Python-centric rather than TypeScript/Playwright-centric,
  which may or may not match the surrounding tooling preference in this repo.
- The quickstart strongly recommends the hosted `ChatBrowserUse` model path,
  which is not the direction we want for local-first repo workflows.

## Working Recommendation

`browser-use` is worth treating as the best non-Playwright local control
alternative.

It is most compelling if we want:

- a direct browser-control CLI
- real-profile reuse
- a persistent daemon-backed session model
- a Python SDK for higher-level agent experimentation

If we choose a non-MCP tool after Playwright CLI, this is the next one to try.

## Key Sources

- Open-source quickstart:
  <https://docs.browser-use.com/open-source/quickstart>
- Coding-agent quickstart:
  <https://docs.browser-use.com/open-source/coding-agent-quickstart>
- Browser Use CLI:
  <https://docs.browser-use.com/open-source/browser-use-cli>
- Available tools:
  <https://docs.browser-use.com/open-source/customize/tools/available>
- MCP server:
  <https://docs.browser-use.com/open-source/customize/integrations/mcp-server>
- GitHub repo:
  <https://github.com/browser-use/browser-use>
