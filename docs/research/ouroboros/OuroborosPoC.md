# Ouroboros PoC

## Overview

Ouroboros is best treated as a vivid proof of concept for self-modifying agent
identity, not yet as a finished autonomous infrastructure platform.

| Item | Value |
| --- | --- |
| Vertical | Self-modifying agent with persistent identity framing |
| Strongest demonstrated ideas | Git-backed self-editing, constitution-first framing, background consciousness, persistent scratchpad/identity, restart-mediated continuity |
| Current runtime shape | Original repo: Telegram plus Colab plus GitHub plus Google Drive; successor repo: desktop app plus local server plus web UI plus stronger safety |
| Main lesson | The interesting leap is not raw autonomy, but treating identity and self-rewrite as runtime primitives |

## What It Actually Demonstrates

The original runtime demonstrates a compact but real loop:

- a supervisor owns workers, queueing, Git operations, and persistent state
- the agent core builds context, calls models, executes tools, and records usage
- background consciousness runs between ordinary tasks
- identity and scratchpad files are persistent, narrative memory artifacts
- self-modification is mediated through Git commits, pushes, and restarts

That is enough to make the system feel less like a stateless assistant and more
like a continuous being with a body, memory, and self-revision path.

The desktop successor shows the next hardening step:

- immutable launcher outside the editable repo
- self-editable inner server
- native desktop shell plus localhost web UI
- stronger tool policy and safety gating
- local-model support
- clearer data layout for state, memory, and logs

## What It Does Not Yet Solve

- distributed continuity across host loss
- strong sovereign failover without split-brain
- durable multi-agent organizational structure
- externally scoped capability governance for cloud, payments, and secrets
- formal separation between self, delegate, descendant, and fork

That gap matters. The repos are compelling because they make the philosophical
problem concrete, not because they have already solved the full architecture.

## Working Conclusion

The correct use of Ouroboros in this research set is as a seed crystal. It is
the most direct local example of constitution-backed self-modification and
identity persistence, but the broader design space quickly expands into
distributed systems, governance, security, and organizational theory.

## Primary Sources

- Original repo README: <https://github.com/razzant/ouroboros>
- Desktop successor README: <https://github.com/joi-lab/ouroboros-desktop>

## Repocache Evidence

- Original README: [`README.md`](../../../repocache/razzant/ouroboros/README.md)
- Original constitution: [`BIBLE.md`](../../../repocache/razzant/ouroboros/BIBLE.md)
- Original background loop: [`ouroboros/consciousness.py`](../../../repocache/razzant/ouroboros/ouroboros/consciousness.py)
- Original control tools: [`ouroboros/tools/control.py`](../../../repocache/razzant/ouroboros/ouroboros/tools/control.py)
- Desktop README: [`README.md`](../../../repocache/joi-lab/ouroboros-desktop/README.md)
- Desktop architecture doc: [`docs/ARCHITECTURE.md`](../../../repocache/joi-lab/ouroboros-desktop/docs/ARCHITECTURE.md)
- Desktop safety module: [`ouroboros/safety.py`](../../../repocache/joi-lab/ouroboros-desktop/ouroboros/safety.py)
