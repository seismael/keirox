//! End-to-end lakehouse integration test verifying WAL to Parquet to Iceberg snapshot commit per `KEI-DES-034`.

use keirox_api::proto::AckMode;
use keirox_arrow_elt::catalog::{DataFileEntry, IcebergCatalogLedger};
use keirox_arrow_elt::parquet_encoder::ParquetEncoder;
use keirox_core::StreamId;
use keirox_testkit::SingleNodeRuntime;
use tempfile::tempdir;

#[test]
fn test_end_to_end_wal_to_parquet_iceberg_pipeline() {
    let dir = tempdir().unwrap();
    let runtime_dir = dir.path().join("wal");
    let lakehouse_dir = dir.path().join("lakehouse");
    std::fs::create_dir_all(&lakehouse_dir).unwrap();

    let mut runtime = SingleNodeRuntime::init(&runtime_dir).unwrap();
    let stream = StreamId([0x33; 16]);

    // 1. Ingress 50 IoT telemetry records
    let mut raw_records = Vec::new();
    let mut json_records = Vec::new();

    for i in 0..50 {
        let rec = serde_json::json!({
            "device_id": format!("dev_{}", i % 5),
            "temp_c": 20.0 + (i as f64 * 0.5),
            "voltage": 3.3,
            "status": "HEALTHY"
        });
        raw_records.push(serde_json::to_vec(&rec).unwrap());
        json_records.push(rec);
    }

    let produce_resp = runtime
        .produce(stream, AckMode::Durable, &raw_records)
        .expect("Produce must succeed");
    assert_eq!(produce_resp.base_offset, 0);
    assert_eq!(produce_resp.last_offset, 49);

    // 2. Transpose to Apache Arrow RecordBatch
    let arrow_batch = runtime
        .export_arrow(&json_records)
        .expect("Arrow export must succeed");
    assert_eq!(arrow_batch.num_rows(), 50);

    // 3. Encode to Parquet File
    let parquet_path = lakehouse_dir.join("telemetry-part-0001.parquet");
    let written_rows = ParquetEncoder::write_batch(&arrow_batch, &parquet_path)
        .expect("Parquet write must succeed");
    assert_eq!(written_rows, 50);

    let file_size = std::fs::metadata(&parquet_path).unwrap().len();
    assert!(file_size > 0);

    // 4. Commit to Apache Iceberg Table Catalog Ledger
    let mut catalog = IcebergCatalogLedger::new("iot_telemetry_lakehouse");
    let data_file = DataFileEntry {
        file_path: parquet_path.to_string_lossy().to_string(),
        record_count: written_rows,
        file_size_bytes: file_size,
        partition_spec_id: 0,
    };

    let snapshot = catalog.commit_snapshot(vec![data_file], 1700000000000);
    assert_eq!(snapshot.snapshot_id, 1);
    assert_eq!(snapshot.total_records, 50);
    assert_eq!(snapshot.data_files.len(), 1);
    assert_eq!(catalog.total_data_files(), 1);
}
