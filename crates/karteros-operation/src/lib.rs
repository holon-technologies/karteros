//! Domain-independent accepted-operation contracts for Karteros.

#![forbid(unsafe_code)]

use std::{error::Error, fmt};

/// Contract validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContractError {
    /// An identifier was empty or contained only whitespace.
    EmptyIdentifier,
    /// Operation schema versions start at one.
    ZeroSchemaVersion,
}

impl fmt::Display for ContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIdentifier => formatter.write_str("identifier must not be empty"),
            Self::ZeroSchemaVersion => formatter.write_str("schema version must be at least one"),
        }
    }
}

impl Error for ContractError {}

macro_rules! identifier {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, Hash, PartialEq)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, ContractError> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(ContractError::EmptyIdentifier);
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

identifier!(CommandId);
identifier!(OperationId);
identifier!(AggregateId);
identifier!(ProjectionKey);
identifier!(OutboxTopic);

/// An operation that application authorization and signature checks accepted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcceptedOperation {
    command_id: CommandId,
    operation_id: OperationId,
    aggregate_id: AggregateId,
    schema_version: u16,
    logical_clock: u64,
    payload: Vec<u8>,
}

impl AcceptedOperation {
    pub fn new(
        command_id: CommandId,
        operation_id: OperationId,
        aggregate_id: AggregateId,
        schema_version: u16,
        logical_clock: u64,
        payload: Vec<u8>,
    ) -> Result<Self, ContractError> {
        if schema_version == 0 {
            return Err(ContractError::ZeroSchemaVersion);
        }
        Ok(Self {
            command_id,
            operation_id,
            aggregate_id,
            schema_version,
            logical_clock,
            payload,
        })
    }

    pub fn command_id(&self) -> &CommandId {
        &self.command_id
    }

    pub fn operation_id(&self) -> &OperationId {
        &self.operation_id
    }

    pub fn aggregate_id(&self) -> &AggregateId {
        &self.aggregate_id
    }

    pub fn schema_version(&self) -> u16 {
        self.schema_version
    }

    pub fn logical_clock(&self) -> u64 {
        self.logical_clock
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
}

/// Application-provided bytes for one rebuildable local projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionMutation {
    key: ProjectionKey,
    value: Vec<u8>,
}

impl ProjectionMutation {
    pub fn new(key: ProjectionKey, value: Vec<u8>) -> Self {
        Self { key, value }
    }

    pub fn key(&self) -> &ProjectionKey {
        &self.key
    }

    pub fn value(&self) -> &[u8] {
        &self.value
    }
}

/// Application-provided payload for durable at-least-once publication.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutboxMessage {
    topic: OutboxTopic,
    payload: Vec<u8>,
}

impl OutboxMessage {
    pub fn new(topic: OutboxTopic, payload: Vec<u8>) -> Self {
        Self { topic, payload }
    }

    pub fn topic(&self) -> &OutboxTopic {
        &self.topic
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AcceptedOperation, AggregateId, CommandId, ContractError, OperationId, OutboxMessage,
        OutboxTopic, ProjectionKey, ProjectionMutation,
    };

    #[test]
    fn identifiers_reject_empty_values() {
        assert_eq!(CommandId::new(""), Err(ContractError::EmptyIdentifier));
        assert_eq!(OperationId::new("   "), Err(ContractError::EmptyIdentifier));
        assert_eq!(AggregateId::new("\t"), Err(ContractError::EmptyIdentifier));
        assert_eq!(
            ProjectionKey::new("\n"),
            Err(ContractError::EmptyIdentifier)
        );
        assert_eq!(OutboxTopic::new(" "), Err(ContractError::EmptyIdentifier));
    }

    #[test]
    fn operation_rejects_zero_schema_version() {
        let result = AcceptedOperation::new(
            CommandId::new("command-1").unwrap(),
            OperationId::new("operation-1").unwrap(),
            AggregateId::new("aggregate-1").unwrap(),
            0,
            1,
            vec![1, 2, 3],
        );

        assert_eq!(result, Err(ContractError::ZeroSchemaVersion));
    }

    #[test]
    fn contracts_preserve_opaque_application_bytes() {
        let operation = AcceptedOperation::new(
            CommandId::new("command-1").unwrap(),
            OperationId::new("operation-1").unwrap(),
            AggregateId::new("aggregate-1").unwrap(),
            2,
            42,
            vec![0, 1, 255],
        )
        .unwrap();
        let projection = ProjectionMutation::new(
            ProjectionKey::new("aggregate/aggregate-1").unwrap(),
            vec![3, 2, 1],
        );
        let outbox =
            OutboxMessage::new(OutboxTopic::new("example.events/1").unwrap(), vec![9, 8, 7]);

        assert_eq!(operation.command_id().as_str(), "command-1");
        assert_eq!(operation.operation_id().as_str(), "operation-1");
        assert_eq!(operation.aggregate_id().as_str(), "aggregate-1");
        assert_eq!(operation.schema_version(), 2);
        assert_eq!(operation.logical_clock(), 42);
        assert_eq!(operation.payload(), &[0, 1, 255]);
        assert_eq!(projection.key().as_str(), "aggregate/aggregate-1");
        assert_eq!(projection.value(), &[3, 2, 1]);
        assert_eq!(outbox.topic().as_str(), "example.events/1");
        assert_eq!(outbox.payload(), &[9, 8, 7]);
    }
}
