//! Stream registry entries aligned to 32 bytes per `KEI-ARC-020` §6.1.

use keirox_core::StreamId;

/// Packed 32-byte Stream Registry Entry (exact 32-byte layout).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamRegistryEntry {
    /// Unique 16-byte stream identifier.
    pub stream_id: [u8; 16],
    /// Current head offset for stream.
    pub head_offset: u64,
    /// Identifier of the currently active WAL segment.
    pub active_segment_id: u32,
    /// Bitflags for stream state.
    pub flags: u16,
    /// Reserved space for 32-byte total size.
    pub _reserved: u16,
}

impl StreamRegistryEntry {
    /// Create a new stream registry entry.
    pub fn new(stream_id: StreamId, active_segment_id: u32) -> Self {
        Self {
            stream_id: stream_id.0,
            head_offset: 0,
            active_segment_id,
            flags: 0,
            _reserved: 0,
        }
    }

    /// Return the typed StreamId.
    pub fn id(&self) -> StreamId {
        StreamId(self.stream_id)
    }

    /// Advance head offset.
    pub fn advance_head(&mut self, next_offset: u64) {
        self.head_offset = next_offset;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn test_stream_registry_entry_size_invariant() {
        assert_eq!(
            size_of::<StreamRegistryEntry>(),
            32,
            "StreamRegistryEntry must be exactly 32 bytes per KEI-ARC-020 §6.1"
        );
    }

    #[test]
    fn test_stream_registry_entry_lifecycle() {
        let stream = StreamId([0x42; 16]);
        let mut entry = StreamRegistryEntry::new(stream, 1);
        assert_eq!(entry.id(), stream);
        assert_eq!(entry.head_offset, 0);
        assert_eq!(entry.active_segment_id, 1);

        entry.advance_head(100);
        assert_eq!(entry.head_offset, 100);
    }
}
