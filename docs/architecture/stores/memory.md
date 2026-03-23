# Memory Store

`InMemoryStore` is the transient store implementation used mainly for tests and
runtime checks that do not need durable filesystem state.

## Characteristics

| Concern | Current behavior |
| --- | --- |
| Persistence | process-local only |
| Eventing | publishes the same logical store events as the file-backed store |
| Best fit | tests, fixtures, fast runtime validation |

## Why It Matters

The in-memory implementation keeps the store contract honest. Runtime and loop
tests can exercise the same CRUD and event model without depending on the real
filesystem layout.
