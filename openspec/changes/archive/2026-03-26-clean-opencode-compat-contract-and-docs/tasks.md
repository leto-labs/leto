## 1. Contract cleanup
- [x] Organize compat DTOs into domain modules and update compat imports
- [x] Remove the catch-all compat DTO organization from routes and helpers
- [x] Keep runtime handlers and docs using the same DTOs

## 2. Doc cleanup
- [x] Remove legacy compat doc assembly that can be replaced by source-side DTO
      naming or route metadata fixes
- [x] Keep only justified generator-format, transport-documentation, and
      parity-test helpers
- [x] Preserve exact prefix-aware `/doc` parity

## 3. Verification
- [x] Keep the prefix-aware `/doc` parity test green
- [x] Run `cargo test -p agent-server`
- [x] Run `cargo test -p agent-core-remote -p agent-server`
- [x] Run `openspec validate clean-opencode-compat-contract-and-docs --strict`
