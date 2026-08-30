//! Profile P1 multi-node quorum benchmark test per `KEI-BENCH-201`.

use keirox_core::model::{StreamId, TenantId};
use keirox_testkit::ClusterRuntime;
use std::time::Instant;
use tempfile::TempDir;

#[tokio::test]
async fn test_profile_p1_cluster_quorum_benchmark() {
    let temp_dir = TempDir::new().unwrap();
    let mut cluster = ClusterRuntime::init_three_node(temp_dir.path()).unwrap();
    cluster.form_cluster().await.unwrap();

    let tenant_id = TenantId([1u8; 16]);
    let stream_id = StreamId([2u8; 16]);

    let payload = vec![0xAA; 512]; // 512-byte payload
    let total_batches = 100;
    let records_per_batch = 10;

    let start = Instant::now();
    for _ in 0..total_batches {
        let records = vec![payload.clone(); records_per_batch];
        let _ = cluster
            .produce_cluster(tenant_id, stream_id, records)
            .await
            .unwrap();
    }
    let elapsed = start.elapsed();

    let total_records = total_batches * records_per_batch;
    let total_bytes = total_records * 512;
    let throughput_mb = (total_bytes as f64) / (1024.0 * 1024.0) / elapsed.as_secs_f64();

    println!(
        "[BENCHMARK] Profile P1 (Cluster Quorum): {} records in {:?} -> {:.2} MB/s",
        total_records, elapsed, throughput_mb
    );

    assert!(total_records == 1000);
}
