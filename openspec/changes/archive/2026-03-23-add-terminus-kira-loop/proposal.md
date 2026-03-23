# Proposal

## Why

`brain` already has a Harbor-compatible direct execution path and a Rust port of
Harbor's `terminus-2` loop, but it does not yet have an equivalent for
Terminus-KIRA.

KIRA differs from Terminus-2 in ways that matter for benchmark behavior:

- native provider tool calling instead of text-parsed JSON commands
- marker-based command polling for earlier terminal completion detection
- multimodal `image_read` support
- a stricter completion confirmation flow tailored to KIRA's prompt/tool model

We now have working Harbor reference runs for KIRA on `gemini-3.1-pro-preview`,
so adding a first-class Rust `terminus-kira` loop gives us a like-for-like
comparison target for later benchmark work.

## What Changes

- Add a new `TerminusKiraLoop` in `brain-loops`.
- Extend the shared message model to support multimodal content parts so loops
  can perform provider-native image analysis without ad hoc provider escape
  hatches.
- Extend the OpenAI-compatible provider serializers so both Responses and Chat
  Completions request builders can emit text-plus-image messages.
- Register `terminus-kira` in the native runtime bootstrap used by `brain exec`
  and Harbor's installed `brain.py` agent path.
- Validate locally through Docker-only Harbor smoke runs on `hello-world` and
  `terminal-bench@2.0/log-summary-date-ranges`.

## Impact

- Affected specs:
  - `brain-types`
  - `brain-loops`
  - `brain-providers-openai-api`
  - `brain-cli`
- Affected crates:
  - `brain-types`
  - `brain-providers`
  - `brain-loops`
  - `brain-cli`
  - `brain-acp`
