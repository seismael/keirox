//! Dynamic cluster membership, node lifecycle, and graceful leader transfer per `KEI-ARC-022`.

use crate::types::{ClusterConfig, NodeId, PeerEndpoint};
use std::collections::HashSet;

/// Node lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeStatus {
    /// Active member participating in consensus and shard hosting.
    Active,
    /// Draining member migrating coordinator shards before decommission.
    Draining,
    /// Fully decommissioned/offline node.
    Decommissioned,
}

/// Cluster membership manager tracking active, draining, and decommissioned nodes.
#[derive(Debug, Clone)]
pub struct MembershipManager {
    local_node_id: NodeId,
    active_members: HashSet<NodeId>,
    draining_members: HashSet<NodeId>,
    endpoints: Vec<PeerEndpoint>,
}

impl MembershipManager {
    /// Initialize membership manager from cluster configuration.
    #[must_use]
    pub fn new(config: &ClusterConfig) -> Self {
        let mut active_members = HashSet::new();
        active_members.insert(config.local_node_id);
        for peer in &config.peers {
            active_members.insert(peer.node_id);
        }

        Self {
            local_node_id: config.local_node_id,
            active_members,
            draining_members: HashSet::new(),
            endpoints: config.peers.clone(),
        }
    }

    /// Local node ID.
    #[must_use]
    pub fn local_node_id(&self) -> NodeId {
        self.local_node_id
    }

    /// Add a new node to the active cluster membership.
    pub fn add_node(&mut self, node_id: NodeId, address: String) {
        self.active_members.insert(node_id);
        self.draining_members.remove(&node_id);
        if !self.endpoints.iter().any(|p| p.node_id == node_id) {
            self.endpoints.push(PeerEndpoint { node_id, address });
        }
    }

    /// Mark a node as draining to allow graceful migration of coordinator shards.
    pub fn drain_node(&mut self, node_id: NodeId) {
        if self.active_members.contains(&node_id) {
            self.active_members.remove(&node_id);
            self.draining_members.insert(node_id);
        }
    }

    /// Remove a decommissioned node from cluster membership.
    pub fn remove_node(&mut self, node_id: NodeId) {
        self.active_members.remove(&node_id);
        self.draining_members.remove(&node_id);
        self.endpoints.retain(|p| p.node_id != node_id);
    }

    /// Node status lookup.
    #[must_use]
    pub fn status_of(&self, node_id: NodeId) -> NodeStatus {
        if self.active_members.contains(&node_id) {
            NodeStatus::Active
        } else if self.draining_members.contains(&node_id) {
            NodeStatus::Draining
        } else {
            NodeStatus::Decommissioned
        }
    }

    /// Active member count.
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.active_members.len()
    }

    /// List of active member node IDs.
    #[must_use]
    pub fn active_nodes(&self) -> Vec<NodeId> {
        let mut list: Vec<NodeId> = self.active_members.iter().copied().collect();
        list.sort_unstable();
        list
    }
}
