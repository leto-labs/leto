# OpenSpec Workflows

## Three-Stage Workflow Overview

### Stage 1: Creating Changes

Create proposal when you need to:
- Add features or functionality
- Make breaking changes (API, schema)
- Change architecture or patterns
- Optimize performance (changes behavior)
- Update security patterns

Skip proposal for:
- Bug fixes (restore intended behavior)
- Typos, formatting, comments
- Dependency updates (non-breaking)
- Configuration changes
- Tests for existing behavior

### Stage 2: Implementing Changes

Track these steps as TODOs and complete them one by one:
1. **Read proposal.md** - Understand what's being built
2. **Read design.md** (if exists) - Review technical decisions
3. **Read tasks.md** - Get implementation checklist
4. **Implement tasks sequentially** - Complete in order
5. **Confirm completion** - Ensure every item in `tasks.md` is finished before updating statuses
6. **Update checklist** - After all work is done, set every task to `- [x]` so the list reflects reality
7. **Approval gate** - Do not start implementation until the proposal is reviewed and approved

### Stage 3: Archiving Changes

After deployment, create separate PR to:
- Move `changes/[name]/` → `changes/archive/YYYY-MM-DD-[name]/`
- Update `specs/` if capabilities changed
- Use `openspec archive <change-id> --skip-specs --yes` for tooling-only changes (always pass the change ID explicitly)
- Run `openspec validate --strict` to confirm the archived change passes checks

## Creating Change Proposals

### Decision Tree

```
New request?
├─ Bug fix restoring spec behavior? → Fix directly
├─ Typo/format/comment? → Fix directly
├─ New feature/capability? → Create proposal
├─ Breaking change? → Create proposal
├─ Architecture change? → Create proposal
└─ Unclear? → Create proposal (safer)
```

### Proposal Creation Steps

1. **Gather Context**
   - Review `openspec/project.md` for conventions
   - Run `openspec list` to see active changes
   - Run `openspec list --specs` to see existing capabilities
   - Check for existing specs that might be affected

2. **Choose Change ID**
   - Use kebab-case, short and descriptive: `add-two-factor-auth`
   - Prefer verb-led prefixes: `add-`, `update-`, `remove-`, `refactor-`
   - Ensure uniqueness; if taken, append `-2`, `-3`, etc.

3. **Create Directory Structure**
   ```bash
   mkdir -p openspec/changes/<change-id>/specs/<capability>
   ```

4. **Write proposal.md**
   ```markdown
   # Change: [Brief description of change]

   ## Why
   [1-2 sentences on problem/opportunity]

   ## What Changes
   - [Bullet list of changes]
   - [Mark breaking changes with **BREAKING**]

   ## Impact
   - Affected specs: [list capabilities]
   - Affected code: [key files/systems]
   ```

5. **Create spec deltas** in `specs/<capability>/spec.md`
   - Use `## ADDED Requirements` for new capabilities
   - Use `## MODIFIED Requirements` for changed behavior
   - Use `## REMOVED Requirements` for deprecated features
   - Use `## RENAMED Requirements` for name changes
   - Each requirement must have at least one `#### Scenario:`

6. **Create tasks.md**
   ```markdown
   ## 1. Implementation
   - [ ] 1.1 Create database schema
   - [ ] 1.2 Implement API endpoint
   - [ ] 1.3 Add frontend component
   - [ ] 1.4 Write tests
   ```

7. **Create design.md (when needed)**
   Create `design.md` if any of the following apply; otherwise omit it:
   - Cross-cutting change (multiple services/modules) or a new architectural pattern
   - New external dependency or significant data model changes
   - Security, performance, or migration complexity
   - Ambiguity that benefits from technical decisions before coding

   Minimal skeleton:
   ```markdown
   ## Context
   [Background, constraints, stakeholders]

   ## Goals / Non-Goals
   - Goals: [...]
   - Non-Goals: [...]

   ## Decisions
   - Decision: [What and why]
   - Alternatives considered: [Options + rationale]

   ## Risks / Trade-offs
   - [Risk] → Mitigation

   ## Migration Plan
   [Steps, rollback]

   ## Open Questions
   - [...]
   ```

8. **Validate**
   ```bash
   openspec validate <change-id> --strict
   ```

9. **Request Approval**
   Do not start implementation until proposal is approved

## Multi-Capability Changes

When a change affects multiple capabilities, create delta files for each:

```
openspec/changes/add-2fa-notify/
├── proposal.md
├── tasks.md
└── specs/
    ├── auth/
    │   └── spec.md   # ADDED: Two-Factor Authentication
    └── notifications/
        └── spec.md   # ADDED: OTP email notification
```

## Happy Path Script

```bash
# 1) Explore current state
openspec spec list --long
openspec list

# 2) Choose change id and scaffold
CHANGE=add-two-factor-auth
mkdir -p openspec/changes/$CHANGE/{specs/auth}
printf "## Why\n...\n\n## What Changes\n- ...\n\n## Impact\n- ...\n" > openspec/changes/$CHANGE/proposal.md
printf "## 1. Implementation\n- [ ] 1.1 ...\n" > openspec/changes/$CHANGE/tasks.md

# 3) Add deltas
cat > openspec/changes/$CHANGE/specs/auth/spec.md << 'EOF'
## ADDED Requirements
### Requirement: Two-Factor Authentication
Users MUST provide a second factor during login.

#### Scenario: OTP required
- **WHEN** valid credentials are provided
- **THEN** an OTP challenge is required
EOF

# 4) Validate
openspec validate $CHANGE --strict
```
