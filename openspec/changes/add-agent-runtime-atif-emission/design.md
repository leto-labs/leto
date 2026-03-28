## Overview

This change ports the append-only ATIF construction flow from legacy
`brain-core` into `agent-runtime`, then treats emitted ATIF events as the
single source of truth for downstream consumers.

## Decisions

### Emit ATIF directly from `agent-runtime`

`agent-runtime` already owns the canonical in-memory transcript for a turn.
That makes it the correct place to:

- announce trajectory metadata at turn start
- derive ATIF steps from newly committed transcript messages
- compute final metrics from observed runtime usage and iteration counts
- emit the completed trajectory before `TurnFinished`

### Persist trajectories in `agent-core`

`agent-runtime` remains storeless. Persistence stays in `agent-core`, which
already owns the session store and existing `trajectory`/`upsert_trajectory`
helpers.

`agent-core` listens for completed ATIF runtime events, caches the latest
trajectory for the active turn, and upserts it once the turn reaches a terminal
boundary.

### Let CLI consume emitted ATIF records

`agent-cli exec` should no longer infer ATIF from plain streamed text. Instead
it assembles ATIF state from received runtime events and writes the validated
completed trajectory to `trajectory.json`.

## Tradeoffs

- `agent-runtime` gains a dependency on `atif`, but that is the correct schema
  boundary and avoids duplicating ATIF logic in multiple consumers.
- `agent-cli exec` still computes run-summary pricing locally because pricing is
  not part of the runtime ATIF metrics contract today.
