# Proposal: Clean OpenCode compat contract and documentation architecture

## Why

The current OpenCode compat implementation reaches exact `/doc` parity, but the
internal architecture is poor:

- DTOs and handlers had accumulated around catch-all compat modules rather than
  route-family and domain-focused ownership
- legacy compat doc assembly was carrying more parity-shaping than was
  acceptable without clear justification
- shared runtime/doc contract cleanup was incomplete, which encouraged more
  route-local hacks when the contract evolved

This makes the compat layer harder to maintain and encourages more hacks when
the contract evolves.

## What

- organize compat DTOs into route-family and shared domain modules under
  `compat/opencode/types/`
- keep runtime handlers and generated docs using the same DTOs
- delete legacy compat doc assembly and keep only justified generator-format,
  transport-documentation, and parity-test helpers
- preserve exact prefix-aware OpenCode `/doc` parity throughout

## Impact

- Internal compat module organization changes substantially.
- No intended external API changes.
- `/v1/compat/opencode/doc` must continue to match `openapi/opencode.json`
  except for the `/v1/compat/opencode` prefix.
