//! Consumer group state definitions and Roaring Bitmap state overlay per `KEI-ARC-021` and `KEI-DES-031`.

use keirox_core::error::{KeiroxError, Result};
use keirox_core::model::{Offset, StreamId};
use keirox_core::traits::StateOverlayEngine;
use roaring::RoaringTreemap;
use std::collections::HashMap;

/// Default maximum delivery retry attempts before mandatory Virtual DLQ eviction per `KEI-DES-031`.
pub const DEFAULT_MAX_RETRIES: u32 = 3;

/// 64-byte aligned State Shard Key identifying a consumption state shard per `KEI-DES-031` §4.1.
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StateShardKey {
    /// Tenant isolation identifier.
    pub tenant_id: u64,
    /// Stream 128-bit micro-stream identifier.
    pub stream_id: [u8; 16],
    /// Consumer group identifier.
    pub group_id: u64,
    /// Shard bucket identifier.
    pub shard_bucket: u16,
    /// Padding to exactly 64 bytes.
    pub _reserved: [u8; 30],
}

impl StateShardKey {
    /// Create a new state shard key.
    pub fn new(tenant_id: u64, stream_id: [u8; 16], group_id: u64, shard_bucket: u16) -> Self {
        Self {
            tenant_id,
            stream_id,
            group_id,
            shard_bucket,
            _reserved: [0u8; 30],
        }
    }
}

/// Active lease tracking entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveLease {
    /// Monotonic fencing lease token (ADR-024).
    pub token: u64,
    /// Lease expiration timestamp in microseconds.
    pub expires_at_us: u64,
    /// Delivery attempt count.
    pub attempt: u32,
}

/// Disjoint states for an offset within a consumer group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsumerState {
    /// Available for delivery.
    Ready,
    /// Currently leased to a consumer instance with expiration timestamp τ and token.
    Leased {
        /// Lease deadline timestamp in microseconds.
        expires_at_us: u64,
        /// Fencing lease token.
        token: u64,
    },
    /// Acknowledged as successfully processed.
    Acked,
    /// Evicted to Virtual Dead-Letter Queue after exceeding max retries.
    EvictedDlq,
}

/// In-memory consumption state overlay for a consumer group shard.
#[derive(Debug)]
pub struct ConsumerGroupState {
    /// Monotonic sliding base watermark ($W_{base}$).
    pub base_watermark: u64,
    /// Head offset written to physical WAL.
    pub head_offset: u64,
    /// Maximum allowed delivery retries before mandatory DLQ eviction.
    pub max_retries: u32,
    /// Next monotonic lease token counter.
    next_lease_token: u64,
    /// 64-bit Roaring Bitmap of acknowledged offsets.
    acked: RoaringTreemap,
    /// 64-bit Roaring Bitmap of evicted DLQ offsets.
    evicted_dlq: RoaringTreemap,
    /// Map of currently active leases (offset -> ActiveLease).
    leases: HashMap<u64, ActiveLease>,
    /// Persistent retry attempt counters per offset.
    retry_counts: HashMap<u64, u32>,
}

impl Default for ConsumerGroupState {
    fn default() -> Self {
        Self {
            base_watermark: 0,
            head_offset: 0,
            max_retries: DEFAULT_MAX_RETRIES,
            next_lease_token: 1,
            acked: RoaringTreemap::new(),
            evicted_dlq: RoaringTreemap::new(),
            leases: HashMap::new(),
            retry_counts: HashMap::new(),
        }
    }
}

impl ConsumerGroupState {
    /// Create a new state overlay starting at base watermark 0 with default retry limits.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new state overlay with a custom retry limit.
    pub fn with_max_retries(max_retries: u32) -> Self {
        Self {
            max_retries,
            ..Self::default()
        }
    }

    /// Return the current state for a given offset.
    pub fn get_state(&self, offset: u64) -> ConsumerState {
        if offset < self.base_watermark {
            if self.evicted_dlq.contains(offset) {
                return ConsumerState::EvictedDlq;
            }
            return ConsumerState::Acked;
        }

        if self.acked.contains(offset) {
            ConsumerState::Acked
        } else if self.evicted_dlq.contains(offset) {
            ConsumerState::EvictedDlq
        } else if let Some(lease) = self.leases.get(&offset) {
            ConsumerState::Leased {
                expires_at_us: lease.expires_at_us,
                token: lease.token,
            }
        } else {
            ConsumerState::Ready
        }
    }

    /// Grant a lease on an offset, returning the generated fencing lease token if successful.
    pub fn lease(&mut self, offset: u64, expires_at_us: u64) -> Option<u64> {
        let token = self.next_lease_token;
        self.next_lease_token += 1;
        if self.lease_with_token(offset, expires_at_us, token) {
            Some(token)
        } else {
            None
        }
    }

    /// Grant a lease on an offset with an explicit fencing token (ADR-024).
    pub fn lease_with_token(&mut self, offset: u64, expires_at_us: u64, token: u64) -> bool {
        if self.get_state(offset) == ConsumerState::Ready {
            let attempt = self.retry_counts.entry(offset).or_insert(0);
            *attempt += 1;

            self.leases.insert(
                offset,
                ActiveLease {
                    token,
                    expires_at_us,
                    attempt: *attempt,
                },
            );
            true
        } else {
            false
        }
    }

    /// Acknowledge an offset with fencing token validation (ADR-024).
    pub fn ack_fenced(&mut self, offset: u64, token: u64) -> Result<()> {
        if offset < self.base_watermark || self.acked.contains(offset) {
            return Ok(());
        }

        if let Some(lease) = self.leases.get(&offset) {
            if lease.token != token {
                return Err(KeiroxError::Internal(
                    "Stale lease token rejected during ACK".into(),
                ));
            }
            self.ack(offset);
            Ok(())
        } else {
            Err(KeiroxError::Internal(
                "Cannot ACK offset that is not currently leased".into(),
            ))
        }
    }

    /// Acknowledge an offset unconditionally (terminal state).
    pub fn ack(&mut self, offset: u64) {
        self.leases.remove(&offset);
        self.retry_counts.remove(&offset);
        self.acked.insert(offset);
        self.advance_watermark();
    }

    /// Negative-acknowledge an offset with explicit retry accounting.
    pub fn nack(&mut self, offset: u64) {
        self.leases.remove(&offset);
        let retries = self.retry_counts.get(&offset).copied().unwrap_or(0);
        if retries >= self.max_retries {
            self.evict_dlq(offset);
        }
    }

    /// Evict a poison-pill offset to Virtual DLQ (terminal state per ADR-004).
    pub fn evict_dlq(&mut self, offset: u64) {
        self.leases.remove(&offset);
        self.retry_counts.remove(&offset);
        self.evicted_dlq.insert(offset);
        self.advance_watermark();
    }

    /// Expire active leases exceeding the specified deadline timestamp.
    /// Automatically evicts to DLQ if max retries exceeded.
    pub fn expire_leases(&mut self, current_time_us: u64) -> Vec<u64> {
        let expired_offsets: Vec<u64> = self
            .leases
            .iter()
            .filter(|(_, lease)| lease.expires_at_us <= current_time_us)
            .map(|(&offset, _)| offset)
            .collect();

        for &offset in &expired_offsets {
            self.leases.remove(&offset);
            let retries = self.retry_counts.get(&offset).copied().unwrap_or(0);
            if retries >= self.max_retries {
                self.evict_dlq(offset);
            }
        }

        expired_offsets
    }

    /// Advance monotonic sliding base watermark ($W_{base}$) purging contiguous terminal bits.
    pub fn advance_watermark(&mut self) {
        while self.base_watermark <= self.head_offset {
            if self.acked.contains(self.base_watermark) {
                self.acked.remove(self.base_watermark);
                self.base_watermark += 1;
            } else if self.evicted_dlq.contains(self.base_watermark) {
                self.evicted_dlq.remove(self.base_watermark);
                self.base_watermark += 1;
            } else {
                break;
            }
        }
    }

    /// Verify core state plane invariants per `KEI-FORMAL-001`.
    pub fn verify_invariants(&self) -> Result<()> {
        // Invariant 1: Disjointness
        for &offset in self.leases.keys() {
            if self.acked.contains(offset) || self.evicted_dlq.contains(offset) {
                return Err(KeiroxError::Internal(
                    "State invariant violation: Leased offset is also Acked or EvictedDlq".into(),
                ));
            }
        }

        // Invariant 2: Watermark boundary cleanliness
        if let Some(min_ack) = self.acked.iter().next() {
            if min_ack < self.base_watermark {
                return Err(KeiroxError::Internal(
                    "State invariant violation: Acked bitmap contains bits below base watermark"
                        .into(),
                ));
            }
        }

        Ok(())
    }

    /// Read-only access to acked Roaring Bitmap.
    pub fn acked(&self) -> &RoaringTreemap {
        &self.acked
    }

    /// Read-only access to evicted DLQ Roaring Bitmap.
    pub fn evicted_dlq(&self) -> &RoaringTreemap {
        &self.evicted_dlq
    }

    /// Set internal bitmaps directly from deserialized snapshot.
    pub fn set_bitmaps(&mut self, acked: RoaringTreemap, evicted_dlq: RoaringTreemap) {
        self.acked = acked;
        self.evicted_dlq = evicted_dlq;
    }
}

impl StateOverlayEngine for ConsumerGroupState {
    fn grant_lease(&mut self, _stream_id: StreamId, offset: Offset, ttl_us: u64) -> Result<bool> {
        Ok(self.lease(offset, ttl_us).is_some())
    }

    fn acknowledge(&mut self, _stream_id: StreamId, offset: Offset) -> Result<()> {
        self.ack(offset);
        Ok(())
    }

    fn negative_acknowledge(&mut self, _stream_id: StreamId, offset: Offset) -> Result<()> {
        self.nack(offset);
        Ok(())
    }

    fn evict_to_dlq(&mut self, _stream_id: StreamId, offset: Offset) -> Result<()> {
        self.evict_dlq(offset);
        Ok(())
    }

    fn base_watermark(&self, _stream_id: StreamId) -> Offset {
        self.base_watermark
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn test_state_shard_key_layout() {
        assert_eq!(
            size_of::<StateShardKey>(),
            64,
            "StateShardKey must be 64 bytes per KEI-DES-031 §4.1"
        );
    }

    #[test]
    fn test_consumer_group_state_lifecycle() {
        let mut state = ConsumerGroupState::new();
        state.head_offset = 10;

        // Offset 0: Ready -> Leased -> Acked
        assert_eq!(state.get_state(0), ConsumerState::Ready);
        let token = state.lease(0, 1000).expect("Lease should succeed");
        assert!(matches!(state.get_state(0), ConsumerState::Leased { .. }));

        state.ack_fenced(0, token).expect("Fenced ack must succeed");
        assert_eq!(state.base_watermark, 1);
        assert_eq!(state.get_state(0), ConsumerState::Acked);
        state.verify_invariants().expect("Invariants must hold");
    }

    #[test]
    fn test_stale_token_rejection() {
        let mut state = ConsumerGroupState::new();
        state.head_offset = 1;

        let token1 = state.lease(0, 1000).unwrap();
        state.nack(0);

        let token2 = state.lease(0, 2000).unwrap();
        assert_ne!(token1, token2);

        // Stale token1 should fail
        assert!(state.ack_fenced(0, token1).is_err());
        // Valid token2 should succeed
        assert!(state.ack_fenced(0, token2).is_ok());
    }

    #[test]
    fn test_automatic_dlq_eviction_on_max_retries() {
        let mut state = ConsumerGroupState::with_max_retries(2);
        state.head_offset = 5;

        // Offset 0 remains in Ready
        // Offset 1: Attempt 1 -> Lease & Expire
        state.lease(1, 1000);
        state.expire_leases(1500);
        assert_eq!(state.get_state(1), ConsumerState::Ready);
        assert_eq!(state.base_watermark, 0);

        // Offset 1: Attempt 2 -> Lease & Expire -> Hits max_retries (2) -> Mandatory DLQ eviction!
        state.lease(1, 2000);
        state.expire_leases(2500);
        assert_eq!(state.get_state(1), ConsumerState::EvictedDlq);
        assert_eq!(state.base_watermark, 0);

        // Now ACK offset 0 -> Watermark unblocked and cascades past 0 and 1 to 2!
        state.ack(0);
        assert_eq!(state.base_watermark, 2);
        state.verify_invariants().expect("Invariants must hold");
    }

    #[test]
    fn test_64bit_large_offsets_without_truncation() {
        let mut state = ConsumerGroupState::new();
        // Offset larger than 2^32 (4.5 billion)
        let large_offset_1: u64 = 4_500_000_000;
        let large_offset_2: u64 = 4_500_000_001;
        state.head_offset = 5_000_000_000;

        assert_eq!(state.get_state(large_offset_1), ConsumerState::Ready);
        let token = state
            .lease(large_offset_1, 1000)
            .expect("Lease large offset");
        assert!(matches!(
            state.get_state(large_offset_1),
            ConsumerState::Leased { .. }
        ));

        state
            .ack_fenced(large_offset_1, token)
            .expect("Ack large offset");
        assert_eq!(state.get_state(large_offset_1), ConsumerState::Acked);

        // Verify that large_offset_2 is still Ready and not collided
        assert_eq!(state.get_state(large_offset_2), ConsumerState::Ready);

        // Evict to DLQ
        state.evict_dlq(large_offset_2);
        assert_eq!(state.get_state(large_offset_2), ConsumerState::EvictedDlq);

        state
            .verify_invariants()
            .expect("Invariants must hold for 64-bit offsets");
    }
}
