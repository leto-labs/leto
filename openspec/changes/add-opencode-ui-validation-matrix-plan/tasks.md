# Tasks: add-opencode-ui-validation-matrix-plan

## Planning

- [ ] Confirm the planning scope against the pinned OpenCode `v1.3.2` app and
      SDK sources in repocache
- [ ] Record that this change is planning-only and does not implement
      automation
- [ ] Record that the primary target is pinned OpenCode web UI compatibility,
      not full SDK parity
- [ ] Define the matrix format to map UI entrypoints to SDK, compat routes,
      events, and visible outcomes

## Interaction Inventory

- [ ] Inventory connection and bootstrap interactions
- [ ] Inventory provider, auth, and model-management interactions
- [ ] Inventory project, workspace, and worktree interactions
- [ ] Inventory session, chat, prompt, and attachment interactions
- [ ] Inventory review, file, search, and diff interactions
- [ ] Inventory permission, question, and todo interactions
- [ ] Inventory terminal, PTY, MCP, and admin interactions
- [ ] Record provider credential mutation, disconnect, and reconnect flows that
      depend on `auth.set`, `auth.remove`, config updates, or instance disposal
- [ ] Record workspace reset and delete flows that depend on dirty-state
      checks, instance disposal, worktree reset, terminal cleanup, and
      archived-session UX
- [ ] Inventory rendering, recovery, and error-state interactions

## Repocache Deepening

- [ ] Perform a deeper source pass through additional OpenCode runtime and
      server modules beyond the first app-focused scan
- [ ] Reconcile app-visible interactions with the browser-client and generated
      v2 SDK inventory
- [ ] Reconcile the matrix with OpenCode runtime and server modules that may
      expose additional UI-relevant semantics
- [ ] Extend the matrix if the deeper repocache pass reveals interactions or
      state transitions that were missed initially
- [ ] Mark any non-web surface discovered during the deeper pass as secondary
      unless it changes browser-visible behavior

## Classification

- [ ] Define the later must-pass critical smoke set
- [ ] Define the later full regression categories
- [ ] Record which interactions are source-confirmed now versus still pending
      deeper repocache confirmation

## Spec

- [ ] Add `repo-tooling` delta requirements for a maintained OpenCode UI
      validation matrix
- [ ] Record that the matrix must stay aligned with the pinned OpenCode release
      and submodule consumer

## Verification

- [ ] Validate the change with
      `openspec validate add-opencode-ui-validation-matrix-plan --strict`
