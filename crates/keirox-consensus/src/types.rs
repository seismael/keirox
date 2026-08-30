//! Core distributed consensus types, node identifiers, terms, and cluster topologies per `KEI-ARC-022`.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique numerical identifier for a cluster node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u64);

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Node-{}", self.0)
    }
}

/// Monotonically increasing consensus term.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub struct Term(pub u64);

impl Term {
    /// Advance term by one.
    #[must_use]
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Term({})", self.0)
    }
}

/// 1-based log entry index in the Raft log.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub struct LogIndex(pub u64);

impl LogIndex {
    /// Advance log index by one.
    #[must_use]
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

impl fmt::Display for LogIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Index({})", self.0)
    }
}

/// Raft replica state role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplicaRole {
    /// Follower accepting heartbeats and log replication from current leader.
    Follower,
    /// Candidate campaigning in leader election.
    Candidate,
    /// Quorum-elected leader coordinating log appends and heartbeats.
    Leader,
}

impl fmt::Display for ReplicaRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Follower => write!(f, "FOLLOWER"),
            Self::Candidate => write!(f, "CANDIDATE"),
            Self::Leader => write!(f, "LEADER"),
        }
    }
}

/// Peer node network endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerEndpoint {
    /// Target node ID.
    pub node_id: NodeId,
    /// Network address (host:port or in-mesh channel URI).
    pub address: String,
}

/// Cluster configuration defining local node ID and peer topology.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    /// Local node ID.
    pub local_node_id: NodeId,
    /// Peer cluster nodes.
    pub peers: Vec<PeerEndpoint>,
    /// Minimum randomized election timeout in milliseconds.
    pub election_timeout_min_ms: u64,
    /// Maximum randomized election timeout in milliseconds.
    pub election_timeout_max_ms: u64,
    /// Leader heartbeat interval in milliseconds.
    pub heartbeat_interval_ms: u64,
}

impl ClusterConfig {
    /// Create a standard 3-node cluster configuration.
    #[must_use]
    pub fn three_node(local_node_id: NodeId, peer_ids: [u64; 2]) -> Self {
        let peers = peer_ids
            .into_iter()
            .map(|id| PeerEndpoint {
                node_id: NodeId(id),
                address: format!("mesh://node-{id}"),
            })
            .collect();

        Self {
            local_node_id,
            peers,
            election_timeout_min_ms: 150,
            election_timeout_max_ms: 300,
            heartbeat_interval_ms: 50,
        }
    }

    /// Single node development configuration.
    #[must_use]
    pub fn single_node(local_node_id: NodeId) -> Self {
        Self {
            local_node_id,
            peers: Vec::new(),
            election_timeout_min_ms: 150,
            election_timeout_max_ms: 300,
            heartbeat_interval_ms: 50,
        }
    }

    /// Total cluster size including local node.
    #[must_use]
    pub fn total_nodes(&self) -> usize {
        self.peers.len() + 1
    }

    /// Quorum size: floor(N/2) + 1.
    #[must_use]
    pub fn quorum_size(&self) -> usize {
        (self.total_nodes() / 2) + 1
    }
}
