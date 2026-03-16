# Change: Add OpenAI provider — real LLM inference

## Why
The engine has a full agent loop (SimpleLoop), tool dispatch (EchoTool), and in-memory persistence (InMemoryStore). The only fake piece is MockProvider. Adding a real OpenAI-compatible provider enables end-to-end testing with actual LLMs while proving the trait-based architecture holds up.

## What Changes
- `brain-types`: `InferenceConfig.model_id` replaced with `model: Option<String>`; `AgentConfig` gains `inference: InferenceConfig` field
- `brain-providers`: new `OpenAiProvider` and `OpenAiConfig` (feature-gated behind `openai`)
- `brain-providers`: adds `reqwest` and `eventsource-stream` as optional dependencies
- `cli-echo`: now reads `OPENAI_API_KEY` from `.env` via dotenvy; interactive REPL loop

## Impact
- Affected specs: openai-provider (new), provider (updated scenario for real inference)
- Affected code: brain-types/config.rs, brain-providers (new files), cli-echo/main.rs
- Breaking change: `InferenceConfig.model_id` renamed to `model: Option<String>` (internal-only, no external consumers yet)
