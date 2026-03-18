# Terminal-First ACP Validation

## Summary

The first real-world validation target for the mock `brain acp` proof of
concept should be a terminal-native ACP client rather than a full editor.

For this repo, that means a headless ACP CLI, not a full-screen TUI.

This keeps the validation loop fast:

- launch `brain acp` locally
- drive the ACP lifecycle from a terminal client
- inspect shell output and ACP logs directly
- keep richer TUI checks separate from the baseline compatibility path

For this repo, `acpx` is the baseline terminal client target because the ACP
research already calls it out as a non-editor ACP shell and its upstream
positions it as a headless, scriptable ACP CLI. This document assumes `acpx`
is installed and available on `PATH`.

## Target Order

1. Primary: `acpx`
2. Secondary: Nori for richer multi-turn ACP TUI validation

If another richer ACP TUI becomes attractive later, it can be evaluated as an
additional client, but `acpx` remains the baseline completion target for this
change. It should not be confused with the eventual first-party TUI tracked
elsewhere in the repo.

## Agent Command

Use the existing `brain-cli` ACP entry point through Cargo so each run picks up
local source changes automatically:

```bash
acpx --agent "cargo run -q -p brain-cli -- acp" exec "Reply exactly with: hello world"
```

This is the canonical local launcher for the current repo state because:

- `brain-cli` is the actual ACP entry point exposed to users
- the `acp` subcommand dispatches directly to `brain_acp::run_stdio()`
- `cargo run` rebuilds when relevant source changes exist

The built-binary form remains possible for faster repeated runs, but it is not
the default local validation path:

```bash
brain acp
```

For an isolated temp working directory or an automated harness, use the repo's
stable launcher script instead of raw `cargo run`:

```bash
./scripts/brain-acp-launcher.sh
```

This avoids `cargo` resolving the workspace from the temp cwd used by `acpx`.

## Terminal Client Assumption

Use the locally installed `acpx` binary. The preferred local setup for repeated
validation is a global install:

```bash
pnpm add -g acpx
```

Then confirm it is available:

```bash
command -v acpx
acpx --help
```

This change does not require `brain` to package or vendor `acpx`; it only
defines `acpx` as the baseline manual validation target.

## Required Validation Flow

The terminal validation pass should verify all of the following against local
`brain acp`:

1. Launch the client against `brain acp` over stdio with the Cargo-based
   launcher.
2. Initialize ACP successfully.
3. Run a one-shot hello-world prompt and observe streamed output.
4. Run one client-owned ACP request and confirm the result is streamed back
   through the session.
5. If needed, expand to persistent-session commands such as `sessions new` and
   `status`.

## Automated Compatibility Harness

For a broader external-client validation pass, run:

```bash
just acpx-compat
```

This opt-in harness exercises:

- one-shot hello-world
- permission request
- file read
- file write
- terminal
- terminal kill
- session create/show/list/status metadata flows

It is intentionally broader than the manual sanity check, but it is not part of
the default `cargo test` path.

At the moment, the harness does not gate on `acpx` prompt-reconnect behavior
such as `session/load` reuse, `set-mode`, config mutation, or cancel-on-reload.
Those remain useful compatibility probes, but they are not yet stable enough in
the current `acpx` + mock `brain-acp` combination to be treated as passing
baseline coverage.

## Expected Success Criteria

The validation is successful when:

- `brain acp` starts cleanly and stays attached to the client over stdio
- prompt turns stream visible mocked agent output
- client-owned ACP requests invoked by the mock `mock:` commands round-trip
  successfully
- any client limitations are recorded as client-specific observations rather
  than treated as failures of the `brain-acp` mock surface

## Recorded Local Validation

The following commands were run successfully in this workspace:

```bash
acpx codex exec "Reply exactly with: hello world"
acpx --agent "cargo run -q -p brain-cli -- acp" exec "Reply exactly with: hello world"
acpx --approve-all --agent "cargo run -q -p brain-cli -- acp" exec "mock:request-permission"
```

Observed outcomes:

- Codex path returned `hello world`
- `brain acp` path returned `Mock brain-acp response: Reply exactly with: hello world`
- permission probe surfaced `session/request_permission` and streamed back
  `allow-once`

## Richer TUI Checks

After the terminal validation path is documented and runnable, richer manual
multi-turn validation may be performed in Nori against the same local `brain
acp` entry point. This is useful for UX confidence, but it is intentionally
separate from the baseline `acpx` compatibility gate.
