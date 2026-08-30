//! # Keirox Bench
//!
//! Benchmark test harness for canonical profiles P1 through P6 per `KEI-OPS-041` and `KEI-BENCH-001`.

#![deny(missing_docs)]

use std::fmt;

/// Workload profile identifiers matching `KEI-OPS-041`.
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

/// Configuration parameters for executing benchmark workload profiles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkConfig {
    /// Target workload profile.
    pub profile: WorkloadProfile,
    /// Individual record payload size in bytes.
    pub payload_size_bytes: usize,
    /// Concurrency level (number of concurrent generator workers).
    pub concurrency: usize,
    /// Target throughput in operations per second (0 = unthrottled maximum).
    pub target_ops_per_sec: u64,
    /// Total duration of measurement phase in seconds.
    pub duration_secs: u64,
}

impl BenchmarkConfig {
    /// Create standard default configuration for a given profile per `KEI-BENCH-001`.
    pub fn for_profile(profile: WorkloadProfile) -> Self {
        match profile {
            WorkloadProfile::P1ExtremeLowLatency => Self {
                profile,
                payload_size_bytes: 1024,
                concurrency: 16,
                target_ops_per_sec: 100_000,
                duration_secs: 60,
            },
            WorkloadProfile::P2HighThroughputStreaming => Self {
                profile,
                payload_size_bytes: 4096,
                concurrency: 64,
                target_ops_per_sec: 0,
                duration_secs: 60,
            },
            _ => Self {
                profile,
                payload_size_bytes: 1024,
                concurrency: 8,
                target_ops_per_sec: 10_000,
                duration_secs: 30,
            },
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

    #[test]
    fn test_benchmark_config_for_profile() {
        let p1_cfg = BenchmarkConfig::for_profile(WorkloadProfile::P1ExtremeLowLatency);
        assert_eq!(p1_cfg.payload_size_bytes, 1024);
        assert_eq!(p1_cfg.concurrency, 16);
    }
}
