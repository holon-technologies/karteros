# Crash-Recovery Reference Process

This reference process proves the Karteros crash-resistant acceptance and
outbox pattern with separate sender and receiver SQLite databases.

Run the automated two-crash scenario:

```sh
cargo test -p karteros-crash-recovery --test two_crash_recovery -- --nocapture
```

The parent test launches the process four times:

1. It aborts after the sender commits accepted state.
2. It restarts, publishes, and aborts after the receiver commits its effect.
3. It restarts, redelivers safely, and acknowledges the outbox item.
4. It restarts once more to prove the converged state is inert.

For manual inspection, pass persistent database paths:

```sh
KARTEROS_CRASH_POINT=after-sender-commit \
  cargo run -p karteros-crash-recovery -- run /tmp/sender.db /tmp/receiver.db

KARTEROS_CRASH_POINT=after-external-effect \
  cargo run -p karteros-crash-recovery -- run /tmp/sender.db /tmp/receiver.db

cargo run -p karteros-crash-recovery -- run /tmp/sender.db /tmp/receiver.db
```

The deliberate abort commands return non-zero. Use only disposable databases
when running the fault-injection modes manually.
