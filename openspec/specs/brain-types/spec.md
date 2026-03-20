# brain-types Specification

## Purpose
Core abstraction for LLM inference. Defines the `Provider` trait, `ChatStream` type, and `MockProvider` for testing.
## Requirements
### Requirement: Provider Trait
The system SHALL define a `Provider` trait that abstracts LLM inference behind a single async method. Implementations MUST accept a slice of messages and tool definitions, and return a stream of response chunks.

#### Scenario: Mock provider echoes input
- **WHEN** a Provider receives messages with the last user message being "hello world"
- **THEN** it SHALL return a stream of token chunks containing "hello world"
- **AND** the stream SHALL terminate with a final chunk

#### Scenario: Provider returns tool calls
- **WHEN** a Provider receives messages and the model decides to call a tool
- **THEN** the stream SHALL include a chunk with tool call information (name + arguments)

### Requirement: ChatStream Type
The system SHALL define a `ChatStream` type as a pinned boxed stream of `Result<ChatChunk, BrainError>`. Each `ChatChunk` SHALL carry either a text delta, a tool call, usage stats, or a done signal.

#### Scenario: Stream yields text tokens
- **WHEN** the provider streams a text response
- **THEN** the stream SHALL yield one or more `ChatChunk::Delta` items containing text fragments

#### Scenario: Stream signals completion
- **WHEN** the provider finishes its response
- **THEN** the stream SHALL yield a `ChatChunk::Done` item

### Requirement: MockProvider
The system SHALL include a `MockProvider` that requires no API key, no network, and no model files. It SHALL echo the last user message as space-delimited token chunks with an optional configurable delay.

#### Scenario: Deterministic echo
- **WHEN** MockProvider receives messages ending with "hello world"
- **THEN** it SHALL stream tokens: "hello", " ", "world"

#### Scenario: Simulated tool call
- **WHEN** MockProvider receives a message starting with "tool:"
- **THEN** it SHALL return a tool call chunk for the "echo" tool

### Requirement: BrainRuntime Trait
The system SHALL define a `BrainRuntime` trait in `brain-types` that represents
the shared runtime boundary for app surfaces.

The initial trait SHALL include:

- project resolution by root
- runtime bus subscription
- turn execution
- turn cancellation
- effective inference lookup for a session
- effective agent config lookup for a session
- model listing and model selection helpers
- provider registry access
- tool registry access
- loop registry access

The trait SHALL expose the underlying `Store` directly and SHALL keep plain
project/session/message CRUD on that store rather than mirroring it as runtime methods.

#### Scenario: Runtime trait is available from the crate root
- **WHEN** a caller imports `brain_types::BrainRuntime`
- **THEN** the trait SHALL resolve from the crate root

#### Scenario: Runtime trait is available from a dedicated module
- **WHEN** a caller imports `brain_types::runtime::BrainRuntime`
- **THEN** the trait SHALL resolve from the dedicated module

#### Scenario: Runtime trait exposes turn execution
- **WHEN** a runtime implements `BrainRuntime`
- **THEN** callers SHALL be able to start a turn with `turn(session_id, input, cancel)`
- **AND** receive an `EventStream`

#### Scenario: Runtime trait exposes a live runtime bus
- **WHEN** a runtime implements `BrainRuntime`
- **THEN** callers SHALL be able to subscribe to a live runtime bus stream
- **AND** receive protocol-neutral runtime bus events

#### Scenario: Runtime trait exposes provider and tool metadata
- **WHEN** a runtime implements `BrainRuntime`
- **THEN** callers SHALL be able to access provider, tool, and loop registries
- **AND** inspect provider metadata and tool definitions from the returned registry items

#### Scenario: Runtime exposes store directly
- **WHEN** a runtime implements `BrainRuntime`
- **THEN** callers SHALL be able to access the underlying `Store` via `store()`
- **AND** use that store for project, session, message, and credential operations

#### Scenario: Runtime exposes model and runtime helper methods
- **WHEN** a runtime implements `BrainRuntime`
- **THEN** callers SHALL be able to use model and runtime-specific helper methods directly from the trait
- **AND** use `store()` for plain CRUD

### Requirement: BrainRuntime Remains Distinct From Store
The `BrainRuntime` trait SHALL NOT inherit from `Store` directly, even when it
provides higher-level helper methods that delegate to the underlying store.

#### Scenario: Runtime is not itself a Store
- **WHEN** a caller implements `BrainRuntime`
- **THEN** it SHALL not be required to declare `impl Store` for the runtime type

#### Scenario: Raw CRUD remains available through store
- **WHEN** a caller needs low-level project, session, message, or credential CRUD
- **THEN** it SHALL still be able to use `runtime.store()`

### Requirement: Store Lifecycle Events
The system SHALL allow `Store` implementations to emit live project and session
lifecycle events so direct store mutations remain observable.

The initial store event surface SHALL include:

- project created
- project updated
- project deleted
- session created
- session updated
- session deleted

It SHALL be live-only.

#### Scenario: Store exposes a live event subscription
- **WHEN** a caller uses a `Store`
- **THEN** it SHALL be able to subscribe to a live store event stream

#### Scenario: Store event type is available from the crate root
- **WHEN** a caller imports `brain_types::StoreEvent`
- **THEN** the type SHALL resolve from the crate root

#### Scenario: Direct store mutation remains observable
- **WHEN** a caller mutates project or session state directly through `store()`
- **THEN** the store SHALL emit the corresponding lifecycle event

### Requirement: Generic CrudStore Trait
The system SHALL define a generic `CrudStore` trait in `brain-types` for
record-oriented stores.

The initial trait SHALL expose:

- `create(key, record)`
- `create_many(entries)`
- `get(key)`
- `get_many(keys)`
- `list()`
- `update(key, record)`
- `update_many(entries)`
- `delete(key)`
- `delete_many(keys)`
- `subscribe()`

`create()` SHALL fail when the key already exists.
`update()` SHALL fail when the key does not already exist.
The trait SHALL NOT include `upsert()` in the initial version.
The batch helper methods SHALL be default trait methods.
The batch helper methods SHALL be fail-fast.

#### Scenario: CrudStore is available from the crate root
- **WHEN** a caller imports `brain_types::CrudStore`
- **THEN** the trait SHALL resolve from the crate root

#### Scenario: CrudStore is available from the store module
- **WHEN** a caller imports `brain_types::store::CrudStore`
- **THEN** the trait SHALL resolve from the store module

#### Scenario: Batch CRUD helpers are available
- **WHEN** a caller uses a `CrudStore`
- **THEN** it SHALL be able to call `create_many`, `get_many`, `update_many`, and `delete_many`
- **AND** those methods SHALL process keyed entries or keys in input order

#### Scenario: Batch CRUD helpers fail fast
- **WHEN** a batch helper encounters an error while processing items
- **THEN** it SHALL return that error immediately
- **AND** SHALL not continue processing later items

### Requirement: Composed Store Accessors
The system SHALL define `Store` as a composed container of sub-stores rather
than as a supertrait over all store domains.

The composed store SHALL expose:

- `projects()`
- `sessions()`
- `messages()`
- `credentials()`
- `subscribe()`

#### Scenario: Store exposes project and session sub-stores
- **WHEN** a caller has `&dyn Store`
- **THEN** it SHALL be able to access `ProjectStore` via `projects()`
- **AND** `SessionStore` via `sessions()`

#### Scenario: Store exposes message and credential sub-stores
- **WHEN** a caller has `&dyn Store`
- **THEN** it SHALL be able to access `MessageStore` via `messages()`
- **AND** `CredentialStore` via `credentials()`

### Requirement: Domain Stores Specialize CrudStore
The system SHALL define all four domain stores as specializations of the
generic `CrudStore`.

`ProjectStore` SHALL use `ProjectId` as its key and SHALL add `find_by_root(root)`.
`SessionStore` SHALL use `Ulid` as its key and SHALL add `list_for_project(project_id)`.
`MessageStore` SHALL use `(session_id, message_id)` as its key and SHALL add `list_for_session(session_id)`.
`CredentialStore` SHALL use `(provider_name, credential_id)` as its key and SHALL add provider-scoped helpers and health mutation.

#### Scenario: ProjectStore extends CrudStore
- **WHEN** a caller uses `ProjectStore`
- **THEN** it SHALL expose generic CRUD operations for `Project`
- **AND** a project-specific root lookup

#### Scenario: SessionStore extends CrudStore
- **WHEN** a caller uses `SessionStore`
- **THEN** it SHALL expose generic CRUD operations for `Session`
- **AND** session-specific listing by project

#### Scenario: MessageStore extends CrudStore
- **WHEN** a caller uses `MessageStore`
- **THEN** it SHALL expose generic CRUD operations for `Message`
- **AND** a session-scoped message listing helper

#### Scenario: CredentialStore extends CrudStore
- **WHEN** a caller uses `CredentialStore`
- **THEN** it SHALL expose generic CRUD operations for `CredentialEntry`
- **AND** provider-scoped listing and health update helpers

### Requirement: Model Helper Methods Live On Runtime
The system SHALL allow `BrainRuntime` to expose model-oriented helper methods
because those depend on provider registry state rather than raw store CRUD.

These helper methods SHALL include:

- list available models
- get the current model for a session
- set a session model by model ID
- get the effective thought level for a session
- set a session thought level by value

#### Scenario: Plain CRUD remains on store
- **WHEN** a caller needs session or message CRUD
- **THEN** it SHALL use `runtime.store()`
- **AND** the runtime trait SHALL not mirror those methods

#### Scenario: Model helper resolves and persists model selection
- **WHEN** a caller sets a session model by model ID
- **THEN** the runtime SHALL resolve that model to a unique provider
- **AND** persist the resulting provider/model inference pair on the session

#### Scenario: Runtime persists session thought level override
- **WHEN** a caller sets a session thought level by value
- **THEN** the runtime SHALL persist that value in the session inference override layer
- **AND** the value SHALL participate in effective session inference resolution

### Requirement: Runtime Bus Events
The system SHALL define a protocol-neutral runtime bus event type in `brain-types`
for shared live runtime observation.

The runtime bus SHALL be distinct from the per-turn `Event` stream.
It SHALL support live-only subscription.

The initial runtime bus event type SHALL include:

- store-composed project lifecycle events
- store-composed session lifecycle events
- duplexed turn events that wrap the existing turn `Event` with session context

#### Scenario: Runtime bus event is available from the crate root
- **WHEN** a caller imports `brain_types::RuntimeBusEvent`
- **THEN** the type SHALL resolve from the crate root

#### Scenario: Runtime bus stream is available from the crate root
- **WHEN** a caller imports `brain_types::RuntimeBusStream`
- **THEN** the type SHALL resolve from the crate root

#### Scenario: Turn events remain separate from the bus event type
- **WHEN** a caller uses `turn()`
- **THEN** it SHALL still receive the existing per-turn `EventStream`
- **AND** the runtime bus SHALL remain a separate subscription surface

### Requirement: Registry Module
The system SHALL define a dedicated `registry` module in `brain-types` for
registry contracts shared across runtime domains.

#### Scenario: Registry module is available
- **WHEN** a caller imports `brain_types::registry`
- **THEN** the module SHALL resolve and expose the registry traits

### Requirement: Generic Registry Trait
The system SHALL define a generic `Registry` trait in `brain-types::registry`
for managing named live runtime instances.

The initial trait SHALL expose collection-style operations:

- `get(name)`
- `list()`
- `set(name, item)`
- `remove(name)`

`set()` SHALL return the previous entry when replacing an existing item.
`list()` SHALL return the live named runtime items, rather than derived metadata.

#### Scenario: Generic registry is available from the crate root
- **WHEN** a caller imports `brain_types::Registry`
- **THEN** the trait SHALL resolve from the crate root

#### Scenario: Generic registry is available from registry module
- **WHEN** a caller imports `brain_types::registry::Registry`
- **THEN** the trait SHALL resolve from the registry module

### Requirement: RegistryProvider Trait
The system SHALL define a `RegistryProvider` trait in `brain-types::registry`
for managing live provider instances by name.

The initial trait SHALL expose collection-style operations:

- `get(name)`
- `list()`
- `set(name, provider)`
- `remove(name)`

`set()` SHALL return the previous provider when replacing an existing entry.
`list()` SHALL return the provider registry key alongside the live provider instance.

#### Scenario: Provider registry is available from the crate root
- **WHEN** a caller imports `brain_types::RegistryProvider`
- **THEN** the trait SHALL resolve from the crate root

#### Scenario: Provider registry is available from registry module
- **WHEN** a caller imports `brain_types::registry::RegistryProvider`
- **THEN** the trait SHALL resolve from the registry module

#### Scenario: Setting a new provider returns no previous value
- **WHEN** a provider is inserted under a previously unused name
- **THEN** `set()` SHALL return `None`

#### Scenario: Setting an existing provider name returns the previous value
- **WHEN** a provider is inserted under a name that already exists
- **THEN** `set()` SHALL replace the existing entry
- **AND** return the previous provider

#### Scenario: Listing providers returns named live entries
- **WHEN** a caller lists the provider registry
- **THEN** it SHALL receive the provider registry key and the live provider instance
- **AND** derive `ProviderInfo` by calling methods on the returned provider instance

### Requirement: RegistryTool Trait
The system SHALL define a `RegistryTool` trait in `brain-types::registry`
for managing live tool instances by name.

The initial trait SHALL expose collection-style operations:

- `get(name)`
- `list()`
- `set(name, tool)`
- `remove(name)`

`set()` SHALL return the previous tool when replacing an existing entry.
`list()` SHALL return the tool registry key alongside the live tool instance.

#### Scenario: Tool registry is available from the crate root
- **WHEN** a caller imports `brain_types::RegistryTool`
- **THEN** the trait SHALL resolve from the crate root

#### Scenario: Tool registry is available from registry module
- **WHEN** a caller imports `brain_types::registry::RegistryTool`
- **THEN** the trait SHALL resolve from the registry module

#### Scenario: Setting a new tool returns no previous value
- **WHEN** a tool is inserted under a previously unused name
- **THEN** `set()` SHALL return `None`

#### Scenario: Setting an existing tool name returns the previous value
- **WHEN** a tool is inserted under a name that already exists
- **THEN** `set()` SHALL replace the existing entry
- **AND** return the previous tool

#### Scenario: Listing tools returns named live entries
- **WHEN** a caller lists the tool registry
- **THEN** it SHALL receive the tool registry key and the live tool instance
- **AND** derive `ToolDef` by calling methods on the returned tool instance

### Requirement: RegistryHashMap Implementation
The system SHALL define a reusable `RegistryHashMap` implementation in
`brain-types::registry` for storing named runtime instances in memory.

The module SHALL also expose provider, tool, and loop aliases over that implementation.

#### Scenario: Generic registry hashmap is available from the crate root
- **WHEN** a caller imports `brain_types::RegistryHashMap`
- **THEN** the type SHALL resolve from the crate root

#### Scenario: Specialized registry hashmap aliases are available
- **WHEN** a caller imports `brain_types::RegistryProviderHashMap`, `brain_types::RegistryToolHashMap`, or `brain_types::RegistryLoopHashMap`
- **THEN** those aliases SHALL resolve from the crate root

### Requirement: RegistryLoop Trait
The system SHALL define a `RegistryLoop` trait in `brain-types::registry`
for managing live agent loop instances by name.

The initial trait SHALL expose collection-style operations:

- `get(name)`
- `list()`
- `set(name, loop)`
- `remove(name)`

`set()` SHALL return the previous loop when replacing an existing entry.
`list()` SHALL return the loop registry key alongside the live loop instance.

#### Scenario: Loop registry is available from the crate root
- **WHEN** a caller imports `brain_types::RegistryLoop`
- **THEN** the trait SHALL resolve from the crate root

#### Scenario: Loop registry is available from registry module
- **WHEN** a caller imports `brain_types::registry::RegistryLoop`
- **THEN** the trait SHALL resolve from the registry module

#### Scenario: Setting a new loop returns no previous value
- **WHEN** a loop is inserted under a previously unused name
- **THEN** `set()` SHALL return `None`

#### Scenario: Setting an existing loop name returns the previous value
- **WHEN** a loop is inserted under a name that already exists
- **THEN** `set()` SHALL replace the existing entry
- **AND** return the previous loop

### Requirement: Loop Selection Lives In Shared Config
The system SHALL carry loop selection in the shared runtime config types so
project defaults and session overrides can select a registered loop.

The project-level default SHALL live on `AgentConfig`.
The session-level override SHALL live on `Session` and `SessionUpdate`.

#### Scenario: Agent config can declare a default loop
- **WHEN** an `AgentConfig` is serialized or deserialized
- **THEN** it SHALL preserve an optional loop name

#### Scenario: Session can override and clear loop selection
- **WHEN** a caller updates a session loop override through `SessionUpdate`
- **THEN** the override SHALL be set or cleared independently of inference overrides

### Requirement: Runtime Supports Session Loop Mutation

The shared `BrainRuntime` surface SHALL provide a helper for updating the
session loop override.

#### Scenario: Runtime helper updates the session loop override
- **WHEN** a caller invokes `set_session_loop(session_id, loop_name)` with a registered loop
- **THEN** the runtime SHALL persist that loop override on the session

#### Scenario: Runtime helper rejects unknown loops
- **WHEN** a caller invokes `set_session_loop(session_id, loop_name)` with an unknown loop
- **THEN** the runtime SHALL return an error

