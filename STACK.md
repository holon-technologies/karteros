# Karteros

| Field | Value |
| --- | --- |
| Organization | Holon Technologies |
| Status | Reusable technology baseline |
| Revision | 1.1 |
| Style | Native-first, local-first, decentralized, selectively coordinated, crash-resistant |

Karteros is Holon Technologies' resilient local-first application stack.

## Scope

This document defines a reusable technology and architecture baseline for Holon Technologies applications. It is intentionally independent of any one product. Product features, domain entities, workflows, and integrations belong in each application's `SPEC.md`.

## Goals

Use this stack for software that should:

- Remain useful without a permanent server connection.
- Store and query data locally on each native device.
- Synchronize directly between authorized peers.
- Move large content peer-to-peer.
- Share one Rust core across desktop, mobile, server, and optional web clients.
- Use a coordinator only where one canonical decision is required.
- Survive actor, process, dependency, and abrupt node failures without losing accepted state or duplicating side effects.

## Non-Goals

This baseline does not define product-specific features, domain entities, workflows, or integrations. It also does not require the technologies and patterns listed in [Explicit Non-Choices](#explicit-non-choices).

## Stack

| Area | Baseline |
| --- | --- |
| Language | Rust |
| Workspace | Cargo workspace |
| Async runtime | Tokio |
| Desktop UI | Dioxus Desktop |
| Mobile UI | Dioxus Mobile |
| Optional web UI | Axum + server-rendered templates + HTMX + SSE |
| Local orchestration | Tokio + Kameo for supervised long-lived workflows |
| Peer transport | Iroh |
| Large content | Iroh Blobs |
| Ephemeral events | Iroh Gossip |
| Durable replication | Versioned signed operations over custom Iroh protocols |
| Local storage | SQLite with WAL, migrations, integrity checks, and rebuildable projections |
| Durable workflows | SQLite workflow journal + transactional outbox/inbox + idempotency keys |
| Server HTTP | Axum |
| Reverse proxy | Caddy |
| Deployment | Docker Compose |
| Logging | `tracing` ecosystem |
| Testing | Unit, property, protocol, integration, UI, and Compose tests |

## Architecture

```text
                         Optional public internet
                                  |
                              Caddy / TLS
                                  |
                          Coordinator Node
                    Linux · Rust · Tokio · Kameo
                      Axum · SQLite · Iroh
                         /       |       \
                        /        |        \
              External adapters |     Optional web UI
                                 |
                     ┌───────── Iroh ─────────┐
                     |                        |
               Desktop Client           Mobile Client
              Dioxus · SQLite           Dioxus · SQLite
               Kameo · Iroh              Iroh while active
```

The coordinator is an always-available peer. It is not necessarily the only source of shared data.

Typical coordinator responsibilities include:

- Authoritative membership and capability decisions.
- Finalization of exclusive state transitions.
- Always-online replication and snapshots.
- Secret-bearing external integrations.
- Initial content seeding.
- Push notifications.
- Browser access.
- Backup and recovery.

## Core Principles

### Local-First

Every native client owns:

- A local SQLite database.
- Materialized application views.
- Retained shared operations or trusted snapshots.
- A queue of pending local changes.

Native clients should support offline reads and safe offline writes.

### Signed Operations

Peers exchange versioned signed operations, not database files.

```rust
pub struct SignedOperation {
    pub operation_id: OperationId,
    pub scope_id: ScopeId,
    pub aggregate_id: AggregateId,
    pub operation_type: OperationType,
    pub schema_version: u16,
    pub author_user_id: UserId,
    pub author_device_id: DeviceId,
    pub authorization_epoch: Option<u64>,
    pub logical_clock: u64,
    pub dependencies: Vec<OperationId>,
    pub authored_at: Timestamp,
    pub payload_hash: Hash,
    pub payload: Vec<u8>,
    pub device_signature: Signature,
}
```

The concrete serialization and signature algorithms are application ADR decisions.

### Crash Resistance

Crash resistance is mandatory across the stack. It is not provided by one framework.

The system must tolerate:

- Actor and task panics.
- Whole-process termination.
- Abrupt machine shutdown.
- Temporary loss of peers or the coordinator.
- Partial external side effects.
- Duplicate delivery and retry.
- Corrupted local projections.
- Optional dependency failure.

Required rules:

- No authoritative state may exist only in actor or task memory.
- Every accepted durable command has a stable idempotency key.
- Durable mutations and their outbox records commit atomically.
- Restarted workflows reconstruct state from SQLite, signed operations, or verified snapshots.
- External side effects are reconciled before ambiguous retries.
- Local projections are rebuildable from retained operations or snapshots.
- Optional integrations degrade independently and must not prevent core startup.
- Restart loops use bounded retry budgets and observable terminal failure states.

### Selective Coordination

Use the weakest consistency model that preserves correct behavior.

| Model | Use when |
| --- | --- |
| Mergeable | Concurrent valid changes may coexist or merge deterministically. |
| Coordinator-finalized | One canonical result, deadline, approval, or transition is required. |
| Single-owner | One node owns the physical or external resource. |

### Replaceable Boundaries

Domain crates must not depend on:

- Dioxus.
- Kameo.
- Iroh.
- SQLite libraries.
- Axum.
- Docker.
- OS APIs.
- Product-specific services.

## Runtime Applications

### Desktop

**Baseline:** Dioxus Desktop + Tokio + SQLite + Iroh, with Kameo for supervised long-lived workflows.

Use for:

- Full local-first operation.
- Filesystem and process integration.
- Background tasks.
- Large content transfer and seeding.
- Native notifications.
- Device-specific functionality.

Privileged features should use a narrow helper process instead of running the full UI as administrator.

### Mobile

**Baseline:** Dioxus Mobile + shared Rust core + Tokio + SQLite + Iroh; use Kameo only for workflows that benefit from supervision.

Mobile is a first-class data client, but not a permanently available peer.

While active, it synchronizes through Iroh. While suspended, it relies on local state, best-effort background work, and optional APNs or FCM notifications.

### Coordinator

**Baseline:** Linux + Tokio + Kameo + Axum + SQLite + Iroh.

Use for:

- Authoritative commands.
- Always-online replication.
- Snapshots.
- External integrations and secrets.
- Content seeding.
- Browser routes.
- Backups and observability.

### Optional Web Client

**Baseline:** Axum + Askama or equivalent + HTMX + SSE.

Use normal HTTP commands and server-sent committed updates.

The web client is secondary and does not need full native parity for offline replicas, filesystem access, or peer seeding.

## Workspace Layout

```text
<application>/
├── apps/
│   ├── desktop/
│   ├── mobile/
│   ├── coordinator/
│   ├── web/
│   └── installer/
│
├── crates/
│   ├── domain/
│   ├── operations/
│   ├── identity/
│   ├── authorization/
│   ├── projections/
│   ├── local-store/
│   ├── sync-engine/
│   ├── protocol/
│   ├── transport-iroh/
│   ├── actors/
│   ├── content-model/
│   ├── content-store/
│   ├── content-transfer/
│   ├── notifications/
│   ├── observability/
│   ├── configuration/
│   ├── ui-components/
│   └── test-support/
│
├── integrations/
│   └── <service>-bridge/
│
├── deploy/
│   └── compose/
│
├── docs/
│   ├── architecture/
│   ├── protocols/
│   └── adr/
│
├── SPEC.md
├── STACK.md
└── Cargo.toml
```

Dependency direction:

```text
domain
  ↑
operations / identity / authorization
  ↑
application services / projections
  ↑
actors / storage / transport / integrations
  ↑
desktop / mobile / coordinator / web
```

## Persistence

SQLite stores local state on native clients and coordinator nodes:

- Signed operations.
- Validation results.
- Synchronization heads and cursors.
- Materialized projections.
- Search indexes.
- Pending and rejected operations.
- Device-local settings.
- Transfer and task state.

Required practices:

- Never synchronize SQLite files.
- Enable foreign keys.
- Use WAL for normal operation and choose an explicit synchronous durability profile.
- Apply operation acceptance, projection changes, idempotency records, and outbox entries transactionally.
- Maintain explicit migrations and migration backups.
- Support projection rebuilds from signed operations or verified snapshots.
- Persist long-running workflow state, leases, attempts, checkpoints, and retry schedules.
- Treat `Running` records found after restart as abandoned work that must be reconciled or resumed.
- Run integrity checks and verify backups through automated restore tests.
- Keep large binary content outside SQLite and verify it by content hash.

Projection flow:

```text
signed operations
      |
validation and authorization
      |
projection transaction
      |
local query tables
      |
view models
      |
UI
```

## Networking and Synchronization

### Iroh

Every native client and coordinator owns an Iroh endpoint identity.

Iroh provides:

- Authenticated encrypted QUIC.
- Direct peer connections.
- NAT traversal.
- Relay fallback.
- ALPN-based protocol routing.

Transport identity does not grant application authorization.

### Protocol Namespace

Each application owns its namespace:

```text
<app>.sync/1
<app>.coordinator/1
<app>.events/1
<app>.content/1
<app>.presence/1
<app>.device-control/1
```

| Protocol | Responsibility |
| --- | --- |
| `sync` | Heads, missing operations, snapshots, resume |
| `coordinator` | Commands requiring authoritative validation |
| `events` | Canonical decisions and final results |
| `content` | Manifests, provider tickets, transfer status |
| `presence` | Ephemeral presence, discovery hints, progress |
| `device-control` | Narrow commands between authorized devices |

### Iroh Blobs

Use for immutable or content-addressed large data:

- Files.
- Media.
- Packages.
- Snapshots.
- Exported artifacts.

A coordinator may seed first; verified peers may become providers according to policy.

### Iroh Gossip

Use only for ephemeral or advisory events:

- Presence.
- Typing.
- Availability hints.
- Progress.
- New-operation notifications.

Canonical durable state must never live only in Gossip.

### Iroh Documents

`iroh-docs` is optional.

Adopt it where its replicated key-value model fits, but keep domain operations, authorization, and projections independent from it.

Prefer a custom sync protocol when the application requires explicit revocation, deadline acceptance, exclusive finalization, or rich rejection states.

## Identity and Authorization

Keep these concepts separate:

- Application user identity.
- Device identity.
- Iroh endpoint identity.
- Membership or capability authorization.

A coordinator may sign:

- Membership and role decisions.
- Device authorizations.
- Revocations.
- Canonical exclusive results.
- Content approvals.

Each native installation has a separate revocable device identity.

Use platform-secure key storage where available:

- Apple Keychain.
- Android Keystore.
- Windows secure credential facilities.
- Restricted Linux secret storage.

Cryptographic algorithms, operation encoding, and revocation semantics require ADRs.

## Crash-Resistant Runtime Orchestration

**Base runtime:** Tokio

**Recommended supervision layer:** Kameo

Use Kameo for long-lived workflows that need:

- Exclusive mutable runtime state.
- Sequential commands.
- Bounded mailboxes and backpressure.
- Parent-child supervision.
- Timers, retries, and lifecycle handling.
- Failure isolation between independent workflows.

Common actors:

- Sync supervisor.
- Pending-operation processor.
- Content-transfer supervisor.
- Peer discovery.
- Coordinator command processor.
- External API rate limiter.
- Notification and outbox delivery.

Applications that do not need actor semantics may use Tokio tasks and bounded channels directly.

Rules:

- Actor references never cross the network.
- Actor state is a cache of durable state, not the authoritative record.
- Actors reconstruct state after restart from SQLite, signed operations, or verified snapshots.
- Supervisors use bounded restart budgets and exponential backoff.
- Repeated failure transitions the workflow to a visible `NeedsAttention` or terminal state instead of looping forever.
- Actors do not map one-to-one to database rows.
- CPU-heavy work runs in bounded worker tasks.
- High-frequency updates are coalesced.
- Network DTOs, domain commands, and actor messages remain separate.
- Cancellation and shutdown are explicit; dropping a task is not considered successful completion.

### Durable Workflow Pattern

Long-running work should use a journaled state machine such as:

```text
Pending
  -> Running
  -> Succeeded

Pending | Running
  -> Retrying
  -> Running

Pending | Running | Retrying
  -> Failed
  -> NeedsAttention
```

A workflow record should contain:

- Stable workflow and command IDs.
- Desired state and observed state.
- Attempt count.
- Lease owner and lease expiration.
- Last heartbeat or checkpoint.
- Next retry time.
- Last error category.
- External correlation ID where available.

Leases must expire after crashes. A restarted worker claims expired work and reconciles remote state before retrying an ambiguous external action.

### Transactional Outbox and Inbox

Use a transactional outbox for events and side effects that follow a durable mutation. Use an inbox or idempotency table for commands and replicated operations that may arrive more than once.

```text
transaction
├── accepted operation
├── projection update
├── idempotency record
└── outbox item

commit
  -> dispatcher publishes or performs side effect
  -> dispatcher records result
```

Delivery is at-least-once; handlers must be idempotent.

## Startup, Recovery, and Shutdown

### Startup Recovery

Each runtime starts in recovery mode before accepting normal commands:

1. Open and validate local storage.
2. Apply migrations.
3. Run required integrity checks.
4. Recover expired workflow leases.
5. Reconcile incomplete external operations.
6. Resume outbox and synchronization queues.
7. Rebuild projections when required.
8. Start network listeners and mark the process ready.

### Graceful Shutdown

1. Stop accepting new commands.
2. Mark the node as draining.
3. Cancel ephemeral work.
4. Allow active transactions to complete.
5. Persist cursors and checkpoints.
6. Release or expire workflow leases.
7. Close Iroh endpoints.
8. Close SQLite.

A bounded shutdown deadline must eventually allow process termination. Correctness must still hold after forced termination at any point.

## UI

### Native UI

Dioxus is the baseline desktop and mobile renderer.

UI code consumes view models and application commands. It does not directly access network actors or storage repositories.

### Optional Web UI

Use server-rendered HTML, HTMX-enhanced commands, and SSE for committed updates.

A JavaScript SPA or LiveView architecture is not required.

### Shared Design System

Share:

- Design tokens.
- Typography and colors.
- Validation messages.
- Domain view models.
- Reusable interaction patterns.

Desktop and mobile may use different navigation and screen composition.

### State Presentation

Every UI should distinguish:

- Local draft.
- Pending synchronization.
- Accepted.
- Rejected.
- Conflicted.
- Stale.
- Offline.
- External.

## External Integrations

External services are adapters, not core dependencies.

```text
integrations/
├── <identity-provider>-bridge/
├── <metadata-provider>-bridge/
├── <storage-provider>-bridge/
├── <automation-provider>-bridge/
└── <infrastructure-provider>-bridge/
```

Each adapter owns:

- Authentication.
- External DTOs.
- Rate limiting.
- Retry behavior.
- Capability detection.
- Data normalization.
- Secret handling.
- Contract tests.

External types and secrets must not leak into domain crates or replicated shared operations.

## Deployment

### Docker Compose

Coordinator deployments use Docker Compose first.

Typical services:

- `coordinator`
- `reverse-proxy`
- Optional integration services

### Caddy

Use Caddy for:

- TLS termination.
- HTTP routing.
- Certificate management.

LAN-only deployments may expose the coordinator directly when explicitly configured.

### Persistent Data

Persist:

- Coordinator SQLite database.
- Operation history.
- Blob storage.
- Configuration.
- Cached assets.
- Backups.
- Integration state.
- TLS certificates.

Production requirements:

- Pinned images.
- Multi-stage Rust builds.
- Non-root coordinator process.
- Explicit networks and volumes.
- Health checks that distinguish liveness from readiness.
- Restart policies for unexpected process failure.
- Graceful stop periods and explicit shutdown signals.
- Persistent workflow, operation, and blob storage.
- Startup recovery before readiness is reported.
- Secret separation.

## Observability

### Logging

Use the `tracing` ecosystem with structured fields such as:

- Service.
- Request ID.
- Operation ID.
- Scope ID where safe.
- Peer or device ID where safe.
- Actor or subsystem.
- Error category.

Never log private keys, tokens, passwords, or sensitive user content by default.

### Metrics

Track:

- Connected peers.
- Synchronization latency.
- Pending and rejected operations.
- Validation failures.
- Actor mailbox depth, restart counts, restart-budget exhaustion, and supervision failures.
- Startup recovery duration and recovered workflow count.
- Outbox backlog, retry count, and oldest pending item.
- Workflow lease expirations and reconciliation outcomes.
- Content-transfer throughput.
- Verification failures.
- SQLite latency.
- Integration health.
- Notification delivery.

### Health

Expose separate health states for:

- Liveness.
- Core readiness.
- Storage.
- Iroh connectivity.
- Optional integrations.

Optional integration failures must not mark the core application unavailable.

## Verification Plan

Required testing layers:

- Domain unit tests.
- Property-based tests for deterministic algorithms and merge rules.
- Signature and authorization tests.
- Projection rebuild and migration tests.
- Synchronization interruption and recovery tests.
- Actor panic and supervisor restart tests.
- Forced process termination at transaction and side-effect boundaries.
- Workflow lease expiry and recovery tests.
- Outbox duplicate-delivery and idempotency tests.
- Ambiguous external side-effect reconciliation tests.
- SQLite WAL and projection rebuild recovery tests.
- Iroh protocol compatibility tests.
- Content corruption and resume tests.
- Integration adapter contract tests.
- Desktop and mobile UI tests.
- Browser end-to-end tests.
- Docker Compose tests.
- Backup and restore tests.

The shared `test-support` crate should provide deterministic clocks, IDs, identities, temporary stores, simulated peers, and fake integrations.

## Backup and Recovery

Coordinator backups should contain:

- Operation history.
- Projection database or rebuild metadata.
- Blob manifests.
- Optional blob content.
- Configuration.
- Required integration state.
- Application and schema versions.

Backups must support integrity validation and optional encryption.

Restore must validate compatibility and support rebuilding projections. Backups are not considered valid until automated restore verification succeeds.

## Optional LAN Overlay

Applications that need remote devices to appear on one virtual LAN may add a later overlay protocol:

```text
<app>.lan-overlay/1
```

The overlay uses:

- Iroh transport.
- Windows Wintun helper.
- Linux TUN gateway.
- Explicit IPv4 routes.
- Application allowlists.

Constraints:

- No default route.
- No general-purpose VPN behavior.
- Signed and expiring grants.
- Source-address anti-spoofing.
- Rate-limited broadcast and multicast.
- Separate privileged helper.
- Explicit route policies.

This is an optional extension, not part of the core stack.

## Holon Technology Names

Reusable internal technologies may use Holon naming where it improves clarity:

| Name | Responsibility |
| --- | --- |
| OikosAuth | User, device, membership, and capability authorization |
| KoinonSync | Signed-operation synchronization and reconciliation |
| SympathLink | Iroh transport integration |
| ArchipelagoStore | Operations, projections, snapshots, and blobs |
| HolonRuntime | Tokio and Kameo runtime conventions |
| HolonProtocol | Shared envelopes and protocol-versioning rules |

Plain descriptive crate names remain preferable when they are easier to understand.

## Explicit Non-Choices

The baseline does not require:

- A JavaScript SPA.
- Server-side LiveView.
- Distributed Kameo actor references.
- Synchronized SQLite files.
- A central server for every read.
- CRDTs for every domain.
- `iroh-docs` as a universal database.
- Kubernetes for initial deployment.
- Product-specific integrations in shared core crates.

## ADR Requirements and Open Questions

Each application must decide and record:

- Operation serialization.
- Signature algorithms and crates.
- Authorization and revocation semantics.
- SQLite access and migrations.
- Snapshot format.
- Sync-range protocol.
- Dioxus version baseline.
- Web template engine.
- Metrics exporter.
- Secret storage.
- Mobile secure-storage bridges.
- Desktop and mobile release pipelines.
- Content manifest format.
- Relay policy.
- Optional `iroh-docs` usage.
- Optional LAN-overlay implementation.
- Supervision trees and restart budgets.
- SQLite durability profile and integrity policy.
- Durable workflow, lease, outbox, and inbox schemas.
- External side-effect reconciliation rules.
- Graceful-shutdown and startup-recovery deadlines.

These items remain intentionally open at the reusable-stack level because their correct choices depend on each application's threat model, operational context, and product requirements.

## Versioning

- Pin the Rust toolchain.
- Commit `Cargo.lock`.
- Pin production container images.
- Version every Iroh protocol through ALPN and message schemas.
- Version signed-operation payloads.
- Version deterministic algorithms.
- Maintain explicit SQLite migrations.
- Support projection rebuilds.
- Preserve at least one previous compatible client protocol during rolling upgrades where practical.

## First Vertical Slice

Before adding product features, prove one complete path:

```text
Desktop Client A
      |
      | signed operation
      v
Coordinator Node
      |
      | validate · persist · project · synchronize
      v
Desktop Client B
```

The slice should prove:

- Native Dioxus shell.
- Local SQLite operation store.
- Device identities and signatures.
- Iroh peer connection.
- Versioned sync protocol.
- Coordinator validation.
- Projection updates and rebuild.
- Offline pending changes.
- Reconnect and synchronization.
- Kameo supervision with bounded restart budgets.
- Durable workflow journal and transactional outbox.
- Idempotent command handling.
- Forced termination after commit but before publication, followed by successful recovery without duplication.
- Projection corruption followed by rebuild from retained operations.
- Structured diagnostics.
- Protocol and crash-recovery tests.

## Evidence and Decision Status

- **Confirmed:** The architecture, constraints, and technology choices in this document come from the Holon Technologies reusable baseline supplied for revision 1.1.
- **Confirmed:** The internal technology names follow the repository's [Naming Conventions](specifications/NAMING_CONVENTIONS.md), which prefer meaningful Holon, Greek, or Latin names where they improve clarity.
- **Application-specific:** Decisions listed in [ADR Requirements and Open Questions](#adr-requirements-and-open-questions) are deliberately unresolved here and must be recorded per application.
- **Product-specific:** Features, domain entities, workflows, and integrations remain outside this baseline and belong in each application's `SPEC.md`.

## Definition

Karteros is:

- Rust.
- Cargo workspace.
- Tokio.
- Dioxus Desktop and Mobile.
- Kameo for supervised workflows.
- Iroh.
- Iroh Blobs.
- Iroh Gossip.
- Versioned signed operations.
- Custom Iroh protocols.
- SQLite local projections.
- Axum.
- Optional server-rendered web UI.
- Caddy.
- Docker Compose.
- `tracing` and metrics.
- Property-based, integration, and crash fault-injection testing.

Its defining model is:

> Autonomous local applications, verifiable shared operations, direct peer connectivity, selective coordination, and durable recovery after partial failure.
