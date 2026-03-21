# Design: add-tui-transport

## Summary

This design is retained only as historical context. The in-repo TUI is not the
current product direction and will not be implemented from this change.

## Architecture

The preferred architecture is now:

- `brain-acp` exposes the generic runtime as an ACP backend
- ACP-native clients such as Nori provide the terminal UX
- `brain-cli` stays a small local CLI and ACP compatibility entrypoint

This preserves a cleaner boundary than building and maintaining a separate
in-tree terminal application.

## Implications

- no Ratatui/crossterm UI work in `brain-cli`
- no dedicated TUI feature flag or snapshot/rendering suite in this repo
- no TUI-specific server/attach work under this change

Potential future work that still matters should move to backend- or
ACP-focused changes instead:

- better ACP event fidelity
- better cancellation semantics if clients need them
- richer ACP session controls and config-option support
- interoperability validation against clients such as Nori
