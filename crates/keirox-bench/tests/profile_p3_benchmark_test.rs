//! Profile P3 massive micro-stream fanout benchmark test per `KEI-OPS-041` and `KEI-BENCH-101`.

use keirox_api::proto::AckMode;
use keirox_bench::{BenchmarkConfig, BenchmarkRunner, WorkloadProfile};
use keirox_core::StreamId;
use keirox_testkit::SingleNodeRuntime;
use tempfile::tempdir;

#[test]
fn test_profile_p3_massive_stream_fanout_benchmark() {
    let dir = tempdir().unwrap();
    let mut runtime = SingleNodeRuntime::init(dir.path()).unwrap();

    let config = BenchmarkConfig::for_profile(WorkloadProfile::P3MassiveStreamFanout);
    let payload = vec![0x33; config.payload_size_bytes];

    // Measure across 500 distinct micro-streams
    let result = BenchmarkRunner::measure(&config, 500, |op_idx| {
        let mut raw = [0u8; 16];
        raw[..4].copy_from_slice(&(op_idx as u32).to_le_bytes());
        let stream = StreamId(raw);
        let batch = vec![payload.clone()];
        runtime
            .produce(stream, AckMode::Durable, &batch)
            .expect("P3 fanout produce must succeed");
    });

    assert_eq!(result.total_operations, 500);
    assert!(result.ops_per_sec > 0.0);
    assert!(result.p99_latency_us > 0);
}
