//! Split-brain epoch fencing and lease token validation per `KEI-ARC-021`, `KEI-ARC-022`, and `ADR-024`.

use crate::shard::{CoordinatorEpoch, ShardId};
use keirox_core::error::{KeiroxError, Result};
use serde::{Deserialize, Serialize};

/// High-assurance epoch-fenced consumer lease token.
///
/// Encodes shard, epoch, offset, and random nonce to fence split-brain coordinators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EpochFencedToken {
    /// Shard identifier hosting this consumer group.
    pub shard_id: ShardId,
    /// Coordinator epoch when lease was issued.
    pub epoch: CoordinatorEpoch,
    /// Stream offset leased.
    pub offset: u64,
    /// Random 32-bit generation nonce.
    pub nonce: u32,
}

impl EpochFencedToken {
    /// Create a new fenced lease token.
    #[must_use]
    pub const fn new(shard_id: ShardId, epoch: CoordinatorEpoch, offset: u64, nonce: u32) -> Self {
        Self {
            shard_id,
            epoch,
            offset,
            nonce,
        }
    }

    /// Pack token into a 64-bit wire token representation.
    ///
    /// Bit Layout:
    /// - [63..48] (16 bits): ShardId (mod 65536)
    /// - [47..32] (16 bits): CoordinatorEpoch (mod 65536)
    /// - [31..0]  (32 bits): Nonce
    #[must_use]
    pub const fn to_u64(&self) -> u64 {
        let shard_bits = (self.shard_id.0 as u64 & 0xFFFF) << 48;
        let epoch_bits = (self.epoch.0 & 0xFFFF) << 32;
        let nonce_bits = self.nonce as u64 & 0xFFFF_FFFF;
        shard_bits | epoch_bits | nonce_bits
    }

    /// Unpack from a 64-bit wire token.
    #[must_use]
    pub const fn from_u64(raw: u64, offset: u64) -> Self {
        let shard_id = ShardId(((raw >> 48) & 0xFFFF) as u32);
        let epoch = CoordinatorEpoch((raw >> 32) & 0xFFFF);
        let nonce = (raw & 0xFFFF_FFFF) as u32;
        Self {
            shard_id,
            epoch,
            offset,
            nonce,
        }
    }

    /// Full 24-byte lossless binary token representation.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; 24] {
        let mut buf = [0u8; 24];
        buf[0..4].copy_from_slice(&self.shard_id.0.to_le_bytes());
        buf[4..12].copy_from_slice(&self.epoch.0.to_le_bytes());
        buf[12..20].copy_from_slice(&self.offset.to_le_bytes());
        buf[20..24].copy_from_slice(&self.nonce.to_le_bytes());
        buf
    }

    /// Unpack from a 24-byte binary token.
    #[must_use]
    pub fn from_bytes(bytes: &[u8; 24]) -> Self {
        let shard_id = ShardId(u32::from_le_bytes(
            bytes[0..4].try_into().unwrap_or_default(),
        ));
        let epoch = CoordinatorEpoch(u64::from_le_bytes(
            bytes[4..12].try_into().unwrap_or_default(),
        ));
        let offset = u64::from_le_bytes(bytes[12..20].try_into().unwrap_or_default());
        let nonce = u32::from_le_bytes(bytes[20..24].try_into().unwrap_or_default());
        Self {
            shard_id,
            epoch,
            offset,
            nonce,
        }
    }

    /// Validate that this token is valid for the current active coordinator epoch.
    ///
    /// Per ADR-024: Stale coordinator operations MUST be immediately rejected to prevent double-leases.
    pub fn validate(&self, expected_shard: ShardId, current_epoch: CoordinatorEpoch) -> Result<()> {
        if self.shard_id != expected_shard {
            return Err(KeiroxError::LeaseConflict(format!(
                "Lease shard mismatch: token shard {} vs active {}",
                self.shard_id, expected_shard
            )));
        }

        if self.epoch.0 < current_epoch.0 {
            return Err(KeiroxError::EpochFenced(format!(
                "Stale coordinator epoch {} fenced by active epoch {} (reassigned during failover)",
                self.epoch, current_epoch
            )));
        }

        if self.epoch.0 > current_epoch.0 {
            return Err(KeiroxError::EpochFenced(format!(
                "Future coordinator epoch {} ahead of active epoch {}",
                self.epoch, current_epoch
            )));
        }

        Ok(())
    }

    /// Validate both coordinator epoch and target stream offset binding.
    pub fn validate_for_offset(
        &self,
        expected_shard: ShardId,
        current_epoch: CoordinatorEpoch,
        expected_offset: u64,
    ) -> Result<()> {
        self.validate(expected_shard, current_epoch)?;
        if self.offset != expected_offset {
            return Err(KeiroxError::LeaseConflict(format!(
                "Token bound offset {} does not match target offset {}",
                self.offset, expected_offset
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_epoch_fencing_token_packing_and_validation() {
        let token = EpochFencedToken::new(ShardId(42), CoordinatorEpoch(7), 1001, 0xCAFE_BABE);
        let raw = token.to_u64();
        let decoded = EpochFencedToken::from_u64(raw, 1001);

        assert_eq!(decoded.shard_id, ShardId(42));
        assert_eq!(decoded.epoch, CoordinatorEpoch(7));
        assert_eq!(decoded.offset, 1001);
        assert_eq!(decoded.nonce, 0xCAFE_BABE);

        // Valid under active epoch 7
        assert!(decoded.validate(ShardId(42), CoordinatorEpoch(7)).is_ok());

        // Fenced under active epoch 8 (failover happened)
        let err = decoded
            .validate(ShardId(42), CoordinatorEpoch(8))
            .unwrap_err();
        assert!(matches!(err, KeiroxError::EpochFenced(_)));
        assert!(err.to_string().contains("Stale coordinator epoch"));
    }

    #[test]
    fn test_epoch_fencing_token_lossless_bytes() {
        let token = EpochFencedToken::new(
            ShardId(1023),
            CoordinatorEpoch(0x1234_5678_9ABC_DEF0),
            0xDEAD_BEEF_0000_1234,
            0xFEED_FACE,
        );
        let bytes = token.to_bytes();
        let decoded = EpochFencedToken::from_bytes(&bytes);

        assert_eq!(decoded.shard_id, ShardId(1023));
        assert_eq!(decoded.epoch, CoordinatorEpoch(0x1234_5678_9ABC_DEF0));
        assert_eq!(decoded.offset, 0xDEAD_BEEF_0000_1234);
        assert_eq!(decoded.nonce, 0xFEED_FACE);
        assert!(decoded
            .validate_for_offset(
                ShardId(1023),
                CoordinatorEpoch(0x1234_5678_9ABC_DEF0),
                0xDEAD_BEEF_0000_1234
            )
            .is_ok());
    }
}
