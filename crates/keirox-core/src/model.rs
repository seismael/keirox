//! Domain identifiers and basic types.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Monotonic log offset.
pub type Offset = u64;

/// 16-byte UUID representation for tenants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TenantId(pub [u8; 16]);

impl fmt::Display for TenantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "tenant-")?;
        for byte in &self.0 {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

/// 16-byte UUID representation for micro-streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StreamId(pub [u8; 16]);

impl fmt::Display for StreamId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "stream-")?;
        for byte in &self.0 {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_id_display() {
        let tenant = TenantId([0x12; 16]);
        assert_eq!(
            tenant.to_string(),
            "tenant-12121212121212121212121212121212"
        );
    }

    #[test]
    fn test_stream_id_display() {
        let stream = StreamId([0xab; 16]);
        assert_eq!(
            stream.to_string(),
            "stream-abababababababababababababababab"
        );
    }

    #[test]
    fn test_equality_and_ordering() {
        let s1 = StreamId([1; 16]);
        let s2 = StreamId([2; 16]);
        assert_ne!(s1, s2);
        assert!(s1 < s2);
    }
}
