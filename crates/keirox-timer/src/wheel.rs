//! Hierarchical timing wheel structure.

/// Core timing wheel scheduler for managing lease timeouts.
#[derive(Debug, Default)]
pub struct TimingWheel {
    current_tick_us: u64,
}

impl TimingWheel {
    /// Create a new timing wheel instance starting at the given timestamp.
    pub fn new(start_us: u64) -> Self {
        Self {
            current_tick_us: start_us,
        }
    }

    /// Advance the wheel by delta microseconds.
    pub fn advance_to(&mut self, now_us: u64) {
        self.current_tick_us = now_us;
    }
}
