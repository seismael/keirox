//! Node crash failure and automated replacement recovery test per `KEI-ENG-200` §10.

use bytes::Bytes;
use keirox_consensus::NodeId;
use keirox_core::model::{StreamId, TenantId};
use keirox_testkit::ClusterRuntime;
use tempfile::TempDir;

#[tokio::test]
async fn test_node_failure_and_rapid_replacement() {
    let temp_dir = TempDir::new().unwrap();
    let mut cluster = ClusterRuntime::init_three_node(temp_dir.path()).unwrap();
    cluster.form_cluster().await.unwrap();

    let tenant_id = TenantId([1u8; 16]);
    let stream_id = StreamId([2u8; 16]);

    // Ingest data
    let records = vec![
        b"event-before-crash-1".to_vec(),
        b"event-before-crash-2".to_vec(),
    ];
    let offset = cluster
        .produce_cluster(tenant_id, stream_id, records)
        .await
        .unwrap();
    assert_eq!(offset, 0);

    // Stream chunk to Tier-1 S3
    let chunk_payload = Bytes::from_static(b"TIER1_SEALED_CHUNK_BYTES");
    let s3_uri = cluster
        .seal_and_stream_tier1(tenant_id, stream_id, 0, 1, chunk_payload)
        .await
        .unwrap();
    assert!(!s3_uri.is_empty());

    // Crash node 3 (kill -9)
    cluster.crash_node(NodeId(3));

    // Ingest continues on quorum of surviving nodes (Node 1 and Node 2)
    let records2 = vec![b"event-after-crash-3".to_vec()];
    let offset2 = cluster
        .produce_cluster(tenant_id, stream_id, records2)
        .await
        .unwrap();
    assert_eq!(offset2, 2);

    // Replace failed node with Node 4 in <5 seconds
    cluster
        .recover_and_replace_node(NodeId(4), NodeId(3), temp_dir.path())
        .await
        .unwrap();

    // Ingestion succeeds across new 3-node cluster (Node 1, 2, 4)
    let records3 = vec![b"event-post-recovery-4".to_vec()];
    let offset3 = cluster
        .produce_cluster(tenant_id, stream_id, records3)
        .await
        .unwrap();
    assert_eq!(offset3, 3);
}
