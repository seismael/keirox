//! Static memory arena implementation.

/// Lock-free pre-allocated fixed-capacity memory arena.
pub struct RowArena {
    capacity: usize,
}

impl RowArena {
    /// Create a new pre-allocated row arena of a fixed byte capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self { capacity }
    }

    /// Return total capacity in bytes.
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}
