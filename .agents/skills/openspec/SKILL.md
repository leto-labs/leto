---
name: openspec
description: Spec-driven development using OpenSpec for managing change proposals, specifications, and implementation workflows. Use when the request mentions planning, proposals, specs, or changes, or when introducing new capabilities, breaking changes, architecture shifts, or significant performance/security work. Also use when the task sounds ambiguous and you need authoritative specs before coding.
---

# OpenSpec: Spec-Driven Development

OpenSpec is a spec-driven development system for managing change proposals, specifications, and implementation workflows.

## TL;DR Quick Checklist

- Search existing work: `openspec spec list --long`, `openspec list`
- Decide scope: new capability vs modify existing capability
- Pick a unique `change-id`: kebab-case, verb-led (`add-`, `update-`, `remove-`, `refactor-`)
- Scaffold: `proposal.md`, `tasks.md`, `design.md` (only if needed), and delta specs per affected capability
- Write deltas: use `## ADDED|MODIFIED|REMOVED|RENAMED Requirements`; include at least one `#### Scenario:` per requirement
- Validate: `openspec validate [change-id] --strict` and fix issues
- Request approval: Do not start implementation until proposal is approved

## When to Use OpenSpec

### Create Proposal When:
- Adding features or functionality
- Making breaking changes (API, schema)
- Changing architecture or patterns
- Optimizing performance (changes behavior)
- Updating security patterns
- Task is ambiguous and needs spec before coding

### Skip Proposal For:
- Bug fixes (restore intended behavior)
- Typos, formatting, comments
- Dependency updates (non-breaking)
- Configuration changes
- Tests for existing behavior

## Three-Stage Workflow

### Stage 1: Creating Changes
1. Review context: `openspec/project.md`, `openspec list`, `openspec list --specs`
2. Choose unique verb-led `change-id` (kebab-case)
3. Scaffold directory: `changes/<change-id>/`
4. Write `proposal.md` (why, what, impact)
5. Create spec deltas in `specs/<capability>/spec.md`
6. Write `tasks.md` (implementation checklist)
7. Add `design.md` if needed (see criteria in [workflows.md](references/workflows.md))
8. Validate: `openspec validate <change-id> --strict`
9. Request approval before implementation

**See [references/workflows.md](references/workflows.md) for detailed workflow guide**

### Stage 2: Implementing Changes
Track as TODOs:
1. Read `proposal.md`, `design.md` (if exists), `tasks.md`
2. Implement tasks sequentially
3. Confirm completion
4. Update checklist (set all to `- [x]`)
5. Approval gate: Do not start until proposal approved

**See [references/workflows.md](references/workflows.md) for implementation details**

### Stage 3: Archiving Changes
After deployment:
1. Move `changes/[name]/` → `changes/archive/YYYY-MM-DD-[name]/`
2. Update `specs/` if capabilities changed
3. Run: `openspec archive <change-id> --yes`
4. Validate: `openspec validate --strict`

**See [references/workflows.md](references/workflows.md) for archiving guide**

## Before Any Task

**Context Checklist:**
- [ ] Read relevant specs in `specs/[capability]/spec.md`
- [ ] Check pending changes in `changes/` for conflicts
- [ ] Read `openspec/project.md` for conventions
- [ ] Run `openspec list` to see active changes
- [ ] Run `openspec list --specs` to see existing capabilities

**Before Creating Specs:**
- Always check if capability already exists
- Prefer modifying existing specs over creating duplicates
- Use `openspec show [spec]` to review current state
- If request is ambiguous, ask 1-2 clarifying questions before scaffolding

## Quick Start

### Essential Commands
```bash
openspec list                    # List active changes
openspec list --specs            # List specifications
openspec show [item]             # Display change or spec
openspec validate [item]         # Validate changes or specs
openspec archive <change-id> -y  # Archive after deployment
```

**See [references/cli-reference.md](references/cli-reference.md) for complete CLI documentation**

### Creating a Change
```bash
# 1. Explore current state
openspec spec list --long
openspec list

# 2. Scaffold change
CHANGE=add-feature-name
mkdir -p openspec/changes/$CHANGE/specs/capability-name

# 3. Write files
# - proposal.md (why, what, impact)
# - tasks.md (implementation steps)
# - specs/capability-name/spec.md (deltas)

# 4. Validate
openspec validate $CHANGE --strict
```

**See [references/workflows.md](references/workflows.md) for detailed examples and happy path scripts**

## Spec File Format

### Critical: Scenario Formatting
**CORRECT:**
```markdown
#### Scenario: User login success
- **WHEN** valid credentials provided
- **THEN** return JWT token
```

**WRONG:**
```markdown
- **Scenario: User login**  ❌
### Scenario: User login      ❌
```

Every requirement MUST have at least one scenario.

### Delta Operations
- `## ADDED Requirements` - New capabilities
- `## MODIFIED Requirements` - Changed behavior (include full requirement text)
- `## REMOVED Requirements` - Deprecated features
- `## RENAMED Requirements` - Name changes

**See [references/spec-format.md](references/spec-format.md) for complete format guide**

## Directory Structure

```
openspec/
├── project.md              # Project conventions
├── specs/                  # Current truth - what IS built
│   └── [capability]/
│       └── spec.md
└── changes/                # Proposals - what SHOULD change
    ├── [change-name]/
    │   ├── proposal.md
    │   ├── tasks.md
    │   ├── design.md      # Optional
    │   └── specs/
    │       └── [capability]/
    │           └── spec.md
    └── archive/            # Completed changes
```

## Common Issues

### "Requirement must have at least one scenario"
- Check scenarios use `#### Scenario:` format (4 hashtags)
- Don't use bullet points or bold for scenario headers

### "Change must have at least one delta"
- Verify `changes/[name]/specs/` exists with `.md` files
- Check files have operation prefixes (`## ADDED Requirements`)

### Silent Scenario Parsing Failures
- Debug with: `openspec show [change] --json --deltas-only`

**See [references/troubleshooting.md](references/troubleshooting.md) for complete troubleshooting guide**

## Best Practices

### Simplicity First
- Default to <100 lines of new code
- Single-file implementations until proven insufficient
- Avoid frameworks without clear justification

### Clear References
- Use `file.ts:42` format for code locations
- Reference specs as `specs/auth/spec.md`

### Naming Conventions
- Capabilities: verb-noun (`user-auth`, `payment-capture`)
- Change IDs: kebab-case with verb prefix (`add-two-factor-auth`)

## Reference Documentation

- **[workflows.md](references/workflows.md)** - Complete guide to creating, implementing, and archiving changes
- **[spec-format.md](references/spec-format.md)** - Detailed spec formatting rules and requirements
- **[cli-reference.md](references/cli-reference.md)** - Complete CLI command documentation
- **[troubleshooting.md](references/troubleshooting.md)** - Common errors and debugging tips

## Quick Reference

### Stage Indicators
- `changes/` - Proposed, not yet built
- `specs/` - Built and deployed
- `archive/` - Completed changes

### File Purposes
- `proposal.md` - Why and what
- `tasks.md` - Implementation steps
- `design.md` - Technical decisions
- `spec.md` - Requirements and behavior

Remember: Specs are truth. Changes are proposals. Keep them in sync.
