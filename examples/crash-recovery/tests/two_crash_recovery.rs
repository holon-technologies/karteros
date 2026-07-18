use std::{path::Path, process::Command};

use karteros_operation::ProjectionKey;
use karteros_outbox::SqliteOutbox;
use karteros_testkit::{CRASH_POINT_ENV, CrashPoint};
use rusqlite::Connection;
use tempfile::tempdir;

const BINARY: &str = env!("CARGO_BIN_EXE_karteros-crash-recovery");

fn run_process(
    sender_database: &Path,
    receiver_database: &Path,
    crash_point: Option<CrashPoint>,
) -> std::process::ExitStatus {
    let mut command = Command::new(BINARY);
    command
        .arg("run")
        .arg(sender_database)
        .arg(receiver_database);
    if let Some(point) = crash_point {
        command.env(CRASH_POINT_ENV, point.as_str());
    }
    command.status().unwrap()
}

fn receiver_effect_count(path: &Path) -> i64 {
    let connection = Connection::open(path).unwrap();
    connection
        .query_row("SELECT COUNT(*) FROM applied_effects", [], |row| row.get(0))
        .unwrap()
}

fn receiver_counter(path: &Path) -> i64 {
    let connection = Connection::open(path).unwrap();
    connection
        .query_row(
            "SELECT value FROM counters WHERE counter_key = 'main'",
            [],
            |row| row.get(0),
        )
        .unwrap()
}

#[test]
fn two_ambiguous_crashes_converge_without_duplicate_effects() {
    let directory = tempdir().unwrap();
    let sender_database = directory.path().join("sender.db");
    let receiver_database = directory.path().join("receiver.db");

    let after_commit = run_process(
        &sender_database,
        &receiver_database,
        Some(CrashPoint::AfterSenderCommit),
    );
    assert!(
        !after_commit.success(),
        "the first child must terminate abruptly"
    );

    let sender = SqliteOutbox::open(&sender_database).unwrap();
    let stats = sender.stats().unwrap();
    assert_eq!(stats.accepted_operations, 1);
    assert_eq!(stats.projections, 1);
    assert_eq!(stats.outbox_total, 1);
    assert_eq!(stats.outbox_pending, 1);
    drop(sender);

    let after_effect = run_process(
        &sender_database,
        &receiver_database,
        Some(CrashPoint::AfterExternalEffect),
    );
    assert!(
        !after_effect.success(),
        "the second child must terminate abruptly"
    );
    assert_eq!(receiver_effect_count(&receiver_database), 1);
    assert_eq!(receiver_counter(&receiver_database), 1);
    assert_eq!(
        SqliteOutbox::open(&sender_database)
            .unwrap()
            .stats()
            .unwrap()
            .outbox_pending,
        1
    );

    assert!(run_process(&sender_database, &receiver_database, None).success());
    assert!(run_process(&sender_database, &receiver_database, None).success());

    let sender = SqliteOutbox::open(&sender_database).unwrap();
    let stats = sender.stats().unwrap();
    assert_eq!(stats.accepted_operations, 1);
    assert_eq!(stats.projections, 1);
    assert_eq!(stats.outbox_total, 1);
    assert_eq!(stats.outbox_pending, 0);
    assert_eq!(stats.outbox_delivered, 1);
    assert_eq!(receiver_effect_count(&receiver_database), 1);
    assert_eq!(receiver_counter(&receiver_database), 1);

    let projection = sender
        .projection(&ProjectionKey::new("counter/main").unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(projection.revision, 1);
    assert_eq!(projection.value, b"counter=1");
}
