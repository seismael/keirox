//! High-throughput native event producer with exponential backoff and jitter per `KEI-DES-032` §5.

use crate::client::KeiroxClient;
use keirox_core::error::{KeiroxError, Result};
use keirox_core::model::{Offset, StreamId};
use rand::Rng;
use std::time::Duration;

/// High-throughput vectorized producer client.
#[derive(Clone)]
pub struct KeiroxProducer {
    client: KeiroxClient,
}

impl KeiroxProducer {
    /// Create a new producer.
    #[must_use]
    pub fn new(client: KeiroxClient) -> Self {
        Self { client }
    }

    /// Produce a single record to a stream.
    pub async fn send(&self, stream_id: StreamId, payload: Vec<u8>) -> Result<Offset> {
        self.send_batch(stream_id, vec![payload]).await
    }

    /// Produce a batch of records to a stream with automatic exponential retry and full jitter.
    pub async fn send_batch(&self, stream_id: StreamId, records: Vec<Vec<u8>>) -> Result<Offset> {
        let tenant_id = self.client.config().tenant_id;
        let max_retries = self.client.config().max_retries;
        let mut backoff = Duration::from_millis(50);

        for attempt in 0..=max_retries {
            let res = self
                .client
                .transport()
                .produce(tenant_id, stream_id, records.clone())
                .await;

            match res {
                Ok(offset) => return Ok(offset),
                Err(e) if attempt < max_retries => {
                    let jitter_ms = rand::thread_rng().gen_range(0..=backoff.as_millis() as u64);
                    tokio::time::sleep(Duration::from_millis(jitter_ms)).await;
                    backoff = (backoff * 2).min(Duration::from_millis(1000));
                    tracing::warn!("Transient produce error (attempt {}): {:?}", attempt + 1, e);
                }
                Err(e) => return Err(e),
            }
        }

        Err(KeiroxError::QuorumUnavailable("Retries exhausted".into()))
    }
}
