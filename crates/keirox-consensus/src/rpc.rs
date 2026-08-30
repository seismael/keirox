//! Raft RPC messages, vote requests, append entries, and snapshot transfer frames per `KEI-ARC-022`.

use crate::log::RaftLogEntry;
use crate::types::{LogIndex, NodeId, Term};
use serde::{Deserialize, Serialize};

/// RequestVote RPC sent by candidates to gather votes during elections.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteRequest {
    /// Candidate's term.
    pub term: Term,
    /// Candidate requesting vote.
    pub candidate_id: NodeId,
    /// Index of candidate's last log entry.
    pub last_log_index: LogIndex,
    /// Term of candidate's last log entry.
    pub last_log_term: Term,
}

/// RequestVote RPC response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteResponse {
    /// Current term of responding node.
    pub term: Term,
    /// True if candidate received vote.
    pub vote_granted: bool,
}

/// AppendEntries RPC sent by leader to replicate log entries and serve as heartbeat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesRequest {
    /// Leader's term.
    pub term: Term,
    /// Leader node ID so follower can redirect clients.
    pub leader_id: NodeId,
    /// Index of log entry immediately preceding new ones.
    pub prev_log_index: LogIndex,
    /// Term of prev_log_index entry.
    pub prev_log_term: Term,
    /// Log entries to store (empty for heartbeat).
    pub entries: Vec<RaftLogEntry>,
    /// Leader's commitIndex.
    pub leader_commit: LogIndex,
}

/// AppendEntries RPC response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesResponse {
    /// Current term of responding node.
    pub term: Term,
    /// True if follower contained entry matching prev_log_index and prev_log_term.
    pub success: bool,
    /// Highest log index acknowledged/matched by follower.
    pub match_index: LogIndex,
}

/// InstallSnapshot RPC sent by leader to catch up laggy followers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallSnapshotRequest {
    /// Leader's term.
    pub term: Term,
    /// Leader node ID.
    pub leader_id: NodeId,
    /// Snapshot replaces all entries up through and including this index.
    pub last_included_index: LogIndex,
    /// Term of last_included_index.
    pub last_included_term: Term,
    /// Raw snapshot payload bytes.
    pub data: Vec<u8>,
}

/// InstallSnapshot RPC response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallSnapshotResponse {
    /// Current term of responding node.
    pub term: Term,
    /// True if snapshot was accepted and applied.
    pub success: bool,
}
