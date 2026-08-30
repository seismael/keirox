//! Hybrid Logical Clock (HLC) for causal ordering across WAN regions per `KEI-ARC-026` and `KEI-MR-401`.

use serde::{Deserialize, Serialize};
use std::cmp::max;
use std::fmt;
use std::sync::RwLock;

/// Monotonically increasing Hybrid Logical Clock (HLC) timestamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct HlcTimestamp {
    /// Physical time component in milliseconds.
    pub physical_ms: u64,
    /// Logical sequence counter incremented on events occurring within the same physical millisecond.
    pub logical: u32,
    /// Originating node identifier to break ties.
    pub node_id: u16,
}

impl fmt::Display for HlcTimestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HLC({}:{}:{})",
            self.physical_ms, self.logical, self.node_id
        )
    }
}

/// Thread-safe Hybrid Logical Clock generator.
pub struct HybridLogicalClock {
    node_id: u16,
    state: RwLock<(u64, u32)>, // (latest_physical_ms, logical_counter)
}

impl HybridLogicalClock {
    /// Initialize a new HLC for a specific node.
    #[must_use]
    pub fn new(node_id: u16) -> Self {
        Self {
            node_id,
            state: RwLock::new((0, 0)),
        }
    }

    /// Generate a new causal timestamp for a local event.
    pub fn now(&self, physical_now_ms: u64) -> HlcTimestamp {
        let mut state = self.state.write().expect("HLC lock poisoned");
        let (last_phys, last_logical) = *state;

        if physical_now_ms > last_phys {
            *state = (physical_now_ms, 0);
            HlcTimestamp {
                physical_ms: physical_now_ms,
                logical: 0,
                node_id: self.node_id,
            }
        } else {
            let next_logical = last_logical + 1;
            *state = (last_phys, next_logical);
            HlcTimestamp {
                physical_ms: last_phys,
                logical: next_logical,
                node_id: self.node_id,
            }
        }
    }

    /// Update local clock upon receiving a causal message from a remote region.
    pub fn update(&self, remote: HlcTimestamp, physical_now_ms: u64) -> HlcTimestamp {
        let mut state = self.state.write().expect("HLC lock poisoned");
        let (last_phys, last_logical) = *state;

        let next_phys = max(max(last_phys, remote.physical_ms), physical_now_ms);
        let next_logical = if next_phys == last_phys && next_phys == remote.physical_ms {
            max(last_logical, remote.logical) + 1
        } else if next_phys == last_phys {
            last_logical + 1
        } else if next_phys == remote.physical_ms {
            remote.logical + 1
        } else {
            0
        };

        *state = (next_phys, next_logical);
        HlcTimestamp {
            physical_ms: next_phys,
            logical: next_logical,
            node_id: self.node_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hlc_monotonicity_and_remote_update() {
        let clock_a = HybridLogicalClock::new(1);
        let clock_b = HybridLogicalClock::new(2);

        let t1 = clock_a.now(1000);
        let t2 = clock_a.now(1000);
        assert!(t2 > t1);
        assert_eq!(t2.physical_ms, 1000);
        assert_eq!(t2.logical, 1);

        // Receive t2 at node B when node B's physical clock is slightly behind (950)
        let t3 = clock_b.update(t2, 950);
        assert!(t3 > t2);
        assert_eq!(t3.physical_ms, 1000);
        assert_eq!(t3.logical, 2);
    }
}
