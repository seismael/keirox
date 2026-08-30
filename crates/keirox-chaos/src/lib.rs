//! # Keirox Chaos
//!
//! Chaos injection framework and Jepsen verification suites per `KEI-OPS-041`.

#![deny(missing_docs)]

use std::fmt;

/// Fault injection scenarios matching `KEI-OPS-041`.
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

/// Specification for a scheduled chaos fault experiment.
#[derive(Debug, Clone, PartialEq)]
pub struct ChaosFaultPlan {
    /// Fault scenario to inject.
    pub scenario: ChaosScenario,
    /// Duration of fault condition in milliseconds.
    pub duration_ms: u64,
    /// Target node ID to affect (None = all nodes).
    pub target_node_id: Option<u32>,
    /// Probability of triggering fault on each iteration (0.0 to 1.0).
    pub probability: f64,
}

impl ChaosFaultPlan {
    /// Create a standard crash-fault injection plan.
    pub fn crash_target(node_id: u32) -> Self {
        Self {
            scenario: ChaosScenario::CrashFault,
            duration_ms: 0,
            target_node_id: Some(node_id),
            probability: 1.0,
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

    #[test]
    fn test_chaos_fault_plan_instantiation() {
        let plan = ChaosFaultPlan::crash_target(2);
        assert_eq!(plan.scenario, ChaosScenario::CrashFault);
        assert_eq!(plan.target_node_id, Some(2));
    }
}
