//! # Keirox Kafka Gateway
//!
//! Kafka wire-protocol compatibility gateway per `KEI-ARC-023` and `KEI-DES-035`.

#![deny(missing_docs)]
#![deny(unsafe_code)]

/// AMQP protocol translation gateway subset.
pub mod amqp;
pub mod gateway_server;
pub mod idempotence;
/// Kafka-to-Keirox migration tooling, offset synchronization, and cutover coordinator.
pub mod migration;
pub mod protocol;
/// AWS SQS protocol translation gateway.
pub mod sqs;
pub mod topic_mapper;

pub use amqp::{AmqpExchangeType, AmqpGatewayServer, AmqpPublishConfirmation, AmqpPublishRequest};
pub use gateway_server::{ClusterIngress, KafkaGatewayServer};
pub use idempotence::{ProducerIdempotenceTracker, ProducerSequenceState};
pub use migration::{KafkaMigrationBridge, MigrationPhase, MigrationStatusReport, OffsetSyncPair};
pub use protocol::{
    KafkaApiKey, KafkaErrorCode, KafkaPartitionResponse, KafkaProduceRecordBatch,
    KafkaProduceResponse, KafkaRequestHeader, KafkaResponseHeader,
};
pub use sqs::{
    QueueLeaseProvider, SqsGatewayServer, SqsMessage, SqsReceiveMessageRequest,
    SqsReceiveMessageResponse, SqsSendMessageRequest, SqsSendMessageResponse,
};
pub use topic_mapper::TopicMapper;
