//! Synthetic benchmark runner and histogram metric collector per `KEI-BENCH-001`.

use crate::BenchmarkConfig;
use std::time::Instant;

/// Benchmark execution results containing throughput and latency percentiles.
#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkResult {
    /// Total operations performed.
    pub total_operations: u64,
    /// Total elapsed execution time in microseconds.
    pub elapsed_us: u64,
    /// Sustained operations per second.
    pub ops_per_sec: f64,
    /// Median latency (P50) in microseconds.
    pub p50_latency_us: u64,
    /// 99th percentile latency (P99) in microseconds.
    pub p99_latency_us: u64,
    /// 99.9th percentile latency (P99.9) in microseconds.
    pub p999_latency_us: u64,
}

/// Benchmark harness runner.
pub struct BenchmarkRunner;

impl BenchmarkRunner {
    /// Execute a benchmark closure recording individual operation latencies.
    pub fn measure<F>(config: &BenchmarkConfig, operations: u64, mut op: F) -> BenchmarkResult
    where
        F: FnMut(u64),
    {
        let mut latencies_us = Vec::with_capacity(operations as usize);
        let start_total = Instant::now();

        for i in 0..operations {
            let start_op = Instant::now();
            op(i);
            latencies_us.push(start_op.elapsed().as_micros() as u64);
        }

        let elapsed_us = start_total.elapsed().as_micros() as u64;
        latencies_us.sort_unstable();

        let p50_idx = ((operations as f64) * 0.50) as usize;
        let p99_idx = ((operations as f64) * 0.99) as usize;
        let p999_idx = ((operations as f64) * 0.999) as usize;

        let p50_latency_us = latencies_us.get(p50_idx).copied().unwrap_or(0);
        let p99_latency_us = latencies_us.get(p99_idx).copied().unwrap_or(0);
        let p999_latency_us = latencies_us.get(p999_idx).copied().unwrap_or(0);

        let ops_per_sec = if elapsed_us > 0 {
            (operations as f64) / (elapsed_us as f64 / 1_000_000.0)
        } else {
            0.0
        };

        let _ = config;

        BenchmarkResult {
            total_operations: operations,
            elapsed_us,
            ops_per_sec,
            p50_latency_us,
            p99_latency_us,
            p999_latency_us,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WorkloadProfile;

    #[test]
    fn test_benchmark_runner_measure() {
        let config = BenchmarkConfig::for_profile(WorkloadProfile::P1ExtremeLowLatency);
        let res = BenchmarkRunner::measure(&config, 1000, |_i| {
            // Simulated microsecond operation
            std::hint::black_box(42 + 1);
        });

        assert_eq!(res.total_operations, 1000);
        assert!(res.ops_per_sec > 0.0);
    }
}
