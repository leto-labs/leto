# LlamaCpp Provider

This page covers the `llamacpp` local provider implementation.

## Main Pieces

| Component | Role |
| --- | --- |
| `LlamaCppProvider` | In-process provider backed by `llama.cpp` |
| `LlamaCppConfig` | GGUF model and runtime configuration |
| `LlamaCppModelPreset` | Named preset catalog for common local models |

## Characteristics

| Concern | Current behavior |
| --- | --- |
| Feature gate | Disabled unless the `llamacpp` feature is enabled |
| Model format | Built around GGUF and local model files |
| Integration point | Exposes the same `Provider` trait as the remote providers |

## Why It Exists

Like `mistralrs`, the `llamacpp` path proves that local inference can be added
without changing the core runtime contract. The runtime still only sees a
registered provider with model metadata and a `chat(...)` implementation.
