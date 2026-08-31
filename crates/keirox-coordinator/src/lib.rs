//! Deterministic coordinator sharding, consistent hashing, and epoch fencing for Keirox per `KEI-ARC-021` and `KEI-ARC-022`.

#![deny(unsafe_code)]

pub mod consistent_hash;
pub mod coordinator_node;
pub mod epoch_fencing;
pub mod lease_journal;
/// Point-in-time recovery and legal hold governance.
pub mod pitr;
pub mod shard;

pub use consistent_hash::{ConsistentHashRing, DEFAULT_VNODES_PER_NODE, TOTAL_SHARDS};
pub use coordinator_node::{ActiveShard, CoordinatorNode};
pub use epoch_fencing::EpochFencedToken;
pub use lease_journal::LeaseJournal;
pub use pitr::{LegalHoldEntry, PitrRecoveryEngine, PitrRestoreReport, PitrRestoreTarget};
pub use shard::{CoordinatorEpoch, ShardId, ShardMetadata};
