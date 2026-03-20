# Proposal: add-real-acp-config-options

## Why

The Nori fork now has a working generic ACP `/session-config` UX, and the mock
`brain-acp` backend already proves that the client-side flow works. The real
backend still only exposes session model state, so Nori correctly renders the
empty-state message instead of real controls.

That leaves one clear next step:

- expose real ACP `configOptions` from `brain-acp`
- keep the surface protocol-native and generic
- use the existing session inference model rather than ACP-only adapter state

## What

This change will:

- extend session inference to carry a persisted reasoning / thought-level override
- expose real ACP `configOptions` for `model` and `thought_level`
- group model config options by provider in ACP responses
- implement `session/set_config_option` in the real backend
- keep existing `session/set_model` support for compatibility

## Impact

- Modified capability: `brain-types`
- Modified capability: `brain-core`
- Modified capability: `brain-acp`
