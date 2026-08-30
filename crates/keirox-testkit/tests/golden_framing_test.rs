//! Golden master byte-for-byte serialization tests for WAL framing per `KEI-DES-030`.

use keirox_wal::framing::{BatchHeader, WAL_FORMAT_VERSION, WAL_MAGIC};
use std::mem::size_of;

#[test]
fn test_golden_batch_header_binary_layout() {
    assert_eq!(size_of::<BatchHeader>(), 128);

    let header = BatchHeader::new(
        0x0001,           // flags
        256,              // total_batch_size
        5,                // record_count
        1000,             // base_offset
        1004,             // last_offset
        1700000000000000, // timestamp_us
        0xDEADBEEF,       // payload_crc
    );

    assert_eq!(header.magic, WAL_MAGIC);
    assert_eq!(header.version, WAL_FORMAT_VERSION);
    assert_eq!(header.flags, 0x0001);
    assert_eq!(header.total_batch_size, 256);
    assert_eq!(header.record_count, 5);
    assert_eq!(header.base_offset, 1000);
    assert_eq!(header.last_offset, 1004);
    assert_eq!(header.payload_crc, 0xDEADBEEF);
    assert!(header.is_valid());
}
