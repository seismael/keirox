//! # Keirox Bench
//!
//! Benchmark test harness for canonical profiles P1 through P6 per `KEI-OPS-041`.

#![deny(missing_docs)]

/// Workload profile identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
