## Summary

- What changed and why?

## Verification

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] `cargo test --workspace --all-features`
- [ ] `pnpm web:typecheck && pnpm web:test && pnpm web:build`
- [ ] Docker Compose smoke test when deployment behavior changed

## Compatibility

- Schema/API/deployment impact:
