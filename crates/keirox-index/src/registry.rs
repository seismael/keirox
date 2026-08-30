//! Stream registry entries aligned to 32 bytes per `KEI-ARC-020` §6.1.

/// Packed 32-byte Stream Registry Entry.
#[repr(C, packed)]
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
