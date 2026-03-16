## Context

The engine supports remote providers via `OpenAiProvider`. For local inference without an external server, we need an in-process model runtime. Researched mistral.rs, llama-cpp-rs, candle, and gguf-runner.

## Goals
- In-process GGUF model inference behind the existing `Provider` trait
- Automatic model download from HuggingFace on first use
- Feature-gated to avoid bloating builds that don't need local inference
- Preset system for known small models (Qwen 3 0.6B/1.7B/4B)

## Non-Goals
- No WASM/mobile support (mistral.rs is native-only; future work with other runtimes)
- No GPU feature flags yet (CUDA/Metal handled by mistral.rs build features, not ours)
- No model management UI or CLI (just download-on-demand)
- No non-GGUF model support (TextModelBuilder/ISQ can be added later)
- No tool calling mapping for local models (add later)

## Decisions

### 1. mistral.rs as the inference runtime

| Option | Pros | Cons |
|--------|------|------|
| mistral.rs | Most mature, GGUF+streaming+tools, auto HF download, CUDA/Metal/CPU | Heavy dep (~hundreds of crates) |
| llama-cpp-rs | Battle-tested C engine | C FFI, build complexity, less ergonomic |
| candle | Pure Rust, HF-native | No GGUF, lower-level, manual streaming |
| gguf-runner | Pure Rust, WASM potential | Very new, CPU only, no tool calling |

mistral.rs is the clear winner for native desktop. Its `GgufModelBuilder` handles HF downloads automatically, and `stream_chat_request` returns OpenAI-compatible chunks that map directly to our `ChatChunk`.

### 2. Feature-gated behind `local` (off by default)

mistral.rs is a heavy dependency. Users who only need remote providers should not pay the compile-time or binary-size cost. The feature is off by default:

```toml
[features]
default = ["openai"]
local = ["dep:mistralrs"]
```

### 3. LocalConfig + LocalModelPreset mirrors OpenAiConfig + OpenAiConfigPreset

Same pattern: a config struct for runtime settings, a preset struct for known models.

```rust
pub struct LocalConfig {
    pub model_id: String,
    pub gguf_files: Vec<String>,
    pub device: DevicePreference,
}

pub struct LocalModelPreset {
    pub name: &'static str,
    pub model_id: &'static str,
    pub gguf_files: &'static [&'static str],
}
```

### 4. Async construction, sync-ish Provider trait

`LocalProvider::new(config).await` loads the model (which may download from HF and takes seconds). Once loaded, `chat()` runs inference and streams results. The `Provider` trait already returns a `BoxFuture`, so this fits naturally.

### 5. Separate example binary `cli-local`

Rather than feature-gating cli-echo, a separate `examples/cli-local/` keeps things clean. It depends on `brain-core` with `features = ["local"]` and provides a minimal REPL.

### 6. Model downloading handled by mistral.rs

`GgufModelBuilder::new(repo_id, files)` downloads from HuggingFace if the files aren't cached locally. Cache lives at `~/.cache/huggingface/hub/` (compatible with Python's `huggingface_hub`). No custom `ModelStore` needed.

## Platform support

| Platform | Backend | Status |
|----------|---------|--------|
| Linux | CPU, optional CUDA | Supported |
| macOS | CPU, Metal | Supported |
| Windows | CPU, optional CUDA | Supported |
| WASM | N/A | Not supported (future: gguf-runner) |
| Mobile | N/A | Not supported (future: platform SDKs) |

## Risks / Trade-offs
- mistral.rs adds significant compile time and binary size — mitigated by feature gate
- First model load downloads from HF which can be slow — acceptable, subsequent loads use cache
- No GPU feature flags from our side — users must build mistral.rs with CUDA/Metal features themselves if needed
