//! Deterministic consistent hashing ring with virtual nodes per `KEI-ARC-021` and `KEI-ARC-022`.

use crate::shard::ShardId;
use keirox_consensus::NodeId;
use std::collections::BTreeMap;
use twox_hash::XxHash64;

/// Default virtual nodes per physical node for uniform shard distribution.
pub const DEFAULT_VNODES_PER_NODE: usize = 128;

/// Total logical shards distributed across cluster.
pub const TOTAL_SHARDS: u32 = 1024;

/// Consistent hashing ring mapping consumer groups to shards and nodes.
#[derive(Debug, Clone)]
pub struct ConsistentHashRing {
    vnodes_per_node: usize,
    ring: BTreeMap<u64, (NodeId, ShardId)>,
    nodes: Vec<NodeId>,
}

impl Default for ConsistentHashRing {
    fn default() -> Self {
        Self::new(DEFAULT_VNODES_PER_NODE)
    }
}

impl ConsistentHashRing {
    /// Create a new consistent hashing ring with custom vnodes per physical node.
    #[must_use]
    pub fn new(vnodes_per_node: usize) -> Self {
        Self {
            vnodes_per_node,
            ring: BTreeMap::new(),
            nodes: Vec::new(),
        }
    }

    /// Hash helper using 64-bit xxHash.
    fn hash_key(key: &str) -> u64 {
        use std::hash::Hasher;
        let mut hasher = XxHash64::default();
        hasher.write(key.as_bytes());
        hasher.finish()
    }

    /// Add a physical node to the ring.
    pub fn add_node(&mut self, node_id: NodeId) {
        if !self.nodes.contains(&node_id) {
            self.nodes.push(node_id);
            self.rebuild();
        }
    }

    /// Remove a physical node from the ring.
    pub fn remove_node(&mut self, node_id: NodeId) {
        if let Some(pos) = self.nodes.iter().position(|&n| n == node_id) {
            self.nodes.remove(pos);
            self.rebuild();
        }
    }

    /// Rebuild virtual node positions across the 64-bit ring.
    fn rebuild(&mut self) {
        self.ring.clear();
        if self.nodes.is_empty() {
            return;
        }

        for (node_idx, &node) in self.nodes.iter().enumerate() {
            for v in 0..self.vnodes_per_node {
                let vnode_key = format!("node_{}_vnode_{}", node.0, v);
                let hash = Self::hash_key(&vnode_key);
                let shard_id =
                    ShardId(((node_idx * self.vnodes_per_node + v) as u32) % TOTAL_SHARDS);
                self.ring.insert(hash, (node, shard_id));
            }
        }
    }

    /// Deterministically map a consumer group ID to its coordinator shard and physical node.
    #[must_use]
    pub fn map_group(&self, group_id: &str) -> Option<(ShardId, NodeId)> {
        if self.ring.is_empty() {
            return None;
        }

        let hash = Self::hash_key(group_id);
        // Find first ring entry >= hash, or wrap around to first entry in ring
        let (&_ring_key, &(node_id, shard_id)) = self
            .ring
            .range(hash..)
            .next()
            .unwrap_or_else(|| self.ring.iter().next().unwrap());

        Some((shard_id, node_id))
    }

    /// Total active nodes in ring.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consistent_hash_mapping_and_rebalance() {
        let mut ring = ConsistentHashRing::new(64);
        ring.add_node(NodeId(1));
        ring.add_node(NodeId(2));
        ring.add_node(NodeId(3));

        let (shard_a, node_a) = ring.map_group("group-payments-prod").unwrap();
        let (shard_b, node_b) = ring.map_group("group-orders-checkout").unwrap();

        // Deterministic repeat check
        assert_eq!(
            ring.map_group("group-payments-prod"),
            Some((shard_a, node_a))
        );
        assert_eq!(
            ring.map_group("group-orders-checkout"),
            Some((shard_b, node_b))
        );

        // Remove node 1, groups should safely remap to remaining nodes 2 or 3
        ring.remove_node(NodeId(1));
        let (_, new_node_a) = ring.map_group("group-payments-prod").unwrap();
        assert!(new_node_a == NodeId(2) || new_node_a == NodeId(3));
    }
}
