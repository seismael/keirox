//! # Keirox Bench
//!
//! Benchmark test harness for canonical profiles P1 through P6 per `KEI-OPS-041`.

#![deny(missing_docs)]

use std::fmt;

/// Workload profile identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkloadProfile {
    /// P1: Extreme low latency (1KB payloads, NVMe Tier-0).
    P1ExtremeLowLatency,
    /// P2: High throughput streaming.
    P2HighThroughputStreaming,
    /// P3: Massive micro-stream fanout.
    P3MassiveStreamFanout,
    /// P4: Mixed stream + queue workload.
    P4MixedStreamAndQueue,
    /// P5: Columnar ELT heavy export.
    P5ColumnarEltExport,
    /// P6: Disaster recovery replication.
    P6MultiRegionReplication,
}

impl fmt::Display for WorkloadProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::P1ExtremeLowLatency => write!(f, "P1-ExtremeLowLatency"),
            Self::P2HighThroughputStreaming => write!(f, "P2-HighThroughputStreaming"),
            Self::P3MassiveStreamFanout => write!(f, "P3-MassiveStreamFanout"),
            Self::P4MixedStreamAndQueue => write!(f, "P4-MixedStreamAndQueue"),
            Self::P5ColumnarEltExport => write!(f, "P5-ColumnarEltExport"),
            Self::P6MultiRegionReplication => write!(f, "P6-MultiRegionReplication"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workload_profile_display() {
        assert_eq!(
            WorkloadProfile::P1ExtremeLowLatency.to_string(),
            "P1-ExtremeLowLatency"
        );
        assert_eq!(
            WorkloadProfile::P4MixedStreamAndQueue.to_string(),
            "P4-MixedStreamAndQueue"
        );
    }
}
