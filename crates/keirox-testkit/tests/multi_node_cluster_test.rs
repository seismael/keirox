//! 3-Node distributed cluster integration test per `KEI-ENG-200` and `KEI-ARC-022`.

use bytes::Bytes;
use keirox_core::model::{StreamId, TenantId};
use keirox_testkit::ClusterRuntime;
use tempfile::TempDir;

#[tokio::test]
async fn test_three_node_cluster_full_lifecycle() {
    let temp_dir = TempDir::new().unwrap();
    let mut cluster = ClusterRuntime::init_three_node(temp_dir.path()).unwrap();

    // 1. Form cluster and elect leader
    cluster.form_cluster().await.unwrap();

    let tenant_id = TenantId([10u8; 16]);
    let stream_id = StreamId([20u8; 16]);

    // 2. Produce records with synchronous 3-node quorum replication
    let records = vec![
        b"cluster-event-alpha".to_vec(),
        b"cluster-event-beta".to_vec(),
        b"cluster-event-gamma".to_vec(),
    ];

    let base_offset = cluster
        .produce_cluster(tenant_id, stream_id, records)
        .await
        .unwrap();
    assert_eq!(base_offset, 0);

    // 3. Lease offset via consistent-hashing coordinator
    let token = cluster
        .lease_cluster("group-payments", 0, 5000, 1_000_000)
        .await
        .unwrap();
    assert_eq!(token.offset, 0);
    assert_eq!(token.epoch.0, 1);

    // 4. Acknowledge leased offset with epoch fencing
    cluster.ack_cluster("group-payments", token).await.unwrap();

    // 5. Seal segment chunk and stream to Tier-1 S3
    let chunk_payload = Bytes::from_static(b"COMPRESSED_PARQUET_CHUNK_DATA_FOR_TIER1_S3");
    let s3_uri = cluster
        .seal_and_stream_tier1(tenant_id, stream_id, 0, 2, chunk_payload)
        .await
        .unwrap();

    assert!(s3_uri.starts_with("s3://keirox-lakehouse-test/chunks/"));
    assert!(s3_uri.ends_with("/0_2.chk"));
}
