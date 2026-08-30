//! Data Plane Raft group managing synchronous 3-node quorum replication over WAL segment heads per `KEI-ARC-022`.

use crate::engine::RaftEngine;
use crate::log::LogPayload;
use crate::transport::RaftTransport;
use crate::types::{ClusterConfig, LogIndex, NodeId, Term};
use keirox_core::error::{KeiroxError, Result};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Data Plane Raft group coordinating synchronous WAL quorum commits.
#[derive(Debug, Clone)]
pub struct DataPlaneRaftGroup {
    engine: Arc<RwLock<RaftEngine>>,
    transport: Arc<dyn RaftTransport>,
}

impl DataPlaneRaftGroup {
    /// Initialize a new Data Plane Raft group.
    pub fn new(config: ClusterConfig, transport: Arc<dyn RaftTransport>) -> Self {
        let engine = Arc::new(RwLock::new(RaftEngine::new(config)));
        Self { engine, transport }
    }

    /// Local node identifier.
    pub async fn local_node_id(&self) -> NodeId {
        self.engine.read().await.local_node_id()
    }

    /// Current consensus term.
    pub async fn current_term(&self) -> Term {
        self.engine.read().await.current_term()
    }

    /// True if local node is cluster leader.
    pub async fn is_leader(&self) -> bool {
        self.engine.read().await.is_leader()
    }

    /// Current known leader.
    pub async fn current_leader(&self) -> Option<NodeId> {
        self.engine.read().await.current_leader()
    }

    /// Commit index.
    pub async fn commit_index(&self) -> LogIndex {
        self.engine.read().await.commit_index()
    }

    /// Start leader election campaign across cluster.
    pub async fn campaign(&self) -> Result<bool> {
        let vote_req = {
            let mut engine = self.engine.write().await;
            engine.start_election()
        };

        if self.is_leader().await {
            return Ok(true);
        }

        let targets: Vec<NodeId> = {
            let engine = self.engine.read().await;
            engine.peer_ids()
        };

        let mut votes_granted = 1; // Self vote
        let quorum = (targets.len() + 1).div_ceil(2);

        for target in targets {
            if let Ok(resp) = self.transport.send_vote(target, vote_req.clone()).await {
                let vote_granted = resp.vote_granted;
                let mut engine = self.engine.write().await;
                if engine.handle_vote_response(target, resp) {
                    return Ok(true);
                }
                if vote_granted {
                    votes_granted += 1;
                    if votes_granted >= quorum {
                        return Ok(true);
                    }
                }
            }
        }

        Ok(self.is_leader().await)
    }

    /// Synchronously replicate active WAL batch payload to 3-node quorum before local commit.
    pub async fn append_batch_quorum(&self, batch_payload: Vec<u8>) -> Result<LogIndex> {
        let proposed_index = {
            let mut engine = self.engine.write().await;
            engine.propose(LogPayload::DataBatch(batch_payload))?
        };

        // Broadcast append entries to peers and collect quorum
        let requests = {
            let engine = self.engine.read().await;
            engine.prepare_append_entries()
        };

        if requests.is_empty() {
            // Single-node mode: already committed locally
            return Ok(proposed_index);
        }

        let mut acks = 1; // Leader local ack
        let quorum = (requests.len() + 1).div_ceil(2);

        for (target, req) in requests {
            if let Ok(resp) = self.transport.send_append_entries(target, req).await {
                let is_match = resp.success && resp.match_index.0 >= proposed_index.0;
                let mut engine = self.engine.write().await;
                engine.handle_append_response(target, resp);
                if is_match {
                    acks += 1;
                }
            }
        }

        let committed = self.commit_index().await;
        if committed.0 >= proposed_index.0 || acks >= quorum {
            Ok(proposed_index)
        } else {
            Err(KeiroxError::QuorumUnavailable(format!(
                "Failed to reach majority quorum for batch index {proposed_index} (acks: {acks}/{quorum})"
            )))
        }
    }

    /// Read raw engine reference for low-level testing.
    pub fn engine(&self) -> Arc<RwLock<RaftEngine>> {
        self.engine.clone()
    }
}
