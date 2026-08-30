//! Binary layouts and batch framing per `KEI-DES-030`.

use crc32fast::Hasher;

/// Magic identifier for segment header/footer (0x4B57414C = 'KWAL').
pub const SEGMENT_MAGIC: u32 = 0x4B57414C;

/// Magic identifier for batch frame header (0x4B424154 = 'KBAT').
pub const BATCH_MAGIC: u32 = 0x4B424154;

/// Current binary format version.
pub const WAL_FORMAT_VERSION: u16 = 1;

// Batch Flags per KEI-DES-030 §5.3
/// Payload is encrypted.
pub const BATCH_FLAG_ENCRYPTED: u16 = 1 << 0;
/// Payload is compressed.
pub const BATCH_FLAG_COMPRESSED: u16 = 1 << 1;
/// Batch is part of a transaction.
pub const BATCH_FLAG_TRANSACTIONAL: u16 = 1 << 2;
/// Batch is a transaction commit marker.
pub const BATCH_FLAG_TXN_COMMIT: u16 = 1 << 3;
/// Batch is a transaction abort marker.
pub const BATCH_FLAG_TXN_ABORT: u16 = 1 << 4;
/// Batch contains at least one tombstone.
pub const BATCH_FLAG_CONTAINS_TOMBSTONES: u16 = 1 << 5;
/// Batch contains records from multiple streams.
pub const BATCH_FLAG_MULTI_STREAM: u16 = 1 << 6;
/// Batch was written during recovery replay.
pub const BATCH_FLAG_RECOVERY_DELTA: u16 = 1 << 7;

// Record Flags per KEI-DES-030 §6.2
/// Record is a deletion marker.
pub const RECORD_FLAG_TOMBSTONE: u16 = 1 << 0;
/// Record carries a schema override.
pub const RECORD_FLAG_SCHEMA_OVERRIDE: u16 = 1 << 2;
/// Record carries a causal lineage tag.
pub const RECORD_FLAG_CAUSAL_TAG: u16 = 1 << 3;

/// 4096-byte Segment Header for physical WAL segment files per `KEI-DES-030` §4.2.
#[repr(C, align(4096))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentHeader {
    /// Magic identifier (0x4B57414C = 'KWAL').
    pub magic: u32,
    /// Format version.
    pub format_version: u16,
    /// Bitflags for segment status.
    pub flags: u16,
    /// Monotonic segment identifier.
    pub segment_id: u64,
    /// Storage volume identifier.
    pub volume_id: u32,
    /// Owning node at creation.
    pub node_id: u32,
    /// Creation timestamp in Unix nanoseconds.
    pub created_timestamp_ns: u64,
    /// First physical sequence number in segment.
    pub physical_seq_start: u64,
    /// Last physical sequence number in segment (filled on seal).
    pub physical_seq_end: u64,
    /// Number of sealed batches.
    pub batch_count: u32,
    /// Total records across all batches.
    pub record_count: u64,
    /// CRC32C over header fields.
    pub segment_crc32c: u32,
    /// Reserved space to pad to exactly 4096 bytes.
    pub reserved: [u8; 4024],
}

impl SegmentHeader {
    /// Create a new open segment header.
    pub fn new(segment_id: u64, volume_id: u32, node_id: u32, physical_seq_start: u64) -> Self {
        let mut header = Self {
            magic: SEGMENT_MAGIC,
            format_version: WAL_FORMAT_VERSION,
            flags: 0,
            segment_id,
            volume_id,
            node_id,
            created_timestamp_ns: 0,
            physical_seq_start,
            physical_seq_end: 0,
            batch_count: 0,
            record_count: 0,
            segment_crc32c: 0,
            reserved: [0u8; 4024],
        };
        header.segment_crc32c = header.compute_crc();
        header
    }

    /// Compute CRC32C over segment header fields (excluding CRC itself and padding).
    pub fn compute_crc(&self) -> u32 {
        let mut hasher = Hasher::new();
        hasher.update(&self.magic.to_le_bytes());
        hasher.update(&self.format_version.to_le_bytes());
        hasher.update(&self.flags.to_le_bytes());
        hasher.update(&self.segment_id.to_le_bytes());
        hasher.update(&self.volume_id.to_le_bytes());
        hasher.update(&self.node_id.to_le_bytes());
        hasher.update(&self.created_timestamp_ns.to_le_bytes());
        hasher.update(&self.physical_seq_start.to_le_bytes());
        hasher.update(&self.physical_seq_end.to_le_bytes());
        hasher.update(&self.batch_count.to_le_bytes());
        hasher.update(&self.record_count.to_le_bytes());
        hasher.finalize()
    }

    /// Verify segment header validity.
    pub fn is_valid(&self) -> bool {
        self.magic == SEGMENT_MAGIC
            && self.format_version == WAL_FORMAT_VERSION
            && self.segment_crc32c == self.compute_crc()
    }
}

/// 4096-byte Segment Footer for sealed WAL segment files per `KEI-DES-030` §4.3.
#[repr(C, align(4096))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentFooter {
    /// Magic identifier (0x4B57414C = 'KWAL').
    pub magic: u32,
    /// Monotonic segment identifier matching header.
    pub segment_id: u64,
    /// Last physical sequence number committed in segment.
    pub physical_seq_end: u64,
    /// Total count of sealed batches in segment.
    pub batch_count: u32,
    /// Total record count across all batches.
    pub record_count: u64,
    /// Timestamp when segment was sealed in Unix nanoseconds.
    pub sealed_timestamp_ns: u64,
    /// CRC32C over footer fields.
    pub footer_crc32c: u32,
    /// Reserved space to pad to exactly 4096 bytes.
    pub reserved: [u8; 4040],
}

impl SegmentFooter {
    /// Create a new segment footer for a sealed segment.
    pub fn new(
        segment_id: u64,
        physical_seq_end: u64,
        batch_count: u32,
        record_count: u64,
        sealed_timestamp_ns: u64,
    ) -> Self {
        let mut footer = Self {
            magic: SEGMENT_MAGIC,
            segment_id,
            physical_seq_end,
            batch_count,
            record_count,
            sealed_timestamp_ns,
            footer_crc32c: 0,
            reserved: [0u8; 4040],
        };
        footer.footer_crc32c = footer.compute_crc();
        footer
    }

    /// Compute CRC32C over footer fields (excluding CRC itself and padding).
    pub fn compute_crc(&self) -> u32 {
        let mut hasher = Hasher::new();
        hasher.update(&self.magic.to_le_bytes());
        hasher.update(&self.segment_id.to_le_bytes());
        hasher.update(&self.physical_seq_end.to_le_bytes());
        hasher.update(&self.batch_count.to_le_bytes());
        hasher.update(&self.record_count.to_le_bytes());
        hasher.update(&self.sealed_timestamp_ns.to_le_bytes());
        hasher.finalize()
    }

    /// Verify segment footer validity.
    pub fn is_valid(&self) -> bool {
        self.magic == SEGMENT_MAGIC && self.footer_crc32c == self.compute_crc()
    }
}

/// 128-byte Batch Header for physical WAL segments per `KEI-DES-030` §5.2.
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchHeader {
    /// Magic identifier (0x4B424154 = 'KBAT').
    pub magic: u32,
    /// Schema/framing version.
    pub version: u16,
    /// Header flags (see §5.3).
    pub flags: u16,
    /// Total batch size including this header and record entries.
    pub total_batch_size: u32,
    /// Number of records encapsulated in this batch.
    pub record_count: u32,
    /// Base physical offset in stream.
    pub base_offset: u64,
    /// Last physical offset in batch.
    pub last_offset: u64,
    /// Ingress timestamp (microseconds).
    pub timestamp_us: u64,
    /// CRC32C over header fields.
    pub header_crc: u32,
    /// CRC32C over payload records.
    pub payload_crc: u32,
    /// Reserved space to fill 128-byte alignment.
    pub _reserved: [u8; 64],
}

impl BatchHeader {
    /// Create a new batch header.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        flags: u16,
        total_batch_size: u32,
        record_count: u32,
        base_offset: u64,
        last_offset: u64,
        timestamp_us: u64,
        payload_crc: u32,
    ) -> Self {
        let mut header = Self {
            magic: BATCH_MAGIC,
            version: WAL_FORMAT_VERSION,
            flags,
            total_batch_size,
            record_count,
            base_offset,
            last_offset,
            timestamp_us,
            header_crc: 0,
            payload_crc,
            _reserved: [0u8; 64],
        };
        header.header_crc = header.compute_header_crc();
        header
    }

    /// Compute CRC32C over header bytes (excluding CRC fields).
    pub fn compute_header_crc(&self) -> u32 {
        let mut hasher = Hasher::new();
        hasher.update(&self.magic.to_le_bytes());
        hasher.update(&self.version.to_le_bytes());
        hasher.update(&self.flags.to_le_bytes());
        hasher.update(&self.total_batch_size.to_le_bytes());
        hasher.update(&self.record_count.to_le_bytes());
        hasher.update(&self.base_offset.to_le_bytes());
        hasher.update(&self.last_offset.to_le_bytes());
        hasher.update(&self.timestamp_us.to_le_bytes());
        hasher.finalize()
    }

    /// Verify header integrity.
    pub fn is_valid(&self) -> bool {
        self.magic == BATCH_MAGIC
            && self.version == WAL_FORMAT_VERSION
            && self.header_crc == self.compute_header_crc()
            && self.base_offset <= self.last_offset
    }
}

/// 46-byte Record Entry pointing into payload block per `KEI-DES-030` §6.1.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordEntry {
    /// Logical micro-stream identifier.
    pub stream_id: [u8; 16],
    /// Monotonic per-stream logical offset.
    pub logical_offset: u64,
    /// Delta from batch producer_seq_start.
    pub producer_seq_delta: u32,
    /// Byte offset into payload block.
    pub payload_offset: u32,
    /// Payload length in bytes.
    pub payload_len: u32,
    /// Delta from batch timestamp (milliseconds).
    pub timestamp_delta_ms: u32,
    /// Per-record flags.
    pub record_flags: u16,
    /// CRC32C of record entry.
    pub record_crc32c: u32,
}

impl RecordEntry {
    /// Create a new record entry.
    pub fn new(
        stream_id: [u8; 16],
        logical_offset: u64,
        payload_offset: u32,
        payload_len: u32,
        record_flags: u16,
    ) -> Self {
        let mut entry = Self {
            stream_id,
            logical_offset,
            producer_seq_delta: 0,
            payload_offset,
            payload_len,
            timestamp_delta_ms: 0,
            record_flags,
            record_crc32c: 0,
        };
        entry.record_crc32c = entry.compute_crc();
        entry
    }

    /// Create a full record entry with sequence and timestamp deltas.
    pub fn with_deltas(
        stream_id: [u8; 16],
        logical_offset: u64,
        producer_seq_delta: u32,
        payload_offset: u32,
        payload_len: u32,
        timestamp_delta_ms: u32,
        record_flags: u16,
    ) -> Self {
        let mut entry = Self {
            stream_id,
            logical_offset,
            producer_seq_delta,
            payload_offset,
            payload_len,
            timestamp_delta_ms,
            record_flags,
            record_crc32c: 0,
        };
        entry.record_crc32c = entry.compute_crc();
        entry
    }

    /// Compute CRC32C over record entry fields.
    pub fn compute_crc(&self) -> u32 {
        let mut hasher = Hasher::new();
        hasher.update(&self.stream_id);
        hasher.update(&self.logical_offset.to_le_bytes());
        hasher.update(&self.producer_seq_delta.to_le_bytes());
        hasher.update(&self.payload_offset.to_le_bytes());
        hasher.update(&self.payload_len.to_le_bytes());
        hasher.update(&self.timestamp_delta_ms.to_le_bytes());
        hasher.update(&self.record_flags.to_le_bytes());
        hasher.finalize()
    }

    /// Verify record entry integrity.
    pub fn is_valid(&self) -> bool {
        self.compute_crc() == { self.record_crc32c }
    }

    /// Return the logical offset.
    pub fn logical_offset(&self) -> u64 {
        self.logical_offset
    }

    /// Return the payload length.
    pub fn payload_len(&self) -> u32 {
        self.payload_len
    }

    /// Return the payload offset.
    pub fn payload_offset(&self) -> u32 {
        self.payload_offset
    }

    /// Return the producer sequence delta.
    pub fn producer_seq_delta(&self) -> u32 {
        self.producer_seq_delta
    }

    /// Return the timestamp delta in milliseconds.
    pub fn timestamp_delta_ms(&self) -> u32 {
        self.timestamp_delta_ms
    }

    /// Return the record flags.
    pub fn record_flags(&self) -> u16 {
        self.record_flags
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::align_of;

    #[test]
    fn test_segment_header_layout_invariants() {
        assert_eq!(
            size_of::<SegmentHeader>(),
            4096,
            "SegmentHeader must be exactly 4096 bytes per KEI-DES-030 §4.2"
        );
        assert_eq!(
            align_of::<SegmentHeader>(),
            4096,
            "SegmentHeader must be 4096-byte aligned for O_DIRECT NVMe page alignment"
        );

        let seg = SegmentHeader::new(1, 10, 100, 0);
        assert!(seg.is_valid());
        assert_eq!(seg.magic, SEGMENT_MAGIC);
    }

    #[test]
    fn test_batch_header_layout_invariants() {
        assert_eq!(
            size_of::<BatchHeader>(),
            128,
            "BatchHeader must be exactly 128 bytes per KEI-DES-030 §5.2"
        );
        assert_eq!(
            align_of::<BatchHeader>(),
            64,
            "BatchHeader must be 64-byte aligned for cache-line alignment"
        );
    }

    #[test]
    fn test_record_entry_layout_and_crc() {
        assert_eq!(
            size_of::<RecordEntry>(),
            46,
            "RecordEntry must be exactly 46 bytes per KEI-DES-030 §6.1"
        );
        let stream = [0xAA; 16];
        let record = RecordEntry::new(stream, 42, 0, 1024, 0);
        assert!(record.is_valid());
        assert_eq!(record.logical_offset(), 42);
        assert_eq!(record.payload_len(), 1024);
    }

    #[test]
    fn test_batch_header_creation_and_validation() {
        let header = BatchHeader::new(
            BATCH_FLAG_COMPRESSED,
            512,
            10,
            100,
            109,
            1700000000000000,
            0x12345678,
        );
        assert!(header.is_valid());
        assert_eq!(header.magic, BATCH_MAGIC);
        assert_eq!(header.version, WAL_FORMAT_VERSION);
        assert_eq!(header.record_count, 10);
        assert_eq!(header.base_offset, 100);
        assert_eq!(header.last_offset, 109);
    }

    #[test]
    fn test_batch_header_crc_corruption_detection() {
        let mut header = BatchHeader::new(0, 512, 10, 100, 109, 1700000000000000, 0x12345678);
        assert!(header.is_valid());

        // Corrupt a field
        header.record_count = 999;
        assert!(
            !header.is_valid(),
            "Corrupted header must fail CRC validation"
        );
    }
}
