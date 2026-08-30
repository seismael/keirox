//! Manifest metadata registry and sealed chunk tracking per `KEI-ARC-020` and `KEI-DES-034`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Sealed Tier-1 cloud object chunk manifest entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkManifestEntry {
    /// Stream identifier UUID bytes.
    pub stream_id: [u8; 16],
    /// Start logical offset of sealed chunk.
    pub start_offset: u64,
    /// End logical offset of sealed chunk (inclusive).
    pub end_offset: u64,
    /// Cloud storage S3 / GCS URI.
    pub s3_uri: String,
    /// Chunk size in bytes.
    pub size_bytes: u64,
    /// CRC32C checksum of physical chunk payload.
    pub crc32: u32,
    /// Nanosecond timestamp when chunk was sealed and committed.
    pub sealed_at_ns: u64,
}

/// Durable manifest registry tracking all sealed Tier-1 cloud chunks per stream.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ManifestRegistry {
    /// Mapping of stream_id -> sorted list of chunk manifests.
    chunks_by_stream: HashMap<[u8; 16], Vec<ChunkManifestEntry>>,
}

impl ManifestRegistry {
    /// Create a new empty manifest registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new sealed chunk manifest.
    pub fn register(&mut self, entry: ChunkManifestEntry) {
        let list = self.chunks_by_stream.entry(entry.stream_id).or_default();
        list.push(entry);
        list.sort_by_key(|c| c.start_offset);
    }

    /// Find the chunk covering a given logical offset for a stream.
    #[must_use]
    pub fn find_chunk_covering(
        &self,
        stream_id: &[u8; 16],
        offset: u64,
    ) -> Option<&ChunkManifestEntry> {
        let list = self.chunks_by_stream.get(stream_id)?;
        list.iter()
            .find(|c| offset >= c.start_offset && offset <= c.end_offset)
    }

    /// List all chunk manifests for a given stream.
    #[must_use]
    pub fn list_chunks_for_stream(&self, stream_id: &[u8; 16]) -> &[ChunkManifestEntry] {
        self.chunks_by_stream
            .get(stream_id)
            .map_or(&[], |v| v.as_slice())
    }

    /// Total registered chunks across all streams.
    #[must_use]
    pub fn total_chunks(&self) -> usize {
        self.chunks_by_stream.values().map(|v| v.len()).sum()
    }

    /// Total bytes registered across all streams.
    #[must_use]
    pub fn total_bytes(&self) -> u64 {
        self.chunks_by_stream
            .values()
            .flat_map(|v| v.iter())
            .map(|c| c.size_bytes)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_registration_and_lookup() {
        let mut registry = ManifestRegistry::new();
        let stream_id = [1u8; 16];

        registry.register(ChunkManifestEntry {
            stream_id,
            start_offset: 0,
            end_offset: 999,
            s3_uri: "s3://bucket/chunks/0_999.chk".into(),
            size_bytes: 65_536,
            crc32: 0x1234_5678,
            sealed_at_ns: 1_700_000_000,
        });

        registry.register(ChunkManifestEntry {
            stream_id,
            start_offset: 1000,
            end_offset: 1999,
            s3_uri: "s3://bucket/chunks/1000_1999.chk".into(),
            size_bytes: 65_536,
            crc32: 0x8765_4321,
            sealed_at_ns: 1_700_001_000,
        });

        let chunk = registry.find_chunk_covering(&stream_id, 1500).unwrap();
        assert_eq!(chunk.start_offset, 1000);
        assert_eq!(chunk.end_offset, 1999);
        assert_eq!(registry.total_chunks(), 2);
    }
}
