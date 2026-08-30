//! Consumer group state definitions and Roaring Bitmap state overlay per `KEI-ARC-021` and `KEI-DES-031`.

use roaring::RoaringBitmap;
use std::collections::HashMap;

/// Disjoint states for an offset within a consumer group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsumerState {
    /// Available for delivery.
    Ready,
    /// Currently leased to a consumer instance with expiration timestamp τ.
    Leased {
        /// Lease deadline timestamp in microseconds.
        expires_at_us: u64,
    },
    /// Acknowledged as successfully processed.
    Acked,
    /// Evicted to Virtual Dead-Letter Queue after exceeding max retries.
    EvictedDlq,
}

/// In-memory consumption state overlay for a consumer group shard.
#[derive(Debug, Default)]
pub struct ConsumerGroupState {
    /// Monotonic sliding base watermark ($W_{base}$).
    pub base_watermark: u64,
    /// Head offset written to physical WAL.
    pub head_offset: u64,
    /// Roaring Bitmap of acknowledged offsets.
    acked: RoaringBitmap,
    /// Roaring Bitmap of evicted DLQ offsets.
    evicted_dlq: RoaringBitmap,
    /// Map of currently active leases (offset -> expiry timestamp).
    leases: HashMap<u64, u64>,
}

impl ConsumerGroupState {
    /// Create a new state overlay starting at base watermark 0.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the current state for a given offset.
    pub fn get_state(&self, offset: u64) -> ConsumerState {
        if offset < self.base_watermark {
            return ConsumerState::Acked;
        }

        let u32_offset = offset as u32;
        if self.acked.contains(u32_offset) {
            ConsumerState::Acked
        } else if self.evicted_dlq.contains(u32_offset) {
            ConsumerState::EvictedDlq
        } else if let Some(&expires_at_us) = self.leases.get(&offset) {
            ConsumerState::Leased { expires_at_us }
        } else {
            ConsumerState::Ready
        }
    }

    /// Grant a lease on an offset.
    pub fn lease(&mut self, offset: u64, expires_at_us: u64) -> bool {
        if self.get_state(offset) == ConsumerState::Ready {
            self.leases.insert(offset, expires_at_us);
            true
        } else {
            false
        }
    }

    /// Acknowledge an offset (terminal state).
    pub fn ack(&mut self, offset: u64) {
        self.leases.remove(&offset);
        self.acked.insert(offset as u32);
        self.advance_watermark();
    }

    /// Negative-acknowledge an offset (re-queue to Ready).
    pub fn nack(&mut self, offset: u64) {
        self.leases.remove(&offset);
    }

    /// Evict a poison-pill offset to DLQ (terminal state).
    pub fn evict_dlq(&mut self, offset: u64) {
        self.leases.remove(&offset);
        self.evicted_dlq.insert(offset as u32);
        self.advance_watermark();
    }

    /// Advance monotonic sliding base watermark ($W_{base}$) purging contiguous terminal bits.
    pub fn advance_watermark(&mut self) {
        while self.base_watermark <= self.head_offset {
            let u32_offset = self.base_watermark as u32;
            if self.acked.contains(u32_offset) {
                self.acked.remove(u32_offset);
                self.base_watermark += 1;
            } else if self.evicted_dlq.contains(u32_offset) {
                self.evicted_dlq.remove(u32_offset);
                self.base_watermark += 1;
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consumer_group_state_lifecycle() {
        let mut state = ConsumerGroupState::new();
        state.head_offset = 10;

        // Offset 0: Ready -> Leased -> Acked
        assert_eq!(state.get_state(0), ConsumerState::Ready);
        assert!(state.lease(0, 1000));
        assert!(matches!(state.get_state(0), ConsumerState::Leased { .. }));

        state.ack(0);
        assert_eq!(state.base_watermark, 1);
        assert_eq!(state.get_state(0), ConsumerState::Acked);
    }

    #[test]
    fn test_out_of_order_ack_and_watermark_advance() {
        let mut state = ConsumerGroupState::new();
        state.head_offset = 5;

        // Lease 0, 1, 2
        state.lease(0, 1000);
        state.lease(1, 1000);
        state.lease(2, 1000);

        // Out-of-order ACK offset 2 -> Watermark remains 0
        state.ack(2);
        assert_eq!(state.base_watermark, 0);
        assert_eq!(state.get_state(2), ConsumerState::Acked);

        // ACK offset 0 -> Watermark advances to 1
        state.ack(0);
        assert_eq!(state.base_watermark, 1);

        // ACK offset 1 -> Watermark advances past 1 and 2 to 3!
        state.ack(1);
        assert_eq!(state.base_watermark, 3);
    }

    #[test]
    fn test_dlq_eviction_unblocks_watermark() {
        let mut state = ConsumerGroupState::new();
        state.head_offset = 3;

        state.lease(0, 1000);
        state.lease(1, 1000);

        // Offset 1 acked out-of-order
        state.ack(1);
        assert_eq!(state.base_watermark, 0);

        // Offset 0 fails and gets evicted to DLQ -> Watermark unblocked and advances to 2!
        state.evict_dlq(0);
        assert_eq!(state.base_watermark, 2);
    }
}
