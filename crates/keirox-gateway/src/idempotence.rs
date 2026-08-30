//! Kafka idempotent produce deduplication and sequence verification per `KEI-DES-035` §6.

use crate::protocol::KafkaErrorCode;
use std::collections::HashMap;
use std::sync::RwLock;

/// Outcome of preflight sequence number validation before physical produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreflightResult {
    /// Sequence is valid and ready for physical cluster produce.
    Proceed,
    /// Batch is an exact duplicate; return previously assigned base offset without re-appending.
    Duplicate(i64),
    /// Sequence gap or epoch mismatch; reject before cluster produce.
    Error(KafkaErrorCode),
}

/// Producer state tracking entry.
#[derive(Debug, Clone, Copy)]
pub struct ProducerSequenceState {
    /// Producer epoch.
    pub epoch: i16,
    /// Highest contiguous sequence number observed.
    pub last_sequence: i32,
    /// Base offset returned for last sequence.
    pub last_offset: i64,
}

/// In-memory tracker for Kafka idempotent producers across topics and partitions.
#[derive(Debug, Default)]
pub struct ProducerIdempotenceTracker {
    states: RwLock<HashMap<(i64, String, i32), ProducerSequenceState>>,
}

impl ProducerIdempotenceTracker {
    /// Create a new idempotence tracker.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Pre-flight validation before executing cluster produce.
    pub fn check_preflight(
        &self,
        producer_id: i64,
        epoch: i16,
        base_sequence: i32,
        topic: &str,
        partition: i32,
    ) -> PreflightResult {
        if producer_id == -1 {
            return PreflightResult::Proceed;
        }

        let key = (producer_id, topic.to_string(), partition);
        let states = self.states.read().unwrap();

        if let Some(state) = states.get(&key) {
            if epoch < state.epoch {
                return PreflightResult::Error(KafkaErrorCode::LeaderNotAvailable);
            }

            if epoch == state.epoch {
                if base_sequence <= state.last_sequence {
                    return PreflightResult::Duplicate(state.last_offset);
                }

                if base_sequence > state.last_sequence + 1 {
                    return PreflightResult::Error(KafkaErrorCode::OutOfOrderSequenceNumber);
                }
            }
        }

        PreflightResult::Proceed
    }

    /// Verify incoming sequence number and update tracker state if valid.
    #[allow(clippy::too_many_arguments)]
    pub fn verify_and_update(
        &self,
        producer_id: i64,
        epoch: i16,
        base_sequence: i32,
        record_count: i32,
        topic: &str,
        partition: i32,
        assigned_offset: i64,
    ) -> Result<i64, KafkaErrorCode> {
        if producer_id == -1 {
            // Non-idempotent producer: skip tracking
            return Ok(assigned_offset);
        }

        let key = (producer_id, topic.to_string(), partition);
        let mut states = self.states.write().unwrap();

        if let Some(state) = states.get_mut(&key) {
            if epoch < state.epoch {
                return Err(KafkaErrorCode::LeaderNotAvailable);
            }

            if epoch == state.epoch {
                if base_sequence <= state.last_sequence {
                    // Duplicate batch re-send: return previously assigned offset
                    return Ok(state.last_offset);
                }

                if base_sequence > state.last_sequence + 1 {
                    // Gap in sequences: out-of-order error
                    return Err(KafkaErrorCode::OutOfOrderSequenceNumber);
                }
            }

            // Valid next sequence: advance state
            state.epoch = epoch;
            state.last_sequence = base_sequence + record_count - 1;
            state.last_offset = assigned_offset;
            Ok(assigned_offset)
        } else {
            // First time seeing this producer on this partition
            let last_seq = base_sequence + record_count - 1;
            states.insert(
                key,
                ProducerSequenceState {
                    epoch,
                    last_sequence: last_seq,
                    last_offset: assigned_offset,
                },
            );
            Ok(assigned_offset)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idempotence_sequence_validation_and_deduplication() {
        let tracker = ProducerIdempotenceTracker::new();

        assert_eq!(
            tracker.check_preflight(1001, 0, 0, "orders", 0),
            PreflightResult::Proceed
        );

        // 1. Initial sequence 0
        let off1 = tracker
            .verify_and_update(1001, 0, 0, 5, "orders", 0, 100)
            .unwrap();
        assert_eq!(off1, 100);

        // 2. Duplicate sequence 0 -> preflight catches duplicate
        assert_eq!(
            tracker.check_preflight(1001, 0, 0, "orders", 0),
            PreflightResult::Duplicate(100)
        );

        // 3. Out-of-order sequence (expected 5, got 10)
        assert_eq!(
            tracker.check_preflight(1001, 0, 10, "orders", 0),
            PreflightResult::Error(KafkaErrorCode::OutOfOrderSequenceNumber)
        );

        // 4. Valid contiguous next sequence 5
        assert_eq!(
            tracker.check_preflight(1001, 0, 5, "orders", 0),
            PreflightResult::Proceed
        );
        let off2 = tracker
            .verify_and_update(1001, 0, 5, 5, "orders", 0, 105)
            .unwrap();
        assert_eq!(off2, 105);
    }
}
