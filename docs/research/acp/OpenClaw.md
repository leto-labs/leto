# OpenClaw ACP Bridge

## Summary

OpenClaw is the clearest current example of ACP used as a bridge into a broader
runtime and gateway ecosystem rather than only an editor-local subprocess tool.

Its ACP docs are unusually explicit about what is and is not ACP-native, which
makes OpenClaw a strong reference for `brain`.

## The Important Part: It Admits The Bridge

The docs say plainly:

> `openclaw acp` is a Gateway-backed ACP bridge, not a full ACP-native editor
> runtime.

That clarity is useful. It means OpenClaw is intentionally translating between:

- ACP session semantics on one side
- OpenClaw Gateway session/chat semantics on the other

## Confirmed Capability Profile

The documented compatibility matrix says:

- `initialize`, `newSession`, `prompt`, and `cancel` are implemented
- `listSessions` and slash commands are implemented
- `loadSession` is partial
- session modes are partial
- session info and usage updates are partial
- tool streaming is partial
- per-session MCP servers are unsupported
- client filesystem methods are unsupported
- client terminal methods are unsupported
- session plans and thought streaming are unsupported in the documented bridge

This is an excellent case study in ACP tradeoffs:

- a bridge can get a lot of value quickly
- but a bridge often leaves client-owned ACP features on the table

## Why It Still Matters

OpenClaw also proves something important that goes beyond editors:

- ACP can be routed through a gateway
- ACP sessions can map onto broader runtime session keys
- ACP can participate in multi-channel systems such as Discord and Telegram
- ACP can be a runtime ingress, not only an IDE feature

That is strong support for your experimental intuition that ACP could matter
outside code editors too.

## The Main Lesson For `brain`

OpenClaw suggests two useful rules.

### 1. Bridging is valid

It is completely reasonable to map ACP sessions onto a different internal
session system and expose a compatible surface.

### 2. But bridge-only design leaves value on the floor

OpenClaw explicitly does not use ACP client filesystem or terminal methods in
this bridge mode, and that is exactly where editor ACP clients can add the most
UX value.

So for `brain`, the best path is likely:

- keep `BrainServer` and other runtime boundaries
- implement ACP as a first-class surface over the same runtime
- avoid reducing ACP to "chat over stdio"

## What This Means For Non-Editor Clients

OpenClaw is one of the best pieces of evidence that ACP can matter beyond a
single IDE window. Its surrounding docs and code show ACP being routed into
gateway-managed sessions and broader channel integrations.

That does not prove ACP should be the only abstraction for all future `brain`
clients. But it does prove that ACP is flexible enough to be more than an
editor-only curiosity.

## Key Sources

- OpenClaw ACP bridge docs: [`docs.acp.md`](../../../repocache/openclaw/openclaw/docs.acp.md)
- ACP translator implementation: [`src/acp/translator.ts`](../../../repocache/openclaw/openclaw/src/acp/translator.ts)
- ACP server startup: [`src/acp/server.ts`](../../../repocache/openclaw/openclaw/src/acp/server.ts)
- ACP runtime controls: [`src/acp/runtime/types.ts`](../../../repocache/openclaw/openclaw/src/acp/runtime/types.ts)
