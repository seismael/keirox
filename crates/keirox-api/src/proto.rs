//! Wire protocol definitions and ACK modes per `KEI-DES-032` (ADR-020).

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ack_mode_display_and_values() {
        assert_eq!(AckMode::Fast.to_string(), "ACK_FAST");
        assert_eq!(AckMode::Durable.to_string(), "ACK_DURABLE");
        assert_ne!(AckMode::Fast, AckMode::Durable);
    }
}
