# Design: add-real-acp-config-options

## Decision: ACP Config Options Use Shared Session Inference

The real backend should not invent ACP-only session config state.

`model` and `thought_level` must both be expressed through the existing shared
session inference layer so:

- project defaults stay the baseline
- session overrides stay durable and partial
- ACP remains a transport over core runtime behavior

## Decision: First Real Surface Includes Model And Thought Level

The first real `configOptions` surface should include:

- `model`
- `thought_level`

`model` already has real session-scoped support. `thought_level` requires a new
field on shared inference/session config, but it is the key missing control the
validated Nori UX was built to expose.

Broader inference tuning such as `temperature`, `max_tokens`, or speed presets
is out of scope for this first real ACP config-option change.

## Decision: Model Options Are Grouped By Provider

The real `model` config option should be a grouped select keyed by provider.

This uses existing runtime model metadata and matches the validated Nori picker
behavior without changing runtime identity semantics. Grouping is presentation
only; persistence still stores provider/model inference overrides.

Ordering should preserve the current `runtime.list_models()` order within each
provider group rather than introducing a new ACP-only sort.

## Decision: Thought Level Is Derived From Effective Model Metadata

The real `thought_level` option should be present only when the current
effective model exposes supported reasoning levels through `ModelInfo.reasoning`.

The allowed ACP values should come directly from those supported levels.
Invalid values for the current effective model should be rejected.

When a model has no reasoning support, the backend should omit the
`thought_level` option entirely rather than surfacing a disabled placeholder.

## Decision: Existing Session Model Support Stays Intact

The real backend should continue to support `session/set_model` and model state
for compatibility with clients that still use that path.

`configOptions` becomes the preferred generic session-settings surface, but the
existing model method should remain wired to the same shared session inference
state.
