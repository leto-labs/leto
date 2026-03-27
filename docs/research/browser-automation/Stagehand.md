# Stagehand

## Summary

`Stagehand` is the strongest local SDK option for AI-native browser automation
if we decide to build reusable browser-driving scripts inside this repo.

It is not the best immediate "Codex should use this tomorrow" tool, because it
is primarily a library rather than a ready-made command surface. But it is the
most interesting option if we want to codify exploratory browser workflows into
repeatable local harnesses.

## Confirmed Capabilities

From the docs:

- core AI-native methods:
  - `act()`
  - `observe()`
  - `extract()`
  - `agent()`
- local browser mode via `env: "LOCAL"`
- local browser launch customization through `localBrowserLaunchOptions`
- headful local debugging with `headless: false`
- keep-alive local browser sessions for debugging handoff
- direct Playwright integration
- deterministic agent caching via `cacheDir`

The deterministic caching story is especially important. The docs describe a
flow where the first run explores with LLM inference and later runs reuse cached
actions, becoming much faster and more predictable.

## Why It Matters For This Repo

`Stagehand` fits the next step after manual exploration:

- use AI-native actions to discover how to drive a browser flow
- keep that discovery in local scripts
- reuse Playwright pages where helpful
- cache the discovered workflow into a more deterministic local routine

That makes it a strong candidate if we want repo-owned scripts for:

- OpenCode attach/setup flows
- session bootstrap workflows
- auth/state seeding
- multi-step UI validation that starts as exploration and later becomes repeatable

## Tradeoffs

- The docs are heavily Browserbase-oriented even though local mode exists and is
  documented cleanly.
- It is a library, not a ready-made coding-agent CLI. We would need to write
  scripts or a small harness around it.
- It is better for "build a local automation capability" than "let Codex poke
  at the browser immediately with zero wrapper code."

## Working Recommendation

Choose `Stagehand` if the goal becomes:

- build repo-owned AI browser scripts
- keep them local
- integrate them with Playwright
- turn exploratory workflows into deterministic cached routines

If the goal is only immediate Codex-driven browser control, `@playwright/cli`
or `browser-use` CLI is the simpler first move. If the goal is to build our own
longer-term browser automation layer in-repo, `Stagehand` is the best current
foundation.

## Key Sources

- Introduction:
  <https://docs.stagehand.dev/v3/first-steps/introduction>
- Quickstart:
  <https://docs.stagehand.dev/v3/first-steps/quickstart>
- Browser configuration:
  <https://docs.stagehand.dev/v3/configuration/browser>
- Playwright integration:
  <https://docs.stagehand.dev/v3/integrations/playwright>
- Deterministic agent scripts:
  <https://docs.stagehand.dev/v3/best-practices/deterministic-agent>
- Observability:
  <https://docs.stagehand.dev/v3/configuration/observability>
- GitHub repo:
  <https://github.com/browserbase/stagehand>
