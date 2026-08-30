//! Queue worker client with monotonic epoch fencing per `KEI-DES-032` §7 and `ADR-024`.

use crate::client::KeiroxClient;
use keirox_coordinator::EpochFencedToken;
use keirox_core::error::Result;

/// Queue worker client providing point-to-point task leasing, ACKs, and NACKs.
#[derive(Clone)]
pub struct KeiroxQueueClient {
    client: KeiroxClient,
    group_id: String,
}

impl KeiroxQueueClient {
    /// Create a new queue worker client.
    #[must_use]
    pub fn new(client: KeiroxClient, group_id: String) -> Self {
        Self { client, group_id }
    }

    /// Lease an offset task with a given TTL in milliseconds and current timestamp.
    pub async fn lease(&self, offset: u64, ttl_ms: u64, now_us: u64) -> Result<EpochFencedToken> {
        self.client
            .transport()
            .lease(&self.group_id, offset, ttl_ms, now_us)
            .await
    }

    /// Acknowledge a leased task with strict epoch fencing validation.
    pub async fn ack(&self, token: EpochFencedToken) -> Result<()> {
        self.client.transport().ack(&self.group_id, token).await
    }

    /// Negative-acknowledge a leased task to trigger immediate requeue or DLQ escalation.
    pub async fn nack(&self, token: EpochFencedToken) -> Result<()> {
        self.client.transport().nack(&self.group_id, token).await
    }

    /// Target consumer group ID.
    #[must_use]
    pub fn group_id(&self) -> &str {
        &self.group_id
    }
}
