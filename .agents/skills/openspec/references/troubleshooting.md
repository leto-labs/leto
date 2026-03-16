# OpenSpec Troubleshooting

## Common Errors

### "Change must have at least one delta"

**Cause:** The change directory doesn't contain any delta spec files, or the files aren't properly formatted.

**Solutions:**
- Check that `changes/[name]/specs/` exists with `.md` files
- Verify files have operation prefixes (`## ADDED Requirements`, `## MODIFIED Requirements`, etc.)
- Ensure at least one operation section contains requirements

### "Requirement must have at least one scenario"

**Cause:** A requirement is missing scenarios or scenarios are incorrectly formatted.

**Solutions:**
- Check scenarios use `#### Scenario:` format (4 hashtags)
- Don't use bullet points or bold for scenario headers
- Ensure every requirement has at least one scenario block

**Correct format:**
```markdown
### Requirement: User Login
The system SHALL authenticate users.

#### Scenario: Valid credentials
- **WHEN** user provides valid credentials
- **THEN** system grants access
```

**Incorrect formats:**
```markdown
- **Scenario: Valid credentials**  ❌
**Scenario**: Valid credentials     ❌
### Scenario: Valid credentials      ❌
```

### Silent Scenario Parsing Failures

**Cause:** Scenarios aren't being detected due to formatting issues.

**Solutions:**
- Exact format required: `#### Scenario: Name`
- Check for extra spaces or incorrect heading levels
- Debug with: `openspec show [change] --json --deltas-only`
- Verify the JSON output shows parsed scenarios

### Validation Passes but Scenarios Missing

**Cause:** Scenarios exist but aren't being parsed correctly.

**Debug steps:**
1. Run `openspec show [change] --json --deltas-only`
2. Check the `scenarios` array in each requirement
3. Verify heading format exactly matches `#### Scenario:`
4. Look for invisible characters or encoding issues

## Validation Tips

### Always Use Strict Mode

```bash
openspec validate [change] --strict
```

Strict mode performs comprehensive checks including:
- All requirements have scenarios
- Proper header formatting
- Complete requirement structure
- Valid delta operations

### Debug Delta Parsing

```bash
openspec show [change] --json | jq '.deltas'
```

This shows exactly what deltas were parsed, helping identify:
- Missing operation headers
- Unparsed requirements
- Scenario parsing issues

### Check Specific Requirements

```bash
openspec show [spec] --json -r 1
```

Shows a specific requirement by index, useful for verifying content after archiving.

## Error Recovery

### Change Conflicts

**Problem:** Multiple changes affecting the same capability.

**Solutions:**
1. Run `openspec list` to see active changes
2. Check for overlapping specs
3. Coordinate with change owners
4. Consider combining proposals or sequencing them

### Validation Failures

**Problem:** Validation fails with unclear errors.

**Debug process:**
1. Run with `--strict` flag for detailed errors
2. Check JSON output for details: `openspec validate [change] --json`
3. Verify spec file format matches expected structure
4. Ensure scenarios are properly formatted
5. Check that all files use correct operation headers

### Missing Context

**Problem:** Unclear how to structure a change or spec.

**Solutions:**
1. Read `openspec/project.md` first for project conventions
2. Check related specs for patterns
3. Review recent archives for examples
4. Look at similar changes in `changes/archive/`
5. Ask for clarification before creating proposal

### Archive Failures

**Problem:** Archive command fails or loses information.

**Common causes:**
- MODIFIED requirements missing full content
- Requirement headers don't match exactly
- Scenarios formatted incorrectly

**Solutions:**
1. Ensure MODIFIED requirements include complete requirement text
2. Verify header text matches exactly (whitespace-insensitive)
3. Run `openspec validate --strict` before archiving
4. Check that scenarios use `#### Scenario:` format
5. Review the delta with `openspec show [change] --json --deltas-only`

## Tool Selection Guide

| Task | Tool | Why |
|------|------|-----|
| Find files by pattern | Glob | Fast pattern matching |
| Search code content | Grep | Optimized regex search |
| Read specific files | Read | Direct file access |
| Explore unknown scope | Task | Multi-step investigation |

## Quick Diagnostics

### Is my change valid?
```bash
openspec validate <change-id> --strict
```

### What deltas did I create?
```bash
openspec show <change-id> --json --deltas-only
```

### What's the current state?
```bash
openspec list              # Active changes
openspec list --specs      # Current capabilities
```

### Did my archive work correctly?
```bash
openspec validate --strict                    # Validate all
openspec show <spec-id> --json                # Check spec content
```

## Getting Help

### Check Project Conventions
```bash
cat openspec/project.md
```

### Review Recent Changes
```bash
ls openspec/changes/archive/ | tail -5
```

### Search for Examples
```bash
rg -n "Requirement:|Scenario:" openspec/specs
rg -n "## ADDED Requirements" openspec/changes
```
