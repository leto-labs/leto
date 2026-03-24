## Design

`provider-preset-gen` already owns the exact emitted Rust source for
`provider-openai/src/presets/`. The simplest fix is to format each generated
file inside the generator itself before it is written or checked.

The generator will:

1. render Rust source as it does today
2. invoke `rustfmt --edition 2024 --emit stdout`
3. compare or write the formatted result

This keeps the source of truth inside the generator and avoids requiring a
separate post-generation manual formatting step.

## Tradeoffs

### Using `rustfmt` directly

Pros:
- matches repository formatting behavior exactly
- no need to maintain a parallel pretty-printer

Cons:
- requires `rustfmt` to be available in the development environment

This is acceptable because the workspace already assumes a normal Rust toolchain
for development.
