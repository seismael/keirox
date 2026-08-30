//! S3 object key hash-prefix partitioning and URI formatting per `KEI-ARC-020`.

use twox_hash::XxHash64;

/// Partitioner computing uniform hash-prefix distribution for cloud object storage keys.
#[derive(Debug, Clone, Default)]
pub struct HashPrefixPartitioner {
    bucket: String,
}

impl HashPrefixPartitioner {
    /// Create partitioner for a target cloud storage bucket.
    #[must_use]
    pub fn new(bucket: impl Into<String>) -> Self {
        Self {
            bucket: bucket.into(),
        }
    }

    /// Generate an S3 URI with a uniform 4-hex-digit hash prefix to prevent S3 prefix throttling.
    ///
    /// Layout: `s3://{bucket}/chunks/{prefix}/{tenant_id}/{stream_id}/{start_offset}_{end_offset}.chk`
    #[must_use]
    pub fn format_chunk_uri(
        &self,
        tenant_id: &[u8; 16],
        stream_id: &[u8; 16],
        start_offset: u64,
        end_offset: u64,
    ) -> String {
        use std::hash::Hasher;
        let mut hasher = XxHash64::default();
        hasher.write(tenant_id);
        hasher.write(stream_id);
        hasher.write_u64(start_offset);
        let hash = hasher.finish();

        let prefix = format!("{:04x}", (hash >> 48) & 0xFFFF);
        let tenant_hex = hex_encode(tenant_id);
        let stream_hex = hex_encode(stream_id);

        format!(
            "s3://{}/chunks/{}/{}/{}/{}_{}.chk",
            self.bucket, prefix, tenant_hex, stream_hex, start_offset, end_offset
        )
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_prefix_partitioning() {
        let partitioner = HashPrefixPartitioner::new("keirox-lakehouse-prod");
        let tenant_id = [1u8; 16];
        let stream_id = [2u8; 16];

        let uri = partitioner.format_chunk_uri(&tenant_id, &stream_id, 0, 9999);
        assert!(uri.starts_with("s3://keirox-lakehouse-prod/chunks/"));
        assert!(uri.ends_with("/0_9999.chk"));
    }
}
