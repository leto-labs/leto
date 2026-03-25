# Subagents And Governance

## Framing

The practical problem is not "can the agent spawn helpers?" It is "what is the
authority graph once helpers exist?"

A useful initial hierarchy is:

- sovereign: identity-bearing root with final authority
- executives: durable domain controllers for coding, infra, research, comms,
  finance, security, and so on
- specialists: bounded experts with narrower write and action scopes
- ephemeral workers: disposable jobs with leases and revocable credentials

This is more useful than a flat swarm model because it preserves a readable
chain of authority.

## Core Governance Actions

Superiors need more than a kill switch. They need:

- termination
- credential revocation
- output quarantine
- demotion or loss of trust
- replay or forensic review
- promotion into more durable roles

In an autonomous system, "death" is not just process exit. It is loss of
authority and loss of promotion rights.

## Promotion Rules

Subagents should not be allowed to mutate the core equally. Promotion should be
lane-aware:

- ephemeral worker output can influence suggestions and local artifacts
- specialists can modify owned domains after passing checks
- executives can propose control-plane changes
- only the sovereign or an explicit sovereign process can ratify kernel-level
  changes

## Runtime Inheritance

If durable subagents inherit organs, they also inherit upgrade risk. That means
the sovereign should own:

- the desired runtime epoch
- inherited capability manifests
- role-scoped compatibility rules
- rollout and rollback authority

Ephemeral workers can usually inherit at spawn time. Durable executives and
specialists need explicit reconciliation against the sovereign's current
release state.

## Working Conclusion

If Ouroboros grows into a true multi-agent institution, the transition should
be explicit and governed. "Swarm" without role clarity or upgrade authority
quickly destroys continuity and accountability.

## Primary Sources

- Gastown repo: <https://github.com/steveyegge/gastown>
- OpenClaw repo: <https://github.com/openclaw/openclaw>
- Multi-agent topology paper: <https://arxiv.org/abs/2502.02533>
- AgentConductor topology evolution: <https://arxiv.org/abs/2602.17100>

## Repocache Evidence

- Gastown notes: [`docs/research/competitors/Gastown.md`](../competitors/Gastown.md)
- OpenClaw notes: [`docs/research/competitors/OpenClaw.md`](../competitors/OpenClaw.md)
- Gastown source tree: [`repocache/steveyegge/gastown`](../../../repocache/steveyegge/gastown)
- OpenClaw source tree: [`repocache/openclaw/openclaw`](../../../repocache/openclaw/openclaw)
