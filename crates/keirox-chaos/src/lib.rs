//! # Keirox Chaos
//!
//! Chaos injection framework and Jepsen verification suites per `KEI-OPS-041`.

#![deny(missing_docs)]

use std::fmt;

/// Fault injection scenarios.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl fmt::Display for ChaosScenario {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NetworkPartition => write!(f, "NetworkPartition"),
            Self::DiskStall => write!(f, "DiskStall"),
            Self::CrashFault => write!(f, "CrashFault"),
            Self::ClockSkew => write!(f, "ClockSkew"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chaos_scenario_display() {
        assert_eq!(
            ChaosScenario::NetworkPartition.to_string(),
            "NetworkPartition"
        );
        assert_eq!(ChaosScenario::CrashFault.to_string(), "CrashFault");
    }
}
