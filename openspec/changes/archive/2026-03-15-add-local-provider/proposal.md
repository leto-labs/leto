# Change: Add local LLM provider via mistral.rs

## Why
The engine currently only supports remote API-based inference (OpenAI, Groq, etc.). For offline use, privacy, and reduced latency on capable hardware, we need in-process local inference. No existing agent project (ZeroClaw, OpenCode, IronClaw) runs models in-process — they all proxy through HTTP to Ollama or llama.cpp server. Embedding inference directly makes our engine self-contained.

## What Changes
- `brain-providers`: new `local` module (feature-gated, off by default) with `LocalConfig`, `LocalProvider`, `LocalModelPreset`
- `brain-providers`: `LocalProvider` implements `Provider` trait using mistral.rs `GgufModelBuilder` + `stream_chat_request`
- `brain-providers`: `LocalModelPreset` with const presets for small GGUF models (Qwen 3 0.6B, 1.7B, 4B)
- New `examples/cli-local/` example binary that uses the local provider
- Workspace: `mistralrs` added to workspace deps

## Impact
- Affected specs: local-provider (new)
- Affected code: brain-providers (new local/ module, Cargo.toml), workspace Cargo.toml, new example binary
- No breaking changes — `local` feature is off by default, existing builds unaffected
