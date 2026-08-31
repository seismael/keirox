//! # Keirox State
//!
//! Replicated consumption state overlay using Roaring Bitmaps.
//! Governed by `KEI-ARC-021` and `KEI-DES-031`.

#![deny(missing_docs)]
#![deny(unsafe_code)]

/// State snapshotting and serialization.
pub mod snapshot;
/// State machine definitions and bitset structures.
pub mod state_machine;

pub use snapshot::StateSnapshot;
pub use state_machine::{
    ActiveLease, ConsumerGroupState, ConsumerState, StateShardKey, DEFAULT_MAX_RETRIES,
};
