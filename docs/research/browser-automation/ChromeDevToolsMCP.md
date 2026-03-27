# Chrome DevTools MCP

## Summary

`chrome-devtools-mcp` is the strongest debugging-oriented MCP browser surface in
this research set.

If the question is "what should an agent use when the browser is already
misbehaving and we need screenshots, console output, network details, or a
performance trace," this is the best current answer.

## Confirmed Capabilities

From the README:

- control and inspect a live Chrome browser
- direct input automation:
  - `click`
  - `fill`
  - `fill_form`
  - `hover`
  - `press_key`
  - `type_text`
  - `upload_file`
- navigation and page management:
  - `new_page`
  - `select_page`
  - `list_pages`
  - `navigate_page`
  - `wait_for`
- debugging and inspection:
  - screenshots
  - console inspection
  - network request inspection
  - performance traces
  - memory snapshots
- explicit Codex installation instructions via `codex mcp add`

## Why It Matters For This Repo

This tool is especially attractive for debugging OpenCode against
`agent-server` because our current pain points are not only UI actions. They
also include:

- wrong backend URL selection
- 404s versus compat-path routing
- CORS issues
- SSE behavior
- auth and storage state

Those are exactly the kinds of failures where console, network, and trace-level
inspection matter more than pure click automation.

## Tradeoffs

- It is Chrome-specific, not a cross-browser abstraction.
- It is MCP-first rather than CLI-first.
- The README states that usage statistics are enabled by default unless opted
  out with `--no-usage-statistics`.

## Working Recommendation

This should be the first MCP browser tool to try if we decide we want a richer
interactive debugging surface than plain CLI automation provides.

For local development/debugging, it is stronger than more automation-centric MCP
tools because it brings DevTools-grade inspection into the agent loop.

## Key Sources

- GitHub repo:
  <https://github.com/ChromeDevTools/chrome-devtools-mcp>
- README:
  <https://raw.githubusercontent.com/ChromeDevTools/chrome-devtools-mcp/main/README.md>
