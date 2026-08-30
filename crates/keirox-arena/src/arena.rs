//! Static memory arena implementation for zero-heap hot-path ingress per `KEI-ARC-020`.

/// Lock-free pre-allocated fixed-capacity memory arena.
pub struct RowArena {
    buffer: Vec<u8>,
    cursor: usize,
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
        if self.cursor + len <= self.buffer.len() {
            let start = self.cursor;
            self.cursor += len;
            Some(&mut self.buffer[start..self.cursor])
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
        slice[0] = 0xAA;
        assert_eq!(arena.allocated(), 256);
        assert_eq!(arena.remaining(), 768);

        // Reset arena
        arena.reset();
        assert_eq!(arena.allocated(), 0);
        assert_eq!(arena.remaining(), 1024);
    }

    #[test]
    fn test_row_arena_out_of_memory() {
        let mut arena = RowArena::with_capacity(100);
        assert!(arena.alloc(150).is_none());
        assert!(arena.alloc(100).is_some());
        assert!(arena.alloc(1).is_none());
    }
}
