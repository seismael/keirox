//! Profile P1 synthetic NVMe append benchmark test per `KEI-BENCH-001`.

use keirox_api::proto::AckMode;
use keirox_bench::{BenchmarkConfig, BenchmarkRunner, WorkloadProfile};
use keirox_core::StreamId;
use keirox_testkit::SingleNodeRuntime;
use tempfile::tempdir;

#[test]
fn test_profile_p1_synthetic_ingress_benchmark() {
    let dir = tempdir().unwrap();
    let mut runtime = SingleNodeRuntime::init(dir.path()).unwrap();
    let stream = StreamId([0x11; 16]);

    let config = BenchmarkConfig::for_profile(WorkloadProfile::P1ExtremeLowLatency);
    let payload = vec![0xAB; config.payload_size_bytes];

    // Measure 1,000 durable append operations
    let result = BenchmarkRunner::measure(&config, 1000, |_op_idx| {
        let batch = vec![payload.clone()];
        runtime
            .produce(stream, AckMode::Durable, &batch)
            .expect("Produce must succeed");
    });

    assert_eq!(result.total_operations, 1000);
    assert!(result.ops_per_sec > 0.0);
    assert!(result.p50_latency_us > 0);
}
