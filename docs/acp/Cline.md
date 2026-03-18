# Cline

## Summary

Cline uses ACP as a portability layer for its existing product runtime, not as a
replacement for its native extension. That makes it a strong example of how a
serious agent can use ACP pragmatically:

- keep the product runtime and features
- expose them through ACP where available
- let editors reuse the same core agent through a standard shell

For `brain`, this is a very relevant model.

## Confirmed ACP Positioning

Cline's ACP docs say:

- `cline --acp` lets any ACP-compatible editor run the full Cline agent
- the ACP path includes Skills, Hooks, and MCP integrations
- JetBrains, Neovim, and Zed are all documented targets
- any ACP editor can launch the CLI in ACP mode

That is a strong claim: ACP is not presented as a degraded compatibility mode.
It is presented as a way to carry the existing runtime into more clients.

## Important Architectural Nuance

The Cline SDK docs are also revealing:

- the SDK "conforms to" ACP
- but the embedded SDK uses an event-emitter pattern instead of a stdio ACP
  transport
- for actual stdio ACP communication, the docs say to use the CLI directly

This reinforces a useful pattern for `brain`:

- internal runtime APIs do not need to be ACP-shaped
- ACP is an external client surface
- the runtime should still align conceptually with ACP so the adapter is not
  awkward or lossy

## What Cline Validates

Cline is good evidence for several claims:

- an editor-standard ACP shell can coexist with a deeper native integration
- one agent can support multiple editors without losing its own runtime identity
- ACP is compatible with nontrivial features, not only plain text turns

## Limitations Of The Signal

Cline's docs are clear about editor integrations, but less explicit in this repo
snapshot about exactly which ACP capability surfaces are native versus adapted.
So the strongest safe conclusion is:

- Cline treats ACP as strategically important for portability
- Cline does not imply that ACP alone replaces every client-specific product
  detail

## What This Means For `brain`

- `brain` should not wait to have its own perfect UI stack before exposing ACP.
- ACP can be a distribution and interoperability multiplier for the runtime
  `brain` already wants to build.
- `brain`'s internal types should stay runtime-first, with an ACP surface built
  around them, rather than forcing the runtime to become an ACP SDK clone.

## Key Sources

- ACP integrations guide: [`docs/cline-cli/acp-editor-integrations.mdx`](../../repocache/cline/cline/docs/cline-cli/acp-editor-integrations.mdx)
- Install docs: [`docs/getting-started/installing-cline.mdx`](../../repocache/cline/cline/docs/getting-started/installing-cline.mdx)
- SDK overview: [`docs/cline-sdk/overview.md`](../../repocache/cline/cline/docs/cline-sdk/overview.md)
