# Proposal: add-brain-server-health-route

## Why

`agent-server` already exposes a lightweight health probe, but the legacy
`brain-server` HTTP surface only exposes `/status`. That makes basic liveness
checks heavier than they need to be and leaves an unnecessary parity gap when
comparing the two servers.

## What Changes

- add a lightweight `GET /health` endpoint to `brain-server`
- return a small JSON response suitable for liveness checks
- cover the route with HTTP integration testing

## Impact

- small additive change to the legacy `brain-server` HTTP surface
- no behavioral changes to existing routes
