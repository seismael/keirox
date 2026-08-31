//! Error taxonomy for Keirox.

use thiserror::Error;

/// Result alias for Keirox operations.
pub type Result<T> = std::result::Result<T, KeiroxError>;

/// Root error enum for the Keirox runtime.
#[derive(Debug, Error)]
pub enum KeiroxError {
    /// Attempted to mutate or rewrite the append-only immutable log.
    #[error("Immutable log violation: {0}")]
    LogMutationViolation(String),

    /// Lease expired or invalidated by newer epoch.
    #[error("Lease error: {0}")]
    LeaseConflict(String),

    /// Stream not found or invalid.
    #[error("Stream not found: {0}")]
    StreamNotFound(String),

    /// Storage tier I/O failure.
    #[error("Storage I/O failure: {0}")]
    StorageIo(#[from] std::io::Error),

    /// Consensus or replication failure.
    #[error("Consensus error: {0}")]
    Consensus(String),

    /// Coordinator shard epoch is stale (fenced).
    #[error("Epoch fenced: {0}")]
    EpochFenced(String),

    /// Quorum could not be achieved.
    #[error("Quorum unavailable: {0}")]
    QuorumUnavailable(String),

    /// Tier-1 object storage error.
    #[error("Tier-1 storage error: {0}")]
    Tier1Storage(String),

    /// Operation rejected due to missing permissions or authorization failure.
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// Attempted to decrypt or access crypto-shredded data.
    #[error("Crypto-shredded key destroyed: {0}")]
    KeyDestroyed(String),

    /// Record schema is incompatible with target schema registry definition.
    #[error("Schema incompatible: {0}")]
    SchemaIncompatible(String),

    /// Audit trail hash chain tampering or integrity verification failure.
    #[error("Audit integrity violation: {0}")]
    AuditIntegrityViolation(String),

    /// Node is not the current leader for the consensus group.
    #[error("Not leader for consensus group")]
    NotLeader,

    /// Internal error.
    #[error("Internal runtime error: {0}")]
    Internal(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = KeiroxError::LogMutationViolation("mutation not allowed".into());
        assert_eq!(
            err.to_string(),
            "Immutable log violation: mutation not allowed"
        );

        let lease_err = KeiroxError::LeaseConflict("expired".into());
        assert_eq!(lease_err.to_string(), "Lease error: expired");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let keirox_err: KeiroxError = io_err.into();
        assert!(matches!(keirox_err, KeiroxError::StorageIo(_)));
    }
}
