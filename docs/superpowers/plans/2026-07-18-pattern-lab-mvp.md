# Karteros Pattern Lab MVP Implementation Plan

## Goal and success criteria

Create the public `holon-technologies/karteros` repository as the canonical,
executable home of the Karteros resilient local-first application stack.

The MVP succeeds when it provides:

- the canonical public Karteros stack specification;
- an experimental Rust workspace with narrow operation, SQLite outbox, and
  fault-injection crates;
- a reference process that proves recovery after termination immediately after
  a durable commit and after an idempotent external effect but before outbox
  acknowledgement;
- automated formatting, linting, unit, integration, documentation, and secret
  checks on the pinned Karteros Rust toolchain;
- public documentation that clearly separates reusable contracts from
  application-owned domain behavior.

## Approach and constraints

- Use one Cargo workspace and repository-wide versioning while APIs are
  experimental.
- Use Rust 1.97.1 to match the Karteros Docker Sandbox kit.
- Use synchronous `rusqlite` for the first persistence pattern so transaction
  boundaries remain explicit; applications may wrap it in bounded Tokio worker
  tasks.
- Keep operation payloads opaque and leave serialization, signature algorithms,
  authorization policy, and domain projection logic to applications.
- Do not publish crates to crates.io or claim stable APIs in the MVP.
- Do not add Dioxus, Iroh, Kameo, or Axum until an executable pattern requires
  them.
- Preserve the immutable `sbx-kits` v0.2.0 release; link it to this repository
  without rewriting its tag.

## Resolved decisions

- `karteros` is public and owns the canonical specification and executable
  reusable patterns.
- `sbx-kits` owns only the development environment and toolchain distribution.
- Application repositories retain domain entities, product protocols, UI,
  integration policy, and application-specific ADRs.
- A pattern graduates to a stable reusable API only after conformance tests and
  adoption by at least two applications.

### Task 1: Establish the public repository contract

**Resources:** `README.md`, `STACK.md`, `LICENSE`, `CONTRIBUTING.md`,
`SECURITY.md`, `.gitignore`, `Cargo.toml`, `rust-toolchain.toml`, CI workflow

**Depends on:** Approved Karteros repository design

**Interfaces and state:** The public README identifies experimental status,
repository boundaries, workspace layout, and the released Sandbox-kit URL.
`STACK.md` becomes the canonical architecture specification.

**Implementation:** Add public project documentation, Apache-2.0 licensing,
workspace metadata, pinned toolchain, and a CI workflow that runs all required
checks.

**Failure behavior:** No existing release or application is rewritten. The
private specification remains recoverable in Git history and becomes a pointer
only after the public source is pushed and verified.

**Validation:** Markdown link inspection, `cargo metadata`, workflow syntax
inspection, secret scan, and clean diff checks.

### Task 2: Define operation contracts test-first

**Resources:** `crates/karteros-operation/src/lib.rs` and its unit tests

**Depends on:** Task 1 workspace

**Interfaces and state:** Validated non-empty command, operation, aggregate, and
projection identifiers; an opaque accepted operation; projection and outbox
mutations; explicit duplicate acceptance outcome.

**Implementation:** Write compile-failing or behavior-failing tests first, then
add the minimum immutable types and validation needed by persistence.

**Failure behavior:** Empty identifiers and zero schema versions are rejected.
Cryptographic verification remains outside this crate and must occur before an
operation becomes accepted.

**Validation:** Focused crate unit tests followed by workspace tests.

### Task 3: Implement atomic SQLite acceptance test-first

**Resources:** `crates/karteros-outbox/src/lib.rs`, temporary SQLite test
databases

**Depends on:** Task 2 contracts

**Interfaces and state:** One transaction records the idempotency key, accepted
operation, projection mutation, and pending outbox item. Replaying a command
returns `Duplicate` without changing projection revision or adding outbox work.

**Implementation:** First add failing tests for successful acceptance, replay,
rollback, and outbox acknowledgement. Then implement migrations and transaction
methods using `rusqlite` 0.40.1.

**Failure behavior:** Any error before commit rolls back every mutation. A
command ID mapped to different operation data returns an idempotency conflict.
Outbox delivery remains at-least-once and consumers must deduplicate by
operation ID.

**Validation:** Focused persistence tests include database reopen and invariant
counts.

### Task 4: Prove process-level crash recovery

**Resources:** `crates/karteros-testkit`, `examples/crash-recovery`, integration
test child processes and temporary sender/receiver databases

**Depends on:** Task 3 persistence contract

**Interfaces and state:** Named crash points abort a child process after sender
commit and after receiver effect but before sender acknowledgement. The final
restart replays safely and reaches one accepted operation, one projection
revision, one outbox record, and one receiver effect.

**Implementation:** Write the parent integration test and observe failure before
the child modes exist. Add the minimal fault-injection helper and reference
binary to satisfy the two-crash scenario.

**Failure behavior:** Abrupt child termination must not corrupt SQLite. A
pending outbox item remains recoverable. Re-delivery after an ambiguous external
effect is absorbed by the receiver's operation-ID inbox.

**Validation:** Run the focused process integration test and inspect both child
exit failures and final durable counts.

### Task 5: Publish and connect repository boundaries

**Resources:** Karteros repository, `.github-private/README.md`,
`.github-private/STACK.md`, `sbx-kits/README.md`, `sbx-kits/karteros/README.md`

**Depends on:** Tasks 1-4 green and reviewed

**Interfaces and state:** Public URLs are canonical; private and Sandbox-kit
repositories link to them. The v0.2.0 kit URL remains unchanged.

**Implementation:** Run the complete verification suite, commit and push
Karteros, confirm public files and CI, then replace the private duplicate with a
canonical pointer and update Sandbox-kit documentation.

**Failure behavior:** If public CI fails, do not remove the private specification
or claim the public source canonical. Fix within scope and rerun validation.

**Validation:** `cargo fmt`, Clippy with warnings denied, all tests, doc tests,
`cargo metadata`, repository secret scan, link fetches, GitHub Actions success,
and clean synchronized worktrees.
