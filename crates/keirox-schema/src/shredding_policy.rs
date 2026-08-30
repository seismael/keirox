//! Top-64 field adaptive columnar shredding policy per `KEI-DES-033` §4.

use arrow::datatypes::{DataType, Field, Schema};
use keirox_core::error::Result;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// Maximum number of typed columnar fields retained before spilling to `_unstructured_payload` JSON.
pub const MAX_SHREDDED_COLUMNS: usize = 64;

/// Reserved name for the fallback JSON overflow column.
pub const UNSTRUCTURED_PAYLOAD_COLUMN: &str = "_unstructured_payload";

/// Adaptive shredding policy governing JSON-to-Arrow schema derivation and field priority.
#[derive(Debug, Clone)]
pub struct AdaptiveShreddingPolicy {
    field_frequencies: HashMap<String, u64>,
    pinned_fields: Vec<String>,
}

impl AdaptiveShreddingPolicy {
    /// Create a new adaptive shredding policy with standard metadata fields pinned.
    #[must_use]
    pub fn new() -> Self {
        Self {
            field_frequencies: HashMap::new(),
            pinned_fields: vec!["_offset".to_string(), "_timestamp_ns".to_string()],
        }
    }

    /// Pinned metadata fields.
    #[must_use]
    pub fn pinned_fields(&self) -> &[String] {
        &self.pinned_fields
    }

    /// Record observed field occurrence from ingested JSON records.
    pub fn record_field_observation(&mut self, field_name: &str) {
        *self
            .field_frequencies
            .entry(field_name.to_string())
            .or_insert(0) += 1;
    }

    /// Derive an Arrow `Schema` by selecting the top observed fields up to the 64-field cap.
    pub fn derive_arrow_schema(
        &self,
        all_detected_fields: &BTreeMap<String, DataType>,
    ) -> Result<Arc<Schema>> {
        let mut fields: Vec<Field> = Vec::new();

        // 1. Mandatory metadata fields
        fields.push(Field::new("_offset", DataType::UInt64, false));
        fields.push(Field::new("_timestamp_ns", DataType::Int64, false));

        // 2. Rank candidate data fields by frequency
        let mut candidates: Vec<(&String, &DataType)> = all_detected_fields
            .iter()
            .filter(|(name, _)| name.as_str() != "_offset" && name.as_str() != "_timestamp_ns")
            .collect();

        candidates.sort_by(|(a_name, _), (b_name, _)| {
            let freq_a = self.field_frequencies.get(*a_name).copied().unwrap_or(0);
            let freq_b = self.field_frequencies.get(*b_name).copied().unwrap_or(0);
            freq_b.cmp(&freq_a).then_with(|| a_name.cmp(b_name))
        });

        // 3. Take top fields up to cap - 1 (leaving 1 slot for _unstructured_payload)
        let max_data_fields = MAX_SHREDDED_COLUMNS - 3; // 2 metadata + 1 overflow
        let selected_count = candidates.len().min(max_data_fields);

        for (name, dt) in candidates.into_iter().take(selected_count) {
            fields.push(Field::new(name.as_str(), (*dt).clone(), true));
        }

        // 4. Always add the overflow fallback column
        fields.push(Field::new(
            UNSTRUCTURED_PAYLOAD_COLUMN,
            DataType::Utf8,
            true,
        ));

        Ok(Arc::new(Schema::new(fields)))
    }
}

impl Default for AdaptiveShreddingPolicy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_shredding_caps_at_64_columns() {
        let mut policy = AdaptiveShreddingPolicy::new();
        let mut detected = BTreeMap::new();

        for i in 0..100 {
            let field_name = format!("field_{i:03}");
            policy.record_field_observation(&field_name);
            detected.insert(field_name, DataType::Utf8);
        }

        let schema = policy.derive_arrow_schema(&detected).unwrap();
        assert!(schema.fields().len() <= MAX_SHREDDED_COLUMNS);
        assert!(schema.field_with_name(UNSTRUCTURED_PAYLOAD_COLUMN).is_ok());
    }
}
