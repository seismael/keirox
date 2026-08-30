//! Domain identifiers and basic types.

use serde::{Deserialize, Serialize};

/// Monotonic log offset.
pub type Offset = u64;

/// 16-byte UUID representation for tenants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TenantId(pub [u8; 16]);

/// 16-byte UUID representation for micro-streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StreamId(pub [u8; 16]);
