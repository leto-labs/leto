# OpenSpec Spec File Format

## Critical: Scenario Formatting

**CORRECT** (use #### headers):
```markdown
#### Scenario: User login success
- **WHEN** valid credentials provided
- **THEN** return JWT token
```

**WRONG** (don't use bullets or bold):
```markdown
- **Scenario: User login**  ❌
**Scenario**: User login     ❌
### Scenario: User login      ❌
```

Every requirement MUST have at least one scenario.

## Requirement Wording

- Use SHALL/MUST for normative requirements (avoid should/may unless intentionally non-normative)

## Delta Operations

### ADDED Requirements

Use for new capabilities or sub-capabilities that can stand alone as a requirement. Prefer ADDED when the change is orthogonal (e.g., adding "Slash Command Configuration") rather than altering the semantics of an existing requirement.

```markdown
## ADDED Requirements
### Requirement: New Feature
The system SHALL provide...

#### Scenario: Success case
- **WHEN** user performs action
- **THEN** expected result
```

### MODIFIED Requirements

Use for changes to the behavior, scope, or acceptance criteria of an existing requirement. Always paste the full, updated requirement content (header + all scenarios). The archiver will replace the entire requirement with what you provide here; partial deltas will drop previous details.

**Authoring a MODIFIED requirement correctly:**

1. Locate the existing requirement in `openspec/specs/<capability>/spec.md`
2. Copy the entire requirement block (from `### Requirement: ...` through its scenarios)
3. Paste it under `## MODIFIED Requirements` and edit to reflect the new behavior
4. Ensure the header text matches exactly (whitespace-insensitive) and keep at least one `#### Scenario:`

```markdown
## MODIFIED Requirements
### Requirement: Existing Feature
[Complete modified requirement with all scenarios]

#### Scenario: Updated behavior
- **WHEN** condition
- **THEN** new expected result
```

**Common pitfall:** Using MODIFIED to add a new concern without including the previous text. This causes loss of detail at archive time. If you aren't explicitly changing the existing requirement, add a new requirement under ADDED instead.

### REMOVED Requirements

Use for deprecated features. Include reason and migration path.

```markdown
## REMOVED Requirements
### Requirement: Old Feature
**Reason**: [Why removing]
**Migration**: [How to handle]
```

### RENAMED Requirements

Use when only the name changes. If you also change behavior, use RENAMED (name) plus MODIFIED (content) referencing the new name.

```markdown
## RENAMED Requirements
- FROM: `### Requirement: Login`
- TO: `### Requirement: User Authentication`
```

## When to use ADDED vs MODIFIED

- **ADDED**: Introduces a new capability or sub-capability that can stand alone as a requirement
  - Example: Adding "Slash Command Configuration" as a new orthogonal feature
  - Example: Adding "Rate Limiting" to an API spec that didn't have it before

- **MODIFIED**: Changes the behavior, scope, or acceptance criteria of an existing requirement
  - Example: Updating "User Authentication" to add 2FA support to existing login flow
  - Example: Changing validation rules in "Form Submission" requirement
  - Always include the full requirement text with all previous details plus your changes

## Headers

Headers are matched with `trim(header)` - whitespace is ignored.

## Directory Structure

```
openspec/
├── project.md              # Project conventions
├── specs/                  # Current truth - what IS built
│   └── [capability]/       # Single focused capability
│       ├── spec.md         # Requirements and scenarios
│       └── design.md       # Technical patterns
└── changes/                # Proposals - what SHOULD change
    ├── [change-name]/
    │   ├── proposal.md     # Why, what, impact
    │   ├── tasks.md        # Implementation checklist
    │   ├── design.md       # Technical decisions (optional)
    │   └── specs/          # Delta changes
    │       └── [capability]/
    │           └── spec.md # ADDED/MODIFIED/REMOVED
    └── archive/            # Completed changes
```

## Best Practices

### Simplicity First
- Default to <100 lines of new code
- Single-file implementations until proven insufficient
- Avoid frameworks without clear justification
- Choose boring, proven patterns

### Complexity Triggers
Only add complexity with:
- Performance data showing current solution too slow
- Concrete scale requirements (>1000 users, >100MB data)
- Multiple proven use cases requiring abstraction

### Clear References
- Use `file.ts:42` format for code locations
- Reference specs as `specs/auth/spec.md`
- Link related changes and PRs

### Capability Naming
- Use verb-noun: `user-auth`, `payment-capture`
- Single purpose per capability
- 10-minute understandability rule
- Split if description needs "AND"

### Change ID Naming
- Use kebab-case, short and descriptive: `add-two-factor-auth`
- Prefer verb-led prefixes: `add-`, `update-`, `remove-`, `refactor-`
- Ensure uniqueness; if taken, append `-2`, `-3`, etc.
