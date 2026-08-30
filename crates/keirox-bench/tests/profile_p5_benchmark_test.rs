//! Profile P5 Columnar ELT Transcoding Benchmark per `KEI-BENCH-101` and `KEI-OPS-041`.

use keirox_bench::{BenchmarkConfig, BenchmarkRunner, WorkloadProfile};
use keirox_testkit::SingleNodeRuntime;
use tempfile::tempdir;

#[test]
fn test_profile_p5_columnar_elt_export_benchmark() {
    let dir = tempdir().unwrap();
    let mut runtime = SingleNodeRuntime::init(dir.path()).unwrap();

    let config = BenchmarkConfig::for_profile(WorkloadProfile::P5ColumnarEltExport);

    // Prepare JSON batches
    let mut json_batch = Vec::new();
    for i in 0..100 {
        let json_val = serde_json::json!({
            "event_id": i,
            "metric_name": "cpu_utilization",
            "val": 42.5 + (i as f64 * 0.1),
            "host": "srv-prod-01"
        });
        json_batch.push(json_val);
    }

    // Benchmark Arrow shredding and record batch creation
    let result = BenchmarkRunner::measure(&config, 50, |_op_idx| {
        let batch = runtime
            .export_arrow(&json_batch)
            .expect("Arrow export must succeed");
        assert_eq!(batch.num_rows(), 100);
    });

    assert_eq!(result.total_operations, 50);
    assert!(result.ops_per_sec > 0.0);
}
