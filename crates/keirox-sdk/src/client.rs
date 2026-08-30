//! Core client configuration and factory per `KEI-DES-032`.

use crate::consumer::KeiroxConsumer;
use crate::flight::ArrowFlightReader;
use crate::producer::KeiroxProducer;
use crate::task_queue::KeiroxQueueClient;
use async_trait::async_trait;
use keirox_coordinator::EpochFencedToken;
use keirox_core::error::Result;
use keirox_core::model::{StreamId, TenantId};
use std::sync::Arc;
use std::time::Duration;

/// Trait defining the cluster transport interface used by the SDK.
#[async_trait]
pub trait ClusterClientTransport: Send + Sync {
    /// Ingest a batch of records into a stream.
    async fn produce(
        &self,
        tenant_id: TenantId,
        stream_id: StreamId,
        records: Vec<Vec<u8>>,
    ) -> Result<u64>;
    /// Lease an offset task.
    async fn lease(
        &self,
        group_id: &str,
        offset: u64,
        ttl_ms: u64,
        now_us: u64,
    ) -> Result<EpochFencedToken>;
    /// Acknowledge a leased task.
    async fn ack(&self, group_id: &str, token: EpochFencedToken) -> Result<()>;
    /// Negative acknowledge a leased task.
    async fn nack(&self, group_id: &str, token: EpochFencedToken) -> Result<()>;
}

/// Configuration options for the Keirox Client SDK.
#[derive(Debug, Clone)]
pub struct KeiroxClientConfig {
    /// Endpoint connection URL (e.g. `keirox://localhost:9092`).
    pub endpoint: String,
    /// Tenant context identifier.
    pub tenant_id: TenantId,
    /// Request timeout duration.
    pub timeout: Duration,
    /// Maximum retry attempts for transient errors.
    pub max_retries: u32,
}

impl Default for KeiroxClientConfig {
    fn default() -> Self {
        Self {
            endpoint: "keirox://127.0.0.1:9092".to_string(),
            tenant_id: TenantId([1u8; 16]),
            timeout: Duration::from_millis(3000),
            max_retries: 3,
        }
    }
}

/// Unified entry point client for the Keirox distributed runtime.
#[derive(Clone)]
pub struct KeiroxClient {
    config: KeiroxClientConfig,
    transport: Arc<dyn ClusterClientTransport>,
}

impl KeiroxClient {
    /// Initialize a new client attached to a cluster transport provider.
    pub fn new(config: KeiroxClientConfig, transport: Arc<dyn ClusterClientTransport>) -> Self {
        Self { config, transport }
    }

    /// Client configuration reference.
    #[must_use]
    pub fn config(&self) -> &KeiroxClientConfig {
        &self.config
    }

    /// Spawn a high-throughput batch producer.
    #[must_use]
    pub fn producer(&self) -> KeiroxProducer {
        KeiroxProducer::new(self.clone())
    }

    /// Spawn a continuous stream consumer starting at `start_offset`.
    #[must_use]
    pub fn consumer(&self, stream_id: StreamId, start_offset: u64) -> KeiroxConsumer {
        KeiroxConsumer::new(self.clone(), stream_id, start_offset)
    }

    /// Spawn a task queue worker client for a consumer group.
    #[must_use]
    pub fn queue(&self, group_id: impl Into<String>) -> KeiroxQueueClient {
        KeiroxQueueClient::new(self.clone(), group_id.into())
    }

    /// Spawn an Arrow Flight vectorized stream reader.
    #[must_use]
    pub fn flight_reader(&self) -> ArrowFlightReader {
        ArrowFlightReader::new(self.clone())
    }

    /// Transport handle reference.
    pub fn transport(&self) -> Arc<dyn ClusterClientTransport> {
        self.transport.clone()
    }
}
