## ADDED Requirements

### Requirement: Agent Runtime Manages Git Worktrees As First-Class Runtime Resources

The runtime SHALL manage git worktrees as explicit runtime resources with
stable identity, lifecycle, and queryable state.

#### Scenario: Create and inspect a managed worktree

- **WHEN** a caller creates a worktree through the runtime for a valid git
  repository
- **THEN** the runtime SHALL assign a stable worktree identifier
- **AND** retain repo-root, worktree-path, and branch metadata in runtime-owned
  state
- **AND** allow that worktree to be listed or fetched by id later

### Requirement: Agent Runtime Exposes Typed Worktree Lifecycle Operations

The runtime SHALL expose typed git worktree lifecycle operations rather than
hiding worktree management behind generic tool payloads or shell conventions.

#### Scenario: Bind and unbind a worktree explicitly

- **WHEN** a caller binds or unbinds a managed worktree through the runtime
- **THEN** the runtime SHALL update attachment state explicitly
- **AND** emit corresponding lifecycle events

#### Scenario: Removing a bound worktree fails safely

- **WHEN** a caller attempts to remove a worktree that is still bound to a
  runtime
- **THEN** the runtime SHALL reject that request with a typed error
- **AND** leave the worktree registry state intact

#### Scenario: Non-git targets fail without partial registration

- **WHEN** a caller requests worktree creation for a path that is not a valid
  git repository
- **THEN** the runtime SHALL fail the request with a typed error
- **AND** SHALL NOT register a partial worktree resource

### Requirement: Child Runtimes May Bind To Existing Worktrees

The runtime SHALL allow child runtimes to start bound to an existing managed
worktree.

#### Scenario: Spawn child with explicit worktree binding

- **WHEN** a parent spawns a child runtime with an existing worktree id
- **THEN** the child runtime SHALL start with that worktree bound
- **AND** the runtime SHALL preserve normal spawn semantics for history,
  reporting, and result flow
- **AND** SHALL NOT implicitly create a new worktree during that spawn

### Requirement: Bound Worktrees Define Default Local Execution Context

The runtime SHALL use a bound worktree as the default local execution directory
for runtime-owned local execution when no explicit `cwd` is provided.

#### Scenario: PTY open inherits bound worktree path

- **WHEN** a runtime bound to a managed worktree opens a PTY without an
  explicit `cwd`
- **THEN** the PTY SHALL start in the bound worktree path

#### Scenario: Explicit cwd overrides bound worktree default

- **WHEN** a caller opens a PTY with an explicit `cwd` while the runtime is
  bound to a worktree
- **THEN** the PTY SHALL use the explicit `cwd`
- **AND** the worktree binding SHALL remain unchanged

### Requirement: Agent Runtime Exposes Provider-Visible Native Worktree Tools

The runtime SHALL expose provider-visible native tools for the bounded managed
worktree lifecycle.

#### Scenario: Provider requests worktree management tools

- **WHEN** the provider asks for runtime-native worktree capabilities
- **THEN** the runtime SHALL expose focused native tools for create, list, get,
  remove, bind, and unbind worktree actions
- **AND** keep runtime orchestration as the source of truth beneath those tools
