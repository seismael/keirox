//! # Keirox Core
//!
//! Domain models, error taxonomy, and core invariants for the Keirox Polymorphic Event Fabric.
//!
//! ## Core Architectural Invariant
//!
//! **The Golden Invariant (KEI-ARC-010 §3)**:
//! Data is written exactly once to an immutable physical log. Consumption semantics
//! (streaming, queuing, dead-lettering, columnar views) are pure state overlays.

#![deny(missing_docs)]
#![deny(unsafe_code)]

/// Concrete error definitions for Keirox operations.
pub mod error;
/// Model definitions for streams, offsets, and records.
pub mod model;

pub use error::{KeiroxError, Result};
pub use model::{Offset, StreamId, TenantId};
