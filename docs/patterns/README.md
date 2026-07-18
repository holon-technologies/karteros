# Karteros Pattern Catalog

Patterns begin here as executable, experimental reference implementations.

Each pattern document must state:

- the invariant it preserves;
- owned durable state and transaction boundaries;
- expected failures and recovery behavior;
- application-owned decisions;
- executable conformance evidence;
- maturity and known consumers.

The initial pattern is [Crash-Resistant Acceptance and
Outbox](crash-resistant-outbox.md), implemented by `karteros-operation`,
`karteros-outbox`, `karteros-testkit`, and the `crash-recovery` example.
