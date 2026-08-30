//! Binary layouts and batch framing per `KEI-DES-030`.

use crc32fast::Hasher;

/// Canonical magic constant for Keirox WAL batches ('KEIR' in ASCII).
pub const WAL_MAGIC: u32 = 0x4B454952;

/// Current binary format version.
pub const WAL_FORMAT_VERSION: u16 = 1;

/// 128-byte Batch Header for physical WAL segments.
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchHeader {
    /// Magic identifier (0x4B454952 = 'KEIR').
    pub magic: u32,
    /// Schema/framing version.
    pub version: u16,
    /// Header flags.
    pub flags: u16,
    /// Total batch size including this header.
    pub total_batch_size: u32,
    /// Number of records encapsulated.
    pub record_count: u32,
    /// Base physical offset.
    pub base_offset: u64,
    /// Last physical offset in batch.
    pub last_offset: u64,
    /// Ingress timestamp (microseconds).
    pub timestamp_us: u64,
    /// CRC32C over header (bytes 0..48).
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
            magic: WAL_MAGIC,
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

    /// Compute CRC32C over header bytes (0..48 excluding crc fields).
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
        self.magic == WAL_MAGIC
            && self.version == WAL_FORMAT_VERSION
            && self.header_crc == self.compute_header_crc()
            && self.base_offset <= self.last_offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{align_of, size_of};

    #[test]
    fn test_batch_header_layout_invariants() {
        assert_eq!(
            size_of::<BatchHeader>(),
            128,
            "BatchHeader must be exactly 128 bytes per KEI-DES-030 §3"
        );
        assert_eq!(
            align_of::<BatchHeader>(),
            64,
            "BatchHeader must be 64-byte aligned for cache-line alignment"
        );
    }

    #[test]
    fn test_batch_header_creation_and_validation() {
        let header = BatchHeader::new(0, 512, 10, 100, 109, 1700000000000000, 0x12345678);
        assert!(header.is_valid());
        assert_eq!(header.magic, WAL_MAGIC);
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
