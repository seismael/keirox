//! # Keirox Index
//!
//! High-density index structures and stream registries.

#![deny(missing_docs)]
#![deny(unsafe_code)]

/// Packed Stream Registry structures.
pub mod registry;
/// Sparse offset index for fast $O(\log n)$ random access.
pub mod sparse_index;

pub use registry::StreamRegistryEntry;
pub use sparse_index::{SparseIndexEntry, SparseOffsetIndex};
