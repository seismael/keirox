//! Crash recovery and memory state machine reconstruction test per `KEI-DES-030` and `KEI-DES-031`.

use keirox_core::StreamId;
use keirox_index::StreamRegistryEntry;
use keirox_state::ConsumerGroupState;
use keirox_wal::framing::{BatchHeader, RecordEntry};
use keirox_wal::recovery::RecoveryReconciler;
use keirox_wal::segment::SegmentFile;
use std::mem::size_of;
use tempfile::tempdir;

#[test]
fn test_crash_recovery_reconstructs_stream_registry_and_consumer_state() {
    let dir = tempdir().unwrap();
    let wal_dir = dir.path();
    let stream_raw = [0x55; 16];

    // 1. Write Segment 1 (Batches 0..5, sealed)
    let seg1_path = wal_dir.join("0000000000000001.kwal");
    {
        let mut seg1 = SegmentFile::create(&seg1_path, 1, 1, 100, 0).unwrap();
        for i in 0..5 {
            let records = vec![RecordEntry::new(stream_raw, i, 0, 16, 0)];
            let payload = vec![0xAA; 16];
            let total_size = (size_of::<BatchHeader>()
                + (records.len() * size_of::<RecordEntry>())
                + payload.len()) as u32;
            let header = BatchHeader::new(0, total_size, 1, i, i, 1000 + i, 0);
            seg1.append_batch(&header, &records, &payload).unwrap();
        }
        seg1.seal(1005).unwrap();
    }

    // 2. Write Segment 2 (Batches 5..8, unsealed - simulated crash)
    let seg2_path = wal_dir.join("0000000000000002.kwal");
    {
        let mut seg2 = SegmentFile::create(&seg2_path, 2, 1, 100, 5).unwrap();
        for i in 5..8 {
            let records = vec![RecordEntry::new(stream_raw, i, 0, 16, 0)];
            let payload = vec![0xBB; 16];
            let total_size = (size_of::<BatchHeader>()
                + (records.len() * size_of::<RecordEntry>())
                + payload.len()) as u32;
            let header = BatchHeader::new(0, total_size, 1, i, i, 2000 + i, 0);
            seg2.append_batch(&header, &records, &payload).unwrap();
        }
        // Segment 2 left unsealed (simulated node crash)
    }

    // 3. Perform Replay & State Reconstruction
    let reconciler = RecoveryReconciler::new(wal_dir);
    let mut registry_entry = StreamRegistryEntry::new(StreamId(stream_raw), 1);
    let mut consumer_state = ConsumerGroupState::new();

    let report = reconciler
        .replay_all(|batch| {
            for record in &batch.records {
                registry_entry.advance_head(record.logical_offset);
                consumer_state.head_offset = record.logical_offset;
            }
            Ok(())
        })
        .expect("Recovery must succeed across sealed and open segments");

    assert_eq!(report.segments_scanned, 2);
    assert_eq!(report.batches_replayed, 8);
    assert_eq!(report.records_reconstructed, 8);
    assert_eq!(report.last_physical_seq, 7);

    // Verify reconstructed stream registry state
    assert_eq!(registry_entry.head_offset, 7);

    // Verify reconstructed consumption state
    assert_eq!(consumer_state.head_offset, 7);
    assert_eq!(consumer_state.base_watermark, 0);

    // Verify consumption operations work seamlessly post-recovery
    let token = consumer_state.lease(0, 5000).expect("Must be leasable");
    consumer_state.ack_fenced(0, token).unwrap();
    assert_eq!(consumer_state.base_watermark, 1);
    consumer_state.verify_invariants().unwrap();
}
