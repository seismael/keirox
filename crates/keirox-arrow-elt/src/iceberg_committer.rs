//! Apache Iceberg Catalog Committer, manifest list coordinator, and snapshot governor per `KEI-DES-034` §7.

use crate::catalog::{CatalogSnapshot, DataFileEntry, IcebergCatalogLedger};
use keirox_core::error::{KeiroxError, Result};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Governed table commit cadence mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitCadenceMode {
    /// Standard batch lakehouse commit (≤60 seconds).
    Standard,
    /// Fast near-real-time streaming commit (≤5 seconds).
    FastStreaming,
}

/// Productionized Iceberg table catalog committer managing multi-stream snapshot ledgers.
#[derive(Debug, Default)]
pub struct IcebergCatalogCommitter {
    tables: RwLock<HashMap<String, IcebergCatalogLedger>>,
    commit_modes: RwLock<HashMap<String, CommitCadenceMode>>,
}

impl IcebergCatalogCommitter {
    /// Initialize a new Iceberg Catalog Committer.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a table with a target commit cadence mode.
    pub fn register_table(&self, table_name: &str, mode: CommitCadenceMode) {
        let mut tables = self.tables.write().unwrap();
        tables
            .entry(table_name.to_string())
            .or_insert_with(|| IcebergCatalogLedger::new(table_name));

        let mut modes = self.commit_modes.write().unwrap();
        modes.insert(table_name.to_string(), mode);
    }

    /// Commit a batch of sealed Parquet data files to an Iceberg table ledger with optimistic concurrency check.
    pub fn commit_data_files(
        &self,
        table_name: &str,
        expected_parent_snapshot_id: Option<u64>,
        new_files: Vec<DataFileEntry>,
        timestamp_ms: u64,
    ) -> Result<CatalogSnapshot> {
        let mut tables = self.tables.write().unwrap();
        let ledger = tables.get_mut(table_name).ok_or_else(|| {
            KeiroxError::Internal(format!(
                "Table '{table_name}' not registered in Iceberg catalog"
            ))
        })?;

        // Optimistic Concurrency Control (OCC)
        let current_parent = ledger.current_snapshot().map(|s| s.snapshot_id);
        if current_parent != expected_parent_snapshot_id {
            return Err(KeiroxError::Internal(format!(
                "Iceberg commit conflict on table '{table_name}': expected parent {expected_parent_snapshot_id:?}, found {current_parent:?}"
            )));
        }

        let snapshot = ledger.commit_snapshot(new_files, timestamp_ms);
        Ok(snapshot)
    }

    /// Expire snapshots older than `retention_cutoff_ms` and prune metadata.
    pub fn expire_snapshots(&self, table_name: &str, retention_cutoff_ms: u64) -> Result<usize> {
        let mut tables = self.tables.write().unwrap();
        let ledger = tables
            .get_mut(table_name)
            .ok_or_else(|| KeiroxError::Internal(format!("Table '{table_name}' not found")))?;

        let removed = ledger.expire_snapshots_before(retention_cutoff_ms);
        Ok(removed)
    }

    /// Query the current snapshot for a table.
    pub fn current_snapshot(&self, table_name: &str) -> Option<CatalogSnapshot> {
        let tables = self.tables.read().unwrap();
        tables
            .get(table_name)
            .and_then(|l| l.current_snapshot().cloned())
    }
}

/// Shared reference to Iceberg Committer.
pub type SharedIcebergCommitter = Arc<IcebergCatalogCommitter>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iceberg_committer_occ_and_snapshot_lifecycle() {
        let committer = IcebergCatalogCommitter::new();
        committer.register_table("events_tbl", CommitCadenceMode::Standard);

        let files1 = vec![DataFileEntry {
            file_path: "s3://lake/events-001.parquet".into(),
            record_count: 1000,
            file_size_bytes: 64 * 1024 * 1024,
            partition_spec_id: 0,
        }];

        // 1. Initial commit (expected parent None)
        let snap1 = committer
            .commit_data_files("events_tbl", None, files1, 1700000000000)
            .unwrap();
        assert_eq!(snap1.snapshot_id, 1);

        // 2. Conflict commit (stale parent None when parent is 1)
        let files_conflict = vec![DataFileEntry {
            file_path: "s3://lake/events-conflict.parquet".into(),
            record_count: 500,
            file_size_bytes: 32 * 1024 * 1024,
            partition_spec_id: 0,
        }];
        let err = committer.commit_data_files("events_tbl", None, files_conflict, 1700000030000);
        assert!(err.is_err());

        // 3. Valid contiguous commit (parent Some(1))
        let files2 = vec![DataFileEntry {
            file_path: "s3://lake/events-002.parquet".into(),
            record_count: 500,
            file_size_bytes: 32 * 1024 * 1024,
            partition_spec_id: 0,
        }];
        let snap2 = committer
            .commit_data_files("events_tbl", Some(1), files2, 1700000060000)
            .unwrap();
        assert_eq!(snap2.snapshot_id, 2);
        assert_eq!(snap2.total_records, 1500);

        // 4. Test snapshot expiration (cutoff between snap1 and snap2)
        let removed = committer
            .expire_snapshots("events_tbl", 1700000030000)
            .unwrap();
        assert_eq!(removed, 1);
        // Current snapshot is still preserved
        let current = committer.current_snapshot("events_tbl").unwrap();
        assert_eq!(current.snapshot_id, 2);
    }
}
