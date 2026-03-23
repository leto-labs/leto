# Apply Patch Tool

## Purpose

| Tool | Role |
| --- | --- |
| `apply_patch` | Apply a V4A patch envelope across relative workspace paths |

## Implementations

| Implementation | Notes |
| --- | --- |
| native | Current implementation |

## Why It Matters

`apply_patch` is the highest-leverage local editing primitive in the tool
suite. It enables structured multi-file edits without asking loops to model
raw filesystem mutations one line at a time.
