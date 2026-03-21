# Design: Bound Tool Output And Isolate ACP Stdio

## Decision: Truncate Tool Output In The Loop Path

The loop is the first place where tool output becomes both:

- visible to downstream clients as `ToolCallDone`
- immediately eligible to re-enter the provider prompt as a `tool` message

That makes it the right place to apply a shared global output cap. Truncating
only at storage time would still allow oversized tool output to blow up the next
provider call.

## Decision: Keep Tool-Specific Limits In The Native Drivers

Some tools benefit from stronger shaping than a shared byte cap alone:

- `file_read` should page by `offset`/`limit` and emit continuation hints
- `glob_search` and `grep` should cap result counts
- `list_directory` should cap traversed entries
- `shell` should cap combined stdout/stderr

These remain tool-specific behaviors. The shared loop cap is a second safety
layer, not the primary UX for those tools.

## Decision: ACP Logging Goes To Stderr Only

ACP stdio transport must remain protocol-only on stdout. The `brain-acp`
binaries should initialize tracing explicitly to stderr so provider and backend
errors do not corrupt ACP message parsing.
