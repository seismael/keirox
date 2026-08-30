//! Sparse offset index for fast $O(\log n)$ random access per `KEI-ARC-020` §6.2.

/// Lightweight 16-byte sparse index entry.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SparseIndexEntry {
    /// Logical record offset at the start of an index bucket.
    pub logical_offset: u64,
    /// Physical segment ID containing the offset.
    pub segment_id: u32,
    /// Byte offset within the physical segment file.
    pub byte_offset: u32,
}

impl SparseIndexEntry {
    /// Create a new sparse index entry.
    pub fn new(logical_offset: u64, segment_id: u32, byte_offset: u32) -> Self {
        Self {
            logical_offset,
            segment_id,
            byte_offset,
        }
    }
}

/// Sparse in-memory index for indexing physical segment positions at periodic offset intervals.
#[derive(Debug, Default)]
pub struct SparseOffsetIndex {
    /// Configurable sample interval (e.g. 4096 records per index entry).
    sample_interval: u64,
    /// Monotonically sorted index entries.
    entries: Vec<SparseIndexEntry>,
}

impl SparseOffsetIndex {
    /// Create a new sparse index with a specified sampling interval.
    pub fn new(sample_interval: u64) -> Self {
        Self {
            sample_interval: sample_interval.max(1),
            entries: Vec::new(),
        }
    }

    /// Maybe index an entry if it falls on the sample interval boundary.
    pub fn maybe_index(&mut self, logical_offset: u64, segment_id: u32, byte_offset: u32) -> bool {
        if logical_offset.is_multiple_of(self.sample_interval) {
            self.entries.push(SparseIndexEntry::new(
                logical_offset,
                segment_id,
                byte_offset,
            ));
            true
        } else {
            false
        }
    }

    /// Binary search for the closest preceding index entry for a given target offset ($O(\log n)$).
    pub fn find_floor(&self, target_offset: u64) -> Option<SparseIndexEntry> {
        if self.entries.is_empty() {
            return None;
        }

        match self
            .entries
            .binary_search_by_key(&target_offset, |e| e.logical_offset)
        {
            Ok(idx) => Some(self.entries[idx]),
            Err(0) => None,
            Err(idx) => Some(self.entries[idx - 1]),
        }
    }

    /// Return total indexed points.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return true if index is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn test_sparse_index_entry_size() {
        assert_eq!(size_of::<SparseIndexEntry>(), 16);
    }

    #[test]
    fn test_sparse_index_binary_search() {
        let mut index = SparseOffsetIndex::new(100);

        // Index at 0, 100, 200, 300
        index.maybe_index(0, 1, 4096);
        index.maybe_index(100, 1, 16384);
        index.maybe_index(200, 2, 4096);
        index.maybe_index(300, 2, 16384);

        assert_eq!(index.len(), 4);

        // Exact match at 100
        let entry100 = index.find_floor(100).unwrap();
        assert_eq!(entry100.logical_offset, 100);
        assert_eq!(entry100.segment_id, 1);

        // Target 250 -> Floor is entry at 200
        let entry250 = index.find_floor(250).unwrap();
        assert_eq!(entry250.logical_offset, 200);
        assert_eq!(entry250.segment_id, 2);

        // Target before first entry
        assert!(index.find_floor(0).is_some());
    }
}
