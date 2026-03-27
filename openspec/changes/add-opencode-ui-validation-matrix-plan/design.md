# Design: add-opencode-ui-validation-matrix-plan

## Decision: Keep This Change Planning-Only

This change defines the validation matrix and future automation expectations
only.

It does not implement:

- browser tests
- agent-browser workflows
- Playwright or other deterministic automation harnesses
- OpenCode UI patches
- compat runtime fixes

Those belong to later implementation changes.

## Decision: Use The Pinned OpenCode App And SDK As The Validation Source Of Truth

The validation matrix should be derived from the pinned OpenCode `v1.3.2`
source already available locally in repocache and the pinned
`submodules/opencode` checkout.

The primary source areas are:

- `packages/app/src` for user-visible interactions
- `packages/sdk/js/src/v2/client.ts` for the browser client wrapper
- `packages/sdk/js/src/v2/gen/sdk.gen.ts` for the callable client inventory
  and wrapper-exposed namespaces
- `packages/sdk/js/src/v2/gen/types.gen.ts` for browser-visible event
  inventory

This must be supplemented by a deeper repocache pass through runtime and server
modules so the matrix can capture interactions or state transitions that are
not obvious from the first browser-oriented scan alone.

The deeper pass already revealed matrix additions that should remain explicit:

- provider disconnect and custom-provider save are not just “provider toggles”;
  they rely on `auth.remove`, `auth.set`, config updates, and refresh behavior
- workspace reset is not just `worktree.reset`; the web app first disposes the
  instance, clears PTYs, enumerates sessions, and shows archived-session UX
- the permission reply/respond split is easy to miss if the matrix only
  follows the newest nominal route family

This planning change is intentionally web-first. TUI-only, installation-only,
or broader product-surface interactions are secondary and belong here only when
they explain browser-visible behavior.

## Decision: Record Interactions As End-To-End Contracts

Each future validation item should be described as an end-to-end contract with:

- UI entrypoint
- user action
- SDK method or methods
- compat route or routes
- required events
- expected state mutation
- expected visible UI outcome
- classification as critical smoke or full regression
- whether the interaction was confirmed in the first pass or discovered later
  during the deeper repocache pass

This keeps the matrix useful for both implementation of the compat server layer
and later automation.

## Decision: Separate Critical Smoke From Full Regression

Later validation should have two layers, both intended to drive and verify
implementation of the compat server layer.

### Critical Smoke

The smoke layer should prove the minimum viable OpenCode workflow works against
our server:

- connect to server
- bootstrap global state
- open a project
- create a session
- send a prompt
- observe live assistant updates
- handle at least one blocking interaction such as permission or question
- review session diff or no-diff state
- exercise one workspace flow
- exercise one provider connection flow
- exercise terminal or PTY connectivity

### Full Regression

The full matrix should cover the broader interaction set across providers,
projects, workspaces, sessions, files, review, PTY, MCP, and recovery flows.

## Decision: Inventory Interactions By UI Subsystem

The matrix should be organized by major OpenCode interaction families.

### Connection and Bootstrap

- home page load
- server picker open
- add, edit, remove, select, and default a server
- healthy and unhealthy server states
- reload and reconnect behavior
- global and directory bootstrap behavior

### Providers and Models

- popular provider connect flows
- API-key auth
- OAuth auth
- disconnect and reconnect
- custom provider creation and editing
- credential mutation through auth routes
- provider disable and enable
- manage-models and model-visibility flows
- model picker and model variant selection

### Projects and Workspaces

- open and reopen projects
- rename and edit projects
- initialize git
- create, rename, reorder, reset, and delete workspaces or worktrees
- pending, ready, and failed workspace states
- workspace reset and delete confirmation state, including dirty-state checks
  and archived-session outcomes
- route recovery across projects and workspaces

### Sessions and Chat

- create, open, archive, delete, and rename sessions
- fork and child-session navigation
- share and unshare
- prompt flows for text, slash commands, shell-mode, attachments, images, and
  context items
- queued follow-up and abort behavior
- model and agent selection around send

### Review, Files, and Search

- review panel visibility
- diff rendering
- unified and split diff toggles
- file opening from diffs
- file tree and file viewer behavior
- file, text, and symbol search
- dirty-file state effects in reset and delete UX

### Permissions, Questions, and Todo UX

- permission request rendering
- permission reply and auto-accept behavior
- compatibility between current reply flows and any deprecated respond flow
  still used by pinned consumers
- question rendering and reply or reject behavior
- todo dock open, update, and close behavior
- blocked-turn notifications and toasts

### Terminal, PTY, MCP, and Admin UX

- terminal panel open and PTY lifecycle
- additional PTY tabs, switching, reordering, reconnecting, and removing
- MCP list, connect, disconnect, and auth-adjacent state
- status popover behavior
- backend-driven status surfaces that must not break the UI

### Rendering and Recovery

- busy versus idle session rendering
- server restart recovery
- page reload recovery
- error-toast rendering
- empty states for projects, sessions, providers, review, MCP, and terminals

## Decision: Require A Deeper Repocache Pass Before Later Test Implementation

The first matrix draft should be treated as broad but not final.

Before later automation or runtime hardening work on the compat server layer
claims completeness,
contributors must perform a deeper repocache pass to compare:

- app-visible interactions
- browser-client and generated v2 SDK exposure
- OpenCode runtime and server modules that may reveal additional expectations
- shared runtime modules that may influence browser behavior beyond the first
  web pass

If that pass uncovers additional interactions, the matrix must be extended
before the corresponding later implementation work is considered complete.
