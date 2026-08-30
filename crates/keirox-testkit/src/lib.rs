//! # Keirox Testkit
//!
//! Deterministic test utilities, fixtures, and assertions for Keirox.

#![deny(missing_docs)]

/// Create a test stream identifier from a seed.
pub fn test_stream_id(seed: u8) -> keirox_core::StreamId {
    keirox_core::StreamId([seed; 16])
}

/// Create a test tenant identifier from a seed.
pub fn test_tenant_id(seed: u8) -> keirox_core::TenantId {
    keirox_core::TenantId([seed; 16])
}
