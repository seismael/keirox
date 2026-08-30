//! Adaptive schema shredder logic.

/// Adaptive schema shredder converting semi-structured rows to columnar Arrow batches.
#[derive(Debug, Default)]
pub struct AdaptiveShredder {
    max_inferred_fields: usize,
}

impl AdaptiveShredder {
    /// Create a new shredder with a field cap (default 64 per `KEI-DES-033`).
    pub fn new(max_inferred_fields: usize) -> Self {
        Self {
            max_inferred_fields,
        }
    }

    /// Return the maximum number of shredded columns allowed before fallback to `_unstructured_payload`.
    pub fn max_fields(&self) -> usize {
        self.max_inferred_fields
    }
}
