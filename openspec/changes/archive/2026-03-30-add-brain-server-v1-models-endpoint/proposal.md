# Proposal: add-brain-server-v1-models-endpoint

## Why

The legacy `brain-server` crate already exposes provider metadata at
`GET /providers`, but OpenAI-compatible clients and tooling commonly probe
`GET /v1/models`. Adding that route closes a compatibility gap without
introducing new provider discovery behavior.

## What Changes

- add a `GET /v1/models` endpoint to `brain-server`
- implement the HTTP handler by returning the existing provider list
- cover the route with HTTP integration testing

## Impact

- additive change to the legacy `brain-server` HTTP surface
- no behavior change for existing `/providers` clients
