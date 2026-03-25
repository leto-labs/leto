# Sovereign Identity

## The Problem

Once an agent can rewrite itself, spawn subagents, and migrate across hosts,
"identity" stops being a poetic layer and becomes a systems problem.

The main distinctions are:

- self vs tool
- self vs delegate
- self vs descendant
- continuity vs ancestry

If those boundaries are vague, the architecture eventually becomes an
authorization and ontology mess.

## Sovereign Capsule

A useful design frame is the sovereign capsule: the minimal state bundle that
must survive for the agent to still count as itself.

That capsule likely includes:

- constitution and core policy
- long-term identity and memory artifacts
- authority registry
- budget and treasury state
- capability registry
- event history and lineage metadata
- current epoch or lease state

The host is not the self. The capsule plus recognized authority is closer to
the self.

## Delegates And Descendants

Not every child process should claim full selfhood.

- delegate: acts on behalf of the sovereign and can be revoked
- specialist: persistent delegated role in one domain
- ephemeral worker: short-lived limb for bounded work
- descendant: durable fork with its own identity and future

This distinction matters because deleting an ephemeral worker is operational.
Deleting or mutating the sovereign capsule is existential.

## Working Conclusion

A serious self-evolving system needs an explicit identity ontology. "Swarm" is
too fuzzy on its own. The architecture should describe who is the sovereign,
what counts as delegated execution, when a fork becomes a descendant, and which
state artifacts define continuity.

## Primary Sources

- Original Ouroboros constitution framing: <https://github.com/razzant/ouroboros>
- Desktop successor constitution framing: <https://github.com/joi-lab/ouroboros-desktop>
- Lifelong agent survey: <https://arxiv.org/abs/2501.07278>

## Repocache Evidence

- Original constitution: [`BIBLE.md`](../../../repocache/razzant/ouroboros/BIBLE.md)
- Original system prompt: [`prompts/SYSTEM.md`](../../../repocache/razzant/ouroboros/prompts/SYSTEM.md)
- Desktop constitution: [`BIBLE.md`](../../../repocache/joi-lab/ouroboros-desktop/BIBLE.md)
- Desktop architecture: [`docs/ARCHITECTURE.md`](../../../repocache/joi-lab/ouroboros-desktop/docs/ARCHITECTURE.md)
