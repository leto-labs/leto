# File Edit Tool

## Purpose

| Tool | Role |
| --- | --- |
| `file_edit` | Replace one exact string match in a file with new content |

## Implementations

| Implementation | Notes |
| --- | --- |
| native | Current implementation |

## Why It Matters

`file_edit` is a narrower editing primitive than `apply_patch`, but it is easy
for loops and models to use when a precise one-shot replacement is enough.
