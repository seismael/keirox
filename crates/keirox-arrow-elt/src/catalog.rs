//! Apache Iceberg table catalog commit structures and ledger per `KEI-DES-034`.

use serde::{Deserialize, Serialize};

/// Metadata entry representing a sealed, lakehouse-registered Parquet data file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataFileEntry {
    /// URI or local path to Parquet file.
    pub file_path: String,
    /// Total row count in file.
    pub record_count: u64,
    /// Total file size in bytes.
    pub file_size_bytes: u64,
    /// Partition specification identifier (0 = unpartitioned).
    pub partition_spec_id: u32,
}

/// Atomic Iceberg table snapshot representing a consistent lakehouse state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogSnapshot {
    /// Monotonic unique 64-bit snapshot identifier.
    pub snapshot_id: u64,
    /// Parent snapshot identifier (None for initial snapshot).
    pub parent_snapshot_id: Option<u64>,
    /// Commit timestamp in Unix milliseconds.
    pub timestamp_ms: u64,
    /// Cumulative record count in table up to this snapshot.
    pub total_records: u64,
    /// Data files committed in this snapshot.
    pub data_files: Vec<DataFileEntry>,
}

/// In-memory catalog commit ledger managing table snapshot evolution per `KEI-DES-034` §6.
#[derive(Debug, Default)]
pub struct IcebergCatalogLedger {
    table_name: String,
    next_snapshot_id: u64,
    snapshots: Vec<CatalogSnapshot>,
    all_data_files: Vec<DataFileEntry>,
}

impl IcebergCatalogLedger {
    /// Create a new catalog ledger for a named Iceberg table.
    pub fn new(table_name: impl Into<String>) -> Self {
        Self {
            table_name: table_name.into(),
            next_snapshot_id: 1,
            snapshots: Vec::new(),
            all_data_files: Vec::new(),
        }
    }

    /// Commit a batch of Parquet files atomically into a new table snapshot.
    pub fn commit_snapshot(
        &mut self,
        new_files: Vec<DataFileEntry>,
        timestamp_ms: u64,
    ) -> CatalogSnapshot {
        let snapshot_id = self.next_snapshot_id;
        self.next_snapshot_id += 1;

        let parent_snapshot_id = self.snapshots.last().map(|s| s.snapshot_id);
        let added_records: u64 = new_files.iter().map(|f| f.record_count).sum();
        let total_records =
            self.snapshots.last().map(|s| s.total_records).unwrap_or(0) + added_records;

        let snapshot = CatalogSnapshot {
            snapshot_id,
            parent_snapshot_id,
            timestamp_ms,
            total_records,
            data_files: new_files.clone(),
        };

        self.all_data_files.extend(new_files);
        self.snapshots.push(snapshot.clone());
        snapshot
    }

    /// Return the table name.
    pub fn table_name(&self) -> &str {
        &self.table_name
    }

    /// Return the current active table snapshot.
    pub fn current_snapshot(&self) -> Option<&CatalogSnapshot> {
        self.snapshots.last()
    }

    /// Return all registered snapshots in chronological order.
    pub fn snapshots(&self) -> &[CatalogSnapshot] {
        &self.snapshots
    }

    /// Return total registered data files across all snapshots.
    pub fn total_data_files(&self) -> usize {
        self.all_data_files.len()
    }

    /// Expire snapshots older than `retention_cutoff_ms`, pruning metadata while preserving the active head snapshot.
    pub fn expire_snapshots_before(&mut self, retention_cutoff_ms: u64) -> usize {
        if self.snapshots.len() <= 1 {
            return 0;
        }

        let keep_head = self.snapshots.pop().unwrap();
        let initial_len = self.snapshots.len();
        self.snapshots
            .retain(|snap| snap.timestamp_ms >= retention_cutoff_ms);
        let removed = initial_len - self.snapshots.len();
        self.snapshots.push(keep_head);
        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iceberg_catalog_ledger_commit_lifecycle() {
        let mut ledger = IcebergCatalogLedger::new("orders_stream_lakehouse");
        assert_eq!(ledger.table_name(), "orders_stream_lakehouse");
        assert!(ledger.current_snapshot().is_none());

        // Commit Snapshot 1
        let files1 = vec![DataFileEntry {
            file_path: "s3://bucket/part-1.parquet".into(),
            record_count: 500,
            file_size_bytes: 64 * 1024 * 1024,
            partition_spec_id: 0,
        }];
        let snap1 = ledger.commit_snapshot(files1, 1700000000000);
        assert_eq!(snap1.snapshot_id, 1);
        assert_eq!(snap1.parent_snapshot_id, None);
        assert_eq!(snap1.total_records, 500);

        // Commit Snapshot 2
        let files2 = vec![DataFileEntry {
            file_path: "s3://bucket/part-2.parquet".into(),
            record_count: 300,
            file_size_bytes: 32 * 1024 * 1024,
            partition_spec_id: 0,
        }];
        let snap2 = ledger.commit_snapshot(files2, 1700000060000);
        assert_eq!(snap2.snapshot_id, 2);
        assert_eq!(snap2.parent_snapshot_id, Some(1));
        assert_eq!(snap2.total_records, 800);
        assert_eq!(ledger.total_data_files(), 2);
    }
}
