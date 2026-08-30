//! # Keirox Testkit
//!
//! Deterministic test utilities, fixtures, and assertions for Keirox.

#![deny(missing_docs)]

/// Unified single-node runtime coordinator.
pub mod engine;

pub use engine::SingleNodeRuntime;

/// Create a test stream identifier from a seed.
pub fn test_stream_id(seed: u8) -> keirox_core::StreamId {
    keirox_core::StreamId([seed; 16])
}

/// Create a test tenant identifier from a seed.
pub fn test_tenant_id(seed: u8) -> keirox_core::TenantId {
    keirox_core::TenantId([seed; 16])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_testkit_helpers() {
        let s = test_stream_id(7);
        assert_eq!(s.0, [7; 16]);

        let t = test_tenant_id(9);
        assert_eq!(t.0, [9; 16]);
    }
}
