//! Static memory arena implementation for zero-heap hot-path ingress per `KEI-ARC-020`.

/// Default pre-allocated row arena capacity (2 MB per thread ingress ring).
pub const DEFAULT_ROW_ARENA_CAPACITY: usize = 2 * 1024 * 1024;

/// Cache-line alignment in bytes.
pub const CACHE_LINE_ALIGNMENT: usize = 64;

/// Lock-free pre-allocated fixed-capacity memory arena ensuring zero dynamic heap allocations.
pub struct RowArena {
    buffer: Vec<u8>,
    cursor: usize,
}

impl Default for RowArena {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_ROW_ARENA_CAPACITY)
    }
}

impl RowArena {
    /// Create a new pre-allocated row arena of a fixed byte capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: vec![0u8; capacity],
            cursor: 0,
        }
    }

    /// Allocate a slice of `len` bytes from the pre-allocated arena.
    pub fn alloc(&mut self, len: usize) -> Option<&mut [u8]> {
        self.alloc_aligned(len, 1)
    }

    /// Allocate a slice of `len` bytes with an explicit alignment requirement.
    pub fn alloc_aligned(&mut self, len: usize, align: usize) -> Option<&mut [u8]> {
        let align_offset = (align - (self.cursor % align)) % align;
        let start = self.cursor + align_offset;
        let end = start + len;

        if end <= self.buffer.len() {
            self.cursor = end;
            Some(&mut self.buffer[start..end])
        } else {
            None
        }
    }

    /// Reset cursor for re-use without freeing memory (zero heap deallocation).
    pub fn reset(&mut self) {
        self.cursor = 0;
    }

    /// Return total capacity in bytes.
    pub fn capacity(&self) -> usize {
        self.buffer.len()
    }

    /// Return currently allocated bytes.
    pub fn allocated(&self) -> usize {
        self.cursor
    }

    /// Return remaining unallocated bytes.
    pub fn remaining(&self) -> usize {
        self.buffer.len().saturating_sub(self.cursor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_row_arena_allocation_and_reset() {
        let mut arena = RowArena::with_capacity(1024);
        assert_eq!(arena.capacity(), 1024);
        assert_eq!(arena.remaining(), 1024);

        let slice = arena.alloc(256).expect("Allocation should succeed");
        assert_eq!(slice.len(), 256);
        assert_eq!(arena.allocated(), 256);
        assert_eq!(arena.remaining(), 768);

        arena.reset();
        assert_eq!(arena.allocated(), 0);
        assert_eq!(arena.remaining(), 1024);
    }

    #[test]
    fn test_row_arena_aligned_allocation() {
        let mut arena = RowArena::with_capacity(1024);
        arena.alloc(7).unwrap(); // Non-aligned cursor

        let aligned_slice = arena
            .alloc_aligned(128, CACHE_LINE_ALIGNMENT)
            .expect("Aligned allocation should succeed");
        assert_eq!(aligned_slice.len(), 128);

        // Verify start offset is aligned to 64
        let start_offset = arena.allocated() - 128;
        assert_eq!(start_offset % CACHE_LINE_ALIGNMENT, 0);
    }

    #[test]
    fn test_row_arena_out_of_memory() {
        let mut arena = RowArena::with_capacity(128);
        assert!(arena.alloc(256).is_none());
    }
}
