# MistralRs Provider

This page covers the `mistralrs` local provider implementation.

## Main Pieces

| Component | Role |
| --- | --- |
| `MistralRsProvider` | In-process local inference provider |
| `MistralRsConfig` | Model and runtime configuration for local inference |
| `MistralRsModelPreset` | Named presets that expand into concrete config |
| `DevicePreference` | Device selection hint for local execution |

## Characteristics

| Concern | Current behavior |
| --- | --- |
| Feature gate | Disabled unless the `mistralrs` feature is enabled |
| Runtime model | Local, in-process model execution |
| Integration point | Appears to the rest of the system as a normal `Provider` |

## Why It Exists

`mistralrs` gives the workspace a local inference path that still fits the same
provider trait and runtime registry model used by remote HTTP-backed providers.
