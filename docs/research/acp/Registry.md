# ACP Registry

## Summary

The registry is what makes ACP strategically important rather than merely
interesting. The protocol makes interoperability possible; the registry makes
distribution practical.

As of 2026-03-18, both Zed and JetBrains publicly position the registry as the
preferred way to discover and install ACP-compatible agents. That changes the
go-to-market calculation for `brain`: implementing ACP can now create a path to
cross-editor discoverability without maintaining separate client packaging for
each editor forever.

## What The Registry Changes

Without a registry, ACP still requires users to:

- know that an agent exists
- install it manually
- wire up commands and arguments by hand
- update it manually

With the registry:

- clients can surface a catalog of agents directly in the UI
- install and update flows become client-managed
- an ACP agent can target multiple clients through one listing

Zed's registry post says this directly: "implement once, work everywhere" was
the promise of ACP, and the registry is the missing distribution layer.

## Confirmed Client Support

### Zed

Zed's external agents docs say:

- starting with `v0.221.x`, the ACP Registry is the preferred installation path
- agent server extensions are expected to be deprecated later
- registry-installed agents update automatically
- if both an extension and registry install exist, the registry version wins

That is a strong signal that ACP in Zed is no longer an "advanced custom config"
feature. It is part of the product surface.

### JetBrains

JetBrains AI Assistant docs say:

- ACP agents can be installed from a curated registry directly in the IDE
- registry-installed agents require no manual setup
- the IDE can handle updates and uninstall flow
- users do not need a JetBrains AI subscription to use ACP agents

JetBrains' registry announcement also positions the registry as direct
distribution to JetBrains IDEs and Zed from one open listing.

## Current Registry Constraints

The JetBrains registry announcement notes that, for now, the registry features
agents that support Agent Auth or Terminal Auth. That means a `brain` ACP agent
should assume authentication and packaging details matter if it wants registry
adoption, not just protocol conformance.

In practice, a registry-ready `brain` probably needs:

- a stable installable CLI or package
- an ACP launch mode with predictable command/args/env
- a documented authentication path
- metadata good enough for registry submission

## What This Means For `brain`

### Near term

Registry support makes ACP more compelling than a one-off Zed integration.
If `brain` implements ACP only as a local experimental bridge, it leaves a lot
of the ecosystem value on the table.

### Product implication

`brain` should think of ACP deliverables in two layers:

1. protocol/runtime correctness
2. distribution readiness

The second layer includes packaging, auth, stable CLI entry points, and agent
metadata. Those requirements are product work, not just protocol work.

### Good default framing

The right question is not "can `brain` speak ACP?" It is:

> Can `brain` become an installable ACP agent that modern clients can discover,
> run, update, and authenticate without hand-wired setup?

## Risks

- The registry is still curated, so discoverability is not automatic just
  because an agent exists.
- Registry expectations may evolve faster than the core protocol.
- If `brain` ships ACP but not a clean packaging/auth story, it may work
  technically while still feeling second-class in client ecosystems.

## Key Sources

- Zed ACP Registry announcement: <https://zed.dev/blog/acp-registry>
- Zed external agents docs: <https://zed.dev/docs/ai/external-agents>
- JetBrains ACP docs: <https://www.jetbrains.com/help/ai-assistant/acp.html>
- JetBrains registry announcement: <https://blog.jetbrains.com/ai/2026/01/acp-agent-registry/>
- Registry repo: <https://github.com/agentclientprotocol/registry>
