//! Adaptive schema shredder logic per `KEI-ARC-023` and `KEI-DES-033`.

use std::collections::HashSet;

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_shredder_64_field_cap() {
        let mut shredder = AdaptiveShredder::new(3);

        assert!(shredder.try_promote_field("user_id"));
        assert!(shredder.try_promote_field("event_type"));
        assert!(shredder.try_promote_field("timestamp"));

        // Fourth field exceeds cap -> Must return false for fallback
        assert!(!shredder.try_promote_field("extra_field"));
        assert_eq!(shredder.promoted_count(), 3);
    }
}
