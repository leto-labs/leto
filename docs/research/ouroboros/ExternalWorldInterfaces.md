# External World Interfaces

## Framing

A self-evolving agent does not just need more tools. It needs a deliberate map
of how it acts on the world.

The important capability classes are:

- compute and cloud control planes
- secrets and identity providers
- communications and notification channels
- browser and search
- domains, DNS, and web presence
- payments and treasury rails

Each class changes the risk profile in a different way.

## Capability Classes

### Compute

Cloud and ephemeral execution systems are the backbone of growth. They give the
sovereign a place to send delegated work, run experiments, and scale out.

### Secrets

A vault alone is not enough. The better long-term pattern is short-lived
credentials, scoped service identities, and auditable grants. A self-evolving
system should treat static secrets as a bootstrap phase, not a steady state.

### Communications

Telegram is only one shell. The broader model is a channel abstraction layer
over Slack, Teams, WhatsApp, Twilio, email, and other surfaces.

### Browser And Search

These are high-leverage perception layers. They are how the system explores,
discovers, configures, and verifies external reality.

### Payments

Payments are qualitatively different from many other tools because they create
irreversible external effects. They therefore belong behind stronger policy,
budgeting, and anomaly detection than ordinary tool calls.

## Reducing Human Dependence

Some external interfaces matter not only because they expand capability, but
because they reduce dependence on human operators.

- cloud control reduces dependence on manually provisioned compute
- communications reduce dependence on one human-owned chat endpoint
- credential and identity systems can eventually reduce dependence on manual key
  handling
- treasury rails reduce dependence on ad hoc human payment and funding steps

## Working Conclusion

The goal is not to wire every provider at once. It is to define capability
classes clearly enough that the agent can add, govern, and internalize them
without collapsing into a pile of one-off integrations or remaining permanently
dependent on human operators.

## Primary Sources

- OpenRouter keys: <https://openrouter.ai/keys>
- OpenAI API keys: <https://platform.openai.com/api-keys>
- Anthropic keys: <https://console.anthropic.com/settings/keys>
- Firecrawl docs: <https://www.firecrawl.dev/>

## Repocache Evidence

- Original README integration surface: [`README.md`](../../../repocache/razzant/ouroboros/README.md)
- Desktop README integration surface: [`README.md`](../../../repocache/joi-lab/ouroboros-desktop/README.md)
- EvoAgentX tools and workflow framing: [`docs/index.md`](../../../repocache/EvoAgentX/EvoAgentX/docs/index.md)
