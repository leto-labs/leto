# Proposal: stabilize-agent-acp-harbor-validation

## Why

The migrated Harbor and ACP workflows are now working on the live `agent-*`
stack, but the OpenSpec history does not yet describe the stability fixes that
made those workflows reliable.

The landed behavior includes:

- backward-compatible loading of older persisted credential records
- stable shared transcript tool-call identity that can preserve both transcript
  ids and execution ids
- ACP tool lifecycle updates that do not emit duplicate pending tool calls
- ACP prompt failures and tracing that no longer corrupt protocol stdout

## What Changes

This change records the shipped stabilization work behind the live Harbor and
ACP validation paths:

1. allow `agent-store` to reopen older credential JSON without load-time
   failure when optional metadata is missing
2. let shared provider transcript messages preserve transcript tool-call ids
   separately from execution call ids
3. require `agent-acp` to emit a coherent ACP tool lifecycle while keeping
   protocol output isolated from debug logging

## Impact

- Modified capability: `agent-store`
- Modified capability: `provider`
- Modified capability: `agent-acp`
