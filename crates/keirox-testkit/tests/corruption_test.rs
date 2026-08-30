//! Corruption detection and fail-fast validation tests per `KEI-DES-030` §7.

use keirox_wal::framing::{BatchHeader, RecordEntry};
use keirox_wal::recovery::RecoveryReconciler;
use keirox_wal::segment::{SegmentFile, SegmentReader};
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::mem::size_of;
use tempfile::tempdir;

#[test]
fn test_segment_header_bit_flip_fails_fast() {
    let dir = tempdir().unwrap();
    let seg_path = dir.path().join("0000000000000001.kwal");

    // 1. Create a valid segment
    {
        let mut seg = SegmentFile::create(&seg_path, 1, 1, 100, 0).unwrap();
        let stream = [0x99; 16];
        let records = vec![RecordEntry::new(stream, 0, 0, 16, 0)];
        let payload = vec![0xCC; 16];
        let total_size = (size_of::<BatchHeader>()
            + (records.len() * size_of::<RecordEntry>())
            + payload.len()) as u32;
        let header = BatchHeader::new(0, total_size, 1, 0, 0, 1000, 0);
        seg.append_batch(&header, &records, &payload).unwrap();
        seg.seal(1001).unwrap();
    }

    // 2. Corrupt segment header magic bytes
    {
        let mut file = OpenOptions::new().write(true).open(&seg_path).unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();
        file.write_all(&[0xFF, 0xFF, 0xFF, 0xFF]).unwrap();
    }

    // 3. Assert SegmentReader rejects it immediately
    let result = SegmentReader::open(&seg_path);
    assert!(
        result.is_err(),
        "Segment reader must reject corrupted magic"
    );

    // 4. Assert RecoveryReconciler fails fast
    let reconciler = RecoveryReconciler::new(dir.path());
    let rec_result = reconciler.replay_all(|_| Ok(()));
    assert!(
        rec_result.is_err(),
        "Reconciler must reject corrupt segment file"
    );
}

#[test]
fn test_batch_header_crc_corruption_halts_replay_cleanly() {
    let dir = tempdir().unwrap();
    let seg_path = dir.path().join("0000000000000001.kwal");

    // 1. Create segment with 1 batch
    {
        let mut seg = SegmentFile::create(&seg_path, 1, 1, 100, 0).unwrap();
        let stream = [0x99; 16];
        let records = vec![RecordEntry::new(stream, 0, 0, 16, 0)];
        let payload = vec![0xCC; 16];
        let total_size = (size_of::<BatchHeader>()
            + (records.len() * size_of::<RecordEntry>())
            + payload.len()) as u32;
        let header = BatchHeader::new(0, total_size, 1, 0, 0, 1000, 0);
        seg.append_batch(&header, &records, &payload).unwrap();
        seg.seal(1001).unwrap();
    }

    // 2. Corrupt batch header CRC (located at offset 4096 + 36 approx)
    {
        let mut file = OpenOptions::new().write(true).open(&seg_path).unwrap();
        file.seek(SeekFrom::Start(4096 + 8)).unwrap(); // Corrupt record_count or flags
        file.write_all(&[0xEE, 0xEE]).unwrap();
    }

    // 3. Assert SegmentReader stops/fails at corrupted batch
    let mut reader = SegmentReader::open(&seg_path).unwrap();
    let replay_result = reader.replay_batches();
    assert!(
        replay_result.is_err(),
        "Replay must fail CRC check on corrupted batch frame"
    );
}
