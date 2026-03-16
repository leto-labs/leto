# Design: add-brain-tools

## Architecture: Driver Trait + Tool Adapter

Mirrors the provider pattern (`Provider` trait in brain-types, `OpenAiProvider`/
`MistralRsProvider` as feature-gated impls in brain-providers).

Each tool has three pieces:

- **Driver trait** (e.g. `FileReadDriver`) — typed operation, no JSON, `Send + Sync`
- **Tool adapter** (e.g. `FileReadTool<T: FileReadDriver>`) — implements `Tool`,
  owns the JSON schema definition and argument parsing, delegates to the driver
- **Platform driver** (e.g. `FileReadDriverNative`) — implements the driver trait
  using OS-specific APIs, feature-gated

### Naming Convention

| Piece            | Pattern              | Example                              |
|------------------|----------------------|--------------------------------------|
| Driver trait     | `{Name}Driver`       | `FileReadDriver`                     |
| Tool adapter     | `{Name}Tool<T>`      | `FileReadTool<T: FileReadDriver>`    |
| Native driver    | `{Name}DriverNative` | `FileReadDriverNative`               |
| Future web       | `{Name}DriverWeb`    | `FileReadDriverWeb`                  |
| Future iOS       | `{Name}DriverIos`    | `FileReadDriverIos`                  |

### Why Driver Traits?

The `Tool` trait is LLM-facing (JSON in, String out). Platform code shouldn't
deal with JSON parsing — that's the adapter's job. The driver trait gives typed
params (`&str`, `Option<usize>`) so platform impls are clean and testable.

"Driver" was chosen over "Backend" (server connotation), "Impl" (conflicts with
Rust keyword semantics on traits), and "Platform" (too narrow — the pattern also
works for service-provider variants like `WebSearchDriverBrave`).

## Feature Gates

```toml
[features]
default = ["native"]
native = ["dep:tokio", "dep:glob", "dep:regex"]
```

Driver traits and adapters are always available (no feature gate). Only the native
driver impls are gated. This lets future platform crates depend on `brain-tools`
without pulling in native-only dependencies.

## Module Structure

One folder per tool (mirrors `brain-providers` where each provider gets a folder):

```
crates/brain-tools/src/
├── lib.rs
├── echo.rs
├── file_read/
│   ├── mod.rs      (FileReadDriver + FileReadTool<T>)
│   └── native.rs   (FileReadDriverNative)
├── file_write/
│   ├── mod.rs
│   └── native.rs
├── file_edit/
│   ├── mod.rs
│   └── native.rs
├── shell/
│   ├── mod.rs
│   └── native.rs
├── glob_search/
│   ├── mod.rs
│   └── native.rs
└── grep/
    ├── mod.rs
    └── native.rs
```

## Preset Function

```rust
#[cfg(feature = "native")]
pub fn native_tools() -> Vec<Arc<dyn Tool>> { ... }
```

## Decisions

- **`glob_search` module name** instead of `glob` to avoid shadowing the `glob`
  crate in imports.
- **EchoTool has no driver** — it's pure logic with no platform dependency.
- **No workspace scoping yet** — tools operate on the real filesystem. Security
  and sandboxing can be layered into drivers later.
- **`brain-loops` re-exports `EchoTool`** from `brain-tools` for backward compat.
