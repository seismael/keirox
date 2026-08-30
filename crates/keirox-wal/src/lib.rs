//! # Keirox WAL
//!
//! Write-Ahead Log implementation targeting `io_uring` + `O_DIRECT` NVMe storage.
//! Follows binary format specifications in `KEI-DES-030`.

#![deny(missing_docs)]

/// WAL batch framing and binary layout structures.
pub mod framing;
/// Crash recovery and segment replay reconciliation.
pub mod recovery;
/// Physical WAL segment file management and replay.
pub mod segment;
/// WAL writer engines.
pub mod writer;

pub use framing::{
    BatchHeader, RecordEntry, SegmentFooter, SegmentHeader, BATCH_FLAG_COMPRESSED,
    BATCH_FLAG_CONTAINS_TOMBSTONES, BATCH_FLAG_ENCRYPTED, BATCH_FLAG_MULTI_STREAM,
    BATCH_FLAG_RECOVERY_DELTA, BATCH_FLAG_TRANSACTIONAL, BATCH_FLAG_TXN_ABORT,
    BATCH_FLAG_TXN_COMMIT, BATCH_MAGIC, RECORD_FLAG_CAUSAL_TAG, RECORD_FLAG_SCHEMA_OVERRIDE,
    RECORD_FLAG_TOMBSTONE, SEGMENT_MAGIC, WAL_FORMAT_VERSION,
};
pub use recovery::{RecoveryReconciler, RecoveryReport};
pub use segment::{ReplayedBatch, SegmentFile, SegmentReader, DEFAULT_SEGMENT_SIZE, PAGE_SIZE};
pub use writer::InMemoryWalEngine;
