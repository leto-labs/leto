# Dev Browser

## Summary

`dev-browser` is the most interesting skill-shaped local browser tool found in
this research pass.

Unlike the more command-by-command CLIs, it gives the agent a sandboxed
JavaScript runtime with access to persistent browser pages and the full
Playwright `Page` API.

That makes it feel closer to "give the agent a safe browser scripting surface"
than "teach the agent a long list of shell subcommands."

## Confirmed Capabilities

From the README:

- installable as `npm install -g dev-browser`
- browser/runtime setup via `dev-browser install`
- supports:
  - launching a fresh Chromium instance
  - connecting to an existing Chrome instance via remote debugging
  - persistent named pages across multiple scripts
  - full Playwright page operations such as `goto`, `click`, `fill`,
    `locator`, `evaluate`, and `screenshot`
- scripts run in a QuickJS WASM sandbox rather than full host Node.js
- includes AI-oriented page snapshots via `page.snapshotForAI(...)`
- explicitly documents Codex/skill installation and allowlist guidance

## Why It Matters For This Repo

This tool is attractive if we want an agent workflow like:

1. keep a browser open across multiple debugging turns
2. run short sandboxed scripts against that browser
3. capture screenshots and AI-friendly snapshots
4. avoid giving the agent unrestricted host-side script execution

That is a good fit for iterative OpenCode debugging where we want a persistent
page session but also want the execution model to stay relatively constrained.

## Tradeoffs

- It is more script-oriented than pure CLI-oriented, so it is not as immediately
  shell-command-friendly as Playwright CLI or `agent-browser`.
- It is built around Playwright concepts, but without Playwright's ecosystem
  depth or standardization.
- The public documentation is concise, which is good for getting started but
  leaves less operational evidence than the Playwright or Browser Use docs.

## Working Recommendation

Keep `dev-browser` on the shortlist, but treat it as a specialized option.

It is most compelling if we specifically want:

- a Codex/skill-friendly browser surface
- sandboxed script execution
- persistent pages instead of one-shot browser sessions

I would not choose it ahead of `@playwright/cli` for the first trial, but I
would rank it ahead of many generic automation frameworks because it is
actually shaped for agent use.

## Key Sources

- README:
  <https://raw.githubusercontent.com/SawyerHood/dev-browser/main/README.md>
