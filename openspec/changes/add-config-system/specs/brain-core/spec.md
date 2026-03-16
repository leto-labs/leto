# brain-core Delta Spec

## MODIFIED Requirements

### Requirement: Brain Construction Flow
The Brain construction flow SHALL be decoupled from `Project`. The Brain engine SHALL hold shared infrastructure (provider, store, loop, tools). Project context SHALL be passed at the call site (e.g. `brain.run(&project, &transport)`).

The construction flow varies by platform but converges on the same types:

**CLI/TUI:**
```rust
let config = resolve_fs_config(&cwd)?;
let project = store.project_create(Project::new(Some("my-project"), Some(cwd), config)).await?;
let brain = Brain::new(provider, store, agent_loop, tools);
brain.run(&project, &transport).await?;
```

**Web:**
```rust
let config = fetch_config_from_api(project_id).await?;
let project = store.project_create(Project::new(Some("web-project"), None, config)).await?;
let brain = Brain::new(provider, store, agent_loop, tools);
brain.run(&project, &transport).await?;
```

**Test:**
```rust
let project = store.project_create(Project::with_defaults("test")).await?;
let brain = Brain::new(provider, store, agent_loop, tools);
brain.run(&project, &transport).await?;
```

#### Scenario: Cross-platform parity
- **WHEN** a CLI starts with a filesystem-resolved config and a web server starts with an API-fetched config, and both configs are equivalent
- **THEN** passing those projects to the same Brain SHALL produce identical behavior

#### Scenario: Single Brain serves multiple projects
- **WHEN** multiple projects are created with different configs
- **THEN** a single Brain instance SHALL be able to `run()` or `turn()` with any of them


## MODIFIED Requirements

### Requirement: Brain Construction Flow
The Brain construction flow SHALL be decoupled from `Project`. The Brain engine SHALL hold shared infrastructure (provider, store, loop, tools). Project context SHALL be passed at the call site (e.g. `brain.run(&project, &transport)`).

The construction flow varies by platform but converges on the same types:

**CLI/TUI:**
```rust
let config = resolve_fs_config(&cwd)?;
let project = store.project_create(Project::new(Some("my-project"), Some(cwd), config)).await?;
let brain = Brain::new(provider, store, agent_loop, tools);
brain.run(&project, &transport).await?;
```

**Web:**
```rust
let config = fetch_config_from_api(project_id).await?;
let project = store.project_create(Project::new(Some("web-project"), None, config)).await?;
let brain = Brain::new(provider, store, agent_loop, tools);
brain.run(&project, &transport).await?;
```

**Test:**
```rust
let project = store.project_create(Project::with_defaults("test")).await?;
let brain = Brain::new(provider, store, agent_loop, tools);
brain.run(&project, &transport).await?;
```

#### Scenario: Cross-platform parity
- **WHEN** a CLI starts with a filesystem-resolved config and a web server starts with an API-fetched config, and both configs are equivalent
- **THEN** passing those projects to the same Brain SHALL produce identical behavior

#### Scenario: Single Brain serves multiple projects
- **WHEN** multiple projects are created with different configs
- **THEN** a single Brain instance SHALL be able to `run()` or `turn()` with any of them
