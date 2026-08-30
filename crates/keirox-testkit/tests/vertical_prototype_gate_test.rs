//! Minimum Vertical Prototype Evidence Gate test suite per `KEI-SPIKE-001` and `KEI-ENG-100` §8.

use keirox_api::proto::AckMode;
use keirox_core::StreamId;
use keirox_testkit::SingleNodeRuntime;
use tempfile::tempdir;

#[test]
fn test_vertical_prototype_evidence_gate() {
    let dir = tempdir().unwrap();
    let stream = StreamId([0x42; 16]);
    let group_id = 2026;

    let mut runtime =
        SingleNodeRuntime::init(dir.path()).expect("Runtime initialization must succeed");

    // 1. Ingress: Produce 100 structured records to immutable physical WAL
    let mut produced_records = Vec::new();
    let mut json_records = Vec::new();
    for i in 0..100 {
        let json_val = serde_json::json!({
            "order_id": format!("ord_{i}"),
            "amount": i * 10,
            "status": if i % 2 == 0 { "PENDING" } else { "COMPLETED" },
            "region": "us-east-1"
        });
        let raw_bytes = serde_json::to_vec(&json_val).unwrap();
        produced_records.push(raw_bytes);
        json_records.push(json_val);
    }

    let resp = runtime
        .produce(stream, AckMode::Durable, &produced_records)
        .expect("100-record batch ingress produce must succeed");

    assert_eq!(resp.base_offset, 0);
    assert_eq!(resp.last_offset, 99);

    // 2. Queuing: Lease first 10 records to Worker 1
    let leased_w1 = runtime
        .lease_records(stream, group_id, 10, 5000)
        .expect("Lease for Worker 1 must succeed");
    assert_eq!(leased_w1.len(), 10);
    assert_eq!(leased_w1[0].0, 0);

    // 3. Out-of-Order ACKs: Worker 1 ACKs records 1..10 (leaving record 0 unacked)
    for &(offset, token) in &leased_w1[1..] {
        runtime
            .ack_record(stream, group_id, offset, token)
            .expect("Fenced ACK must succeed");
    }

    // Watermark MUST remain at 0 (blocked by offset 0)
    assert_eq!(runtime.base_watermark(stream, group_id), 0);

    // 4. Poison-pill resolution: ACK offset 0
    runtime
        .ack_record(stream, group_id, 0, leased_w1[0].1)
        .expect("Ack offset 0 must succeed");

    // Watermark MUST cascade past all 10 acked records to offset 10!
    assert_eq!(runtime.base_watermark(stream, group_id), 10);

    // 5. Columnar Transposition: Export batch to Apache Arrow RecordBatch
    let arrow_batch = runtime
        .export_arrow(&json_records)
        .expect("Columnar shredding to Arrow RecordBatch must succeed");

    assert_eq!(arrow_batch.num_rows(), 100);
    assert!(arrow_batch.num_columns() >= 4);

    // Verify Arrow schema contains shredded fields
    let schema = arrow_batch.schema();
    let field_names: Vec<&str> = schema.fields().iter().map(|f| f.name().as_str()).collect();
    assert!(field_names.contains(&"order_id"));
    assert!(field_names.contains(&"amount"));
    assert!(field_names.contains(&"status"));
    assert!(field_names.contains(&"region"));
    assert!(field_names.contains(&"_unstructured_payload"));
}
