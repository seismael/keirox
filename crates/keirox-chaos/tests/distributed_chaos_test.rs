//! Distributed chaos and split-brain network partition verification test per `KEI-OPS-041` and `KEI-ENG-200`.

use keirox_consensus::NodeId;
use keirox_coordinator::{CoordinatorEpoch, EpochFencedToken, ShardId};
use keirox_core::error::KeiroxError;
use keirox_core::model::{StreamId, TenantId};
use keirox_testkit::ClusterRuntime;
use tempfile::TempDir;

#[tokio::test]
async fn test_network_partition_and_split_brain_fencing() {
    let temp_dir = TempDir::new().unwrap();
    let mut cluster = ClusterRuntime::init_three_node(temp_dir.path()).unwrap();
    cluster.form_cluster().await.unwrap();

    let tenant_id = TenantId([10u8; 16]);
    let stream_id = StreamId([20u8; 16]);

    // 1. Initial produce on 3-node cluster
    let offset = cluster
        .produce_cluster(tenant_id, stream_id, vec![b"before-partition".to_vec()])
        .await
        .unwrap();
    assert_eq!(offset, 0);

    // 2. Lease message under Epoch 1
    let token = cluster
        .lease_cluster("group-orders", 0, 5000, 1_000_000)
        .await
        .unwrap();
    assert_eq!(token.epoch.0, 1);

    // 3. Inject network partition: [Node 1, Node 2] vs [Node 3]
    cluster.partition_cluster(&[NodeId(1), NodeId(2)], &[NodeId(3)]);

    // 4. Majority partition (Node 1, Node 2) continues to form quorum and accept writes
    let offset2 = cluster
        .produce_cluster(
            tenant_id,
            stream_id,
            vec![b"majority-partition-event".to_vec()],
        )
        .await
        .unwrap();
    assert_eq!(offset2, 1);

    // 5. Simulate stale epoch ACK attempt (e.g. from an isolated old coordinator holding Epoch 0)
    let stale_token =
        EpochFencedToken::new(ShardId(token.shard_id.0), CoordinatorEpoch(0), 0, 9999);
    let ack_result = cluster.ack_cluster("group-orders", stale_token).await;
    assert!(ack_result.is_err());
    let err = ack_result.unwrap_err();
    assert!(matches!(err, KeiroxError::EpochFenced(_)));

    // 6. Valid token ACKs successfully on active coordinator
    cluster.ack_cluster("group-orders", token).await.unwrap();

    // 7. Heal network partition
    cluster.heal_partitions();

    // 8. Normal operation resumes repo-wide
    let offset3 = cluster
        .produce_cluster(tenant_id, stream_id, vec![b"post-heal-event".to_vec()])
        .await
        .unwrap();
    assert_eq!(offset3, 2);
}
