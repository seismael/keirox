//! Crash recovery and segment replay reconciliation per `KEI-DES-030` §7.2.

use crate::segment::{ReplayedBatch, SegmentReader};
use keirox_core::error::{KeiroxError, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Summary of recovered WAL segments after replay.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RecoveryReport {
    /// Total segment files successfully scanned.
    pub segments_scanned: usize,
    /// Total batch frames verified and replayed.
    pub batches_replayed: usize,
    /// Total individual records reconstructed.
    pub records_reconstructed: u64,
    /// Last verified physical sequence number.
    pub last_physical_seq: u64,
}

/// Recovery engine for discovering and replaying physical WAL segments.
pub struct RecoveryReconciler {
    wal_dir: PathBuf,
}

impl RecoveryReconciler {
    /// Create a new recovery reconciler for a given WAL directory.
    pub fn new<P: AsRef<Path>>(wal_dir: P) -> Self {
        Self {
            wal_dir: wal_dir.as_ref().to_path_buf(),
        }
    }

    /// Discover and sort all `.kwal` segment files in the WAL directory.
    pub fn discover_segments(&self) -> Result<Vec<PathBuf>> {
        if !self.wal_dir.exists() {
            return Ok(Vec::new());
        }

        let mut segments = Vec::new();
        for entry in fs::read_dir(&self.wal_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("kwal") {
                segments.push(path);
            }
        }

        // Sort lexically / numerically by segment filename
        segments.sort();
        Ok(segments)
    }

    /// Replay all segments in order and execute a callback for each verified batch.
    pub fn replay_all<F>(&self, mut on_batch: F) -> Result<RecoveryReport>
    where
        F: FnMut(&ReplayedBatch) -> Result<()>,
    {
        let segments = self.discover_segments()?;
        let mut report = RecoveryReport::default();

        for segment_path in segments {
            let mut reader = match SegmentReader::open(&segment_path) {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(
                        "Skipping corrupt segment file {}: {:?}",
                        segment_path.display(),
                        e
                    );
                    return Err(KeiroxError::Internal(format!(
                        "Fatal corruption in segment header: {}",
                        segment_path.display()
                    )));
                }
            };

            let batches = reader.replay_batches()?;
            for batch in &batches {
                on_batch(batch)?;
                report.batches_replayed += 1;
                report.records_reconstructed += batch.records.len() as u64;
                report.last_physical_seq = batch.header.last_offset;
            }

            report.segments_scanned += 1;
        }

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framing::{BatchHeader, RecordEntry};
    use crate::segment::SegmentFile;
    use std::mem::size_of;
    use tempfile::tempdir;

    #[test]
    fn test_recovery_reconciler_replays_multiple_segments() {
        let dir = tempdir().unwrap();
        let wal_dir = dir.path();

        // Write Segment 1
        let seg1_path = wal_dir.join("0000000000000001.kwal");
        {
            let mut seg1 = SegmentFile::create(&seg1_path, 1, 1, 1, 0).unwrap();
            let stream = [0x11; 16];
            let records = vec![RecordEntry::new(stream, 0, 0, 8, 0)];
            let payload = vec![0xAA; 8];
            let total_size = (size_of::<BatchHeader>()
                + (records.len() * size_of::<RecordEntry>())
                + payload.len()) as u32;
            let header = BatchHeader::new(0, total_size, 1, 0, 0, 1000, 0);
            seg1.append_batch(&header, &records, &payload).unwrap();
            seg1.seal(1001).unwrap();
        }

        // Write Segment 2
        let seg2_path = wal_dir.join("0000000000000002.kwal");
        {
            let mut seg2 = SegmentFile::create(&seg2_path, 2, 1, 1, 1).unwrap();
            let stream = [0x11; 16];
            let records = vec![
                RecordEntry::new(stream, 1, 0, 8, 0),
                RecordEntry::new(stream, 2, 8, 8, 0),
            ];
            let payload = vec![0xBB; 16];
            let total_size = (size_of::<BatchHeader>()
                + (records.len() * size_of::<RecordEntry>())
                + payload.len()) as u32;
            let header = BatchHeader::new(0, total_size, 2, 1, 2, 2000, 0);
            seg2.append_batch(&header, &records, &payload).unwrap();
            seg2.seal(2001).unwrap();
        }

        // Run Reconciler
        let reconciler = RecoveryReconciler::new(wal_dir);
        let mut replayed_offsets = Vec::new();

        let report = reconciler
            .replay_all(|batch| {
                for r in &batch.records {
                    replayed_offsets.push(r.logical_offset);
                }
                Ok(())
            })
            .unwrap();

        assert_eq!(report.segments_scanned, 2);
        assert_eq!(report.batches_replayed, 2);
        assert_eq!(report.records_reconstructed, 3);
        assert_eq!(report.last_physical_seq, 2);
        assert_eq!(replayed_offsets, vec![0, 1, 2]);
    }
}
