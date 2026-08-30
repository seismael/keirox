//! # Keirox WAL
//!
//! Write-Ahead Log implementation targeting `io_uring` + `O_DIRECT` NVMe storage.
//! Follows binary format specifications in `KEI-DES-030`.

#![deny(missing_docs)]

/// WAL batch framing and binary layout structures.
pub mod framing;

pub use framing::BatchHeader;
