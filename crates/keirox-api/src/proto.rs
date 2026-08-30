//! Wire protocol definitions and message structures per `KEI-DES-032` (ADR-020).

use keirox_core::model::{Offset, StreamId, TenantId};
use std::fmt;

/// Client-selectable acknowledgment durability modes (ADR-020).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AckMode {
    /// Acknowledged once recorded in memory arena and replicated to coordinator.
    Fast = 0,
    /// Acknowledged only after physical NVMe flush on Raft quorum ($JML=0$).
    Durable = 1,
}

impl fmt::Display for AckMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fast => write!(f, "ACK_FAST"),
            Self::Durable => write!(f, "ACK_DURABLE"),
        }
    }
}

/// Request to append a batch of records to an immutable stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProduceBatchRequest {
    /// Tenant isolation identifier.
    pub tenant_id: TenantId,
    /// Target micro-stream identifier.
    pub stream_id: StreamId,
    /// Client-specified acknowledgment durability requirement.
    pub ack_mode: AckMode,
    /// Raw payload bytes for each individual record in batch.
    pub records: Vec<Vec<u8>>,
}

/// Response returned after successfully appending a record batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProduceBatchResponse {
    /// Base physical offset assigned to the first record in batch.
    pub base_offset: Offset,
    /// Physical offset assigned to the last record in batch.
    pub last_offset: Offset,
    /// Timestamp when batch was accepted by coordinator (microseconds).
    pub timestamp_us: u64,
}

/// Request to lease available records for exclusive worker processing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseRecordsRequest {
    /// Tenant isolation identifier.
    pub tenant_id: TenantId,
    /// Target micro-stream identifier.
    pub stream_id: StreamId,
    /// Consumer group identifier.
    pub group_id: u64,
    /// Worker instance identity.
    pub worker_id: u64,
    /// Maximum number of records to lease in single call.
    pub max_records: u32,
    /// Lease duration in milliseconds before automatic expiration.
    pub ttl_ms: u32,
}

/// Request to acknowledge a leased record as successfully processed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcknowledgeRequest {
    /// Tenant isolation identifier.
    pub tenant_id: TenantId,
    /// Target micro-stream identifier.
    pub stream_id: StreamId,
    /// Consumer group identifier.
    pub group_id: u64,
    /// Logical record offset.
    pub offset: Offset,
    /// Fencing lease token issued at lease grant (ADR-024).
    pub lease_token: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ack_mode_display_and_values() {
        assert_eq!(AckMode::Fast.to_string(), "ACK_FAST");
        assert_eq!(AckMode::Durable.to_string(), "ACK_DURABLE");
        assert_ne!(AckMode::Fast, AckMode::Durable);
    }

    #[test]
    fn test_produce_and_lease_request_instantiation() {
        let tenant = TenantId([1; 16]);
        let stream = StreamId([2; 16]);

        let produce_req = ProduceBatchRequest {
            tenant_id: tenant,
            stream_id: stream,
            ack_mode: AckMode::Durable,
            records: vec![vec![1, 2, 3]],
        };
        assert_eq!(produce_req.ack_mode, AckMode::Durable);
        assert_eq!(produce_req.records.len(), 1);

        let ack_req = AcknowledgeRequest {
            tenant_id: tenant,
            stream_id: stream,
            group_id: 100,
            offset: 42,
            lease_token: 999,
        };
        assert_eq!(ack_req.offset, 42);
        assert_eq!(ack_req.lease_token, 999);
    }
}
