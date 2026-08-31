//! # Keirox Timer
//!
//! Hierarchical timing wheel for O(1) lease scheduling and timeout eviction.

#![deny(missing_docs)]
#![deny(unsafe_code)]

/// Timing wheel implementation.
pub mod wheel;

pub use wheel::TimingWheel;
