## Context
The agent loop, tool dispatch, and persistence all work with MockProvider. To validate the architecture with real inference we need an OpenAI-compatible provider that handles streaming chat completions with tool calls.

## Goals
- Real LLM inference through the existing Provider trait
- Library never reads env vars — config passed as data via `OpenAiConfig`
- SSE streaming with incremental tool call argument accumulation
- Feature-gated so users without an API key pay zero compile cost
- Works with any OpenAI-compatible endpoint (OpenAI, Azure, Ollama, etc.)

## Non-Goals
- No retry/backoff logic (add later)
- No token counting or context window management
- No streaming function call approval flow
- No config file system (caller's concern)

## Decisions

### 1. Config as data, not env vars
`OpenAiConfig` is a plain struct with `api_key`, `base_url`, `default_model`. The library never calls `std::env::var`. The CLI or app is responsible for sourcing config (env vars, file, database, etc.). For the smoke test CLI, env vars are acceptable.

### 2. Feature-gated behind `openai`
`reqwest` and `eventsource-stream` are optional deps in `brain-providers` gated behind `features = ["openai"]` (on by default). Building with `--no-default-features` compiles without any HTTP dependencies.

### 3. InferenceConfig.model becomes Option
When `model` is `None`, the provider uses its own `default_model`. This lets existing code work without knowing which model to request — the provider decides.

### 4. SSE parsing via eventsource-stream
We use `eventsource-stream::Eventsource` trait on reqwest's byte stream. Tool call arguments arrive incrementally across multiple SSE chunks, accumulated by index and flushed on `finish_reason`.

### 5. AgentConfig carries InferenceConfig
Previously `SimpleLoop` hardcoded `InferenceConfig::default()`. Now `AgentConfig.inference` lets the caller control model, temperature, and max_tokens per run.

## Risks / Trade-offs
- `reqwest` adds ~97 transitive crates — mitigated by feature gate
- No retry means transient API errors surface directly — acceptable for initial version
- `eventsource-stream` 0.2 is adequate but not actively maintained — could switch to `reqwest-eventsource` later if needed
