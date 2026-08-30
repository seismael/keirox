//! Coordinator node hosting assigned state shards and executing fast failover per `KEI-ARC-021` and `KEI-ARC-022`.

use crate::consistent_hash::ConsistentHashRing;
use crate::epoch_fencing::EpochFencedToken;
use crate::lease_journal::LeaseJournal;
use crate::shard::{CoordinatorEpoch, ShardId};
use keirox_consensus::{LeaseDeltaRecord, NodeId};
use keirox_core::error::{KeiroxError, Result};
use keirox_state::ConsumerGroupState;
use keirox_timer::TimingWheel;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Active coordinator shard state hosted on local node.
pub struct ActiveShard {
    /// Shard identifier.
    pub shard_id: ShardId,
    /// Monotonic active epoch.
    pub epoch: CoordinatorEpoch,
    /// Consumer group state machines.
    pub groups: HashMap<String, ConsumerGroupState>,
    /// Timing wheels for lease expiration.
    pub timing_wheels: HashMap<String, TimingWheel>,
    /// Incremental lease journals.
    pub journals: HashMap<String, LeaseJournal>,
}

impl ActiveShard {
    /// Create a new active shard with an initial epoch.
    #[must_use]
    pub fn new(shard_id: ShardId, epoch: CoordinatorEpoch) -> Self {
        Self {
            shard_id,
            epoch,
            groups: HashMap::new(),
            timing_wheels: HashMap::new(),
            journals: HashMap::new(),
        }
    }
}

/// Coordinator node coordinating consumer groups and managing assigned shards.
#[derive(Clone)]
pub struct CoordinatorNode {
    local_node_id: NodeId,
    hash_ring: Arc<RwLock<ConsistentHashRing>>,
    shards: Arc<RwLock<HashMap<ShardId, ActiveShard>>>,
    nonce_gen: Arc<AtomicU32>,
}

impl CoordinatorNode {
    /// Initialize coordinator node with local node ID and consistent hash ring.
    #[must_use]
    pub fn new(local_node_id: NodeId, hash_ring: ConsistentHashRing) -> Self {
        Self {
            local_node_id,
            hash_ring: Arc::new(RwLock::new(hash_ring)),
            shards: Arc::new(RwLock::new(HashMap::new())),
            nonce_gen: Arc::new(AtomicU32::new(1000)),
        }
    }

    /// Local node ID.
    #[must_use]
    pub fn local_node_id(&self) -> NodeId {
        self.local_node_id
    }

    /// Host a new shard on this node.
    pub async fn host_shard(&self, shard_id: ShardId, initial_epoch: CoordinatorEpoch) {
        let mut shards = self.shards.write().await;
        shards.insert(shard_id, ActiveShard::new(shard_id, initial_epoch));
    }

    /// True if this node currently hosts the given shard.
    pub async fn hosts_shard(&self, shard_id: ShardId) -> bool {
        self.shards.read().await.contains_key(&shard_id)
    }

    /// Map a consumer group ID to its designated shard ID via consistent hashing.
    pub async fn resolve_shard_for_group(&self, group_id: &str) -> Option<(ShardId, NodeId)> {
        self.hash_ring.read().await.map_group(group_id)
    }

    /// Lease an offset for a consumer group within an epoch-fenced token.
    pub async fn lease_offset(
        &self,
        group_id: &str,
        offset: u64,
        ttl_ms: u64,
        now_us: u64,
    ) -> Result<EpochFencedToken> {
        let (shard_id, target_node) = self
            .resolve_shard_for_group(group_id)
            .await
            .ok_or_else(|| KeiroxError::Internal("Hash ring is empty".into()))?;

        if target_node != self.local_node_id {
            return Err(KeiroxError::Consensus(format!(
                "Routing mismatch: group {group_id} maps to {target_node}, but local node is {}",
                self.local_node_id
            )));
        }

        let mut shards = self.shards.write().await;
        let shard = shards.get_mut(&shard_id).ok_or_else(|| {
            KeiroxError::Consensus(format!("Shard {shard_id} not hosted on local node"))
        })?;

        let deadline_us = now_us + (ttl_ms * 1_000);
        let group_state = shard
            .groups
            .entry(group_id.to_string())
            .or_insert_with(ConsumerGroupState::new);

        let nonce = self.nonce_gen.fetch_add(1, Ordering::Relaxed);
        let token = EpochFencedToken::new(shard_id, shard.epoch, offset, nonce);

        let leased = group_state.lease_with_token(offset, deadline_us, token.to_u64());
        if !leased {
            return Err(KeiroxError::LeaseConflict(format!(
                "Offset {offset} cannot be leased: current state is {:?}",
                group_state.get_state(offset)
            )));
        }

        // Record in journal for replication
        let journal = shard
            .journals
            .entry(group_id.to_string())
            .or_insert_with(LeaseJournal::new);
        journal.record(LeaseDeltaRecord::Acquire {
            offset,
            token: token.to_u64(),
            deadline_us,
        });

        // Register in timing wheel
        let timing_wheel = shard
            .timing_wheels
            .entry(group_id.to_string())
            .or_insert_with(TimingWheel::default);
        timing_wheel.schedule_timeout(offset, deadline_us);

        Ok(token)
    }

    /// Acknowledge a leased offset with epoch fencing validation.
    pub async fn ack_offset(&self, group_id: &str, token: EpochFencedToken) -> Result<()> {
        let mut shards = self.shards.write().await;
        let shard = shards.get_mut(&token.shard_id).ok_or_else(|| {
            KeiroxError::Consensus(format!(
                "Shard {} not hosted on local node (may have failed over)",
                token.shard_id
            ))
        })?;

        // Validate epoch fencing per ADR-024
        token.validate(shard.shard_id, shard.epoch)?;

        let group_state = shard
            .groups
            .entry(group_id.to_string())
            .or_insert_with(ConsumerGroupState::new);
        group_state.ack_fenced(token.offset, token.to_u64())?;

        let journal = shard
            .journals
            .entry(group_id.to_string())
            .or_insert_with(LeaseJournal::new);
        journal.record(LeaseDeltaRecord::Ack {
            offset: token.offset,
            token: token.to_u64(),
        });

        Ok(())
    }

    /// Negative-acknowledge (NACK) an offset and requeue.
    pub async fn nack_offset(&self, group_id: &str, token: EpochFencedToken) -> Result<()> {
        let mut shards = self.shards.write().await;
        let shard = shards.get_mut(&token.shard_id).ok_or_else(|| {
            KeiroxError::Consensus(format!(
                "Shard {} not hosted on local node (may have failed over)",
                token.shard_id
            ))
        })?;

        token.validate(shard.shard_id, shard.epoch)?;

        let group_state = shard
            .groups
            .entry(group_id.to_string())
            .or_insert_with(ConsumerGroupState::new);
        group_state.nack(token.offset);

        let journal = shard
            .journals
            .entry(group_id.to_string())
            .or_insert_with(LeaseJournal::new);
        journal.record(LeaseDeltaRecord::Nack {
            offset: token.offset,
        });

        Ok(())
    }

    /// Execute fast coordinator failover takeover (<3.5s SLA).
    ///
    /// Successor assumes ownership, increments epoch, restores snapshot, and replays deltas.
    pub async fn failover_takeover_shard(
        &self,
        shard_id: ShardId,
        prior_epoch: CoordinatorEpoch,
        group_snapshots: HashMap<String, (u64, Vec<u8>)>,
        deltas: HashMap<String, Vec<LeaseDeltaRecord>>,
    ) -> Result<CoordinatorEpoch> {
        let new_epoch = prior_epoch.next();
        let mut new_shard = ActiveShard::new(shard_id, new_epoch);

        for (group_id, (_watermark, snapshot_bytes)) in group_snapshots {
            let mut state = if snapshot_bytes.is_empty() {
                ConsumerGroupState::new()
            } else {
                keirox_state::StateSnapshot::restore_from_bytes(&snapshot_bytes)?
            };

            let timing_wheel = new_shard.timing_wheels.entry(group_id.clone()).or_default();

            if let Some(delta_list) = deltas.get(&group_id) {
                for delta in delta_list {
                    LeaseJournal::apply_delta(&mut state, delta)?;
                    if let LeaseDeltaRecord::Acquire {
                        offset,
                        deadline_us,
                        ..
                    } = delta
                    {
                        timing_wheel.schedule_timeout(*offset, *deadline_us);
                    }
                }
            }

            new_shard.groups.insert(group_id, state);
        }

        let mut shards = self.shards.write().await;
        shards.insert(shard_id, new_shard);

        Ok(new_epoch)
    }
}
