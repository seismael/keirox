//! Core Raft consensus engine and state machine per `KEI-ARC-022`.

use crate::log::{LogPayload, RaftLog};
use crate::rpc::{
    AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest, InstallSnapshotResponse,
    VoteRequest, VoteResponse,
};
use crate::types::{ClusterConfig, LogIndex, NodeId, ReplicaRole, Term};
use keirox_core::error::{KeiroxError, Result};
use std::collections::{HashMap, HashSet};

/// Core Raft consensus state machine.
#[derive(Debug)]
pub struct RaftEngine {
    /// Cluster configuration.
    config: ClusterConfig,
    /// Current consensus term.
    current_term: Term,
    /// Candidate voted for in current term.
    voted_for: Option<NodeId>,
    /// Current role (Follower, Candidate, Leader).
    role: ReplicaRole,
    /// Replicated log storage.
    log: RaftLog,
    /// Leader tracking: next log index to send to each peer.
    next_index: HashMap<NodeId, LogIndex>,
    /// Leader tracking: highest log index known to be replicated on each peer.
    match_index: HashMap<NodeId, LogIndex>,
    /// Votes granted in current election (if Candidate).
    votes_received: HashSet<NodeId>,
    /// Current leader node ID if known.
    current_leader: Option<NodeId>,
}

impl RaftEngine {
    /// Initialize a new Raft consensus engine.
    #[must_use]
    pub fn new(config: ClusterConfig) -> Self {
        let is_single = config.peers.is_empty();
        let role = if is_single {
            ReplicaRole::Leader
        } else {
            ReplicaRole::Follower
        };
        let current_leader = if is_single {
            Some(config.local_node_id)
        } else {
            None
        };

        Self {
            config,
            current_term: Term(0),
            voted_for: None,
            role,
            log: RaftLog::new(),
            next_index: HashMap::new(),
            match_index: HashMap::new(),
            votes_received: HashSet::new(),
            current_leader,
        }
    }

    /// Local node identifier.
    #[must_use]
    pub fn local_node_id(&self) -> NodeId {
        self.config.local_node_id
    }

    /// List of configured peer node IDs.
    #[must_use]
    pub fn peer_ids(&self) -> Vec<NodeId> {
        self.config.peers.iter().map(|p| p.node_id).collect()
    }

    /// Current consensus term.
    #[must_use]
    pub fn current_term(&self) -> Term {
        self.current_term
    }

    /// Current replica role.
    #[must_use]
    pub fn role(&self) -> ReplicaRole {
        self.role
    }

    /// Current known cluster leader.
    #[must_use]
    pub fn current_leader(&self) -> Option<NodeId> {
        self.current_leader
    }

    /// Replicated log reference.
    #[must_use]
    pub fn log(&self) -> &RaftLog {
        &self.log
    }

    /// Mutable replicated log reference.
    pub fn log_mut(&mut self) -> &mut RaftLog {
        &mut self.log
    }

    /// Commit index.
    #[must_use]
    pub fn commit_index(&self) -> LogIndex {
        self.log.commit_index()
    }

    /// True if this node is currently the quorum-elected leader.
    #[must_use]
    pub fn is_leader(&self) -> bool {
        self.role == ReplicaRole::Leader
    }

    /// Propose a new payload to the Raft cluster (Leader only).
    pub fn propose(&mut self, payload: LogPayload) -> Result<LogIndex> {
        if self.role != ReplicaRole::Leader {
            return Err(KeiroxError::Consensus(format!(
                "Cannot propose to non-leader node (current leader: {:?})",
                self.current_leader
            )));
        }

        let index = self.log.append_new(self.current_term, payload);
        self.match_index.insert(self.config.local_node_id, index);

        if self.config.peers.is_empty() {
            // Single-node mode: commit immediately
            self.log.set_commit_index(index);
        }

        Ok(index)
    }

    /// Transition to candidate and start leader election.
    pub fn start_election(&mut self) -> VoteRequest {
        self.role = ReplicaRole::Candidate;
        self.current_term = self.current_term.next();
        self.voted_for = Some(self.config.local_node_id);
        self.votes_received.clear();
        self.votes_received.insert(self.config.local_node_id);
        self.current_leader = None;

        if self.votes_received.len() >= self.config.quorum_size() {
            self.become_leader();
        }

        VoteRequest {
            term: self.current_term,
            candidate_id: self.config.local_node_id,
            last_log_index: self.log.last_log_index(),
            last_log_term: self.log.last_log_term(),
        }
    }

    /// Handle vote response from peer. Returns true if node transitioned to leader.
    pub fn handle_vote_response(&mut self, from: NodeId, resp: VoteResponse) -> bool {
        if resp.term > self.current_term {
            self.step_down_to_follower(resp.term);
            return false;
        }

        if self.role == ReplicaRole::Candidate
            && resp.term == self.current_term
            && resp.vote_granted
        {
            self.votes_received.insert(from);
            if self.votes_received.len() >= self.config.quorum_size() {
                self.become_leader();
                return true;
            }
        }

        false
    }

    /// Transition node to Leader role.
    fn become_leader(&mut self) {
        self.role = ReplicaRole::Leader;
        self.current_leader = Some(self.config.local_node_id);
        let next_idx = self.log.last_log_index().next();
        for peer in &self.config.peers {
            self.next_index.insert(peer.node_id, next_idx);
            self.match_index.insert(peer.node_id, LogIndex(0));
        }
        self.match_index
            .insert(self.config.local_node_id, self.log.last_log_index());

        // Append leader election no-op barrier per Raft spec
        let _ = self.log.append_new(self.current_term, LogPayload::Noop);
        self.match_index
            .insert(self.config.local_node_id, self.log.last_log_index());
    }

    /// Step down to Follower role upon encountering a higher term.
    pub fn step_down_to_follower(&mut self, higher_term: Term) {
        self.current_term = higher_term;
        self.role = ReplicaRole::Follower;
        self.voted_for = None;
        self.votes_received.clear();
        self.current_leader = None;
    }

    /// Handle RequestVote RPC from a candidate peer.
    pub fn handle_vote_request(&mut self, req: VoteRequest) -> VoteResponse {
        if req.term > self.current_term {
            self.step_down_to_follower(req.term);
        }

        let mut vote_granted = false;
        if req.term == self.current_term
            && (self.voted_for.is_none() || self.voted_for == Some(req.candidate_id))
        {
            let last_log_term = self.log.last_log_term();
            let last_log_index = self.log.last_log_index();

            let log_is_up_to_date = req.last_log_term > last_log_term
                || (req.last_log_term == last_log_term && req.last_log_index >= last_log_index);

            if log_is_up_to_date {
                vote_granted = true;
                self.voted_for = Some(req.candidate_id);
            }
        }

        VoteResponse {
            term: self.current_term,
            vote_granted,
        }
    }

    /// Prepare AppendEntries requests for all peers (heartbeat or new entries).
    #[must_use]
    pub fn prepare_append_entries(&self) -> Vec<(NodeId, AppendEntriesRequest)> {
        if self.role != ReplicaRole::Leader {
            return Vec::new();
        }

        let mut requests = Vec::new();
        for peer in &self.config.peers {
            let next_idx = self
                .next_index
                .get(&peer.node_id)
                .copied()
                .unwrap_or_else(|| self.log.last_log_index().next());

            let prev_idx = LogIndex(next_idx.0.saturating_sub(1));
            let prev_term = self.log.term_at(prev_idx).unwrap_or(Term(0));
            let entries = self.log.entries_from(next_idx);

            requests.push((
                peer.node_id,
                AppendEntriesRequest {
                    term: self.current_term,
                    leader_id: self.config.local_node_id,
                    prev_log_index: prev_idx,
                    prev_log_term: prev_term,
                    entries,
                    leader_commit: self.log.commit_index(),
                },
            ));
        }

        requests
    }

    /// Handle AppendEntries RPC from leader.
    pub fn handle_append_entries(&mut self, req: AppendEntriesRequest) -> AppendEntriesResponse {
        if req.term > self.current_term {
            self.step_down_to_follower(req.term);
        }

        if req.term < self.current_term {
            return AppendEntriesResponse {
                term: self.current_term,
                success: false,
                match_index: self.log.last_log_index(),
            };
        }

        // Valid leader message: reset follower state
        self.role = ReplicaRole::Follower;
        self.current_leader = Some(req.leader_id);

        // Check log consistency at prev_log_index
        if req.prev_log_index.0 > 0 {
            match self.log.term_at(req.prev_log_index) {
                Some(term) if term == req.prev_log_term => {}
                _ => {
                    return AppendEntriesResponse {
                        term: self.current_term,
                        success: false,
                        match_index: self.log.last_log_index(),
                    };
                }
            }
        }

        // Append new entries and resolve conflicts
        self.log.append_replicated(req.prev_log_index, req.entries);

        // Advance commit index
        if req.leader_commit.0 > self.log.commit_index().0 {
            let new_commit = req.leader_commit.min(self.log.last_log_index());
            self.log.set_commit_index(new_commit);
        }

        AppendEntriesResponse {
            term: self.current_term,
            success: true,
            match_index: self.log.last_log_index(),
        }
    }

    /// Handle AppendEntries response from follower (Leader only).
    pub fn handle_append_response(&mut self, from: NodeId, resp: AppendEntriesResponse) {
        if resp.term > self.current_term {
            self.step_down_to_follower(resp.term);
            return;
        }

        if self.role != ReplicaRole::Leader || resp.term != self.current_term {
            return;
        }

        if resp.success {
            self.next_index.insert(from, resp.match_index.next());
            self.match_index.insert(from, resp.match_index);

            // Check if commit index can be advanced
            let mut match_indices: Vec<u64> = self.match_index.values().map(|i| i.0).collect();
            match_indices.push(self.log.last_log_index().0);
            match_indices.sort_unstable();
            if match_indices.len() >= self.config.quorum_size() {
                let quorum_idx = match_indices[match_indices.len() - self.config.quorum_size()];
                let target_commit = LogIndex(quorum_idx);

                if target_commit.0 > self.log.commit_index().0
                    && self.log.term_at(target_commit) == Some(self.current_term)
                {
                    self.log.set_commit_index(target_commit);
                }
            }
        } else {
            // Decrement next_index for retry
            if let Some(next_idx) = self.next_index.get_mut(&from) {
                if next_idx.0 > 1 {
                    *next_idx = LogIndex(next_idx.0 - 1);
                }
            }
        }
    }

    /// Handle InstallSnapshot RPC from leader.
    pub fn handle_install_snapshot(
        &mut self,
        req: InstallSnapshotRequest,
    ) -> InstallSnapshotResponse {
        if req.term > self.current_term {
            self.step_down_to_follower(req.term);
        }

        if req.term < self.current_term {
            return InstallSnapshotResponse {
                term: self.current_term,
                success: false,
            };
        }

        self.role = ReplicaRole::Follower;
        self.current_leader = Some(req.leader_id);

        if req.last_included_index.0 > self.log.commit_index().0 {
            self.log
                .compact_to(req.last_included_index, req.last_included_term);
            self.log.set_commit_index(req.last_included_index);
            self.log.set_last_applied(req.last_included_index);
        }

        InstallSnapshotResponse {
            term: self.current_term,
            success: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_three_node_election_and_quorum_commit() {
        let config1 = ClusterConfig::three_node(NodeId(1), [2, 3]);
        let config2 = ClusterConfig::three_node(NodeId(2), [1, 3]);
        let config3 = ClusterConfig::three_node(NodeId(3), [1, 2]);

        let mut node1 = RaftEngine::new(config1);
        let mut node2 = RaftEngine::new(config2);
        let mut node3 = RaftEngine::new(config3);

        // Node 1 starts election
        let vote_req = node1.start_election();
        assert_eq!(node1.role(), ReplicaRole::Candidate);
        assert_eq!(vote_req.term, Term(1));

        // Node 2 grants vote
        let vote_resp2 = node2.handle_vote_request(vote_req.clone());
        assert!(vote_resp2.vote_granted);
        let became_leader = node1.handle_vote_response(NodeId(2), vote_resp2);
        assert!(became_leader);
        assert_eq!(node1.role(), ReplicaRole::Leader);

        // Node 3 also grants vote
        let vote_resp3 = node3.handle_vote_request(vote_req);
        assert!(vote_resp3.vote_granted);
        node1.handle_vote_response(NodeId(3), vote_resp3);

        // Propose data entry on leader
        let data_idx = node1
            .propose(LogPayload::DataBatch(vec![10, 20, 30]))
            .unwrap();
        assert_eq!(data_idx, LogIndex(2)); // Index 1 was the Noop barrier

        // Replicate to followers
        let appends = node1.prepare_append_entries();
        assert_eq!(appends.len(), 2);

        for (target, req) in appends {
            if target == NodeId(2) {
                let resp = node2.handle_append_entries(req);
                assert!(resp.success);
                node1.handle_append_response(NodeId(2), resp);
            } else if target == NodeId(3) {
                let resp = node3.handle_append_entries(req);
                assert!(resp.success);
                node1.handle_append_response(NodeId(3), resp);
            }
        }

        // Leader commit index should now be advanced to 2
        assert_eq!(node1.commit_index(), LogIndex(2));
    }

    #[test]
    fn test_handle_install_snapshot() {
        let config = ClusterConfig::three_node(NodeId(2), [1, 3]);
        let mut follower = RaftEngine::new(config);

        let snap_req = InstallSnapshotRequest {
            term: Term(3),
            leader_id: NodeId(1),
            last_included_index: LogIndex(100),
            last_included_term: Term(3),
            data: vec![1, 2, 3, 4],
        };

        let resp = follower.handle_install_snapshot(snap_req);
        assert!(resp.success);
        assert_eq!(follower.current_term(), Term(3));
        assert_eq!(follower.commit_index(), LogIndex(100));
        assert_eq!(follower.current_leader(), Some(NodeId(1)));
    }
}
