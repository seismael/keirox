//! Kafka wire-protocol gateway conformance and compatibility tests per `KEI-COMPAT-301` and `KEI-ENG-300`.

use keirox_core::model::TenantId;
use keirox_gateway::{
    KafkaApiKey, KafkaErrorCode, KafkaGatewayServer, KafkaProduceRecordBatch, KafkaRequestHeader,
};
use keirox_testkit::{ClusterRuntime, SharedClusterHandle};
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::test]
async fn test_kafka_gateway_produce_idempotence_and_metadata_conformance() {
    let temp_dir = TempDir::new().unwrap();
    let mut cluster = ClusterRuntime::init_three_node(temp_dir.path()).unwrap();
    cluster.form_cluster().await.unwrap();

    let shared_cluster = Arc::new(SharedClusterHandle::new(cluster));
    let default_tenant = TenantId([0xAA; 16]);
    let gateway = KafkaGatewayServer::new(shared_cluster, default_tenant);

    // 1. ApiVersions Negotiation
    let versions = gateway.handle_api_versions();
    assert!(versions
        .iter()
        .any(|(k, min, max)| *k == KafkaApiKey::Produce && *min == 0 && *max >= 8));
    assert!(versions.iter().any(|(k, _, _)| *k == KafkaApiKey::Fetch));
    assert!(versions.iter().any(|(k, _, _)| *k == KafkaApiKey::Metadata));

    // 2. Initial Idempotent Produce Batch (Sequence 0)
    let batch1 = KafkaProduceRecordBatch {
        topic: "telemetry-events".to_string(),
        partition: 0,
        producer_id: 2048,
        producer_epoch: 0,
        base_sequence: 0,
        records: vec![
            b"kafka-metric-cpu-90%".to_vec(),
            b"kafka-metric-mem-45%".to_vec(),
        ],
    };

    let resp1 = gateway.process_produce(vec![batch1]).await.unwrap();
    let p_resp1 = &resp1.responses["telemetry-events"][0];
    assert_eq!(p_resp1.error_code, KafkaErrorCode::None);
    assert_eq!(p_resp1.base_offset, 0);

    // 3. Duplicate Idempotent Produce Batch (Same Sequence 0) -> Deduplicated cleanly
    let batch_dup = KafkaProduceRecordBatch {
        topic: "telemetry-events".to_string(),
        partition: 0,
        producer_id: 2048,
        producer_epoch: 0,
        base_sequence: 0,
        records: vec![
            b"kafka-metric-cpu-90%".to_vec(),
            b"kafka-metric-mem-45%".to_vec(),
        ],
    };
    let resp_dup = gateway.process_produce(vec![batch_dup]).await.unwrap();
    let p_dup = &resp_dup.responses["telemetry-events"][0];
    assert_eq!(p_dup.error_code, KafkaErrorCode::None);
    assert_eq!(
        p_dup.base_offset, 0,
        "Duplicate produce must return cached offset"
    );

    // 4. Out-of-Order Sequence Produce Batch (Expected Sequence 2, sent 10)
    let batch_ooo = KafkaProduceRecordBatch {
        topic: "telemetry-events".to_string(),
        partition: 0,
        producer_id: 2048,
        producer_epoch: 0,
        base_sequence: 10,
        records: vec![b"kafka-gap-record".to_vec()],
    };
    let resp_ooo = gateway.process_produce(vec![batch_ooo]).await.unwrap();
    let p_ooo = &resp_ooo.responses["telemetry-events"][0];
    assert_eq!(p_ooo.error_code, KafkaErrorCode::OutOfOrderSequenceNumber);

    // 5. Contiguous Next Batch (Sequence 2)
    let batch2 = KafkaProduceRecordBatch {
        topic: "telemetry-events".to_string(),
        partition: 0,
        producer_id: 2048,
        producer_epoch: 0,
        base_sequence: 2,
        records: vec![b"kafka-metric-disk-10%".to_vec()],
    };
    let resp2 = gateway.process_produce(vec![batch2]).await.unwrap();
    let p_resp2 = &resp2.responses["telemetry-events"][0];
    assert_eq!(p_resp2.error_code, KafkaErrorCode::None);
    assert_eq!(p_resp2.base_offset, 2);

    // 6. Explicit Error on Unsupported Transactional API
    let unsupported_header = KafkaRequestHeader {
        api_key: KafkaApiKey::Unsupported(65), // AddPartitionsToTxn
        api_version: 1,
        correlation_id: 99,
        client_id: Some("tx-producer".into()),
    };
    let err_res = gateway.dispatch_request(&unsupported_header).await.unwrap();
    assert_eq!(err_res, KafkaErrorCode::UnsupportedVersion);
}
