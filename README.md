# Karteros

Karteros is Holon Technologies' resilient local-first application stack.

This repository is the public, executable home of its architecture, reusable
Rust patterns, conformance tests, and reference implementations.

## Name

*Karteros* (Ancient Greek: καρτερός) means steadfast, enduring, or tough. The
name describes the stack's defining promise: applications retain useful local
state, withstand partial failure, and recover without losing accepted work.
Project names follow the public [Holon Technologies naming
conventions](docs/naming-conventions.md).

> [!IMPORTANT]
> Karteros is experimental. The workspace is not published to crates.io and its
> APIs may change while patterns are validated in real applications. Pin Git
> dependencies to an immutable tag or commit.

## What Karteros optimizes for

- Useful native applications without a permanent server connection.
- Durable recovery after process, node, dependency, and network failures.
- Versioned signed operations instead of synchronized database files.
- Direct peer connectivity with coordination only where canonical decisions
  require it.
- Explicit idempotency, transactional outboxes, rebuildable projections, and
  observable terminal failures.

Read the canonical [Karteros stack specification](STACK.md) for the complete
architecture and its application boundaries.

## Repository boundaries

| Repository | Responsibility |
| --- | --- |
| `karteros` | Architecture, reusable contracts, conformance tests, and reference implementations |
| [`sbx-kits`](https://github.com/holon-technologies/sbx-kits) | Development environment and pinned toolchain |
| Application repositories | Domain models, product protocols, UI, integrations, and application policy |

## Workspace

```text
karteros/
├── crates/
│   ├── karteros-operation/  # Domain-independent accepted-operation contracts
│   ├── karteros-outbox/     # Atomic SQLite acceptance and recoverable outbox
│   └── karteros-testkit/    # Explicit process fault-injection helpers
├── examples/
│   └── crash-recovery/      # Two-crash durable recovery reference slice
├── docs/
│   ├── patterns/
│   └── superpowers/plans/
└── STACK.md                 # Canonical architecture specification
```

The first pattern proves recovery after both:

1. Termination after accepting durable state but before publication.
2. Termination after an idempotent external effect but before acknowledging the
   sender's outbox item.

## Development environment

Allow the Holon GitHub organization as a Docker Sandbox kit publisher once:

```sh
sbx settings set kit.allowedSources \
  '["docker.io/","github.com/holon-technologies/"]'
```

Start a sandbox using the immutable Karteros kit release:

```sh
sbx run \
  --kit "git+https://github.com/holon-technologies/sbx-kits.git#ref=v0.2.0&dir=karteros" \
  codex .
```

Or use a local Rust 1.97.1 toolchain:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --doc --workspace
```

## Pattern maturity

A pattern graduates to a stable reusable API only after it has:

- executable conformance and failure-recovery tests;
- adoption by at least two applications;
- a narrow domain-independent contract;
- documented compatibility and migration behavior.

Until then, prefer adapting the reference implementation over treating it as a
stable framework API.

## Contributing and security

See [CONTRIBUTING.md](CONTRIBUTING.md) before proposing a pattern. Report
security issues according to [SECURITY.md](SECURITY.md).

Licensed under the [Apache License 2.0](LICENSE).
