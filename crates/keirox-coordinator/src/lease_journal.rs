//! Lease delta journaling and state machine reconciliation per `KEI-ARC-021` and `KEI-ARC-022`.

use keirox_consensus::LeaseDeltaRecord;
use keirox_core::error::{KeiroxError, Result};
use keirox_state::ConsumerGroupState;
use serde::{Deserialize, Serialize};

/// In-memory lease delta journal capturing changes between periodic snapshots.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct LeaseJournal {
    deltas: Vec<LeaseDeltaRecord>,
}

impl LeaseJournal {
    /// Create a new empty lease journal.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a new delta in the journal.
    pub fn record(&mut self, delta: LeaseDeltaRecord) {
        self.deltas.push(delta);
    }

    /// Total deltas recorded.
    #[must_use]
    pub fn len(&self) -> usize {
        self.deltas.len()
    }

    /// True if journal contains zero deltas.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.deltas.is_empty()
    }

    /// Drain all recorded deltas (e.g. after snapshot compaction).
    pub fn drain(&mut self) -> Vec<LeaseDeltaRecord> {
        std::mem::take(&mut self.deltas)
    }

    /// All recorded deltas view.
    #[must_use]
    pub fn deltas(&self) -> &[LeaseDeltaRecord] {
        &self.deltas
    }

    /// Replay all deltas in this journal against a `ConsumerGroupState`.
    pub fn replay_against(&self, state: &mut ConsumerGroupState) -> Result<()> {
        for delta in &self.deltas {
            Self::apply_delta(state, delta)?;
        }
        Ok(())
    }

    /// Apply a single delta record to a `ConsumerGroupState`.
    pub fn apply_delta(state: &mut ConsumerGroupState, delta: &LeaseDeltaRecord) -> Result<()> {
        match delta {
            LeaseDeltaRecord::Acquire {
                offset,
                deadline_us,
                token,
            } => {
                if !state.lease_with_token(*offset, *deadline_us, *token) {
                    return Err(KeiroxError::LeaseConflict(format!(
                        "Cannot apply Acquire delta for offset {offset}: current state is {:?}",
                        state.get_state(*offset)
                    )));
                }
            }
            LeaseDeltaRecord::Ack { offset, token } => {
                state.ack_fenced(*offset, *token)?;
            }
            LeaseDeltaRecord::Nack { offset } => {
                state.nack(*offset);
            }
            LeaseDeltaRecord::EvictDlq { offset } => {
                state.evict_dlq(*offset);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keirox_state::ConsumerState;

    #[test]
    fn test_lease_journal_record_and_replay() {
        let mut journal = LeaseJournal::new();
        journal.record(LeaseDeltaRecord::Acquire {
            offset: 0,
            token: 1,
            deadline_us: 5_000_000,
        });
        journal.record(LeaseDeltaRecord::Ack {
            offset: 0,
            token: 1,
        });
        journal.record(LeaseDeltaRecord::Acquire {
            offset: 100,
            token: 2,
            deadline_us: 5_000_000,
        });
        journal.record(LeaseDeltaRecord::Ack {
            offset: 100,
            token: 2,
        });

        let mut state = ConsumerGroupState::new();
        journal.replay_against(&mut state).unwrap();

        assert_eq!(state.get_state(0), ConsumerState::Acked);
        assert_eq!(state.get_state(100), ConsumerState::Acked);
        assert_eq!(state.base_watermark, 1);
    }
}
