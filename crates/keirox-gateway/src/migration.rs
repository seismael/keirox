//! Kafka-to-Keirox Zero-Downtime Migration Bridge, Offset Sync, and Cutover Protocol per `KEI-MIG-501` and `KEI-ENG-500 §5.3`.

use crate::gateway_server::ClusterIngress;
use keirox_core::error::{KeiroxError, Result};
use keirox_core::model::{StreamId, TenantId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Operational state of the migration lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationPhase {
    /// Phase A: Bridge deployed; replicating Kafka stream to Keirox; consumers read from Kafka.
    PhaseABridgeReplicating,
    /// Phase B: Dual-write validation and offset parity verification.
    PhaseBDualWriteValidation,
    /// Phase C: Consumer cutover to Keirox gateway; Kafka remains active fallback.
    PhaseCConsumerCutover,
    /// Phase D: Decommission Kafka cluster and archive legacy logs.
    PhaseDDecommissioned,
    /// Emergency Rollback: Consumers revert back to Kafka.
    Rollback,
}

/// Status report for an active migration pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationStatusReport {
    /// Owning tenant ID.
    pub tenant_id: TenantId,
    /// Source Kafka topic name.
    pub topic: String,
    /// Target Keirox stream ID.
    pub stream_id: StreamId,
    /// Current migration phase.
    pub phase: MigrationPhase,
    /// Highest offset replicated from Kafka.
    pub kafka_high_watermark: i64,
    /// Highest offset ingested into Keirox.
    pub keirox_head_offset: u64,
    /// Offset sync delta (lag).
    pub offset_lag: u64,
    /// Dual write total messages processed.
    pub dual_write_count: u64,
}

/// Kafka consumer offset sync mapping record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffsetSyncPair {
    /// Kafka partition index.
    pub partition: i32,
    /// Kafka committed offset.
    pub kafka_offset: i64,
    /// Corresponding Keirox logical offset.
    pub keirox_offset: u64,
}

/// Migration coordinator managing Kafka topic mirroring and consumer offset synchronization.
pub struct KafkaMigrationBridge {
    ingress: Arc<dyn ClusterIngress>,
    tenant_id: TenantId,
    phase: RwLock<MigrationPhase>,
    offset_mappings: RwLock<HashMap<(String, i32), (i64, u64)>>, // (topic, partition) -> (kafka_offset, keirox_offset)
    dual_write_counter: RwLock<u64>,
}

impl KafkaMigrationBridge {
    /// Initialize a new migration bridge attached to cluster ingress.
    #[must_use]
    pub fn new(ingress: Arc<dyn ClusterIngress>, tenant_id: TenantId) -> Self {
        Self {
            ingress,
            tenant_id,
            phase: RwLock::new(MigrationPhase::PhaseABridgeReplicating),
            offset_mappings: RwLock::new(HashMap::new()),
            dual_write_counter: RwLock::new(0),
        }
    }

    /// Current phase of the migration pipeline.
    pub fn current_phase(&self) -> MigrationPhase {
        self.phase
            .read()
            .map(|g| *g)
            .unwrap_or(MigrationPhase::PhaseABridgeReplicating)
    }

    /// Advance migration phase.
    pub fn transition_phase(&self, new_phase: MigrationPhase) -> Result<()> {
        let mut phase = self
            .phase
            .write()
            .map_err(|_| KeiroxError::Internal("Migration phase lock poisoned".into()))?;
        *phase = new_phase;
        Ok(())
    }

    /// Replicate a batch of records from a Kafka topic partition into Keirox stream.
    pub async fn replicate_from_kafka(
        &self,
        topic: &str,
        partition: i32,
        kafka_base_offset: i64,
        records: Vec<Vec<u8>>,
    ) -> Result<u64> {
        let stream_id = self.derive_stream_id(topic, partition);
        let record_count = records.len() as u64;

        let assigned_keirox_offset = self
            .ingress
            .produce(self.tenant_id, stream_id, records)
            .await?;

        let mut mappings = self
            .offset_mappings
            .write()
            .map_err(|_| KeiroxError::Internal("Offset mappings lock poisoned".into()))?;
        let last_kafka_offset = kafka_base_offset + record_count as i64 - 1;
        let last_keirox_offset = assigned_keirox_offset + record_count - 1;
        mappings.insert(
            (topic.to_string(), partition),
            (last_kafka_offset, last_keirox_offset),
        );

        Ok(assigned_keirox_offset)
    }

    /// Dual-write produce batch during Phase B validation.
    pub async fn dual_write_produce(
        &self,
        topic: &str,
        partition: i32,
        records: Vec<Vec<u8>>,
    ) -> Result<(u64, i64)> {
        let stream_id = self.derive_stream_id(topic, partition);
        let count = records.len() as u64;

        // Ingest into Keirox
        let keirox_offset = self
            .ingress
            .produce(self.tenant_id, stream_id, records)
            .await?;

        // Simulated synchronous mirror to Kafka
        let mut counter = self
            .dual_write_counter
            .write()
            .map_err(|_| KeiroxError::Internal("Dual write counter lock poisoned".into()))?;
        let simulated_kafka_offset = *counter as i64;
        *counter += count;

        let mut mappings = self
            .offset_mappings
            .write()
            .map_err(|_| KeiroxError::Internal("Offset mappings lock poisoned".into()))?;
        mappings.insert(
            (topic.to_string(), partition),
            (
                simulated_kafka_offset + count as i64 - 1,
                keirox_offset + count - 1,
            ),
        );

        Ok((keirox_offset, simulated_kafka_offset))
    }

    /// Synchronize and translate consumer group committed offsets.
    pub fn translate_consumer_offset(
        &self,
        topic: &str,
        partition: i32,
        kafka_committed_offset: i64,
    ) -> Result<u64> {
        let mappings = self
            .offset_mappings
            .read()
            .map_err(|_| KeiroxError::Internal("Offset mappings lock poisoned".into()))?;

        if let Some(&(k_last, kei_last)) = mappings.get(&(topic.to_string(), partition)) {
            if kafka_committed_offset > k_last {
                return Err(KeiroxError::Internal(format!(
                    "Kafka committed offset {kafka_committed_offset} exceeds bridge high watermark {k_last}"
                )));
            }
            let diff = (k_last - kafka_committed_offset) as u64;
            if diff > kei_last {
                Ok(0)
            } else {
                Ok(kei_last - diff)
            }
        } else {
            // Default 1:1 if no mapping recorded yet
            Ok(kafka_committed_offset.max(0) as u64)
        }
    }

    /// Generate comprehensive migration status report.
    pub fn generate_status_report(&self, topic: &str, partition: i32) -> MigrationStatusReport {
        let stream_id = self.derive_stream_id(topic, partition);
        let phase = self.current_phase();
        let mappings = self.offset_mappings.read().ok();

        let (k_high, kei_head) = mappings
            .as_ref()
            .and_then(|m| m.get(&(topic.to_string(), partition)).copied())
            .unwrap_or((-1, 0));

        let offset_lag = if k_high >= 0 && kei_head >= k_high as u64 {
            kei_head - k_high as u64
        } else {
            0
        };

        let dual_count = self.dual_write_counter.read().map(|g| *g).unwrap_or(0);

        MigrationStatusReport {
            tenant_id: self.tenant_id,
            topic: topic.to_string(),
            stream_id,
            phase,
            kafka_high_watermark: k_high,
            keirox_head_offset: kei_head,
            offset_lag,
            dual_write_count: dual_count,
        }
    }

    /// Deterministically map (topic, partition) to StreamId.
    #[must_use]
    pub fn derive_stream_id(&self, topic: &str, partition: i32) -> StreamId {
        let mut hasher = twox_hash::XxHash64::default();
        std::hash::Hasher::write(&mut hasher, topic.as_bytes());
        std::hash::Hasher::write(&mut hasher, &partition.to_le_bytes());
        let hash = std::hash::Hasher::finish(&hasher);

        let mut raw = [0u8; 16];
        raw[..8].copy_from_slice(&self.tenant_id.0[..8]);
        raw[8..16].copy_from_slice(&hash.to_le_bytes());
        StreamId(raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockIngress;
    #[async_trait::async_trait]
    impl ClusterIngress for MockIngress {
        async fn produce(
            &self,
            _tenant_id: TenantId,
            _stream_id: StreamId,
            _records: Vec<Vec<u8>>,
        ) -> Result<u64> {
            Ok(500)
        }
    }

    #[tokio::test]
    async fn test_migration_bridge_lifecycle_and_offset_sync() {
        let tenant = TenantId([0x10; 16]);
        let bridge = KafkaMigrationBridge::new(Arc::new(MockIngress), tenant);

        // 1. Initial phase A
        assert_eq!(
            bridge.current_phase(),
            MigrationPhase::PhaseABridgeReplicating
        );

        // 2. Replicate Kafka records
        let assigned = bridge
            .replicate_from_kafka(
                "orders-topic",
                0,
                100,
                vec![b"rec1".to_vec(), b"rec2".to_vec()],
            )
            .await
            .unwrap();
        assert_eq!(assigned, 500);

        // 3. Translate consumer offset
        let translated = bridge
            .translate_consumer_offset("orders-topic", 0, 101)
            .unwrap();
        assert_eq!(translated, 501);

        // 4. Transition to Dual-Write and Cutover
        bridge
            .transition_phase(MigrationPhase::PhaseBDualWriteValidation)
            .unwrap();
        assert_eq!(
            bridge.current_phase(),
            MigrationPhase::PhaseBDualWriteValidation
        );

        let (kei_off, k_off) = bridge
            .dual_write_produce("orders-topic", 0, vec![b"dual-1".to_vec()])
            .await
            .unwrap();
        assert_eq!(kei_off, 500);
        assert_eq!(k_off, 0);

        bridge
            .transition_phase(MigrationPhase::PhaseCConsumerCutover)
            .unwrap();
        assert_eq!(
            bridge.current_phase(),
            MigrationPhase::PhaseCConsumerCutover
        );

        let report = bridge.generate_status_report("orders-topic", 0);
        assert_eq!(report.phase, MigrationPhase::PhaseCConsumerCutover);
        assert_eq!(report.topic, "orders-topic");
    }
}
