# Lineage And Forks

## Framing

This page tracks three different relationships that are easy to blur together:

- direct lineage: the same project continuing in a new repo or runtime shape
- literal forks: code forks with shared ancestry but potentially diverging goals
- concept cousins: projects that are not forks, but explore the same design
  space of self-evolution, workflow evolution, or agent governance

The Ouroboros research story needs all three.

## Direct Lineage

The clearest lineage today is:

1. the original Ouroboros runtime centered on Colab, Telegram, GitHub, and
   Drive-backed memory
2. the later desktop successor with an immutable launcher, local HTTP/WebSocket
   server, local model support, and explicit safety layers

That is an architectural maturation story, not just a repo rename. The
important shift is from a cloud notebook shell to a more sovereign local host.

## Literal Forks

The public GitHub surface shows substantial fork activity around the original
project. For this research section, the interesting forks are not the raw count
but the forks that materially diverge in runtime model, safety model, or
deployment target.

This folder does not try to maintain an exhaustive public fork graph. It keeps
a working list of the forks and successors that change the architecture
conversation.

## Concept Cousins

Several projects belong in the same conceptual neighborhood even when they are
not part of the Ouroboros family tree:

- EvoAgentX for explicit workflow evolution, evaluation, and memory
- OpenClaw for gateway-centric control-plane and policy-heavy multi-surface agents
- Gastown for durable multi-agent orchestration above existing runtimes
- self-improving coding-agent writeups and survey repos that treat agent
  improvement as an iterative engineering loop rather than a single static prompt

These are useful because they show different answers to the same larger
question: how much of the organism is fixed, how much is generated, and who
gets to change the body over time?

## Working Conclusion

Ouroboros should be researched as a lineage plus an ecosystem. The family tree
matters because it shows how the original idea hardens over time. The concept
cousins matter because the strongest future version will likely borrow from
multiple traditions: sovereign identity from Ouroboros, workflow evolution from
EvoAgentX, control-plane patterns from OpenClaw and Gastown, and evaluation and
safety ideas from the broader agent-research literature.

## Primary Sources

- Original repo: <https://github.com/razzant/ouroboros>
- Successor desktop repo: <https://github.com/joi-lab/ouroboros-desktop>
- EvoAgentX repo: <https://github.com/EvoAgentX/EvoAgentX>
- Self-Evolving-Agents survey repo: <https://github.com/CharlesQ9/Self-Evolving-Agents>

## Repocache Evidence

- Original Ouroboros: [`repocache/razzant/ouroboros`](../../../repocache/razzant/ouroboros)
- Desktop successor: [`repocache/joi-lab/ouroboros-desktop`](../../../repocache/joi-lab/ouroboros-desktop)
- EvoAgentX: [`repocache/EvoAgentX/EvoAgentX`](../../../repocache/EvoAgentX/EvoAgentX)
- OpenClaw: [`repocache/openclaw/openclaw`](../../../repocache/openclaw/openclaw)
- Gastown: [`repocache/steveyegge/gastown`](../../../repocache/steveyegge/gastown)
