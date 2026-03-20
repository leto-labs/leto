# Design: add-session-loop-config

## Summary

Add ACP session config support for `loop` using the existing runtime loop
registry and session loop override model.

The implementation should treat `loop` like the existing `model` and
`thought_level` config options:

- include it in `configOptions` when it is editable
- mutate persisted session state through the shared runtime surface
- return the full current config snapshot after updates

## Decisions

### `loop` is the next ACP config option

`loop_name` already exists in `AgentConfig`, `Session`, and `SessionUpdate`, and
`BrainRuntimeNative` already resolves turns from the loop registry. This makes
`loop` an incremental ACP addition rather than a new product concept.

### The real backend hides `loop` when only one loop is registered

A one-value selector is not useful. The real backend currently registers only
`simple`, so it should continue to expose only model/thought-level config until
multiple loops are available.

### The mock ACP agent always exposes multiple loop values

The mock ACP path should validate the client UX with at least two loops so ACP
clients can exercise top-level and value-picking behavior immediately.

### Loop category uses a custom ACP category

ACP reserves standard categories like `mode`, `model`, and `thought_level`.
`loop` should therefore use a custom category such as `_loop`, while remaining
correct if a client ignores categories entirely.

## Implementation Notes

- Add `set_session_loop(session_id, loop_name)` to `BrainRuntime`
- Implement the default behavior through the existing store + loop registry
- Build real ACP loop choices from `runtime.loops().list()`, sorted by loop name
- Derive the current loop from `effective_agent_config_for_session()`
- Only include the real backend `loop` config option when the loop list has more
  than one item
- Extend the mock session state with a current loop field and handle
  `session/set_config_option(loop=...)`
