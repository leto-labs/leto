## ADDED Requirements

### Requirement: Agent Runtime Provides A Storeless Bidirectional Session Engine

The system SHALL provide an `agent-runtime` crate that owns reusable in-memory
session execution over the standalone `provider` crate.

The crate SHALL NOT depend on any `brain-*` crate.
The crate SHALL NOT require a persistence store to execute sessions.

#### Scenario: Session engine runs without a store

- **WHEN** a caller constructs a session engine with a provider, tool executor,
  loop strategy, runtime config, and initial transcript
- **THEN** it SHALL be able to execute a session entirely in memory

### Requirement: Agent Runtime Accepts Source-Aware Session Commands

The runtime SHALL expose a command surface for long-lived sessions rather than a
single per-turn entrypoint.

#### Scenario: Session accepts input and control commands

- **WHEN** a caller submits user input, control events, approval decisions, or
  agent spawn or routing commands
- **THEN** the runtime SHALL serialize them through one authoritative session
  lane
- **AND** preserve optional source metadata for multiplexing outer channels

#### Scenario: Agent commands use typed runtime actions

- **WHEN** a caller requests child spawn, wait, input delivery, interrupt,
  listing, or agent messaging
- **THEN** the runtime SHALL expose those operations through typed agent
  actions rather than requiring callers to construct raw routing envelopes

### Requirement: Agent Runtime Uses Provider Messages As The Canonical Transcript

The runtime SHALL use `provider::Message` as the canonical model-visible
transcript representation.

#### Scenario: Assistant and tool results update transcript

- **WHEN** a turn produces assistant output and tool results
- **THEN** the runtime SHALL commit those updates into the session transcript
  using the shared provider message/content model

### Requirement: Agent Runtime Exposes A Reusable Loop Strategy Surface

The runtime SHALL define a public loop strategy trait and a reusable
decision-oriented loop context for loop implementations.

#### Scenario: External loop crate implements the runtime trait

- **WHEN** a crate depends on `agent-runtime`
- **THEN** it SHALL be able to implement the public loop strategy trait without
  importing legacy loop/runtime crates

#### Scenario: Loop decides without owning execution plumbing

- **WHEN** a loop strategy runs on top of the runtime
- **THEN** it SHALL be able to request provider execution, tool execution,
  approval waiting, turn completion, child-runtime waiting, or child-runtime
  messaging without manually consuming provider streams or mutating the
  transcript vector

### Requirement: Agent Runtime Normalizes Provider Output Into Runtime Events

The runtime SHALL expose its own runtime event stream while preserving provider
block semantics, tool lifecycle events, transcript commits, turn lifecycle, and
session control state.

#### Scenario: Provider block events surface through runtime events

- **WHEN** the provider emits block start, delta, usage, or completion events
- **THEN** the runtime event stream SHALL expose corresponding normalized
  runtime events

#### Scenario: Boundaries and approvals surface through runtime events

- **WHEN** the runtime reaches a safe boundary, pauses for approval, applies
  steering, routes an envelope, or tracks a child runtime
- **THEN** the runtime SHALL expose corresponding control and lifecycle events

### Requirement: Agent Runtime Owns Tool Execution Lifecycle Helpers

The runtime SHALL define a tool executor trait and reusable helpers for
executing provider-requested tool calls and reinjecting their results into the
transcript.

#### Scenario: Runtime executes a tool call and commits the result

- **WHEN** a loop asks the runtime to execute a provider-requested tool call
- **THEN** the runtime SHALL emit tool lifecycle events
- **AND** append a tool-result transcript update for the next provider step

#### Scenario: Runtime exposes native agent tools to the provider

- **WHEN** the provider asks for agent-management tools
- **THEN** the runtime SHALL expose focused native tools for spawning,
  messaging, reading mail, interrupting, listing, and waiting on runtimes
- **AND** keep runtime orchestration as the source of truth beneath those tools

### Requirement: Agent Runtime Supports Safe-Boundary Steering And Interruption

The runtime SHALL support steering and interruption as explicit control
operations rather than requiring callers to rewrite transcript state directly.

#### Scenario: Steering queues until a safe boundary

- **WHEN** steering is submitted while provider or tool work is in progress
- **THEN** the runtime SHALL queue that steering input
- **AND** apply it only at a safe boundary

#### Scenario: Interrupt mode pauses at a boundary

- **WHEN** an interrupt targets the next safe boundary or the end of the current
  tool execution
- **THEN** the runtime SHALL pause the session at that boundary before
  continuing

### Requirement: Agent Runtime Supports Approval Gates And Child Sessions

The runtime SHALL provide first-class approval and child-session lifecycle
concepts so advanced loops do not need to invent parallel control channels.

#### Scenario: Runtime waits for approval before tool execution

- **WHEN** pending tool calls require approval
- **THEN** the runtime SHALL enter an approval boundary and resume only after an
  approval decision

#### Scenario: Runtime tracks child-session lifecycle

- **WHEN** a loop requests child-session work
- **THEN** the runtime SHALL expose child-session request and completion state
  through its public command, state, and event surfaces

### Requirement: Agent Runtime Uses A Flat Runtime Registry For Child Runtimes

The runtime SHALL keep live runtimes in a flat registry keyed by runtime id,
with parent-child structure represented by lineage metadata and registry
indexes rather than recursive runtime ownership.

#### Scenario: Parent lists direct child runtimes

- **WHEN** a parent runtime spawns one or more child runtimes
- **THEN** the runtime state SHALL expose those children by runtime id and child
  status without embedding child runtime handles directly into session state

#### Scenario: Child spawn inherits parent runtime defaults

- **WHEN** a child runtime is spawned without explicit provider, model, or
  runtime overrides
- **THEN** the runtime SHALL inherit the parent runtime’s effective
  provider/model/runtime settings
- **AND** default the child to a non-blocking background posture unless the
  caller explicitly chooses otherwise

### Requirement: Agent Runtime Supports Parent-Child Duplex Messaging

The runtime SHALL support registry-routed runtime-to-runtime messaging through a
routed transport model with direct and mailbox delivery semantics.

#### Scenario: Runtime routes direct message by runtime id

- **WHEN** a caller sends a direct message to a known runtime id
- **THEN** the runtime SHALL route that message through the registry
- **AND** surface queueing and delivery through runtime events
- **AND** make the message eligible for safe-boundary receiver notification

#### Scenario: Runtime stores mailbox messages for later pull

- **WHEN** a caller sends a mailbox message to a known runtime id
- **THEN** the runtime SHALL store that message in mailbox state
- **AND** SHALL NOT automatically inject it into provider transcript flow
- **AND** SHALL allow later filtered retrieval through a read-mail operation

#### Scenario: Registry routing is broader than default discovery

- **WHEN** a caller knows a valid runtime id outside its default child listing
- **THEN** the registry MAY route to that runtime id when policy allows
- **AND** the default agent listing surface SHALL remain scoped and
  conservative

### Requirement: Agent Runtime Promotes Structured Child Reports Into Model Context

The runtime SHALL support semantic child reports that can be promoted into the
parent model-visible transcript at safe boundaries.

#### Scenario: Child report is injected at a safe boundary

- **WHEN** a child emits a model-relevant report such as progress, question,
  observation, result, or failure
- **THEN** the runtime SHALL preserve child provenance and semantic kind
- **AND** inject that report into the parent transcript only at a safe
  boundary before the next provider step

### Requirement: Agent Runtime Supports Reusable Provider Subcalls

The runtime SHALL support loop-authored provider subcalls without corrupting the
outer session transcript bookkeeping.

#### Scenario: Loop performs a nested provider call

- **WHEN** a loop performs a nested provider inference using the runtime
- **THEN** the runtime SHALL preserve correct transcript and event bookkeeping
  for the outer turn
