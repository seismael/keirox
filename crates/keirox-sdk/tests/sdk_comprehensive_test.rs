use async_trait::async_trait;
use keirox_coordinator::{CoordinatorEpoch, EpochFencedToken, ShardId};
use keirox_core::error::{KeiroxError, Result};
use keirox_core::model::{StreamId, TenantId};
use keirox_sdk::client::{ClusterClientTransport, KeiroxClient, KeiroxClientConfig};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

struct MockClusterTransport {
    offset_counter: AtomicU64,
}

impl MockClusterTransport {
    fn new() -> Self {
        Self {
            offset_counter: AtomicU64::new(100),
        }
    }
}

#[async_trait]
impl ClusterClientTransport for MockClusterTransport {
    async fn produce(
        &self,
        _tenant_id: TenantId,
        _stream_id: StreamId,
        records: Vec<Vec<u8>>,
    ) -> Result<u64> {
        let count = records.len() as u64;
        let base = self.offset_counter.fetch_add(count, Ordering::SeqCst);
        Ok(base + count - 1)
    }

    async fn lease(
        &self,
        _group_id: &str,
        offset: u64,
        _ttl_ms: u64,
        _now_us: u64,
    ) -> Result<EpochFencedToken> {
        Ok(EpochFencedToken::new(
            ShardId(42),
            CoordinatorEpoch(3),
            offset,
            0xAA55_CC33,
        ))
    }

    async fn ack(&self, group_id: &str, token: EpochFencedToken) -> Result<()> {
        if group_id.is_empty() {
            return Err(KeiroxError::Internal("Empty group".into()));
        }
        if token.offset == 999 {
            return Err(KeiroxError::LeaseConflict("Stale lease".into()));
        }
        Ok(())
    }

    async fn nack(&self, group_id: &str, _token: EpochFencedToken) -> Result<()> {
        if group_id.is_empty() {
            return Err(KeiroxError::Internal("Empty group".into()));
        }
        Ok(())
    }
}

#[tokio::test]
async fn test_sdk_producer_batching_and_concurrency() {
    let config = KeiroxClientConfig {
        endpoint: "keirox://cluster.corp:9092".to_string(),
        tenant_id: TenantId([2u8; 16]),
        timeout: Duration::from_secs(5),
        max_retries: 5,
    };
    let transport = Arc::new(MockClusterTransport::new());
    let client = KeiroxClient::new(config, transport);

    let producer = client.producer();
    let stream = StreamId([0x10; 16]);

    // Single send
    let offset = producer.send(stream, b"payload1".to_vec()).await.unwrap();
    assert_eq!(offset, 100);

    // Batch send
    let payloads = vec![
        b"batch_1".to_vec(),
        b"batch_2".to_vec(),
        b"batch_3".to_vec(),
    ];
    let last_offset = producer.send_batch(stream, payloads).await.unwrap();
    assert_eq!(last_offset, 103);
}

#[tokio::test]
async fn test_sdk_consumer_navigation_and_state() {
    let config = KeiroxClientConfig::default();
    let transport = Arc::new(MockClusterTransport::new());
    let client = KeiroxClient::new(config, transport);

    let stream = StreamId([0x33; 16]);
    let mut consumer = client.consumer(stream, 500);

    assert_eq!(consumer.position(), 500);
    assert_eq!(consumer.stream_id(), stream);

    consumer.seek(1200);
    assert_eq!(consumer.position(), 1200);
}

#[tokio::test]
async fn test_sdk_queue_worker_lease_ack_nack_lifecycle() {
    let config = KeiroxClientConfig::default();
    let transport = Arc::new(MockClusterTransport::new());
    let client = KeiroxClient::new(config, transport);

    let queue = client.queue("order-processing-worker");
    assert_eq!(queue.group_id(), "order-processing-worker");

    // Acquire lease
    let token = queue.lease(105, 10_000, 1_700_000_000).await.unwrap();
    assert_eq!(token.offset, 105);
    assert_eq!(token.shard_id, ShardId(42));
    assert_eq!(token.epoch, CoordinatorEpoch(3));

    // ACK valid token
    assert!(queue.ack(token).await.is_ok());

    // NACK valid token
    assert!(queue.nack(token).await.is_ok());

    // ACK stale token
    let stale_token = EpochFencedToken::new(ShardId(42), CoordinatorEpoch(3), 999, 0x1111);
    let ack_err = queue.ack(stale_token).await.unwrap_err();
    assert!(matches!(ack_err, KeiroxError::LeaseConflict(_)));
}

#[tokio::test]
async fn test_sdk_flight_reader_batch_reading() {
    let config = KeiroxClientConfig::default();
    let transport = Arc::new(MockClusterTransport::new());
    let client = KeiroxClient::new(config, transport);

    let flight_reader = client.flight_reader();
    let stream = StreamId([0x77; 16]);

    let records = vec![b"event_alpha".to_vec(), b"event_beta".to_vec()];
    let batch = flight_reader
        .read_stream_batch(stream, 200, &records)
        .await
        .unwrap();

    assert_eq!(batch.num_rows(), 2);
    assert_eq!(batch.num_columns(), 3);
    assert_eq!(batch.schema().field(0).name(), "_offset");
    assert_eq!(batch.schema().field(1).name(), "_timestamp_ns");
    assert_eq!(batch.schema().field(2).name(), "payload");
}
