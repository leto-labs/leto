# Non-Editor ACP Clients

## Summary

The official ACP clients page now makes it impossible to describe ACP as
"basically for Zed and maybe a few editors." As of 2026-03-18 it lists:

- desktop and web clients
- notebook and data tooling
- mobile clients
- messaging integrations
- connectors that bridge ACP into other environments

This does not mean ACP is already the universal client abstraction for all
`brain` UIs. It does mean the idea is plausible enough to take seriously.

## What Is Actually Listed

### Desktop, web, and app-style clients

The official clients page includes:

- ACP UI
- acpx CLI
- desktop wrappers like `gemini-cli-desktop`
- several web or app-style agent shells

This suggests ACP is already being used as a general agent frontend protocol,
not only an editor protocol.

### Notebook and data tools

The clients page lists:

- `agent-client-kernel` for Jupyter notebooks
- DuckDB ACP integrations
- `marimo`

That matters because notebook and data environments have very different UX needs
from code editors. ACP is at least being attempted there.

### Mobile

The official page lists:

- Agmente
- Happy
- Mobvibe

These appear to be mobile-first or mobile-capable ACP clients. This is
important evidence that ACP is already being stretched beyond keyboard-heavy IDE
workflows.

### Messaging

The clients page lists:

- ACP Discord
- Slack integrations
- Telegram ACP bots

That is probably the strongest sign that ACP can serve as an ingress protocol
for conversational shells that are not editors at all.

### Connectors

The connectors section includes bridges like:

- Aptove Bridge, which maps stdio ACP agents into a mobile client over WebSocket
- OpenClaw's ACP bridge into Gateway-backed sessions

This is especially relevant for `brain`: it suggests a future where ACP is not
always the deepest runtime protocol, but is still the lingua franca between
agent runtimes and client surfaces.

## What To Believe, And What Not To Overclaim

It is safe to believe:

- ACP is already broader than editors
- ACP has enough ecosystem motion to justify designing for broader client types
- connectors can bridge stdio ACP agents into other transports and shells

It is not yet safe to conclude:

- ACP should replace every `brain` transport
- ACP is equally mature for mobile, notebooks, messaging, and editors
- the current protocol is fully converged for remote/hosted client scenarios

The editor and IDE story is still the most mature, documented, and directly
validated.

## Best Framing For `brain`

The right stance is:

- ACP is already a strong editor/IDE interoperability layer
- ACP is a plausible cross-client abstraction candidate
- `brain` should design toward ACP compatibility where it fits naturally
- `brain` should still keep `BrainServer` and other native surfaces because
  those remain valuable even if ACP expands further

## Practical Implication

For `brain` roadmap thinking:

- use ACP to unlock editor clients now
- use a terminal-native ACP client as the fastest manual validation loop for
  the mock `agent acp` surface
- watch Streamable HTTP and connector patterns closely
- avoid building `brain` internals so tightly around editor-only assumptions
  that broader ACP clients become awkward later

## Key Sources

- Official clients page: <https://agentclientprotocol.com/get-started/clients>
- OpenClaw ACP bridge docs: [`docs.acp.md`](../../../repocache/openclaw/openclaw/docs.acp.md)
