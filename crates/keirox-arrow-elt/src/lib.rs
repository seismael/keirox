//! # Keirox Arrow ELT
//!
//! Internalized Columnar ELT transforming JSON/binary micro-streams into Apache Arrow
//! batches and committing to Apache Iceberg lakehouses per `KEI-ARC-023`, `KEI-DES-033`, and `KEI-DES-034`.

#![deny(missing_docs)]

/// Adaptive schema shredder.
pub mod shredder;

pub use shredder::AdaptiveShredder;
