# File Read Tool

## Purpose

| Tool | Role |
| --- | --- |
| `file_read` | Read file contents with line offset and limit controls |

## Implementations

| Implementation | Notes |
| --- | --- |
| native | Reads from the local filesystem |
| ACP-backed | Can route reads through an ACP client-owned filesystem capability |

## Why It Matters

`file_read` is one of the clearest examples of the tool adapter model: the tool
schema stays stable while the actual read path changes depending on runtime
context.
