## MODIFIED Requirements

### Requirement: OpenCode Compatibility Surface Is Secondary
The system MUST expose an OpenCode-compatible secondary surface under
`/v1/compat/opencode`.

That compatibility surface MUST remain separate from the canonical API and MUST
cover the OpenCode contract while keeping its internal implementation organized
around the compat route families and shared domain concepts rather than a single
catch-all DTO file.

#### Scenario: Compat DTOs are organized by domain
- **WHEN** OpenCode compat request, response, query, path, and event DTOs are
  defined
- **THEN** they SHALL live in route-family or shared domain modules
- **AND** the implementation SHALL NOT rely on a monolithic dumping-ground DTO
  file as the primary contract source

#### Scenario: Compat `/doc` remains exact while internal cleanup proceeds
- **WHEN** the compat contract is generated at `/v1/compat/opencode/doc`
- **THEN** it SHALL remain exactly equivalent to the pinned OpenCode contract
  under the existing prefix-aware comparison
- **AND** any remaining doc-generation helpers SHALL be limited to justified
  generator-format or transport-documentation adaptation
