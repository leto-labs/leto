# Proposal: Bound Tool Output And Isolate ACP Stdio

## Why

Testing `brain-acp` through Nori exposed two backend issues:

1. oversized tool output can still amplify prompt size enough to trigger provider
   request-size failures
2. ACP stdio can be polluted by backend logging, which causes protocol parse
   failures and hides the real provider error from the client

Codex/OpenCode-style agent backends solve this by bounding tool output at the
tool and history layers, and by keeping protocol transport separate from human
logging.

## What Changes

- add bounded tool-result retention in the loop path before tool output is fed
  back to the provider and stored in session history
- add tool-specific output caps and continuation hints for file, search,
  directory, and shell tools
- force ACP tracing/logging to stderr so stdout remains protocol-only
- add config support for a global tool-output byte budget

## Expected Outcome

- large tool outputs no longer expand the next provider turn without bounds
- `file_read`, `glob_search`, `grep`, `list_directory`, and `shell` encourage
  incremental inspection rather than giant single responses
- ACP clients receive parseable protocol failures instead of stdout corruption
