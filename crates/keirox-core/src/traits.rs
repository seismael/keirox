//! Abstract domain interfaces and contracts per SOLID / DDD principles.

use crate::error::Result;
use crate::model::{Offset, StreamId};

/// Core contract for physical append-only storage engines.
pub trait WalEngine: Send + Sync {
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
