# Change: Rename local provider to mistralrs

## Why
"local" is too generic — future local providers (gguf-runner for WASM, candle, etc.) would all be "local". Naming the module after its runtime (`mistralrs`) matches the pattern used by `openai/` and avoids naming collisions. The `openai/` folder refactor (flat files → folder) was also applied for consistency.

## What Changes
- Renamed module: `local/` → `mistralrs/`
- Renamed types: `LocalConfig` → `MistralRsConfig`, `LocalProvider` → `MistralRsProvider`, `LocalModelPreset` → `MistralRsModelPreset`
- Renamed feature flag: `local` → `mistralrs`
- Refactored `openai.rs` + `openai_types.rs` + `presets.rs` into `openai/` folder (config.rs, provider.rs, types.rs, presets.rs, mod.rs)

## Impact
- Affected specs: local-provider (type/feature name updates)
- No functional changes — same behavior, different names
