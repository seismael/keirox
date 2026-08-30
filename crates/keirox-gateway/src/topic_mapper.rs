//! Kafka topic and virtual partition namespace mapper per `KEI-DES-035` §4.

use keirox_core::model::{StreamId, TenantId};
use std::hash::Hasher;
use twox_hash::XxHash64;

/// Topic mapper translating between Kafka topic names and Keirox Stream identifiers.
#[derive(Debug, Clone)]
pub struct TopicMapper {
    default_tenant: TenantId,
}

impl TopicMapper {
    /// Initialize topic mapper with default tenant context.
    #[must_use]
    pub fn new(default_tenant: TenantId) -> Self {
        Self { default_tenant }
    }

    /// Derive a deterministic 128-bit `StreamId` from a Kafka topic name and partition index.
    #[must_use]
    pub fn map_to_stream(&self, topic: &str, partition: i32) -> StreamId {
        let mut hasher1 = XxHash64::with_seed(0xCAFE_BABE);
        hasher1.write(topic.as_bytes());
        hasher1.write_i32(partition);
        let h1 = hasher1.finish();

        let mut hasher2 = XxHash64::with_seed(0xDEAD_BEEF);
        hasher2.write(topic.as_bytes());
        hasher2.write_i32(partition);
        let h2 = hasher2.finish();

        let mut stream_bytes = [0u8; 16];
        stream_bytes[..8].copy_from_slice(&h1.to_le_bytes());
        stream_bytes[8..].copy_from_slice(&h2.to_le_bytes());

        StreamId(stream_bytes)
    }

    /// Tenant identifier for gateway operations.
    #[must_use]
    pub fn tenant_id(&self) -> TenantId {
        self.default_tenant
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_to_stream_deterministic_mapping() {
        let mapper = TopicMapper::new(TenantId([1u8; 16]));
        let s1 = mapper.map_to_stream("user-events", 0);
        let s2 = mapper.map_to_stream("user-events", 0);
        let s3 = mapper.map_to_stream("user-events", 1);

        assert_eq!(s1, s2);
        assert_ne!(s1, s3);
    }
}
