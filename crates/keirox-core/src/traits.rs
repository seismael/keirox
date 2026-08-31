//! Abstract domain interfaces and contracts per SOLID / DDD principles.

use crate::error::Result;
use crate::model::{Offset, StreamId};

/// Core contract for physical append-only storage engines.
pub trait StorageEngine: Send + Sync {
    /// Append a pre-allocated byte batch to physical storage.
    fn append_batch(&mut self, stream_id: StreamId, batch: &[u8]) -> Result<Offset>;

    /// Read records for a stream starting at a given offset.
    fn read_records(
        &self,
        stream_id: StreamId,
        start_offset: Offset,
        max_records: usize,
    ) -> Result<Vec<u8>>;
}

/// Alias for StorageEngine for backward compatibility.
pub trait WalEngine: StorageEngine {}
impl<T: StorageEngine + ?Sized> WalEngine for T {}

/// Core contract for distributed consensus coordinators per AGENTS.md §3 and KEI-ARC-022.
pub trait ConsensusCoordinator: Send + Sync {
    /// Propose a state machine metadata change to quorum.
    fn propose_command(&self, command: &[u8]) -> Result<u64>;

    /// Check if local coordinator node is the elected active leader.
    fn is_leader(&self) -> bool;

    /// Return the active consensus term / epoch.
    fn current_term(&self) -> u64;
}

/// Core contract for columnar catalog and lakehouse sync per AGENTS.md §3 and KEI-DES-034.
pub trait CatalogSync: Send + Sync {
    /// Register a sealed snapshot commit in the metadata catalog.
    fn register_snapshot(&self, table_name: &str, snapshot_data: &[u8]) -> Result<u64>;

    /// Retrieve the current serialized active catalog snapshot metadata.
    fn current_snapshot(&self, table_name: &str) -> Result<Option<Vec<u8>>>;

    /// Prune snapshots older than retention threshold.
    fn expire_snapshots_before(&self, table_name: &str, cutoff_timestamp_ms: u64) -> Result<usize>;
}

/// Core contract for consumption state overlays (streaming, queuing, DLQ).
pub trait StateOverlayEngine: Send + Sync {
    /// Grant an exclusive lease on an offset for a duration.
    fn grant_lease(&mut self, stream_id: StreamId, offset: Offset, ttl_us: u64) -> Result<bool>;

    /// Acknowledge an offset as terminal.
    fn acknowledge(&mut self, stream_id: StreamId, offset: Offset) -> Result<()>;

    /// Negative-acknowledge an offset to return it to the Ready queue.
    fn negative_acknowledge(&mut self, stream_id: StreamId, offset: Offset) -> Result<()>;

    /// Evict an offset to the virtual Dead-Letter Queue.
    fn evict_to_dlq(&mut self, stream_id: StreamId, offset: Offset) -> Result<()>;

    /// Return the current monotonic base watermark for a stream.
    fn base_watermark(&self, stream_id: StreamId) -> Offset;
}
