use std::{env, error::Error, ffi::OsString, io, path::PathBuf};

use karteros_operation::{
    AcceptedOperation, AggregateId, CommandId, OperationId, OutboxMessage, OutboxTopic,
    ProjectionKey, ProjectionMutation,
};
use karteros_outbox::{PendingOutboxItem, SqliteOutbox};
use karteros_testkit::{CrashPoint, abort_if_configured};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

fn main() {
    if let Err(error) = run() {
        eprintln!("crash-recovery example failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let (sender_database, receiver_database) = arguments()?;
    let (operation, projection, message) = acceptance_request()?;
    let mut sender = SqliteOutbox::open(sender_database)?;

    let outcome = sender.accept(&operation, &projection, &message)?;
    eprintln!("sender acceptance: {outcome:?}");
    abort_if_configured(CrashPoint::AfterSenderCommit)?;

    if let Some(pending) = sender.next_pending()? {
        let mut receiver = open_receiver(receiver_database)?;
        let inserted = apply_effect(&mut receiver, &pending)?;
        eprintln!(
            "receiver effect: {}",
            if inserted { "applied" } else { "duplicate" }
        );
        abort_if_configured(CrashPoint::AfterExternalEffect)?;
        sender.acknowledge(pending.id)?;
    }

    let stats = sender.stats()?;
    eprintln!(
        "sender state: operations={}, projections={}, pending={}, delivered={}",
        stats.accepted_operations, stats.projections, stats.outbox_pending, stats.outbox_delivered
    );
    Ok(())
}

fn arguments() -> Result<(PathBuf, PathBuf), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let command = arguments.next();
    let sender = arguments.next();
    let receiver = arguments.next();
    if command.as_deref() != Some(OsString::from("run").as_os_str())
        || sender.is_none()
        || receiver.is_none()
        || arguments.next().is_some()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: karteros-crash-recovery run <sender.db> <receiver.db>",
        )
        .into());
    }
    Ok((
        PathBuf::from(sender.unwrap()),
        PathBuf::from(receiver.unwrap()),
    ))
}

fn acceptance_request()
-> Result<(AcceptedOperation, ProjectionMutation, OutboxMessage), Box<dyn Error>> {
    Ok((
        AcceptedOperation::new(
            CommandId::new("command/increment-main")?,
            OperationId::new("operation/increment-main/1")?,
            AggregateId::new("counter/main")?,
            1,
            1,
            b"increment=1".to_vec(),
        )?,
        ProjectionMutation::new(ProjectionKey::new("counter/main")?, b"counter=1".to_vec()),
        OutboxMessage::new(
            OutboxTopic::new("counter.events/1")?,
            b"counter-incremented=1".to_vec(),
        ),
    ))
}

fn open_receiver(path: PathBuf) -> Result<Connection, rusqlite::Error> {
    let connection = Connection::open(path)?;
    connection.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = FULL;

         CREATE TABLE IF NOT EXISTS applied_effects (
             operation_id TEXT PRIMARY KEY NOT NULL,
             topic TEXT NOT NULL,
             payload BLOB NOT NULL,
             applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
         );

         CREATE TABLE IF NOT EXISTS counters (
             counter_key TEXT PRIMARY KEY NOT NULL,
             value INTEGER NOT NULL
         );

         INSERT OR IGNORE INTO counters (counter_key, value) VALUES ('main', 0);",
    )?;
    Ok(connection)
}

fn apply_effect(
    receiver: &mut Connection,
    pending: &PendingOutboxItem,
) -> Result<bool, Box<dyn Error>> {
    let transaction = receiver.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let existing = transaction
        .query_row(
            "SELECT topic, payload FROM applied_effects WHERE operation_id = ?1",
            [&pending.operation_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?)),
        )
        .optional()?;

    if let Some((topic, payload)) = existing {
        if topic != pending.topic || payload != pending.payload {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "receiver operation ID was replayed with changed effect data",
            )
            .into());
        }
        return Ok(false);
    }

    transaction.execute(
        "INSERT INTO applied_effects (operation_id, topic, payload) VALUES (?1, ?2, ?3)",
        params![pending.operation_id, pending.topic, pending.payload],
    )?;
    transaction.execute(
        "UPDATE counters SET value = value + 1 WHERE counter_key = 'main'",
        [],
    )?;
    transaction.commit()?;
    Ok(true)
}
