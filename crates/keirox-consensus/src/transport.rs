//! Asynchronous transport layer and deterministic in-memory simulated mesh router per `KEI-ARC-022`.

use crate::rpc::{
    AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest, InstallSnapshotResponse,
    VoteRequest, VoteResponse,
};
use crate::types::NodeId;
use async_trait::async_trait;
use keirox_core::error::{KeiroxError, Result};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;

/// Asynchronous transport interface for Raft peer communication.
#[async_trait]
pub trait RaftTransport: std::fmt::Debug + Send + Sync {
    /// Send RequestVote RPC to target node.
    async fn send_vote(&self, target: NodeId, req: VoteRequest) -> Result<VoteResponse>;

    /// Send AppendEntries RPC to target node.
    async fn send_append_entries(
        &self,
        target: NodeId,
        req: AppendEntriesRequest,
    ) -> Result<AppendEntriesResponse>;

    /// Send InstallSnapshot RPC to target node.
    async fn send_install_snapshot(
        &self,
        target: NodeId,
        req: InstallSnapshotRequest,
    ) -> Result<InstallSnapshotResponse>;
}

/// Message envelope for in-memory channel mesh simulation.
#[derive(Debug)]
pub enum RaftMessage {
    /// Vote request and one-shot reply channel.
    Vote(
        VoteRequest,
        tokio::sync::oneshot::Sender<Result<VoteResponse>>,
    ),
    /// Append entries request and one-shot reply channel.
    AppendEntries(
        AppendEntriesRequest,
        tokio::sync::oneshot::Sender<Result<AppendEntriesResponse>>,
    ),
    /// Snapshot request and one-shot reply channel.
    InstallSnapshot(
        InstallSnapshotRequest,
        tokio::sync::oneshot::Sender<Result<InstallSnapshotResponse>>,
    ),
}

/// Shared mesh network router for multi-node simulation with partition controls.
#[derive(Debug, Clone, Default)]
pub struct ChannelMesh {
    /// Registered node inbound channels.
    inbound: Arc<RwLock<HashMap<NodeId, mpsc::Sender<RaftMessage>>>>,
    /// Disconnected directional edges (from -> to).
    disconnected: Arc<RwLock<HashSet<(NodeId, NodeId)>>>,
}

impl ChannelMesh {
    /// Create a new channel mesh.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a node into the mesh and return its receiver channel.
    pub fn register(&self, node_id: NodeId, buffer: usize) -> mpsc::Receiver<RaftMessage> {
        let (tx, rx) = mpsc::channel(buffer);
        self.inbound.write().unwrap().insert(node_id, tx);
        rx
    }

    /// Create transport handle for a specific source node.
    #[must_use]
    pub fn create_transport(&self, local_node_id: NodeId) -> MeshTransport {
        MeshTransport {
            local_node_id,
            mesh: self.clone(),
        }
    }

    /// Isolate a single node from all peers.
    pub fn isolate_node(&self, node: NodeId) {
        let nodes: Vec<NodeId> = self.inbound.read().unwrap().keys().copied().collect();
        let mut disc = self.disconnected.write().unwrap();
        for other in nodes {
            if other != node {
                disc.insert((node, other));
                disc.insert((other, node));
            }
        }
    }

    /// Partition the cluster into two disjoint groups.
    pub fn partition(&self, group_a: &[NodeId], group_b: &[NodeId]) {
        let mut disc = self.disconnected.write().unwrap();
        for &a in group_a {
            for &b in group_b {
                disc.insert((a, b));
                disc.insert((b, a));
            }
        }
    }

    /// Heal all partitions and restore full connectivity.
    pub fn heal(&self) {
        self.disconnected.write().unwrap().clear();
    }

    /// Check if communication is permitted from -> to.
    #[must_use]
    pub fn is_connected(&self, from: NodeId, to: NodeId) -> bool {
        !self.disconnected.read().unwrap().contains(&(from, to))
    }
}

/// Mesh-backed RaftTransport handle for a local node.
#[derive(Debug, Clone)]
pub struct MeshTransport {
    local_node_id: NodeId,
    mesh: ChannelMesh,
}

#[async_trait]
impl RaftTransport for MeshTransport {
    async fn send_vote(&self, target: NodeId, req: VoteRequest) -> Result<VoteResponse> {
        if !self.mesh.is_connected(self.local_node_id, target) {
            return Err(KeiroxError::Consensus(format!(
                "Network partition: cannot route VoteRequest from {} to {}",
                self.local_node_id, target
            )));
        }

        let sender = {
            let map = self.mesh.inbound.read().unwrap();
            map.get(&target).cloned().ok_or_else(|| {
                KeiroxError::Consensus(format!("Target node {} not found in mesh", target))
            })?
        };

        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        sender
            .send(RaftMessage::Vote(req, reply_tx))
            .await
            .map_err(|e| KeiroxError::Consensus(format!("Send failed: {e}")))?;

        reply_rx
            .await
            .map_err(|_| KeiroxError::Consensus("Target node dropped reply".into()))?
    }

    async fn send_append_entries(
        &self,
        target: NodeId,
        req: AppendEntriesRequest,
    ) -> Result<AppendEntriesResponse> {
        if !self.mesh.is_connected(self.local_node_id, target) {
            return Err(KeiroxError::Consensus(format!(
                "Network partition: cannot route AppendEntries from {} to {}",
                self.local_node_id, target
            )));
        }

        let sender = {
            let map = self.mesh.inbound.read().unwrap();
            map.get(&target).cloned().ok_or_else(|| {
                KeiroxError::Consensus(format!("Target node {} not found in mesh", target))
            })?
        };

        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        sender
            .send(RaftMessage::AppendEntries(req, reply_tx))
            .await
            .map_err(|e| KeiroxError::Consensus(format!("Send failed: {e}")))?;

        reply_rx
            .await
            .map_err(|_| KeiroxError::Consensus("Target node dropped reply".into()))?
    }

    async fn send_install_snapshot(
        &self,
        target: NodeId,
        req: InstallSnapshotRequest,
    ) -> Result<InstallSnapshotResponse> {
        if !self.mesh.is_connected(self.local_node_id, target) {
            return Err(KeiroxError::Consensus(format!(
                "Network partition: cannot route InstallSnapshot from {} to {}",
                self.local_node_id, target
            )));
        }

        let sender = {
            let map = self.mesh.inbound.read().unwrap();
            map.get(&target).cloned().ok_or_else(|| {
                KeiroxError::Consensus(format!("Target node {} not found in mesh", target))
            })?
        };

        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        sender
            .send(RaftMessage::InstallSnapshot(req, reply_tx))
            .await
            .map_err(|e| KeiroxError::Consensus(format!("Send failed: {e}")))?;

        reply_rx
            .await
            .map_err(|_| KeiroxError::Consensus("Target node dropped reply".into()))?
    }
}
