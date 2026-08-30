//! Binary layouts and batch framing per `KEI-DES-030`.

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
