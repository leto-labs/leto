# ARCHIVED SPEC

> [!NOTE] Historical archive snapshot. Use active canonical specs for current requirements.

## MODIFIED Requirements

### Requirement: AgentLoop Trait
The system SHALL define an `AgentLoop` trait with a single `run` method that accepts a Provider, a slice of Tools, a Vec of Messages, an AgentConfig, and a CancellationToken. It SHALL return a `Stream<Item = Event>`. Different implementations encode different agent strategies (simple tool loop, plan-then-act, multi-agent delegation, etc.).

The `AgentLoop` trait remains unchanged. However, callers SHOULD prefer invoking it through `Brain.turn()` rather than calling `run()` directly. `Brain.turn()` handles message history loading, user message construction, and post-turn persistence, whereas calling `AgentLoop.run()` directly requires the caller to manage all of this.

#### Scenario: Trait is object-safe and swappable
- **WHEN** a caller has a `&dyn AgentLoop`
- **THEN** it SHALL be able to call `run()` without knowing the concrete implementation

#### Scenario: Brain dispatches via AgentLoop
- **WHEN** `Brain.turn()` is called
- **THEN** it SHALL delegate to the configured `AgentLoop.run()` with the loaded message history, provider, tools, config, and cancellation token
