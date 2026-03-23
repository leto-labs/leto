# Stores

`brain-stores` owns durable runtime state. It implements the store contracts
from `brain-types` for both in-memory tests and the real filesystem-backed
runtime.

## Table Of Contents

| Topic | Document |
| --- | --- |
| Store overview, domains, and patterns | [`README.md`](README.md) |
| In-memory implementation | [`memory.md`](memory.md) |
| Filesystem-backed implementation | [`file.md`](file.md) |

## Persisted Domains

| Domain | Why it exists |
| --- | --- |
| Projects | Root-level configuration and workspace identity |
| Sessions | Conversation identity plus session-level inference and loop overrides |
| Messages | Ordered turn history |
| Credentials | Provider auth material and health state |
| Trajectories | ATIF transcript state kept separately from messages |

## Store Implementations

| Store | Use case | Notes |
| --- | --- | --- |
| `InMemoryStore` | Tests and transient local flows | Good for unit-style runtime checks |
| `FileStore` | Real local runtime state under `brain_home()` | Current persistent default |

## Storage Model

| Pattern | Current behavior |
| --- | --- |
| Domain-specific CRUD traits | Each record family gets a specialized store on top of shared CRUD semantics |
| Event emission | Store implementations publish lifecycle events as records change |
| Trajectory separation | ATIF transcript state is not rebuilt from messages every turn |

## Why Trajectories Are Separate

Keeping trajectories in their own domain avoids re-deriving historical ATIF
steps from the latest config or latest loop behavior. That matters because the
benchmark/export view needs transcript fidelity across turns, not just the
current message list.
