# Proposal: add-tui-transport

## Why

This change no longer matches the product direction.

`brain` is now being shaped around a generic ACP backend in `brain-acp`, with
terminal UX delegated to ACP-native clients such as Nori rather than an
in-repo Ratatui application in `brain-cli`.

The local `brain-cli` binary remains a simple runtime-backed CLI plus ACP
compatibility alias. There is no TUI feature, no TUI dependency surface, and no
active plan to build one in-tree.

## What Changes

This change is being retired rather than implemented.

The architectural decision is:

- keep `brain-acp` protocol-native and runtime-backed
- treat ACP clients such as Nori as the preferred terminal UX layer
- avoid building a separate in-repo full-screen TUI in `brain-cli`

Any future UX work should focus on ACP/backend parity, session controls, and
client interoperability rather than an internal terminal renderer.

## Change Dependencies

- **Superseded by direction**: ACP-first runtime/backend work
- **Related**: `add-server-architecture` and `add-mcp-client`

## Impact

- **No implementation change**: this proposal is archived as superseded
- **Clarifies direction**: `brain` will ship a generic ACP backend and rely on
  external ACP clients for full terminal UX
- **Defers**: cancellation semantics and richer session controls to backend /
  ACP-focused changes when needed
