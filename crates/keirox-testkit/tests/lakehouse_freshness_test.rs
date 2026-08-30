//! Lakehouse Iceberg catalog commits, schema evolution, and freshness tests per `KEI-LAKE-301` and `KEI-ENG-300`.

use arrow::datatypes::DataType;
use keirox_arrow_elt::catalog::DataFileEntry;
use keirox_arrow_elt::iceberg_committer::{CommitCadenceMode, IcebergCatalogCommitter};
use keirox_schema::compatibility::{FieldType, SchemaDefinition};
use keirox_schema::registry::{SchemaRegistry, SchemaVersion};
use keirox_schema::shredding_policy::{
    AdaptiveShreddingPolicy, MAX_SHREDDED_COLUMNS, UNSTRUCTURED_PAYLOAD_COLUMN,
};
use std::collections::BTreeMap;

#[tokio::test]
async fn test_lakehouse_catalog_commit_schema_evolution_and_adaptive_shredding() {
    // 1. Iceberg Catalog Committer
    let committer = IcebergCatalogCommitter::new();
    committer.register_table("fact_orders", CommitCadenceMode::FastStreaming);

    let files1 = vec![DataFileEntry {
        file_path: "s3://lakehouse/fact_orders/part-001.parquet".into(),
        record_count: 5_000,
        file_size_bytes: 64 * 1024 * 1024,
        partition_spec_id: 0,
    }];

    // Commit snapshot 1
    let snap1 = committer
        .commit_data_files("fact_orders", None, files1, 1_700_000_000_000)
        .unwrap();
    assert_eq!(snap1.snapshot_id, 1);
    assert_eq!(snap1.total_records, 5_000);

    // Commit snapshot 2 (contiguous commit)
    let files2 = vec![DataFileEntry {
        file_path: "s3://lakehouse/fact_orders/part-002.parquet".into(),
        record_count: 3_000,
        file_size_bytes: 48 * 1024 * 1024,
        partition_spec_id: 0,
    }];
    let snap2 = committer
        .commit_data_files("fact_orders", Some(1), files2, 1_700_000_005_000)
        .unwrap();
    assert_eq!(snap2.snapshot_id, 2);
    assert_eq!(snap2.parent_snapshot_id, Some(1));
    assert_eq!(snap2.total_records, 8_000);

    // 2. Schema Registry Evolution & Versioning
    let schema_registry = SchemaRegistry::new();

    let mut schema_v1 = SchemaDefinition::new();
    schema_v1.add_field("order_id", FieldType::Int64, true);
    schema_v1.add_field("amount", FieldType::Float64, false);

    let reg_v1 = schema_registry
        .register("fact_orders", schema_v1.clone())
        .await
        .unwrap();
    assert_eq!(reg_v1.version, SchemaVersion(1));

    let mut schema_v2 = schema_v1.clone();
    schema_v2.add_field("shipping_zip", FieldType::Utf8, false); // Optional field addition
    let reg_v2 = schema_registry
        .register("fact_orders", schema_v2)
        .await
        .unwrap();
    assert_eq!(reg_v2.version, SchemaVersion(2));

    // 3. Top-64 Adaptive Shredding Policy
    let mut shredding_policy = AdaptiveShreddingPolicy::new();
    let mut detected_fields = BTreeMap::new();

    for i in 0..80 {
        let name = format!("col_{i:03}");
        shredding_policy.record_field_observation(&name);
        detected_fields.insert(name, DataType::Utf8);
    }

    let arrow_schema = shredding_policy
        .derive_arrow_schema(&detected_fields)
        .unwrap();
    assert!(arrow_schema.fields().len() <= MAX_SHREDDED_COLUMNS);
    assert!(arrow_schema
        .field_with_name(UNSTRUCTURED_PAYLOAD_COLUMN)
        .is_ok());
}
