# Safety And Control

## Framing

A self-evolving agent is interesting because it can change itself. That also
makes control surfaces more important than in ordinary tool-using assistants.

The key question is not whether the system can act. It is how it constrains and
audits acts that change its body, secrets, or the outside world.

## Control Surface

Useful high-level controls include:

- budget ceilings and spending attribution
- irreversible-action classes
- approval or review gates for sensitive mutations
- tool and capability policies
- rollout, rollback, and rescue snapshots
- output quarantine for risky delegated work

Human approval may exist in some phases or domains, but the mature system
should not require humans on the critical path for ordinary continuity.

## Mutation Control

Self-modification should be lane-aware:

- capability-level changes are easier to stage and roll back
- control-plane changes require stronger evaluation
- constitutional or identity-core changes require the highest bar

The system should be able to improve itself, but not to silently dissolve the
very structures that make the notion of "self" meaningful.

## Human-Optional Control

The long-term target here is not endless operator babysitting. It is a control
surface strong enough that the runtime remains governable even if no human is
watching in real time.

That means:

- ordinary restart and rollback must be autonomous
- ordinary upgrades must be evaluable and promotable without manual clicks
- human review should become an optional governance layer, not a hard runtime
  dependency
- notifications should mostly serve observability, not permission gating

## Treasury And Secrets

Payments and secrets are the two highest-risk outward-facing surfaces in this
research set.

- secrets should move toward scoped and temporary credentials
- payments should live behind policy, anomaly detection, and explicit budget
  logic
- both should be fully auditable and replayable

## Working Conclusion

The research target is not maximum freedom. It is governed self-transformation:
enough autonomy to grow, enough control to preserve identity and safety, and
enough internal governance that the system does not collapse when operators are
absent.

## Primary Sources

- Agent-SafetyBench: <https://arxiv.org/abs/2412.14470>
- Agent Security Bench: <https://arxiv.org/abs/2410.02644>
- EvoAgentX survey repo: <https://github.com/EvoAgentX/Awesome-Self-Evolving-Agents>

## Repocache Evidence

- Desktop safety layer: [`ouroboros/safety.py`](../../../repocache/joi-lab/ouroboros-desktop/ouroboros/safety.py)
- Desktop tool policy: [`ouroboros/tool_policy.py`](../../../repocache/joi-lab/ouroboros-desktop/ouroboros/tool_policy.py)
- Original control tools: [`ouroboros/tools/control.py`](../../../repocache/razzant/ouroboros/ouroboros/tools/control.py)
- Original supervisor state and logs: [`supervisor/state.py`](../../../repocache/razzant/ouroboros/supervisor/state.py)
