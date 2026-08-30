//! # Keirox Chaos
//!
//! Chaos injection framework and Jepsen verification suites per `KEI-OPS-041`.

#![deny(missing_docs)]

/// Fault injection scenarios.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChaosScenario {
    /// Network partition between coordinator nodes.
    NetworkPartition,
    /// NVMe disk stall or latency spike.
    DiskStall,
    /// Abrupt process kill (SIGKILL).
    CrashFault,
    /// Clock drift exceeding drift threshold.
    ClockSkew,
}
