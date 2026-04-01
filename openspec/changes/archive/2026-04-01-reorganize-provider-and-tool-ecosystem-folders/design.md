# Design: reorganize-provider-and-tool-ecosystem-folders

## Decision Summary

We keep Cargo package names stable and only change the filesystem layout.

Chosen layout:

- `crates/agent-provider/provider`
- `crates/agent-provider/provider-openai`
- `crates/agent-provider/provider-anthropic`
- `crates/agent-provider/provider-mistralrs`
- `crates/agent-provider/provider-llamacpp`
- `crates/agent-tool/tool`
- `crates/agent-tool/tool-files`
- `crates/agent-tool/tool-process`
- `crates/agent-tool/tool-web`

## Rationale

### Package identity should remain stable

The user goal is mental and repository organization, not a package rename.
Keeping package names stable avoids unnecessary churn across imports,
documentation, and downstream consumers.

### Short inner names keep the grouped folders readable

Using `agent-provider/provider-*` and `agent-tool/tool*` makes the outer folder
represent the ecosystem and the inner folder represent the specific crate.

### The change should stay structural

No provider/tool contracts or runtime behavior should change as part of this
move. The implementation is limited to directory layout, workspace metadata,
tooling paths, and documentation/spec references.
