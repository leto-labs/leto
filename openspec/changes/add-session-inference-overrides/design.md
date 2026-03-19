# Design: add-session-inference-overrides

## Decision: Sessions Keep Project Affinity

Sessions remain required to belong to a project.

`brain-types` stays general enough by allowing embeddings to create a default or
synthetic project when they do not have a strong user-visible project concept.
That keeps config defaults, session grouping, and root/workspace identity
anchored in one place.

## Decision: Session Inference Uses Partial Override Semantics

`Session` gains an optional `inference: Option<InferenceConfig>` field.

This field stores only the session-local override layer:

- project config remains the default baseline
- session inference overrides only the fields that are set
- `Brain` resolves the effective runtime inference config on each turn

This avoids copying project defaults into every session while still supporting
per-session model switching and future per-session tuning.

## Decision: Brain Owns Session Config Resolution

`Brain::turn()` should no longer accept a caller-supplied `AgentConfig`.

Instead it should:

- load the session
- load the owning project
- merge project inference defaults with session inference overrides
- pass the effective config into the agent loop

That keeps session runtime behavior in `brain-core` instead of duplicating merge
logic in ACP, server, or CLI layers.

## Decision: ProviderRouter Must Be Provider-Aware

ACP `session/set_model` only sends a model ID, not a provider ID.

To make that deterministic, `ProviderRouter` keeps provider registrations by
provider name and indexes model IDs back to provider ownership.

Resolution order is:

1. explicit provider from `InferenceConfig.provider`
2. unambiguous provider lookup from `InferenceConfig.model`
3. configured default provider when neither field is set

Ambiguous model IDs are not valid model-only selections and should not be
advertised through ACP model lists.

When the router surfaces available model metadata for transports such as ACP, it
should preserve the order declared by each provider instead of re-sorting model
IDs globally. That keeps provider-curated ordering, such as newest-first model
lists, visible to clients.

## Decision: ACP Uses Core Session Inference, Not Adapter State

The real ACP backend computes session model state from:

- `Brain::effective_inference_for_session`
- `ProviderRouter` model metadata

`session/set_model` resolves the ACP model ID through the router, then persists
provider+model on the real `Session`.

The mock backend remains separate, but the real backend no longer depends on a
mock-style ACP-only session model.
