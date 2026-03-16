# OpenSpec CLI Reference

## Essential Commands

### List Active Changes
```bash
openspec list
```
Lists all active changes in the `changes/` directory (excludes archives).

### List Specifications
```bash
openspec list --specs
```
Lists all specifications in the `specs/` directory.

### Show Change or Spec Details
```bash
openspec show [item]           # Interactive prompt if item not specified
openspec show <change-id>      # Show change details
openspec show <spec-id>        # Show spec details
```

### Validate Changes or Specs
```bash
openspec validate              # Bulk validation mode (interactive)
openspec validate [item]       # Validate specific change or spec
openspec validate [item] --strict  # Comprehensive validation
```

### Archive a Change
```bash
openspec archive <change-id>              # Interactive archive with prompts
openspec archive <change-id> --yes        # Non-interactive archive
openspec archive <change-id> -y           # Short form
openspec archive <change-id> --skip-specs # Archive without spec updates
```

**Important:** Always pass the change ID explicitly. Add `--yes` or `-y` for non-interactive runs (e.g., in scripts or automation).

## Project Management

### Initialize OpenSpec
```bash
openspec init [path]
```
Initializes OpenSpec in the specified directory (defaults to current directory).

### Update Instruction Files
```bash
openspec update [path]
```
Updates the AGENTS.md instruction files in the project.

## Command Flags

### Common Flags

- `--json` - Machine-readable output
- `--type change|spec` - Disambiguate items when names conflict
- `--strict` - Comprehensive validation (recommended before approval)
- `--no-interactive` - Disable prompts
- `--yes` / `-y` - Skip confirmation prompts (for non-interactive archive)
- `--skip-specs` - Archive without spec updates (for tooling-only changes)

### List Command Flags

```bash
openspec list --long           # Show additional details
openspec list --json           # JSON output for scripts
openspec spec list --long      # List specs with details
openspec spec list --json      # JSON output for scripts
```

### Show Command Flags

```bash
openspec show <item> --json           # Machine-readable output
openspec show <item> --deltas-only    # Show only delta changes
openspec show <spec> --json -r 1      # Show specific requirement by index
```

## Search Guidance

### Enumerate Items
```bash
openspec spec list --long      # List all specs with details
openspec spec list --json      # JSON for scripts
openspec list                  # List all active changes
openspec change list --json    # Deprecated but available
```

### Show Details
```bash
# Show spec details
openspec show <spec-id> --type spec
openspec show <spec-id> --json          # JSON with filters

# Show change details
openspec show <change-id> --json --deltas-only
```

### Full-Text Search
Use ripgrep for full-text searches:
```bash
rg -n "Requirement:|Scenario:" openspec/specs
rg -n "^#|Requirement:" openspec/changes
```

## Debugging

### Debug Delta Parsing
```bash
openspec show [change] --json | jq '.deltas'
openspec show [change] --json --deltas-only
```

### Check Specific Requirement
```bash
openspec show [spec] --json -r 1
```

### Comprehensive Validation
```bash
openspec validate [change] --strict
```

## Interactive Mode

When running commands without arguments, OpenSpec enters interactive mode with prompts:

```bash
openspec show       # Prompts for selection
openspec validate   # Bulk validation mode
```

Use `--no-interactive` to disable prompts in automated scripts.
