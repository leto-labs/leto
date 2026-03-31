# Zed

## Summary

Zed is still the reference ACP client, but the important 2026 shift is that it
is no longer only a custom-config playground. Zed now treats the ACP Registry
as the preferred install path for external agents, while still supporting
manual `agent_servers` configuration and managed adapters for key agents.

For `brain`, Zed is the clearest proof that ACP can replace a large amount of
custom UI work if the runtime is strong enough.

## Confirmed Behavior

The official Zed external agents docs show all of the following:

- Zed supports external agents through ACP
- Zed can install some agents on demand and manage versions for the user
- registry installs are now the preferred path starting in `v0.221.x`
- manual `agent_servers` configuration is still supported
- registry agents can still accept custom environment variables
- Zed exposes ACP logs for debugging via a dedicated command

Zed also documents agent-specific setup for Claude Agent, Codex, and Gemini,
which is useful because it shows what a first-class ACP client expects from
serious agents:

- authentication outside the editor or through agent-specific methods
- stable commands and arguments
- clean environment variable integration
- support for slash commands and related workflow features when the agent can
  provide them

## Configuration Shape

Zed's documented manual ACP config looks like:

```json
{
  "agent_servers": {
    "My Agent": {
      "type": "custom",
      "command": "node",
      "args": ["~/projects/agent/index.js", "--acp"],
      "env": {}
    }
  }
}
```

This is important for `brain` because it means the Zed story can be good very
early if `brain` has:

- one stable executable
- one stable ACP mode
- minimal required environment

## Managed Agent Expectations

Zed's docs for Codex and Claude Agent show a recurring pattern:

- Zed can manage the ACP adapter installation itself
- Zed keeps that install updated
- the editor still lets the user influence auth and env where necessary

That means an eventual registry-ready `brain` can fit a product path where:

- a user installs `brain` from the registry
- Zed fetches and updates the ACP launcher
- `brain` handles its own auth and runtime behavior

## Limits And Caveats

- Zed says access to MCP servers installed from Zed can vary by ACP
  implementation.
- Zed's relationship with some agents is not purely ACP; for example, its docs
  note additional integration details for Claude Agent.
- Registry support is preferred, but not every external agent will get equal
  depth of integration.

That means ACP gets `brain` into Zed quickly, but some product polish will still
depend on how well `brain` maps its runtime onto the client features Zed already
supports.

## What This Means For `brain`

- Zed is the best near-term ACP target for `brain`.
- A well-packaged `agent acp` mode would immediately unlock a serious editor
  experience without building a Zed extension.
- If `brain` later wants top-tier Zed polish, the work should happen after ACP
  works well, not before.

## Key Sources

- Zed external agents docs: <https://zed.dev/docs/ai/external-agents>
- Zed registry announcement: <https://zed.dev/blog/acp-registry>
