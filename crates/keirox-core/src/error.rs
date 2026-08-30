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

    /// Internal error.
    #[error("Internal runtime error: {0}")]
    Internal(String),
}
