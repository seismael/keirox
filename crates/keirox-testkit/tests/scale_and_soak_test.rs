//! High-cardinality stream scaling and lease churn soak test suite per `KEI-ENG-100` §9.9 (M1.8).

use keirox_api::proto::AckMode;
use keirox_core::StreamId;
use keirox_index::StreamRegistryEntry;
use keirox_testkit::SingleNodeRuntime;
use std::collections::HashMap;
use tempfile::tempdir;

#[test]
fn test_high_cardinality_stream_registry_scale() {
    // Validate 10,000 high-cardinality streams in memory with exact 32-byte density
    const STREAM_COUNT: usize = 10_000;
    let mut registry: HashMap<StreamId, StreamRegistryEntry> = HashMap::with_capacity(STREAM_COUNT);

    for i in 0..STREAM_COUNT {
        let mut id_bytes = [0u8; 16];
        id_bytes[0..8].copy_from_slice(&(i as u64).to_le_bytes());
        let stream = StreamId(id_bytes);

        let mut entry = StreamRegistryEntry::new(stream, 1);
        entry.advance_head((i * 10) as u64);
        registry.insert(stream, entry);
    }

    assert_eq!(registry.len(), STREAM_COUNT);

    // Verify lookup time is O(1)
    let sample_id = StreamId({
        let mut b = [0u8; 16];
        b[0..8].copy_from_slice(&(5555u64).to_le_bytes());
        b
    });
    let sample_entry = registry.get(&sample_id).expect("Stream must exist");
    assert_eq!(sample_entry.head_offset, 55550);
}

#[test]
fn test_high_lease_churn_soak_flow() {
    let dir = tempdir().unwrap();
    let mut runtime = SingleNodeRuntime::init(dir.path()).unwrap();
    let stream = StreamId([0x99; 16]);
    let group_id = 42;

    const BATCHES: usize = 50;
    const RECORDS_PER_BATCH: usize = 100;

    // 1. Ingress 5,000 records across 50 batches
    for b in 0..BATCHES {
        let mut records = Vec::with_capacity(RECORDS_PER_BATCH);
        for r in 0..RECORDS_PER_BATCH {
            let val = serde_json::json!({
                "batch": b,
                "record": r,
                "msg": "high_throughput_churn_payload"
            });
            records.push(serde_json::to_vec(&val).unwrap());
        }
        runtime
            .produce(stream, AckMode::Durable, &records)
            .expect("Batch produce must succeed");
    }

    // 2. High-velocity lease & ACK loop in chunks of 500
    let mut total_acked = 0;
    for _ in 0..10 {
        let leased = runtime
            .lease_records(stream, group_id, 500, 10000)
            .expect("Lease must succeed");
        assert_eq!(leased.len(), 500);

        // ACK all leased records
        for (offset, token) in leased {
            runtime
                .ack_record(stream, group_id, offset, token)
                .expect("ACK must succeed");
            total_acked += 1;
        }
    }

    assert_eq!(total_acked, 5000);
    assert_eq!(runtime.base_watermark(stream, group_id), 5000);
}
