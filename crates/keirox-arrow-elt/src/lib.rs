//! # Keirox Arrow ELT
//!
//! Internalized Columnar ELT transforming JSON/binary micro-streams into Apache Arrow
//! batches and committing to Apache Iceberg lakehouses per `KEI-ARC-023`, `KEI-DES-033`, and `KEI-DES-034`.

#![deny(missing_docs)]
#![deny(unsafe_code)]

/// Apache Iceberg catalog commit structures and ledger.
pub mod catalog;
/// Apache Iceberg catalog committer and snapshot governor.
pub mod iceberg_committer;
/// Parquet file serialization and encoding.
pub mod parquet_encoder;
/// Adaptive schema shredder.
pub mod shredder;

pub use catalog::{CatalogSnapshot, DataFileEntry, IcebergCatalogLedger};
pub use iceberg_committer::{CommitCadenceMode, IcebergCatalogCommitter, SharedIcebergCommitter};
pub use parquet_encoder::ParquetEncoder;
pub use shredder::AdaptiveShredder;
