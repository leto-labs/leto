# File Write Tool

## Purpose

| Tool | Role |
| --- | --- |
| `file_write` | Create or overwrite a file with supplied content |

## Implementations

| Implementation | Notes |
| --- | --- |
| native | Writes to the local filesystem |
| ACP-backed | Can route writes through an ACP client-managed workspace |

## Why It Matters

This is one of the main tools that makes ACP-backed editing semantically
different from backend-host-only execution.
