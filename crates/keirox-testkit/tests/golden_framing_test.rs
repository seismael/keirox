//! Golden master byte-for-byte serialization tests for WAL framing per `KEI-DES-030`.

use keirox_wal::framing::{
    BatchHeader, SegmentHeader, BATCH_FLAG_COMPRESSED, BATCH_MAGIC, SEGMENT_MAGIC,
    WAL_FORMAT_VERSION,
};
use std::mem::size_of;

#[test]
fn test_golden_segment_header_binary_layout() {
    assert_eq!(size_of::<SegmentHeader>(), 4096);
    let seg = SegmentHeader::new(100, 1, 2, 0);
    assert_eq!(seg.magic, SEGMENT_MAGIC);
    assert_eq!(seg.format_version, WAL_FORMAT_VERSION);
    assert_eq!(seg.segment_id, 100);
    assert_eq!(seg.volume_id, 1);
    assert_eq!(seg.node_id, 2);
    assert!(seg.is_valid());
}

#[test]
fn test_golden_batch_header_binary_layout() {
    assert_eq!(size_of::<BatchHeader>(), 128);

    let header = BatchHeader::new(
        BATCH_FLAG_COMPRESSED, // flags
        256,                   // total_batch_size
        5,                     // record_count
        1000,                  // base_offset
        1004,                  // last_offset
        1700000000000000,      // timestamp_us
        0xDEADBEEF,            // payload_crc
    );

    assert_eq!(header.magic, BATCH_MAGIC);
    assert_eq!(header.version, WAL_FORMAT_VERSION);
    assert_eq!(header.flags, BATCH_FLAG_COMPRESSED);
    assert_eq!(header.total_batch_size, 256);
    assert_eq!(header.record_count, 5);
    assert_eq!(header.base_offset, 1000);
    assert_eq!(header.last_offset, 1004);
    assert_eq!(header.payload_crc, 0xDEADBEEF);
    assert!(header.is_valid());
}

#[test]
fn test_golden_record_entry_binary_layout() {
    use keirox_wal::framing::RecordEntry;
    assert_eq!(
        size_of::<RecordEntry>(),
        46,
        "RecordEntry must be exactly 46 bytes per KEI-DES-030 §6.1"
    );

    let stream = [0x77; 16];
    let entry = RecordEntry::with_deltas(stream, 500, 10, 64, 128, 5, 0x0001);
    assert_eq!(entry.logical_offset(), 500);
    assert_eq!(entry.producer_seq_delta(), 10);
    assert_eq!(entry.payload_offset(), 64);
    assert_eq!(entry.payload_len(), 128);
    assert_eq!(entry.timestamp_delta_ms(), 5);
    assert_eq!(entry.record_flags(), 0x0001);
    assert!(entry.is_valid());
}
