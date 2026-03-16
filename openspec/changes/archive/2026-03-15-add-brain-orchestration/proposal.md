# Change: Add Brain orchestration engine + Transport trait

## Why
`brain-core` is currently a hollow facade that re-exports the other crates. All orchestration logic (wiring Provider + Store + AgentLoop + Tools, managing session history, dispatching events) lives in `cli-echo`. This means every new frontend (CLI, Telegram bot, HTTP API, Tauri app) must duplicate the same orchestration boilerplate. Meanwhile, the system has no abstraction for IO — input and output are hardcoded to stdin/stdout in the example binary.

## What Changes
- `brain-types`: new `Transport` trait (recv/send) and `InputEvent` enum alongside existing traits
- `brain-core`: promoted from re-export facade to orchestration engine — new `Brain` struct that owns Provider, Store, AgentLoop, Tools and manages session lifecycle + turn dispatch
- New crate `brain-transports`: `CliTransport` implementation (stdin/stdout)
- `cli-echo`: simplified from ~80 lines of manual wiring to ~15 lines using `Brain` + `CliTransport`
- Workspace `Cargo.toml`: add `brain-transports` to members and workspace deps

## Impact
- Affected specs: transport (new), brain-engine (new), agent-loop (modified — Brain now dispatches loops)
- Affected code: brain-types (new trait + types), brain-core (new Brain struct + deps), brain-transports (new crate), cli-echo (simplified)
- No breaking changes to existing trait signatures
