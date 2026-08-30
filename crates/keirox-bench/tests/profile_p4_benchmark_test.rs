//! Profile P4 Mixed Stream and Queue Benchmark per `KEI-BENCH-101` and `KEI-OPS-041`.

use keirox_api::proto::AckMode;
use keirox_bench::{BenchmarkConfig, BenchmarkRunner, WorkloadProfile};
use keirox_core::StreamId;
use keirox_testkit::SingleNodeRuntime;
use tempfile::tempdir;

#[test]
fn test_profile_p4_mixed_stream_and_queue_benchmark() {
    let dir = tempdir().unwrap();
    let mut runtime = SingleNodeRuntime::init(dir.path()).unwrap();
    let stream = StreamId([0x44; 16]);
    let group_id = 4040;

    let config = BenchmarkConfig::for_profile(WorkloadProfile::P4MixedStreamAndQueue);
    let payload = vec![0xEF; config.payload_size_bytes];

    // Pre-populate 500 records
    for _ in 0..500 {
        let batch = vec![payload.clone()];
        runtime
            .produce(stream, AckMode::Durable, &batch)
            .expect("Produce must succeed");
    }

    // Benchmark lease acquisition and ACK cycle
    let result = BenchmarkRunner::measure(&config, 200, |_op_idx| {
        let leased = runtime
            .lease_records(stream, group_id, 1, 30_000)
            .expect("Lease must succeed");
        if let Some(&(offset, token)) = leased.first() {
            runtime
                .ack_record(stream, group_id, offset, token)
                .expect("ACK must succeed");
        }
    });

    assert_eq!(result.total_operations, 200);
    assert!(result.ops_per_sec > 0.0);
    assert_eq!(runtime.base_watermark(stream, group_id), 200);
}
