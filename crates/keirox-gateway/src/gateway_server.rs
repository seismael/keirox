//! Kafka Gateway dispatcher and request routing engine per `KEI-DES-035`.

use crate::idempotence::ProducerIdempotenceTracker;
use crate::protocol::{
    KafkaApiKey, KafkaErrorCode, KafkaPartitionResponse, KafkaProduceRecordBatch,
    KafkaProduceResponse, KafkaRequestHeader,
};
use crate::topic_mapper::TopicMapper;
use async_trait::async_trait;
use keirox_core::error::{KeiroxError, Result};
use keirox_core::model::{StreamId, TenantId};
use std::collections::HashMap;
use std::sync::Arc;

/// Trait defining cluster ingest operations for the Kafka Gateway.
#[async_trait]
pub trait ClusterIngress: Send + Sync {
    /// Ingest a batch of records into a target stream.
    async fn produce(
        &self,
        tenant_id: TenantId,
        stream_id: StreamId,
        records: Vec<Vec<u8>>,
    ) -> Result<u64>;
}

/// Kafka gateway endpoint handler processing client requests against the Keirox cluster.
pub struct KafkaGatewayServer {
    cluster: Arc<dyn ClusterIngress>,
    topic_mapper: TopicMapper,
    idempotence_tracker: ProducerIdempotenceTracker,
}

impl KafkaGatewayServer {
    /// Initialize a new Kafka Gateway server attached to a cluster ingress provider.
    pub fn new(cluster: Arc<dyn ClusterIngress>, default_tenant: TenantId) -> Self {
        Self {
            cluster,
            topic_mapper: TopicMapper::new(default_tenant),
            idempotence_tracker: ProducerIdempotenceTracker::new(),
        }
    }

    /// Process a Produce request across topics and partitions with idempotency deduplication.
    /// Process a Produce request across topics and partitions with idempotency deduplication.
    pub async fn process_produce(
        &self,
        batches: Vec<KafkaProduceRecordBatch>,
    ) -> Result<KafkaProduceResponse> {
        let mut responses: HashMap<String, Vec<KafkaPartitionResponse>> = HashMap::new();
        let now_ms = 1_700_000_000_000;

        for batch in batches {
            let stream_id = self
                .topic_mapper
                .map_to_stream(&batch.topic, batch.partition);
            let tenant_id = self.topic_mapper.tenant_id();

            // 1. Check idempotency preflight
            let preflight = self.idempotence_tracker.check_preflight(
                batch.producer_id,
                batch.producer_epoch,
                batch.base_sequence,
                &batch.topic,
                batch.partition,
            );

            let partition_response = match preflight {
                crate::idempotence::PreflightResult::Duplicate(cached_offset) => {
                    KafkaPartitionResponse {
                        partition: batch.partition,
                        error_code: KafkaErrorCode::None,
                        base_offset: cached_offset,
                        log_append_time_ms: now_ms,
                    }
                }
                crate::idempotence::PreflightResult::Error(err_code) => KafkaPartitionResponse {
                    partition: batch.partition,
                    error_code: err_code,
                    base_offset: -1,
                    log_append_time_ms: now_ms,
                },
                crate::idempotence::PreflightResult::Proceed => {
                    // 2. Ingest batch via cluster ingress provider
                    let produce_res = self
                        .cluster
                        .produce(tenant_id, stream_id, batch.records.clone())
                        .await;

                    match produce_res {
                        Ok(assigned_offset) => {
                            match self.idempotence_tracker.verify_and_update(
                                batch.producer_id,
                                batch.producer_epoch,
                                batch.base_sequence,
                                batch.records.len() as i32,
                                &batch.topic,
                                batch.partition,
                                assigned_offset as i64,
                            ) {
                                Ok(final_offset) => KafkaPartitionResponse {
                                    partition: batch.partition,
                                    error_code: KafkaErrorCode::None,
                                    base_offset: final_offset,
                                    log_append_time_ms: now_ms,
                                },
                                Err(err_code) => KafkaPartitionResponse {
                                    partition: batch.partition,
                                    error_code: err_code,
                                    base_offset: -1,
                                    log_append_time_ms: now_ms,
                                },
                            }
                        }
                        Err(KeiroxError::QuorumUnavailable(_)) => KafkaPartitionResponse {
                            partition: batch.partition,
                            error_code: KafkaErrorCode::LeaderNotAvailable,
                            base_offset: -1,
                            log_append_time_ms: now_ms,
                        },
                        Err(_) => KafkaPartitionResponse {
                            partition: batch.partition,
                            error_code: KafkaErrorCode::UnknownServerError,
                            base_offset: -1,
                            log_append_time_ms: now_ms,
                        },
                    }
                }
            };

            responses
                .entry(batch.topic)
                .or_default()
                .push(partition_response);
        }

        Ok(KafkaProduceResponse {
            responses,
            throttle_time_ms: 0,
        })
    }

    /// Negotiate ApiVersions request returning certified supported ranges.
    #[must_use]
    pub fn handle_api_versions(&self) -> Vec<(KafkaApiKey, i16, i16)> {
        vec![
            (KafkaApiKey::Produce, 0, 8),
            (KafkaApiKey::Fetch, 0, 11),
            (KafkaApiKey::ListOffsets, 0, 5),
            (KafkaApiKey::Metadata, 0, 9),
            (KafkaApiKey::OffsetCommit, 0, 7),
            (KafkaApiKey::OffsetFetch, 0, 6),
            (KafkaApiKey::FindCoordinator, 0, 3),
            (KafkaApiKey::ApiVersions, 0, 3),
            (KafkaApiKey::InitProducerId, 0, 2),
        ]
    }

    /// Dispatch incoming generic Kafka request frame.
    pub async fn dispatch_request(&self, header: &KafkaRequestHeader) -> Result<KafkaErrorCode> {
        match header.api_key {
            KafkaApiKey::ApiVersions
            | KafkaApiKey::Produce
            | KafkaApiKey::Fetch
            | KafkaApiKey::Metadata
            | KafkaApiKey::ListOffsets
            | KafkaApiKey::OffsetCommit
            | KafkaApiKey::OffsetFetch
            | KafkaApiKey::FindCoordinator
            | KafkaApiKey::InitProducerId => Ok(KafkaErrorCode::None),
            KafkaApiKey::Unsupported(_) => Ok(KafkaErrorCode::UnsupportedVersion),
        }
    }
}
