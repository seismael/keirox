//! Multi-Region Mode A (Single-Writer Primary + Asynchronous Replica) Replication per `KEI-MR-401` and `KEI-ARC-026`.

use crate::hlc::{HlcTimestamp, HybridLogicalClock};
use keirox_core::error::{KeiroxError, Result};
use keirox_core::model::{StreamId, TenantId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

/// Unique identifier for a cloud or geographical region (e.g. 1 = us-east-1, 2 = eu-west-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RegionId(pub u16);

/// Regional epoch counter incremented on cross-region failover to fence stale primary writes.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub struct RegionEpoch(pub u64);

impl RegionEpoch {
    /// Advance epoch by one.
    #[must_use]
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

/// Regional operating role for a stream or cluster partition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegionRole {
    /// Primary region accepting client writes.
    Primary,
    /// Secondary replica region asynchronously replicating primary WAL batches.
    SecondaryReplica,
}

/// Cross-region replication batch frame carrying causal HLC metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationBatch {
    /// Originating tenant ID.
    pub tenant_id: TenantId,
    /// Target micro-stream ID.
    pub stream_id: StreamId,
    /// Base offset in the primary region.
    pub base_offset: u64,
    /// Raw batch records.
    pub records: Vec<Vec<u8>>,
    /// Originating region identifier.
    pub origin_region: RegionId,
    /// Hybrid Logical Clock timestamp from primary.
    pub hlc_timestamp: HlcTimestamp,
    /// Current regional epoch.
    pub region_epoch: RegionEpoch,
}

/// Multi-region replication manager enforcing Mode A single-writer invariants and epoch fencing.
pub struct MultiRegionReplicator {
    local_region: RegionId,
    role: RwLock<RegionRole>,
    current_epoch: RwLock<RegionEpoch>,
    hlc: HybridLogicalClock,
    replicated_offsets: RwLock<HashMap<StreamId, u64>>,
}

impl MultiRegionReplicator {
    /// Create a new MultiRegionReplicator.
    #[must_use]
    pub fn new(local_region: RegionId, role: RegionRole) -> Self {
        Self {
            local_region,
            role: RwLock::new(role),
            current_epoch: RwLock::new(RegionEpoch(1)),
            hlc: HybridLogicalClock::new(local_region.0),
            replicated_offsets: RwLock::new(HashMap::new()),
        }
    }

    /// Current operational role of this region.
    pub fn role(&self) -> RegionRole {
        self.role
            .read()
            .map(|g| *g)
            .unwrap_or(RegionRole::SecondaryReplica)
    }

    /// Current active regional epoch.
    pub fn epoch(&self) -> RegionEpoch {
        self.current_epoch
            .read()
            .map(|g| *g)
            .unwrap_or(RegionEpoch(1))
    }

    /// Prepare a batch for cross-region replication (invoked on Primary).
    pub fn create_replication_batch(
        &self,
        tenant_id: TenantId,
        stream_id: StreamId,
        base_offset: u64,
        records: Vec<Vec<u8>>,
        physical_now_ms: u64,
    ) -> Result<ReplicationBatch> {
        let role = self.role();
        if role != RegionRole::Primary {
            return Err(KeiroxError::Internal(
                "Cannot create replication batch on secondary replica region".into(),
            ));
        }

        let epoch = self.epoch();
        let hlc_timestamp = self.hlc.now(physical_now_ms);

        Ok(ReplicationBatch {
            tenant_id,
            stream_id,
            base_offset,
            records,
            origin_region: self.local_region,
            hlc_timestamp,
            region_epoch: epoch,
        })
    }

    /// Apply an incoming replication batch (invoked on Secondary Replica).
    pub fn apply_replication_batch(
        &self,
        batch: &ReplicationBatch,
        physical_now_ms: u64,
    ) -> Result<u64> {
        let role = self.role();
        if role == RegionRole::Primary {
            return Err(KeiroxError::EpochFenced(
                "Primary region rejected incoming replication batch; potential dual-primary split-brain"
                    .into(),
            ));
        }

        let current_epoch = self.epoch();
        if batch.region_epoch < current_epoch {
            return Err(KeiroxError::EpochFenced(format!(
                "Stale replication batch epoch {:?} < active regional epoch {:?}",
                batch.region_epoch, current_epoch
            )));
        }

        // Advance local HLC with remote timestamp
        self.hlc.update(batch.hlc_timestamp, physical_now_ms);

        let mut offsets = self
            .replicated_offsets
            .write()
            .map_err(|_| KeiroxError::Internal("Replicated offsets lock poisoned".into()))?;
        let last_offset = batch.base_offset + batch.records.len() as u64 - 1;
        offsets.insert(batch.stream_id, last_offset);

        Ok(last_offset)
    }

    /// Promote secondary replica to new primary during regional failover.
    pub fn promote_to_primary(&self) -> Result<RegionEpoch> {
        let mut role = self
            .role
            .write()
            .map_err(|_| KeiroxError::Internal("Replicator role lock poisoned".into()))?;
        let mut epoch = self
            .current_epoch
            .write()
            .map_err(|_| KeiroxError::Internal("Replicator epoch lock poisoned".into()))?;

        let new_epoch = epoch.next();
        *epoch = new_epoch;
        *role = RegionRole::Primary;

        Ok(new_epoch)
    }

    /// Demote active primary to secondary replica (e.g. after planned switchover or isolation).
    pub fn demote_to_replica(&self, new_epoch: RegionEpoch) -> Result<()> {
        let mut role = self
            .role
            .write()
            .map_err(|_| KeiroxError::Internal("Replicator role lock poisoned".into()))?;
        let mut epoch = self
            .current_epoch
            .write()
            .map_err(|_| KeiroxError::Internal("Replicator epoch lock poisoned".into()))?;

        *epoch = new_epoch;
        *role = RegionRole::SecondaryReplica;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_region_replication_and_fenced_failover() {
        let region_us = RegionId(1);
        let region_eu = RegionId(2);

        let primary_us = MultiRegionReplicator::new(region_us, RegionRole::Primary);
        let replica_eu = MultiRegionReplicator::new(region_eu, RegionRole::SecondaryReplica);

        let tenant = TenantId([0x01; 16]);
        let stream = StreamId([0x02; 16]);

        // 1. Primary creates batch and replica applies it
        let batch = primary_us
            .create_replication_batch(
                tenant,
                stream,
                0,
                vec![b"record 1".to_vec(), b"record 2".to_vec()],
                1000,
            )
            .unwrap();

        let applied = replica_eu.apply_replication_batch(&batch, 1005).unwrap();
        assert_eq!(applied, 1);

        // 2. Perform failover: EU is promoted to Primary (epoch advances to 2)
        let new_epoch = replica_eu.promote_to_primary().unwrap();
        assert_eq!(new_epoch, RegionEpoch(2));
        assert_eq!(replica_eu.role(), RegionRole::Primary);

        // 3. Stale primary US tries to send a batch with old epoch 1 to EU
        let stale_batch = primary_us
            .create_replication_batch(tenant, stream, 2, vec![b"stale record".to_vec()], 1010)
            .unwrap();

        // EU is now primary and in epoch 2, so applying incoming stale batch must be rejected!
        let res = replica_eu.apply_replication_batch(&stale_batch, 1012);
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), KeiroxError::EpochFenced(_)));
    }
}
