//! Coordinator shard models, identifiers, and monotonic epoch definitions per `KEI-ARC-021` and `KEI-ARC-022`.

use keirox_consensus::NodeId;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique numerical identifier for a coordinator state shard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ShardId(pub u32);

impl fmt::Display for ShardId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Shard-{}", self.0)
    }
}

/// Monotonically increasing coordinator epoch counter.
///
/// Incremented on every coordinator failover or shard rebalance per ADR-024.
/// Used to fence stale lease ACKs/NACKs from demoted or partitioned coordinators.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub struct CoordinatorEpoch(pub u64);

impl CoordinatorEpoch {
    /// Advance epoch by one.
    #[must_use]
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

impl fmt::Display for CoordinatorEpoch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Epoch({})", self.0)
    }
}

/// Shard ownership metadata registered in the Metadata Raft group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShardMetadata {
    /// Shard identifier.
    pub shard_id: ShardId,
    /// Current monotonic coordinator epoch.
    pub epoch: CoordinatorEpoch,
    /// Primary coordinator cluster node hosting this shard.
    pub primary_node_id: NodeId,
    /// Consumer group IDs mapped to this shard.
    pub consumer_groups: Vec<String>,
}
