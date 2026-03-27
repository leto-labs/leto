# BrowserMCP

## Summary

`BrowserMCP` is the most interesting "use my real browser" option in this
research set.

It is explicitly local-first and profile-reusing, and it is designed around an
extension that connects the current browser tab to an MCP server.

## Confirmed Capabilities

From the docs and README:

- local MCP server setup via `npx @browsermcp/mcp@latest`
- browser extension setup
- automation happens locally on the machine
- it uses the existing browser profile
- it is explicitly positioned as private/local and friendly to logged-in flows
- browser actions are performed on the connected tab

The README also states that the project was adapted from `playwright-mcp` to
automate the user's existing browser rather than creating new browser instances.

## Why It Matters For This Repo

This is appealing if the main concern is not generic automation, but:

- use the browser you already have open
- stay logged in
- avoid separate automation profiles
- reduce basic bot-detection issues that appear when launching synthetic browser
  sessions

That makes it relevant for local debugging where we may want the agent to work
against a real browser session rather than a clean automation browser.

## Tradeoffs

- The setup requires both the MCP server and the extension.
- The public documentation is thinner than the Chrome DevTools MCP and
  Microsoft Playwright MCP documentation.
- The repo README notes that the repository currently cannot be built
  standalone due to monorepo dependencies.

## Working Recommendation

Treat `BrowserMCP` as a niche but promising option.

It is worth exploring if we specifically need agent control over the real,
logged-in browser that a developer is already using. It is not the best first
choice for this repo's broader browser-automation layer because the tooling and
documentation surface looks less mature than the stronger alternatives.

## Key Sources

- Docs home:
  <https://docs.browsermcp.io/>
- Setup server:
  <https://docs.browsermcp.io/setup-server>
- Setup extension:
  <https://docs.browsermcp.io/setup-extension>
- Start automating:
  <https://docs.browsermcp.io/start-automating>
- GitHub repo:
  <https://github.com/browsermcp/mcp>
- README:
  <https://raw.githubusercontent.com/browsermcp/mcp/main/README.md>
