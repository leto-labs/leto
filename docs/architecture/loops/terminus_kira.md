# Terminus Kira Loop

`TerminusKiraLoop` is the KIRA-inspired terminal loop.

## Core Ideas

| Idea | Current behavior |
| --- | --- |
| native tool calling | uses provider-native tool calls rather than text-parsed JSON command plans |
| persistent terminal state | still relies on `terminal_session` as a central capability |
| multimodal support | can work with image-aware messages for KIRA-style flows |
| stricter completion loop | uses a more deliberate confirmation path before finishing |

## Why It Matters

This loop is where benchmark pressure most directly affected the shared message
and provider surfaces. It needed multimodal message parts and image-capable
OpenAI-compatible serialization, which makes it one of the most architecture-
shaping experimental loops in the repo.
