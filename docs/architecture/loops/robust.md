# Robust Loop

`RobustLoop` is the hardened general-purpose loop built on top of the same
basic provider/tool cycle as `SimpleLoop`.

## What It Adds

| Feature | Purpose |
| --- | --- |
| transient provider retries | improve resilience against recoverable inference failures |
| history compaction | reduce context pressure before the loop fails hard |
| doom-loop detection | detect repeated tool invocations and steer or error |
| richer progress/error events | make long-running turns more observable |

## Execution Reading

`RobustLoop` is still the closest thing to the repo's default local loop, but
it is already more policy-heavy than `SimpleLoop`. It is where the generic
runtime currently experiments with retries, summarization, and anti-stuck
behavior before moving to the more benchmark-specialized Terminus family.
