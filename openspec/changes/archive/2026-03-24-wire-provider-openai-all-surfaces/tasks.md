## 1. Shared adapter routing

- [x] Make `OpenAiProvider` dispatch by resolved API surface
- [x] Add Chat Completions request mapping from shared `provider::Request`
- [x] Add Chat Completions shared-event translation
- [x] Make `OpenAiProvider::info()` surface-aware

## 2. Smoke coverage

- [x] Expand wire-client smoke tests across every supported preset/surface combination
- [x] Expand shared-adapter smoke tests across every supported preset/surface combination
- [x] Keep skips limited to missing credentials and clear auth/quota/rate-limit conditions

## 3. Spec sync

- [x] Validate the OpenSpec change
- [x] Archive the change and validate canonical specs
