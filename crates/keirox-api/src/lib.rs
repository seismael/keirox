//! # Keirox API
//!
//! Client RPC protocols and wire translations per `KEI-DES-032`.

#![deny(missing_docs)]
#![deny(unsafe_code)]

/// Administrative introspection and inspection endpoints.
pub mod admin;
/// Health, liveness, and readiness probes.
pub mod health;
/// Kafka wire protocol translations and framing.
pub mod kafka;
/// Machine-readable metrics telemetry engine.
pub mod metrics;
/// Protocol definitions and request/response messages.
pub mod proto;

pub use admin::{ConsumerGroupInspectionReport, StorageStatsReport, StreamInspectionReport};
pub use health::{HealthProbeService, HealthStatus, ProbeReport, SharedHealthProbe};
pub use kafka::{KafkaApiKey, KafkaRequestHeader, KafkaResponseHeader};
pub use metrics::{MetricSnapshot, SharedTelemetry, TelemetryRegistry};
pub use proto::{
    AckMode, AcknowledgeRequest, LeaseRecordsRequest, ProduceBatchRequest, ProduceBatchResponse,
};
