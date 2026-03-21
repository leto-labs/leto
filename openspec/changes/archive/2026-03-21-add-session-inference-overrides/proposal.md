# Proposal: add-session-inference-overrides

## Why

The real ACP backend now runs on `brain-core`, but it still could not support
session model switching the way the mock backend did.

That exposed a deeper issue in the current engine design:

- model/provider selection was effectively project-scoped
- sessions had no durable inference override state
- `Brain::turn()` depended on a caller-supplied `AgentConfig`
- `ProviderRouter` routed mostly by model name rather than explicit provider

That shape made ACP model switching an adapter-only feature instead of a real
runtime capability.

## What

This change makes session-scoped inference a first-class concept:

- `Session` stores optional partial `InferenceConfig` overrides
- `Brain` resolves effective session inference from project defaults plus
  session overrides
- `Brain::turn()` loads session/project config internally instead of requiring
  callers to pass `AgentConfig`
- `ProviderRouter` becomes provider-aware and resolves model-only requests only
  when the model belongs to a single provider
- `ProviderRouter::available_models()` preserves provider-declared model order
  so transports can present model choices in the same order providers expose
  them
- the real ACP backend advertises and implements `session/set_model` on top of
  the new core session inference model

## Impact

- Modified capability: `brain-types`
- Modified capability: `brain-core`
- Modified capability: `brain-acp`
