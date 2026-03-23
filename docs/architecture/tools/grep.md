# Grep Tool

## Purpose

| Tool | Role |
| --- | --- |
| `grep` | Search file contents by regex and return path/line context |

## Implementations

| Implementation | Notes |
| --- | --- |
| native | Current implementation family |
| `GrepDriverAuto` | Chooses the best available native search path |
| `GrepDriverRipgrep` | Prefers `rg` when available |

## Why It Matters

`grep` is one of the most important high-signal navigation tools for code-aware
loops because it turns large workspaces into queryable structure quickly.
