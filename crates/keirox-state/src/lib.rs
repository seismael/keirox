//! # Keirox State
//!
//! Replicated consumption state overlay using Roaring Bitmaps.
//! Governed by `KEI-ARC-021` and `KEI-DES-031`.

#![deny(missing_docs)]

/// State machine definitions and bitset structures.
pub mod state_machine;

pub use state_machine::ConsumerState;
