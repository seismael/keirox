//! Hierarchical timing wheel structure per `KEI-DES-031`.

use std::collections::BTreeMap;

/// Core timing wheel scheduler for managing lease timeouts.
#[derive(Debug, Default)]
pub struct TimingWheel {
    current_tick_us: u64,
    /// Scheduled timeouts mapped by deadline (timestamp_us -> list of offset/tokens).
    buckets: BTreeMap<u64, Vec<u64>>,
}

impl TimingWheel {
    /// Create a new timing wheel instance starting at the given timestamp.
    pub fn new(start_us: u64) -> Self {
        Self {
            current_tick_us: start_us,
            buckets: BTreeMap::new(),
        }
    }

    /// Schedule an offset lease timeout at a specific expiration timestamp.
    pub fn schedule_timeout(&mut self, offset: u64, expires_at_us: u64) {
        self.buckets.entry(expires_at_us).or_default().push(offset);
    }

    /// Advance the wheel and return all offsets that have expired up to `now_us`.
    pub fn advance_to(&mut self, now_us: u64) -> Vec<u64> {
        self.current_tick_us = now_us;
        let mut expired = Vec::new();

        let split_key = now_us.saturating_add(1);
        let mut remaining = self.buckets.split_off(&split_key);
        std::mem::swap(&mut self.buckets, &mut remaining);

        for (_, offsets) in remaining {
            expired.extend(offsets);
        }

        expired
    }

    /// Return current wheel timestamp.
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
        expired.sort();
        assert_eq!(expired, vec![101, 103]);

        // Advance to 1100 -> 102 expired
        let expired = wheel.advance_to(1100);
        assert_eq!(expired, vec![102]);
    }
}
