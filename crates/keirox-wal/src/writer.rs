//! Segment WAL engine implementations per `KEI-DES-030`.

use keirox_core::error::Result;
use keirox_core::model::{Offset, StreamId};
use keirox_core::traits::WalEngine;
use std::collections::HashMap;

/// In-memory WAL engine implementation for tests and prototype verification.
#[derive(Debug, Default)]
pub struct InMemoryWalEngine {
    streams: HashMap<StreamId, Vec<u8>>,
    offsets: HashMap<StreamId, Offset>,
}

impl InMemoryWalEngine {
    /// Create a new in-memory WAL engine.
    pub fn new() -> Self {
        Self::default()
    }
}

impl WalEngine for InMemoryWalEngine {
    fn append_batch(&mut self, stream_id: StreamId, batch: &[u8]) -> Result<Offset> {
        let entry = self.streams.entry(stream_id).or_default();
        entry.extend_from_slice(batch);

        let current_offset = self.offsets.entry(stream_id).or_insert(0);
        let assigned_offset = *current_offset;
        *current_offset += 1;

        Ok(assigned_offset)
    }

    fn read_records(
        &self,
        stream_id: StreamId,
        _start_offset: Offset,
        _max_records: usize,
    ) -> Result<Vec<u8>> {
        let bytes = self.streams.get(&stream_id).cloned().unwrap_or_default();
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framing::BatchHeader;

    #[test]
    fn test_in_memory_wal_engine_append_and_read() {
        let mut engine = InMemoryWalEngine::new();
        let stream = StreamId([0xEE; 16]);

        let batch_header = BatchHeader::new(0, 128, 1, 0, 0, 1000, 0);
        let header_bytes = unsafe {
            std::slice::from_raw_parts(
                &batch_header as *const _ as *const u8,
                std::mem::size_of::<BatchHeader>(),
            )
        };

        let offset = engine.append_batch(stream, header_bytes).unwrap();
        assert_eq!(offset, 0);

        let read_bytes = engine.read_records(stream, 0, 10).unwrap();
        assert_eq!(read_bytes.len(), 128);
    }
}
