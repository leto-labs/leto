# Capabilities And Organs

## Framing

The most useful metaphor here is anatomy. Capabilities begin as external aids,
but some of them become internal organs once the agent wraps them in policy,
memory, testing, and stable interfaces.

That leads to a useful split:

- seeded organs: present from the beginning because they are too fundamental to
  defer
- absorbed organs: capabilities the system decides to internalize later
- disposable tools: useful, but not worth carrying as first-class anatomy

## Seeded Organs

The early body should usually include:

- code editing and deployment
- memory and narrative continuity
- evaluation and rollback
- secrets bootstrap
- one compute provider path
- one operator communication path

Without these, the agent cannot responsibly grow the rest of itself.

## Absorbed Organs

Over time, likely organs include:

- cloud provisioning and job execution
- browser and search
- comms adapters such as Slack, Teams, WhatsApp, Twilio, email
- payments and treasury controls
- domain and DNS management
- structured knowledge and registry systems

The key is that each absorbed organ needs its own lifecycle:
versioning, permission model, tests, health checks, and deprecation path.

## Plugin Systems

A plugin or add-on system is valuable, but it should be treated as an organ
growth interface, not a dumping ground. The plugin system itself is part of the
evolution machinery and may later need to evolve too.

The system should therefore separate:

- stable capability contracts
- policy and permission declarations
- evaluation hooks
- promotion and rollback hooks

## Working Conclusion

The right goal is not "give the agent every API." It is "give the agent the
ability to notice which recurring capabilities deserve to become part of its
body, and then absorb them through a governed path."

## Primary Sources

- EvoAgentX docs: <https://evoagentx.github.io/EvoAgentX/>
- OpenClaw repo: <https://github.com/openclaw/openclaw>
- EvoAgentX framework paper: <https://arxiv.org/abs/2507.03616>

## Repocache Evidence

- Desktop tool registry and policy: [`ouroboros/tools/registry.py`](../../../repocache/joi-lab/ouroboros-desktop/ouroboros/tools/registry.py)
- Desktop tool policy: [`ouroboros/tool_policy.py`](../../../repocache/joi-lab/ouroboros-desktop/ouroboros/tool_policy.py)
- EvoAgentX README: [`README.md`](../../../repocache/EvoAgentX/EvoAgentX/README.md)
- OpenClaw concept notes: [`docs/concepts/system-prompt.md`](../../../repocache/openclaw/openclaw/docs/concepts/system-prompt.md)
