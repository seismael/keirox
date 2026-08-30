//! Adaptive schema shredder logic per `KEI-ARC-023` and `KEI-DES-033`.

use arrow::array::{ArrayRef, StringBuilder};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use keirox_core::error::{KeiroxError, Result};
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

/// Default maximum promoted top-level columns per `KEI-DES-033` §3.
pub const DEFAULT_MAX_INFERRED_FIELDS: usize = 64;

/// Fallback column name for excess fields.
pub const UNSTRUCTURED_PAYLOAD_COLUMN: &str = "_unstructured_payload";

/// Adaptive schema shredder converting semi-structured rows to columnar Arrow batches.
#[derive(Debug)]
pub struct AdaptiveShredder {
    max_inferred_fields: usize,
    promoted_fields: HashSet<String>,
}

impl Default for AdaptiveShredder {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_INFERRED_FIELDS)
    }
}

impl AdaptiveShredder {
    /// Create a new shredder with a field cap (default 64 per `KEI-DES-033`).
    pub fn new(max_inferred_fields: usize) -> Self {
        Self {
            max_inferred_fields,
            promoted_fields: HashSet::new(),
        }
    }

    /// Return the maximum number of shredded columns allowed before fallback to `_unstructured_payload`.
    pub fn max_fields(&self) -> usize {
        self.max_inferred_fields
    }

    /// Try to promote a field name. Returns true if promoted, false if routed to unstructured fallback.
    pub fn try_promote_field(&mut self, field_name: &str) -> bool {
        if self.promoted_fields.contains(field_name) {
            return true;
        }

        if self.promoted_fields.len() < self.max_inferred_fields {
            self.promoted_fields.insert(field_name.to_string());
            true
        } else {
            false
        }
    }

    /// Count of currently promoted fields.
    pub fn promoted_count(&self) -> usize {
        self.promoted_fields.len()
    }

    /// Shred a slice of JSON values into an Apache Arrow `RecordBatch`.
    pub fn shred_json_records(&mut self, records: &[serde_json::Value]) -> Result<RecordBatch> {
        if records.is_empty() {
            let empty_schema = Arc::new(Schema::new(vec![Field::new(
                UNSTRUCTURED_PAYLOAD_COLUMN,
                DataType::Utf8,
                true,
            )]));
            let empty_array: ArrayRef = Arc::new(StringBuilder::new().finish());
            return RecordBatch::try_new(empty_schema, vec![empty_array])
                .map_err(|e| KeiroxError::Internal(e.to_string()));
        }

        // 1. Discover fields across all records and attempt promotion
        for record in records {
            if let Some(obj) = record.as_object() {
                for key in obj.keys() {
                    self.try_promote_field(key);
                }
            }
        }

        // Sort promoted fields for deterministic column ordering
        let mut sorted_fields: Vec<String> = self.promoted_fields.iter().cloned().collect();
        sorted_fields.sort();

        // 2. Build column arrays
        let mut column_builders: BTreeMap<String, StringBuilder> = sorted_fields
            .iter()
            .map(|f| {
                (
                    f.clone(),
                    StringBuilder::with_capacity(records.len(), records.len() * 16),
                )
            })
            .collect();
        let mut unstructured_builder =
            StringBuilder::with_capacity(records.len(), records.len() * 32);

        for record in records {
            let mut excess = serde_json::Map::new();
            if let Some(obj) = record.as_object() {
                for (k, v) in obj {
                    if let Some(builder) = column_builders.get_mut(k) {
                        match v {
                            serde_json::Value::String(s) => builder.append_value(s),
                            _ => builder.append_value(v.to_string()),
                        }
                    } else {
                        excess.insert(k.clone(), v.clone());
                    }
                }

                // Fill nulls for promoted fields not present in this record
                for (field_name, builder) in &mut column_builders {
                    if !obj.contains_key(field_name) {
                        builder.append_null();
                    }
                }
            } else {
                for builder in column_builders.values_mut() {
                    builder.append_null();
                }
                excess.insert("raw".to_string(), record.clone());
            }

            if excess.is_empty() {
                unstructured_builder.append_null();
            } else {
                unstructured_builder.append_value(serde_json::Value::Object(excess).to_string());
            }
        }

        // 3. Assemble Schema and RecordBatch
        let mut arrow_fields = Vec::new();
        let mut arrow_columns: Vec<ArrayRef> = Vec::new();

        for (field_name, mut builder) in column_builders {
            arrow_fields.push(Field::new(&field_name, DataType::Utf8, true));
            arrow_columns.push(Arc::new(builder.finish()));
        }

        arrow_fields.push(Field::new(
            UNSTRUCTURED_PAYLOAD_COLUMN,
            DataType::Utf8,
            true,
        ));
        arrow_columns.push(Arc::new(unstructured_builder.finish()));

        let schema = Arc::new(Schema::new(arrow_fields));
        RecordBatch::try_new(schema, arrow_columns)
            .map_err(|e| KeiroxError::Internal(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_shredder_64_field_cap() {
        let mut shredder = AdaptiveShredder::new(64);

        for i in 0..64 {
            assert!(shredder.try_promote_field(&format!("col_{i}")));
        }
        assert_eq!(shredder.promoted_count(), 64);

        // 65th field must be rejected
        assert!(!shredder.try_promote_field("col_65"));
        assert_eq!(shredder.promoted_count(), 64);
    }

    #[test]
    fn test_shred_json_records_to_arrow_record_batch() {
        let mut shredder = AdaptiveShredder::new(3);

        let records = vec![
            serde_json::json!({"user_id": "u1", "action": "login", "extra1": "val1"}),
            serde_json::json!({"user_id": "u2", "action": "click", "extra2": "val2"}),
            serde_json::json!({"user_id": "u3", "amount": 99, "excess_field": "overflow"}),
        ];

        let batch = shredder
            .shred_json_records(&records)
            .expect("Shredding must succeed");

        assert_eq!(batch.num_rows(), 3);
        assert!(batch.num_columns() >= 4); // promoted + _unstructured_payload
    }
}
