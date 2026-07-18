# Crash-Resistant Acceptance and Outbox

| Field | Value |
| --- | --- |
| Status | Experimental reference pattern |
| Crates | `karteros-operation`, `karteros-outbox`, `karteros-testkit` |
| Evidence | `examples/crash-recovery/tests/two_crash_recovery.rs` |
| Known consumers | Karteros reference example only |

## Invariant

An accepted command must survive abrupt termination without losing its durable
operation or projection and without creating duplicate application effects when
publication is retried.

The pattern provides at-least-once delivery. It does not claim exactly-once
transport. Effectively-once behavior comes from stable operation IDs and an
idempotent receiver inbox.

## Sender transaction

One SQLite transaction commits:

```text
accepted command ID
├── accepted operation
├── projection mutation
└── pending outbox item
```

If any statement fails before commit, the transaction rolls back every write.
Replaying identical operation data for an accepted command returns `Duplicate`
without incrementing a projection revision or adding an outbox record. Changed
operation data for the same command returns an idempotency conflict.

## Receiver transaction

The reference receiver commits its operation-ID inbox record and counter effect
together. A repeated operation with identical effect data is inert. Reusing the
operation ID with different data is rejected.

Production receivers should apply the same rule to their own durable effect and
inbox state. When the external system provides an idempotency key, use the
operation ID or a stable derivative as that key.

## Failure and recovery behavior

| Failure boundary | Durable state after failure | Recovery |
| --- | --- | --- |
| Before sender commit | No partial command, operation, projection, or outbox state | Retry the command |
| After sender commit, before publish | One accepted operation and one pending outbox item | Restart dispatcher and publish pending item |
| After receiver effect, before sender acknowledgement | Receiver effect exists; sender item remains pending | Redeliver; receiver deduplicates; acknowledge |
| After sender acknowledgement | Receiver effect and delivered sender item exist | No work remains |

## Application-owned decisions

Karteros does not define:

- operation serialization or signature algorithms;
- authorization and revocation policy;
- domain projection content;
- external-system reconciliation semantics;
- retention, compaction, backup, or migration policy;
- dispatcher scheduling, retry budgets, or terminal failure UX.

The synchronous SQLite API makes transaction ownership explicit. Tokio
applications should call it from bounded blocking workers rather than blocking
an async executor thread.

## Run the evidence

```sh
cargo test -p karteros-crash-recovery --test two_crash_recovery -- --nocapture
```

The test launches child processes and requires two non-success exits caused by
deliberate `abort` fault injection before verifying final durable state.
