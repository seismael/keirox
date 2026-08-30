//! Native Client SDK integration and performance test per `KEI-SDK-301` and `KEI-ENG-300`.

use keirox_core::model::{StreamId, TenantId};
use keirox_sdk::{KeiroxClient, KeiroxClientConfig};
use keirox_testkit::{ClusterRuntime, SharedClusterHandle};
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::test]
async fn test_native_sdk_producer_consumer_queue_and_flight_reader() {
    let temp_dir = TempDir::new().unwrap();
    let mut cluster = ClusterRuntime::init_three_node(temp_dir.path()).unwrap();
    cluster.form_cluster().await.unwrap();

    let shared_cluster = Arc::new(SharedClusterHandle::new(cluster));
    let client_config = KeiroxClientConfig {
        endpoint: "keirox://cluster-in-memory:9092".into(),
        tenant_id: TenantId([0x55; 16]),
        timeout: std::time::Duration::from_millis(2000),
        max_retries: 3,
    };

    let client = KeiroxClient::new(client_config, shared_cluster);
    let stream_id = StreamId([0x77; 16]);

    // 1. Native Producer
    let producer = client.producer();
    let batch = vec![
        b"sdk-payload-record-0".to_vec(),
        b"sdk-payload-record-1".to_vec(),
        b"sdk-payload-record-2".to_vec(),
    ];
    let offset = producer.send_batch(stream_id, batch.clone()).await.unwrap();
    assert_eq!(offset, 0);

    // 2. Native Consumer
    let mut consumer = client.consumer(stream_id, 0);
    assert_eq!(consumer.position(), 0);
    consumer.seek(10);
    assert_eq!(consumer.position(), 10);

    // 3. Task Queue Client with Epoch Fencing
    let queue_client = client.queue("analytics-workers");
    let token = queue_client.lease(0, 5000, 1_000_000).await.unwrap();
    assert_eq!(token.offset, 0);
    assert_eq!(token.epoch.0, 1);

    // Valid ACK
    queue_client.ack(token).await.unwrap();

    // 4. Arrow Flight Reader Vectorized Batch Transfer
    let flight_reader = client.flight_reader();
    let record_batch = flight_reader
        .read_stream_batch(stream_id, 0, &batch)
        .await
        .unwrap();

    assert_eq!(record_batch.num_rows(), 3);
    assert_eq!(record_batch.num_columns(), 3);
    assert!(record_batch.schema().field_with_name("_offset").is_ok());
    assert!(record_batch
        .schema()
        .field_with_name("_timestamp_ns")
        .is_ok());
    assert!(record_batch.schema().field_with_name("payload").is_ok());
}
