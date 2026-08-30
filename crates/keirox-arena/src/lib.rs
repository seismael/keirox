//! # Keirox Arena
//!
//! Lock-free pre-allocated memory arenas for zero-heap-allocation ingress and append loops.

#![deny(missing_docs)]

/// Pre-allocated row arenas.
pub mod arena;

pub use arena::{RowArena, CACHE_LINE_ALIGNMENT, DEFAULT_ROW_ARENA_CAPACITY};
