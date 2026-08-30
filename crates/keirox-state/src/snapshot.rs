//! Binary state snapshotting and serialization for consumer group states per `KEI-DES-031` §12.

use crate::state_machine::ConsumerGroupState;
use crc32fast::Hasher;
use keirox_core::error::{KeiroxError, Result};
use roaring::RoaringTreemap;
use serde::{Deserialize, Serialize};

/// Magic identifier for consumption state snapshots ('KSNP' = 0x4B534E50).
pub const SNAPSHOT_MAGIC: u32 = 0x4B534E50;

/// Serializable snapshot representation of a consumer group state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateSnapshotPayload {
    /// Monotonic base watermark.
    pub base_watermark: u64,
    /// Head offset.
    pub head_offset: u64,
    /// Serialized acked Roaring Bitmap bytes.
    pub acked_bytes: Vec<u8>,
    /// Serialized evicted DLQ Roaring Bitmap bytes.
    pub evicted_dlq_bytes: Vec<u8>,
}

/// Binary state snapshot header with hardware CRC32C integrity.
pub struct StateSnapshot;

impl StateSnapshot {
    /// Create a binary snapshot from a live `ConsumerGroupState`.
    pub fn create_bytes(state: &ConsumerGroupState) -> Result<Vec<u8>> {
        let mut acked_bytes = Vec::new();
        state
            .acked()
            .serialize_into(&mut acked_bytes)
            .map_err(|e| KeiroxError::Internal(format!("Failed to serialize acked bitmap: {e}")))?;

        let mut evicted_dlq_bytes = Vec::new();
        state
            .evicted_dlq()
            .serialize_into(&mut evicted_dlq_bytes)
            .map_err(|e| KeiroxError::Internal(format!("Failed to serialize DLQ bitmap: {e}")))?;

        let payload = StateSnapshotPayload {
            base_watermark: state.base_watermark,
            head_offset: state.head_offset,
            acked_bytes,
            evicted_dlq_bytes,
        };

        let payload_serialized = serde_json::to_vec(&payload)
            .map_err(|e| KeiroxError::Internal(format!("Failed to encode snapshot: {e}")))?;

        let mut hasher = Hasher::new();
        hasher.update(&SNAPSHOT_MAGIC.to_le_bytes());
        hasher.update(&payload_serialized);
        let crc = hasher.finalize();

        let mut out = Vec::with_capacity(8 + payload_serialized.len());
        out.extend_from_slice(&SNAPSHOT_MAGIC.to_le_bytes());
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&payload_serialized);

        Ok(out)
    }

    /// Restore a `ConsumerGroupState` from binary snapshot bytes.
    pub fn restore_from_bytes(bytes: &[u8]) -> Result<ConsumerGroupState> {
        if bytes.len() < 8 {
            return Err(KeiroxError::Internal("Snapshot bytes too short".into()));
        }

        let magic = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        if magic != SNAPSHOT_MAGIC {
            return Err(KeiroxError::Internal(
                "Invalid snapshot magic identifier".into(),
            ));
        }

        let expected_crc = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let payload_bytes = &bytes[8..];

        let mut hasher = Hasher::new();
        hasher.update(&magic.to_le_bytes());
        hasher.update(payload_bytes);
        let actual_crc = hasher.finalize();

        if actual_crc != expected_crc {
            return Err(KeiroxError::Internal(
                "Snapshot failed CRC32C verification".into(),
            ));
        }

        let payload: StateSnapshotPayload = serde_json::from_slice(payload_bytes).map_err(|e| {
            KeiroxError::Internal(format!("Failed to decode snapshot payload: {e}"))
        })?;

        let acked = RoaringTreemap::deserialize_from(&payload.acked_bytes[..])
            .map_err(|e| KeiroxError::Internal(format!("Failed to restore acked bitmap: {e}")))?;

        let evicted_dlq = RoaringTreemap::deserialize_from(&payload.evicted_dlq_bytes[..])
            .map_err(|e| KeiroxError::Internal(format!("Failed to restore DLQ bitmap: {e}")))?;

        let mut state = ConsumerGroupState::new();
        state.base_watermark = payload.base_watermark;
        state.head_offset = payload.head_offset;
        state.set_bitmaps(acked, evicted_dlq);

        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_snapshot_serialization_and_restoration() {
        let mut state = ConsumerGroupState::new();
        state.head_offset = 20;
        state.base_watermark = 5;

        // Lease & ACK out-of-order
        state.lease(10, 1000);
        state.ack(10);
        state.evict_dlq(15);

        // Snapshot
        let snap_bytes = StateSnapshot::create_bytes(&state).unwrap();
        assert!(!snap_bytes.is_empty());

        // Restore
        let restored = StateSnapshot::restore_from_bytes(&snap_bytes).unwrap();
        assert_eq!(restored.base_watermark, 5);
        assert_eq!(restored.head_offset, 20);
        assert_eq!(
            restored.get_state(10),
            crate::state_machine::ConsumerState::Acked
        );
        assert_eq!(
            restored.get_state(15),
            crate::state_machine::ConsumerState::EvictedDlq
        );
        assert_eq!(
            restored.get_state(12),
            crate::state_machine::ConsumerState::Ready
        );
    }
}
