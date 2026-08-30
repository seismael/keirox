//! Consumer group state definitions.

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
