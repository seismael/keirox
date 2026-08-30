//! Wire protocol definitions.

/// Client-selectable acknowledgment durability modes (ADR-020).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AckMode {
    /// Acknowledged once recorded in memory arena and replicated to coordinator.
    Fast,
    /// Acknowledged only after physical NVMe flush on Raft quorum ($JML=0$).
    Durable,
}
