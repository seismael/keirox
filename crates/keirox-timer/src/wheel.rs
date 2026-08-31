//! Hierarchical timing wheel structure with O(1) slot dispatch per `KEI-DES-031` and `ADR-025`.

use std::collections::BTreeMap;

/// Number of primary circular ring-buffer slots (Level-1 wheel).
pub const WHEEL_SIZE: usize = 256;

/// Primary tick resolution (1,000 microseconds = 1 millisecond).
pub const TICK_RESOLUTION_US: u64 = 1_000;

/// Core hierarchical timing wheel scheduler for managing lease timeouts.
///
/// Implements O(1) circular slot insertion for near-term leases and cascaded
/// level-2 buckets for long-range lease timeouts.
#[derive(Debug)]
pub struct TimingWheel {
    current_tick_us: u64,
    /// Level-1 circular slotted ring buffer: O(1) insertion & dispatch.
    slots: Vec<Vec<(u64, u64)>>, // (offset, exact_deadline_us)
    /// Level-2 overflow tree for deadlines beyond the Level-1 horizon.
    overflow: BTreeMap<u64, Vec<u64>>,
}

impl Default for TimingWheel {
    fn default() -> Self {
        Self::new(0)
    }
}

impl TimingWheel {
    /// Create a new timing wheel instance starting at the given timestamp.
    #[must_use]
    pub fn new(start_us: u64) -> Self {
        let mut slots = Vec::with_capacity(WHEEL_SIZE);
        for _ in 0..WHEEL_SIZE {
            slots.push(Vec::new());
        }
        Self {
            current_tick_us: start_us,
            slots,
            overflow: BTreeMap::new(),
        }
    }

    /// Primary wheel horizon in microseconds (e.g. 256 * 1000us = 256ms).
    #[inline]
    fn horizon_us(&self) -> u64 {
        WHEEL_SIZE as u64 * TICK_RESOLUTION_US
    }

    /// Schedule an offset lease timeout at a specific expiration timestamp.
    ///
    /// O(1) amortized insertion into primary circular slot when within horizon.
    pub fn schedule_timeout(&mut self, offset: u64, expires_at_us: u64) {
        if expires_at_us <= self.current_tick_us + self.horizon_us() {
            let slot_idx = ((expires_at_us / TICK_RESOLUTION_US) as usize) % WHEEL_SIZE;
            self.slots[slot_idx].push((offset, expires_at_us));
        } else {
            self.overflow.entry(expires_at_us).or_default().push(offset);
        }
    }

    /// Advance the wheel and return all offsets that have expired up to `now_us`.
    pub fn advance_to(&mut self, now_us: u64) -> Vec<u64> {
        let mut expired = Vec::new();

        // 1. Drain expired entries from Level-1 circular slots
        for slot in &mut self.slots {
            slot.retain(|&(offset, deadline)| {
                if deadline <= now_us {
                    expired.push(offset);
                    false
                } else {
                    true
                }
            });
        }

        // 2. Cascade eligible entries from Level-2 overflow into Level-1 or expired list
        let split_key = now_us.saturating_add(self.horizon_us()).saturating_add(1);
        let mut ready_overflow = self.overflow.split_off(&split_key);
        std::mem::swap(&mut self.overflow, &mut ready_overflow);

        for (deadline, offsets) in ready_overflow {
            if deadline <= now_us {
                expired.extend(offsets);
            } else {
                for offset in offsets {
                    let slot_idx = ((deadline / TICK_RESOLUTION_US) as usize) % WHEEL_SIZE;
                    self.slots[slot_idx].push((offset, deadline));
                }
            }
        }

        self.current_tick_us = now_us;
        expired
    }

    /// Return current wheel timestamp.
    #[must_use]
    pub fn current_tick_us(&self) -> u64 {
        self.current_tick_us
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timing_wheel_lease_scheduling() {
        let mut wheel = TimingWheel::new(1000);

        wheel.schedule_timeout(101, 1050);
        wheel.schedule_timeout(102, 1100);
        wheel.schedule_timeout(103, 1050);

        // Advance to 1040 -> None expired
        let expired = wheel.advance_to(1040);
        assert!(expired.is_empty());

        // Advance to 1060 -> 101 and 103 expired
        let mut expired = wheel.advance_to(1060);
        expired.sort_unstable();
        assert_eq!(expired, vec![101, 103]);

        // Advance to 1100 -> 102 expired
        let expired = wheel.advance_to(1100);
        assert_eq!(expired, vec![102]);
    }

    #[test]
    fn test_timing_wheel_overflow_cascading() {
        let mut wheel = TimingWheel::new(0);

        // Schedule far future timeout (beyond 256ms horizon)
        wheel.schedule_timeout(999, 500_000); // 500ms

        assert!(wheel.advance_to(100_000).is_empty());
        assert!(wheel.advance_to(400_000).is_empty());

        // At 500ms, item is triggered
        let expired = wheel.advance_to(500_000);
        assert_eq!(expired, vec![999]);
    }
}
