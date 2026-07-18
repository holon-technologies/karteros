# Contributing to Karteros

Karteros accepts narrowly scoped, evidence-backed application patterns.

## Before proposing a reusable pattern

Document:

- the invariant the pattern preserves;
- the failure boundary it addresses;
- which concerns remain application-owned;
- evidence from at least one real consumer;
- compatibility and migration implications.

New patterns begin as reference implementations. They become stable crates only
after adoption by at least two applications and executable conformance tests.

## Development checks

Use Rust 1.97.1 and run:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --doc --workspace
git diff --check
```

Tests for durability behavior should exercise real SQLite transactions and
process boundaries. Avoid mocks for failures that can be reproduced safely.

## Changes and review

- Keep public interfaces narrow and immutable by default.
- Use Conventional Commit messages.
- Update architecture documentation when an invariant or boundary changes.
- Do not add secrets, credentials, private endpoints, or production data.
- Treat all install scripts, network capabilities, cryptographic choices, and
  external side effects as security-sensitive changes.
