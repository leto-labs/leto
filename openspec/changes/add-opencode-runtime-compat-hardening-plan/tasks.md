# Tasks: add-opencode-runtime-compat-hardening-plan

## Planning

- [ ] Confirm the planning scope against the pinned OpenCode `v1.3.2` sources
      in `repocache/anomalyco/opencode`
- [ ] Record that this change is planning-only and does not implement compat
      behavior
- [ ] Record that the primary target is pinned OpenCode web UI compatibility,
      not full SDK or TUI parity
- [ ] Inventory the OpenCode browser-client route families relevant to
      `agent-server` compatibility
- [ ] Reconcile the raw generated SDK inventory with the higher-level
      `v2/client.ts` wrapper so wrapper-exposed namespaces are not missed
- [ ] Inventory the OpenCode-generated event union relevant to the web UI and
      browser reducers

## Repocache Deepening

- [ ] Perform a deeper source pass through additional OpenCode runtime and
      server modules beyond the initial web-app scan
- [ ] Reconcile the web app interaction inventory with
      `packages/opencode/src/server/routes/*`
- [ ] Reconcile the current checklist with OpenCode runtime modules for
      sessions, projects, providers, permissions, questions, PTY, MCP,
-      workspace, and shared runtime behavior
- [ ] Reconcile top-level server routes outside `server/routes/*.ts`, including
      auth and instance-disposal surfaces
- [ ] Amend the later implementation checklist if new compat expectations are
      discovered during that deeper repocache pass
- [ ] Explicitly mark any non-web or TUI-only surface as secondary unless it
      changes browser-visible behavior

## Runtime Scope

- [ ] Define behavior-level requirements for bootstrap and global compat routes
- [ ] Define behavior-level requirements for OpenCode-native SSE event
      semantics
- [ ] Record that the next execution change must implement the `agent-server`
      compat server layer itself under `/v1/compat/opencode`
- [ ] Define behavior-level requirements for `config.providers`, `auth.set`,
      `auth.remove`, and reconnect or reload behavior after credential changes
- [ ] Define behavior-level requirements for session lifecycle, prompt, diff,
      todo, summarize, revert, unrevert, share, and fork behavior
- [ ] Define behavior-level requirements for project, workspace, and worktree
      behavior
- [ ] Define behavior-level requirements for provider, auth, config, and model
      flows
- [ ] Define behavior-level requirements for permission and question flows
- [ ] Define behavior-level requirements for file, search, review, and VCS
      behavior
- [ ] Define behavior-level requirements for PTY, MCP, and browser-relevant
      admin surfaces
- [ ] Define behavior-level requirements for experimental tool, resource, and
      global-session inventory routes only where they affect the web UI
- [ ] Define behavior-level requirements for permission route compatibility
      where pinned consumers still use both `permission.reply` and
      `permission.respond`

## Spec

- [ ] Add `agent-server` delta requirements for behaviorally real OpenCode
      compatibility
- [ ] Add canonical wording that future hardening work must implement missing
      behavior in the compat server layer rather than in the UI consumer
- [ ] Record that later implementation must reconcile against pinned OpenCode
      source, not only the OpenAPI artifact
- [ ] Record that later implementation may treat non-web pinned surfaces as
      secondary unless they are required for browser compatibility

## Verification

- [ ] Validate the change with
      `openspec validate add-opencode-runtime-compat-hardening-plan --strict`
