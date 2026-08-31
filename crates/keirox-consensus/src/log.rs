//! Replicated log data structures, payload types, and compaction mechanisms per `KEI-ARC-022`.

use crate::types::{LogIndex, Term};
use serde::{Deserialize, Serialize};

/// Lease delta state change record replicated via Metadata Raft.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LeaseDeltaRecord {
    /// Consumer acquired lease on offset.
    Acquire {
        /// Offset leased.
        offset: u64,
        /// Lease fencing token.
        token: u64,
        /// Microsecond deadline timestamp.
        deadline_us: u64,
    },
    /// Consumer successfully ACKed offset.
    Ack {
        /// Offset acknowledged.
        offset: u64,
        /// Fencing token.
        token: u64,
    },
    /// Consumer NACKed offset (requeue).
    Nack {
        /// Offset negative acknowledged.
        offset: u64,
    },
    /// Max retries exceeded; offset evicted to DLQ.
    EvictDlq {
        /// Offset evicted.
        offset: u64,
    },
}

/// Metadata command payload replicated via Metadata Raft group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetadataCommand {
    /// Assign coordinator shard to a cluster node with monotonic epoch.
    AssignShard {
        /// Shard identifier.
        shard_id: u32,
        /// Assigned coordinator node ID.
        coordinator_node_id: u64,
        /// Monotonic coordinator epoch.
        epoch: u64,
    },
    /// Register a sealed Tier-1 chunk manifest.
    RegisterChunkManifest {
        /// Stream UUID.
        stream_id: [u8; 16],
        /// Start logical offset.
        start_offset: u64,
        /// End logical offset (inclusive).
        end_offset: u64,
        /// Cloud object storage S3 URI.
        s3_uri: String,
        /// Chunk payload size in bytes.
        size_bytes: u64,
        /// CRC32C checksum of sealed chunk.
        crc32: u32,
    },
    /// Replicate a Roaring Bitmap state machine binary snapshot.
    ReplicateStateSnapshot {
        /// Consumer group identifier.
        group_id: String,
        /// Monotonic sliding base watermark at snapshot time.
        base_watermark: u64,
        /// Encoded snapshot bytes (`KSNP`).
        snapshot_bytes: Vec<u8>,
    },
    /// Replicate an incremental lease journal delta.
    ReplicateLeaseDelta {
        /// Consumer group identifier.
        group_id: String,
        /// Specific lease state transition.
        delta: LeaseDeltaRecord,
    },
    /// Replicate monotonic sliding base watermark advance.
    UpdateWatermark {
        /// Consumer group identifier.
        group_id: String,
        /// Advanced base watermark.
        watermark: u64,
    },
}

/// Replicated entry payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogPayload {
    /// Data plane active WAL batch payload (BatchHeader + Record Entries).
    DataBatch(Vec<u8>),
    /// Metadata & state plane administrative command.
    Metadata(MetadataCommand),
    /// Leader election no-op commitment barrier.
    Noop,
}

/// Replicated log entry in the Raft consensus log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaftLogEntry {
    /// Consensus term when entry was received by leader.
    pub term: Term,
    /// 1-based monotonically increasing log index.
    pub index: LogIndex,
    /// Command or data payload.
    pub payload: LogPayload,
}

/// Raft in-memory and durable log storage engine.
#[derive(Debug, Default, Clone)]
pub struct RaftLog {
    /// Ordered log entries.
    entries: Vec<RaftLogEntry>,
    /// Index of last entry compacted into snapshot.
    last_snapshot_index: LogIndex,
    /// Term of last entry compacted into snapshot.
    last_snapshot_term: Term,
    /// Commit index (highest index known to be committed).
    commit_index: LogIndex,
    /// Last applied index (highest index applied to state machine).
    last_applied: LogIndex,
}

impl RaftLog {
    /// Create a new empty Raft log.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Highest log index present in the log.
    #[must_use]
    pub fn last_log_index(&self) -> LogIndex {
        self.entries
            .last()
            .map_or(self.last_snapshot_index, |e| e.index)
    }

    /// Term of the highest log index present in the log.
    #[must_use]
    pub fn last_log_term(&self) -> Term {
        self.entries
            .last()
            .map_or(self.last_snapshot_term, |e| e.term)
    }

    /// Commit index.
    #[must_use]
    pub fn commit_index(&self) -> LogIndex {
        self.commit_index
    }

    /// Last included snapshot index.
    #[must_use]
    pub fn snapshot_index(&self) -> LogIndex {
        self.last_snapshot_index
    }

    /// Last included snapshot term.
    #[must_use]
    pub fn snapshot_term(&self) -> Term {
        self.last_snapshot_term
    }

    /// Advance commit index.
    pub fn set_commit_index(&mut self, index: LogIndex) {
        if index.0 > self.commit_index.0 {
            self.commit_index = index;
        }
    }

    /// Last applied index.
    #[must_use]
    pub fn last_applied(&self) -> LogIndex {
        self.last_applied
    }

    /// Advance last applied index.
    pub fn set_last_applied(&mut self, index: LogIndex) {
        if index.0 > self.last_applied.0 {
            self.last_applied = index;
        }
    }

    /// Get term for a given log index.
    #[must_use]
    pub fn term_at(&self, index: LogIndex) -> Option<Term> {
        if index == self.last_snapshot_index {
            return Some(self.last_snapshot_term);
        }
        if index.0 < self.last_snapshot_index.0 {
            return None;
        }
        let offset = (index.0 - self.last_snapshot_index.0).checked_sub(1)? as usize;
        self.entries.get(offset).map(|e| e.term)
    }

    /// Get entry at a specific log index.
    #[must_use]
    pub fn entry_at(&self, index: LogIndex) -> Option<&RaftLogEntry> {
        if index.0 <= self.last_snapshot_index.0 {
            return None;
        }
        let offset = (index.0 - self.last_snapshot_index.0).checked_sub(1)? as usize;
        self.entries.get(offset)
    }

    /// Append a new payload to the log as a leader.
    pub fn append_new(&mut self, term: Term, payload: LogPayload) -> LogIndex {
        let index = self.last_log_index().next();
        self.entries.push(RaftLogEntry {
            term,
            index,
            payload,
        });
        index
    }

    /// Append replicated entries received from leader, resolving log conflicts.
    pub fn append_replicated(&mut self, prev_index: LogIndex, entries: Vec<RaftLogEntry>) {
        if prev_index.0 > self.last_log_index().0 {
            return;
        }
        for entry in entries {
            if entry.index.0 <= self.last_snapshot_index.0 {
                continue;
            }
            let offset = (entry.index.0 - self.last_snapshot_index.0 - 1) as usize;
            if offset < self.entries.len() {
                if self.entries[offset].term != entry.term {
                    // Conflict detected: truncate log from this offset onwards
                    self.entries.truncate(offset);
                    self.entries.push(entry);
                }
            } else {
                self.entries.push(entry);
            }
        }
    }

    /// Get entries starting from index.
    #[must_use]
    pub fn entries_from(&self, start_index: LogIndex) -> Vec<RaftLogEntry> {
        if start_index.0 <= self.last_snapshot_index.0 {
            return self.entries.clone();
        }
        let offset = (start_index.0 - self.last_snapshot_index.0 - 1) as usize;
        if offset < self.entries.len() {
            self.entries[offset..].to_vec()
        } else {
            Vec::new()
        }
    }

    /// Compact log entries up to `snapshot_index`.
    pub fn compact_to(&mut self, snapshot_index: LogIndex, snapshot_term: Term) {
        if snapshot_index.0 <= self.last_snapshot_index.0 {
            return;
        }
        let remove_count = (snapshot_index.0 - self.last_snapshot_index.0) as usize;
        if remove_count <= self.entries.len() {
            self.entries.drain(0..remove_count);
        } else {
            self.entries.clear();
        }
        self.last_snapshot_index = snapshot_index;
        self.last_snapshot_term = snapshot_term;
        if self.commit_index.0 < snapshot_index.0 {
            self.commit_index = snapshot_index;
        }
        if self.last_applied.0 < snapshot_index.0 {
            self.last_applied = snapshot_index;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raft_log_append_and_compaction() {
        let mut log = RaftLog::new();
        assert_eq!(log.last_log_index(), LogIndex(0));
        assert_eq!(log.last_log_term(), Term(0));

        let idx1 = log.append_new(Term(1), LogPayload::Noop);
        assert_eq!(idx1, LogIndex(1));
        let idx2 = log.append_new(Term(1), LogPayload::DataBatch(vec![1, 2, 3]));
        assert_eq!(idx2, LogIndex(2));
        assert_eq!(log.last_log_index(), LogIndex(2));
        assert_eq!(log.last_log_term(), Term(1));

        log.compact_to(LogIndex(1), Term(1));
        assert_eq!(log.last_log_index(), LogIndex(2));
        assert_eq!(
            log.entry_at(LogIndex(2)).unwrap().payload,
            LogPayload::DataBatch(vec![1, 2, 3])
        );
        assert!(log.entry_at(LogIndex(1)).is_none());
    }
}
