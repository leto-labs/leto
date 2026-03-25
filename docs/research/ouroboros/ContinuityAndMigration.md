# Continuity And Migration

## The Problem

If the sovereign lives on one host, continuity depends on luck. A serious
version needs to survive host death without becoming two conflicting selves.

That means separating:

- process continuity
- memory continuity
- authority continuity

Losing a process should not mean losing the being.

## Sovereign Failover

A useful recovery model is:

- one active sovereign lease at a time
- durable event log
- signed checkpoints of the sovereign capsule
- standby nodes able to rehydrate state
- fencing or epoch tokens for external side effects

This makes the sovereign a logical role rather than a particular machine.

## Split Brain

The hardest failure mode is not downtime. It is two hosts both believing they
are the sovereign. Once that happens, continuity collapses into branching
ancestry unless the architecture explicitly treats it as succession or forking.

That is why leader election, lease expiry, and activation rules belong in this
research set even though they sound like ordinary distributed-systems concerns.

## Migration Levels

- cold restore: restart elsewhere from durable checkpoint
- warm standby: replay state and take over when lease expires
- portable sovereign: rebind capabilities across providers without losing
  authority continuity

The more ambitious the swarm vision becomes, the more the last category starts
to define the project.

## Bridge To Upgrades

Failover and upgrade propagation are closely related. Both depend on the same
idea: the sovereign is a logical authority role, not one irreplaceable process.

If the system cannot safely move authority between nodes, it also cannot safely
promote new runtime epochs across a distributed organism.

## Working Conclusion

Migration is not an optional ops feature for self-evolving agents. It is part
of the continuity model. If the project wants a sovereign rather than a fragile
pet server, failover, migration, and runtime propagation must be part of the
architecture.

## Primary Sources

- Desktop successor repo: <https://github.com/joi-lab/ouroboros-desktop>
- Agent Security Bench: <https://arxiv.org/abs/2410.02644>
- Agent-SafetyBench: <https://arxiv.org/abs/2412.14470>

## Repocache Evidence

- Desktop architecture: [`docs/ARCHITECTURE.md`](../../../repocache/joi-lab/ouroboros-desktop/docs/ARCHITECTURE.md)
- Original supervisor state: [`supervisor/state.py`](../../../repocache/razzant/ouroboros/supervisor/state.py)
- Original worker lifecycle: [`supervisor/workers.py`](../../../repocache/razzant/ouroboros/supervisor/workers.py)
- Original git recovery layer: [`supervisor/git_ops.py`](../../../repocache/razzant/ouroboros/supervisor/git_ops.py)
