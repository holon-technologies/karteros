//! Atomic SQLite acceptance and recoverable outbox patterns for Karteros.

#![forbid(unsafe_code)]

use std::{error::Error, fmt, path::Path};

use karteros_operation::{AcceptedOperation, OutboxMessage, ProjectionKey, ProjectionMutation};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

/// Result of attempting to accept an idempotent command.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AcceptanceOutcome {
    /// The operation and all derived durable state committed together.
    Accepted,
    /// An identical operation for the same command was already committed.
    Duplicate,
}

/// Persistence failure with idempotency conflicts separated from SQLite errors.
#[derive(Debug)]
pub enum StoreError {
    Sqlite(rusqlite::Error),
    IdempotencyConflict { command_id: String },
    MissingOutboxItem { id: i64 },
    NumericOutOfRange { field: &'static str },
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(error) => write!(formatter, "SQLite error: {error}"),
            Self::IdempotencyConflict { command_id } => {
                write!(
                    formatter,
                    "command {command_id} was replayed with changed operation data"
                )
            }
            Self::MissingOutboxItem { id } => {
                write!(formatter, "pending outbox item {id} was not found")
            }
            Self::NumericOutOfRange { field } => {
                write!(formatter, "{field} exceeds SQLite's signed integer range")
            }
        }
    }
}

impl Error for StoreError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Sqlite(error) => Some(error),
            Self::IdempotencyConflict { .. }
            | Self::MissingOutboxItem { .. }
            | Self::NumericOutOfRange { .. } => None,
        }
    }
}

impl From<rusqlite::Error> for StoreError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

/// Durable projection row used for conformance checks and local reads.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionRecord {
    pub revision: i64,
    pub value: Vec<u8>,
}

/// One pending at-least-once delivery.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingOutboxItem {
    pub id: i64,
    pub operation_id: String,
    pub topic: String,
    pub payload: Vec<u8>,
}

/// Durable row counts exposed for conformance and operational diagnostics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StoreStats {
    pub accepted_operations: i64,
    pub projections: i64,
    pub outbox_total: i64,
    pub outbox_pending: i64,
    pub outbox_delivered: i64,
}

#[derive(Debug, Eq, PartialEq)]
struct ExistingOperation {
    operation_id: String,
    aggregate_id: String,
    schema_version: u16,
    logical_clock: i64,
    payload: Vec<u8>,
}

/// SQLite-backed atomic acceptance and recoverable outbox store.
pub struct SqliteOutbox {
    connection: Connection,
}

impl SqliteOutbox {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let connection = Connection::open(path)?;
        connection.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = FULL;

             CREATE TABLE IF NOT EXISTS accepted_commands (
                 command_id TEXT PRIMARY KEY NOT NULL,
                 operation_id TEXT UNIQUE NOT NULL
             );

             CREATE TABLE IF NOT EXISTS operations (
                 operation_id TEXT PRIMARY KEY NOT NULL,
                 command_id TEXT UNIQUE NOT NULL
                     REFERENCES accepted_commands(command_id),
                 aggregate_id TEXT NOT NULL,
                 schema_version INTEGER NOT NULL CHECK (schema_version > 0),
                 logical_clock INTEGER NOT NULL CHECK (logical_clock >= 0),
                 payload BLOB NOT NULL
             );

             CREATE TABLE IF NOT EXISTS projections (
                 projection_key TEXT PRIMARY KEY NOT NULL,
                 revision INTEGER NOT NULL CHECK (revision > 0),
                 value BLOB NOT NULL,
                 last_operation_id TEXT NOT NULL
                     REFERENCES operations(operation_id)
             );

             CREATE TABLE IF NOT EXISTS outbox (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 operation_id TEXT UNIQUE NOT NULL
                     REFERENCES operations(operation_id),
                 topic TEXT NOT NULL,
                 payload BLOB NOT NULL,
                 created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 delivered_at TEXT
             );

             PRAGMA user_version = 1;",
        )?;
        Ok(Self { connection })
    }

    pub fn accept(
        &mut self,
        operation: &AcceptedOperation,
        projection: &ProjectionMutation,
        message: &OutboxMessage,
    ) -> Result<AcceptanceOutcome, StoreError> {
        let logical_clock = i64::try_from(operation.logical_clock()).map_err(|_| {
            StoreError::NumericOutOfRange {
                field: "logical_clock",
            }
        })?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        let existing = transaction
            .query_row(
                "SELECT operation_id, aggregate_id, schema_version, logical_clock, payload
                 FROM operations
                 WHERE command_id = ?1",
                [operation.command_id().as_str()],
                |row| {
                    Ok(ExistingOperation {
                        operation_id: row.get(0)?,
                        aggregate_id: row.get(1)?,
                        schema_version: row.get(2)?,
                        logical_clock: row.get(3)?,
                        payload: row.get(4)?,
                    })
                },
            )
            .optional()?;

        if let Some(existing) = existing {
            let replayed = ExistingOperation {
                operation_id: operation.operation_id().as_str().to_owned(),
                aggregate_id: operation.aggregate_id().as_str().to_owned(),
                schema_version: operation.schema_version(),
                logical_clock,
                payload: operation.payload().to_vec(),
            };
            if existing == replayed {
                return Ok(AcceptanceOutcome::Duplicate);
            }
            return Err(StoreError::IdempotencyConflict {
                command_id: operation.command_id().as_str().to_owned(),
            });
        }

        transaction.execute(
            "INSERT INTO accepted_commands (command_id, operation_id) VALUES (?1, ?2)",
            params![
                operation.command_id().as_str(),
                operation.operation_id().as_str()
            ],
        )?;
        transaction.execute(
            "INSERT INTO operations (
                 operation_id, command_id, aggregate_id, schema_version, logical_clock, payload
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                operation.operation_id().as_str(),
                operation.command_id().as_str(),
                operation.aggregate_id().as_str(),
                operation.schema_version(),
                logical_clock,
                operation.payload()
            ],
        )?;
        transaction.execute(
            "INSERT INTO projections (
                 projection_key, revision, value, last_operation_id
             ) VALUES (?1, 1, ?2, ?3)
             ON CONFLICT(projection_key) DO UPDATE SET
                 revision = projections.revision + 1,
                 value = excluded.value,
                 last_operation_id = excluded.last_operation_id",
            params![
                projection.key().as_str(),
                projection.value(),
                operation.operation_id().as_str()
            ],
        )?;
        transaction.execute(
            "INSERT INTO outbox (operation_id, topic, payload) VALUES (?1, ?2, ?3)",
            params![
                operation.operation_id().as_str(),
                message.topic().as_str(),
                message.payload()
            ],
        )?;
        transaction.commit()?;

        Ok(AcceptanceOutcome::Accepted)
    }

    pub fn projection(&self, key: &ProjectionKey) -> Result<Option<ProjectionRecord>, StoreError> {
        self.connection
            .query_row(
                "SELECT revision, value FROM projections WHERE projection_key = ?1",
                [key.as_str()],
                |row| {
                    Ok(ProjectionRecord {
                        revision: row.get(0)?,
                        value: row.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(StoreError::from)
    }

    pub fn next_pending(&self) -> Result<Option<PendingOutboxItem>, StoreError> {
        self.connection
            .query_row(
                "SELECT id, operation_id, topic, payload
                 FROM outbox
                 WHERE delivered_at IS NULL
                 ORDER BY id
                 LIMIT 1",
                [],
                |row| {
                    Ok(PendingOutboxItem {
                        id: row.get(0)?,
                        operation_id: row.get(1)?,
                        topic: row.get(2)?,
                        payload: row.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(StoreError::from)
    }

    pub fn acknowledge(&mut self, id: i64) -> Result<(), StoreError> {
        let changed = self.connection.execute(
            "UPDATE outbox
             SET delivered_at = CURRENT_TIMESTAMP
             WHERE id = ?1 AND delivered_at IS NULL",
            [id],
        )?;
        if changed == 0 {
            return Err(StoreError::MissingOutboxItem { id });
        }
        Ok(())
    }

    pub fn stats(&self) -> Result<StoreStats, StoreError> {
        Ok(StoreStats {
            accepted_operations: self.connection.query_row(
                "SELECT COUNT(*) FROM operations",
                [],
                |row| row.get(0),
            )?,
            projections: self.connection.query_row(
                "SELECT COUNT(*) FROM projections",
                [],
                |row| row.get(0),
            )?,
            outbox_total: self
                .connection
                .query_row("SELECT COUNT(*) FROM outbox", [], |row| row.get(0))?,
            outbox_pending: self.connection.query_row(
                "SELECT COUNT(*) FROM outbox WHERE delivered_at IS NULL",
                [],
                |row| row.get(0),
            )?,
            outbox_delivered: self.connection.query_row(
                "SELECT COUNT(*) FROM outbox WHERE delivered_at IS NOT NULL",
                [],
                |row| row.get(0),
            )?,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use karteros_operation::{
        AcceptedOperation, AggregateId, CommandId, OperationId, OutboxMessage, OutboxTopic,
        ProjectionKey, ProjectionMutation,
    };
    use tempfile::tempdir;

    use super::{AcceptanceOutcome, SqliteOutbox, StoreError};

    fn request(
        command_id: &str,
        operation_id: &str,
        projection_key: &str,
    ) -> (AcceptedOperation, ProjectionMutation, OutboxMessage) {
        (
            AcceptedOperation::new(
                CommandId::new(command_id).unwrap(),
                OperationId::new(operation_id).unwrap(),
                AggregateId::new("aggregate-1").unwrap(),
                1,
                7,
                format!("operation:{operation_id}").into_bytes(),
            )
            .unwrap(),
            ProjectionMutation::new(
                ProjectionKey::new(projection_key).unwrap(),
                format!("projection:{operation_id}").into_bytes(),
            ),
            OutboxMessage::new(
                OutboxTopic::new("example.events/1").unwrap(),
                format!("event:{operation_id}").into_bytes(),
            ),
        )
    }

    fn open(path: &Path) -> SqliteOutbox {
        SqliteOutbox::open(path).unwrap()
    }

    #[test]
    fn acceptance_survives_reopen_and_can_be_acknowledged() {
        let directory = tempdir().unwrap();
        let database = directory.path().join("sender.db");
        let (operation, projection, message) = request("command-1", "operation-1", "item-1");

        {
            let mut store = open(&database);
            assert_eq!(
                store.accept(&operation, &projection, &message).unwrap(),
                AcceptanceOutcome::Accepted
            );
            assert_eq!(store.stats().unwrap().accepted_operations, 1);
            assert_eq!(store.stats().unwrap().projections, 1);
            assert_eq!(store.stats().unwrap().outbox_total, 1);
            assert_eq!(store.stats().unwrap().outbox_pending, 1);
            assert_eq!(store.stats().unwrap().outbox_delivered, 0);

            let record = store
                .projection(&ProjectionKey::new("item-1").unwrap())
                .unwrap()
                .unwrap();
            assert_eq!(record.revision, 1);
            assert_eq!(record.value, b"projection:operation-1");
        }

        let mut reopened = open(&database);
        let pending = reopened.next_pending().unwrap().unwrap();
        assert_eq!(pending.operation_id, "operation-1");
        assert_eq!(pending.topic, "example.events/1");
        assert_eq!(pending.payload, b"event:operation-1");
        reopened.acknowledge(pending.id).unwrap();

        let stats = reopened.stats().unwrap();
        assert_eq!(stats.accepted_operations, 1);
        assert_eq!(stats.projections, 1);
        assert_eq!(stats.outbox_total, 1);
        assert_eq!(stats.outbox_pending, 0);
        assert_eq!(stats.outbox_delivered, 1);
    }

    #[test]
    fn replaying_the_same_command_is_inert() {
        let directory = tempdir().unwrap();
        let (operation, projection, message) = request("command-1", "operation-1", "item-1");
        let mut store = open(&directory.path().join("sender.db"));

        assert_eq!(
            store.accept(&operation, &projection, &message).unwrap(),
            AcceptanceOutcome::Accepted
        );
        assert_eq!(
            store.accept(&operation, &projection, &message).unwrap(),
            AcceptanceOutcome::Duplicate
        );

        let stats = store.stats().unwrap();
        assert_eq!(stats.accepted_operations, 1);
        assert_eq!(stats.outbox_total, 1);
        assert_eq!(
            store
                .projection(&ProjectionKey::new("item-1").unwrap())
                .unwrap()
                .unwrap()
                .revision,
            1
        );
    }

    #[test]
    fn changed_data_for_an_existing_command_is_a_conflict() {
        let directory = tempdir().unwrap();
        let mut store = open(&directory.path().join("sender.db"));
        let (first_operation, first_projection, first_message) =
            request("command-1", "operation-1", "item-1");
        let (changed_operation, changed_projection, changed_message) =
            request("command-1", "operation-2", "item-1");

        store
            .accept(&first_operation, &first_projection, &first_message)
            .unwrap();
        let error = store
            .accept(&changed_operation, &changed_projection, &changed_message)
            .unwrap_err();

        assert!(matches!(
            error,
            StoreError::IdempotencyConflict { ref command_id } if command_id == "command-1"
        ));
        assert_eq!(store.stats().unwrap().accepted_operations, 1);
        assert_eq!(store.stats().unwrap().outbox_total, 1);
    }

    #[test]
    fn a_late_constraint_failure_rolls_back_earlier_transaction_writes() {
        let directory = tempdir().unwrap();
        let mut store = open(&directory.path().join("sender.db"));
        let (first_operation, first_projection, first_message) =
            request("command-1", "operation-1", "item-1");
        store
            .accept(&first_operation, &first_projection, &first_message)
            .unwrap();

        let (duplicate_id, other_projection, other_message) =
            request("command-2", "operation-1", "item-2");
        assert!(matches!(
            store.accept(&duplicate_id, &other_projection, &other_message),
            Err(StoreError::Sqlite(_))
        ));
        assert_eq!(store.stats().unwrap().accepted_operations, 1);
        assert_eq!(store.stats().unwrap().projections, 1);
        assert!(
            store
                .projection(&ProjectionKey::new("item-2").unwrap())
                .unwrap()
                .is_none()
        );

        let (retry, retry_projection, retry_message) =
            request("command-2", "operation-2", "item-2");
        assert_eq!(
            store
                .accept(&retry, &retry_projection, &retry_message)
                .unwrap(),
            AcceptanceOutcome::Accepted
        );
        assert_eq!(store.stats().unwrap().accepted_operations, 2);
    }
}
