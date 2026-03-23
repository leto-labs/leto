# Design

## Source of Truth

This change ports the control flow of upstream Terminus-KIRA into `brain`
without importing Harbor-specific runtime assumptions.

The goal is behavioral parity at the loop level:

- provider-native tool calling
- KIRA semantic tools
- marker-based terminal polling
- multimodal image reads
- completion confirmation

## Architectural Fit

`brain` should continue to model loops as strategy objects over generic
`Provider` and `Tool` interfaces.

To preserve that:

- the runtime remains loop-composable
- the model-facing tool schema is loop-owned
- internal execution still uses registered runtime tools

`TerminusKiraLoop` therefore uses:

- provider-visible semantic tools:
  - `execute_commands`
  - `task_complete`
  - `image_read`
- runtime-visible internal tools:
  - `terminal_session`
  - `shell`

The loop passes only the KIRA semantic tool definitions to the provider, then
implements those semantics through the registered runtime tools.

## Multimodal Message Model

The current `brain_types::Message` is text-only. That is insufficient for
KIRA's `image_read`, which performs a second provider call with text plus an
inline image payload.

The shared message model should therefore move from plain text to structured
content:

- `MessageContent::Text(String)`
- `MessageContent::Parts(Vec<ContentPart>)`

with:

- `ContentPart::Text { text }`
- `ContentPart::ImageUrl { url }`

This keeps the `Provider::chat(...)` trait stable while making providers
responsible for serializing richer message inputs.

## Provider Scope

The primary implementation target is the OpenAI-compatible provider family used
for Gemini/OpenAI API-key flows.

Both request builders need multimodal support:

- Responses API request builder
- Chat Completions request builder

The provider should fail clearly when a message contains unsupported content for
the selected API surface, rather than silently flattening or dropping parts.

## KIRA Loop Flow

Each main turn follows this structure:

1. Capture or reuse terminal state
2. Call the provider with KIRA semantic tool definitions
3. Record streamed text/tool-call events
4. Parse the final tool call set into one of:
   - command batch
   - image read
   - completion request
5. Execute the chosen action
6. Feed the resulting observation back as the next user message

Marker-based polling is implemented by appending a unique marker command after
each command batch and polling terminal output until the marker appears or the
requested wait duration elapses.

## Image Read Strategy

`image_read` is implemented in-loop, not as a generic runtime tool.

The loop:

- reads the image file via the registered `shell` tool using `base64`
- infers MIME type from the path extension
- constructs a `data:` URL
- performs a multimodal provider subcall using the shared `Provider::chat(...)`
  surface and the new structured `MessageContent`
- returns the model's analysis as the observation for that KIRA tool action

## Non-Goals

- Reworking existing `SimpleLoop`, `RobustLoop`, or `Terminus2Loop` behavior
- Adding a generic nested-provider-call facility to the `Tool` trait
- Adding Daytona-based validation during development
