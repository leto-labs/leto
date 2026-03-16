# Proposal: add-brain-tools

## Why

The only tool implementation (`EchoTool`) lives in `brain-loops`, coupling tool
logic with agent loop orchestration. Every reference framework (zeroclaw,
ironclaw, openclaw, opencode) separates tools from the loop. We need a dedicated
crate so tools can grow independently and be reused across different loops.

Brain also needs to support multiple platforms (desktop, mobile, web). Tools like
file-read use OS-specific APIs, so we need a driver-trait pattern that lets each
platform provide its own implementation while sharing the LLM-facing JSON schema
and argument parsing.

## What

Create a `brain-tools` crate with:

1. **Driver trait + Tool adapter** pattern for each tool — the driver trait
   defines typed platform operations, the adapter wraps any driver into a `Tool`
   impl (handling JSON schema, arg parsing, output formatting).
2. **Native drivers** feature-gated under `native` — use `tokio::fs`,
   `tokio::process`, `glob`, and `regex` crates for desktop/server.
3. **7 tools** covering the universal baseline from reference projects:
   - `echo` (platform-independent, no driver)
   - `file_read`, `file_write`, `file_edit` (filesystem)
   - `shell` (process execution)
   - `glob_search` (file pattern matching)
   - `grep` (content search)
4. **`native_tools()` preset** — convenience function returning all native-backed
   tools as `Vec<Arc<dyn Tool>>`.
5. **Move EchoTool** from `brain-loops` to `brain-tools`.

## Impact

- New crate: `brain-tools` added to workspace
- `brain-loops` loses `echo_tool.rs`, re-exports from `brain-tools` for compat
- `brain-core` gains `brain-tools` dependency and re-export
- Examples updated to use `native_tools()` preset
- `AGENTS.md` crate map updated
