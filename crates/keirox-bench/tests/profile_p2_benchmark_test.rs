//! Profile P2 High Throughput Streaming Benchmark per `KEI-BENCH-101` and `KEI-OPS-041`.

use keirox_api::proto::AckMode;
use keirox_bench::{BenchmarkConfig, BenchmarkRunner, WorkloadProfile};
use keirox_core::StreamId;
use keirox_testkit::SingleNodeRuntime;
use tempfile::tempdir;

#[test]
fn test_profile_p2_high_throughput_streaming_benchmark() {
    let dir = tempdir().unwrap();
    let mut runtime = SingleNodeRuntime::init(dir.path()).unwrap();
    let stream = StreamId([0x22; 16]);

    let config = BenchmarkConfig::for_profile(WorkloadProfile::P2HighThroughputStreaming);
    let payload = vec![0xCD; config.payload_size_bytes];

    // Measure 500 batched append operations (each batch containing 4KB payload)
    let result = BenchmarkRunner::measure(&config, 500, |_op_idx| {
        let batch = vec![payload.clone()];
        runtime
            .produce(stream, AckMode::Durable, &batch)
            .expect("Produce must succeed");
    });

    assert_eq!(result.total_operations, 500);
    assert!(result.ops_per_sec > 0.0);
    assert!(result.p99_latency_us < 50_000); // Strict local latency threshold
}
