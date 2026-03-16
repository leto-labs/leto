# brain-core Delta Spec

## MODIFIED Requirements

### Requirement: Brain Run Loop
`Brain.run(&project, &transport)` SHALL remain functional but is no longer the primary entry point for interactive use. The preferred pattern for interactive clients is:

1. Construct a `BrainServer` from a `Brain` — `BrainServer::new(brain)`
2. Create projects and sessions via the `BrainApi` trait
3. Use `send_message()` or `send_message_stream()` for turns

`BrainServer` supports multiple projects. Project management (create, list, get, update, delete) is handled through `BrainApi`. `send_message()` dynamically looks up the session's project to obtain `AgentConfig`.

`Brain.run(&project, &transport)` remains useful for simple scripting, testing, and non-interactive use cases where the server/client split adds unnecessary complexity.

#### Scenario: Brain.run still works
- **WHEN** `Brain.run(&project, &transport)` is called
- **THEN** it SHALL behave exactly as before (create session scoped to project, loop recv/turn/send)

#### Scenario: Server is preferred for interactive use
- **WHEN** building an interactive client (TUI, IDE extension)
- **THEN** the preferred pattern is `BrainServer::new(brain)` + `BrainApi` client

### Requirement: Provider Info
The `Provider` trait SHALL include an `info(&self) -> ProviderInfo` method alongside `chat()`. All provider implementations (OpenAiProvider, MistralRsProvider, LlamaCppProvider, MockProvider, ProviderRouter) SHALL implement `info()`.

#### Scenario: Provider reports metadata
- **WHEN** `provider.info()` is called
- **THEN** it SHALL return `ProviderInfo { name, default_model, models }`


## MODIFIED Requirements

### Requirement: Brain Run Loop
`Brain.run(&project, &transport)` SHALL remain functional but is no longer the primary entry point for interactive use. The preferred pattern for interactive clients is:

1. Construct a `BrainServer` from a `Brain` — `BrainServer::new(brain)`
2. Create projects and sessions via the `BrainApi` trait
3. Use `send_message()` or `send_message_stream()` for turns

`BrainServer` supports multiple projects. Project management (create, list, get, update, delete) is handled through `BrainApi`. `send_message()` dynamically looks up the session's project to obtain `AgentConfig`.

`Brain.run(&project, &transport)` remains useful for simple scripting, testing, and non-interactive use cases where the server/client split adds unnecessary complexity.

#### Scenario: Brain.run still works
- **WHEN** `Brain.run(&project, &transport)` is called
- **THEN** it SHALL behave exactly as before (create session scoped to project, loop recv/turn/send)

#### Scenario: Server is preferred for interactive use
- **WHEN** building an interactive client (TUI, IDE extension)
- **THEN** the preferred pattern is `BrainServer::new(brain)` + `BrainApi` client

### Requirement: Provider Info
The `Provider` trait SHALL include an `info(&self) -> ProviderInfo` method alongside `chat()`. All provider implementations (OpenAiProvider, MistralRsProvider, LlamaCppProvider, MockProvider, ProviderRouter) SHALL implement `info()`.

#### Scenario: Provider reports metadata
- **WHEN** `provider.info()` is called
- **THEN** it SHALL return `ProviderInfo { name, default_model, models }`
