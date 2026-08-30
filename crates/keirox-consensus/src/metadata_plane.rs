//! Metadata & State Raft group managing replicated coordinator assignments, manifests, and snapshots per `KEI-ARC-022`.

use crate::engine::RaftEngine;
use crate::log::{LeaseDeltaRecord, LogPayload, MetadataCommand};
use crate::transport::RaftTransport;
use crate::types::{ClusterConfig, LogIndex};
use keirox_core::error::{KeiroxError, Result};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Metadata & State Raft group managing replicated cluster metadata.
#[derive(Debug, Clone)]
pub struct MetadataRaftGroup {
    engine: Arc<RwLock<RaftEngine>>,
    transport: Arc<dyn RaftTransport>,
}

impl MetadataRaftGroup {
    /// Initialize a new Metadata Raft group.
    pub fn new(config: ClusterConfig, transport: Arc<dyn RaftTransport>) -> Self {
        let engine = Arc::new(RwLock::new(RaftEngine::new(config)));
        Self { engine, transport }
    }

    /// True if local node is leader of the metadata group.
    pub async fn is_leader(&self) -> bool {
        self.engine.read().await.is_leader()
    }

    /// Replicate a metadata command to quorum.
    async fn replicate_command(&self, cmd: MetadataCommand) -> Result<LogIndex> {
        let proposed_index = {
            let mut engine = self.engine.write().await;
            engine.propose(LogPayload::Metadata(cmd))?
        };

        let requests = {
            let engine = self.engine.read().await;
            engine.prepare_append_entries()
        };

        if requests.is_empty() {
            return Ok(proposed_index);
        }

        let mut acks = 1;
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

        if acks >= quorum {
            Ok(proposed_index)
        } else {
            Err(KeiroxError::QuorumUnavailable(format!(
                "Failed to reach metadata quorum for index {proposed_index}"
            )))
        }
    }

    /// Assign a coordinator shard to a cluster node with a monotonic epoch.
    pub async fn assign_shard(
        &self,
        shard_id: u32,
        coordinator_node_id: u64,
        epoch: u64,
    ) -> Result<LogIndex> {
        self.replicate_command(MetadataCommand::AssignShard {
            shard_id,
            coordinator_node_id,
            epoch,
        })
        .await
    }

    /// Register a sealed Tier-1 chunk manifest.
    pub async fn register_manifest(
        &self,
        stream_id: [u8; 16],
        start_offset: u64,
        end_offset: u64,
        s3_uri: String,
        size_bytes: u64,
        crc32: u32,
    ) -> Result<LogIndex> {
        self.replicate_command(MetadataCommand::RegisterChunkManifest {
            stream_id,
            start_offset,
            end_offset,
            s3_uri,
            size_bytes,
            crc32,
        })
        .await
    }

    /// Replicate a Roaring Bitmap state machine binary snapshot.
    pub async fn replicate_snapshot(
        &self,
        group_id: String,
        base_watermark: u64,
        snapshot_bytes: Vec<u8>,
    ) -> Result<LogIndex> {
        self.replicate_command(MetadataCommand::ReplicateStateSnapshot {
            group_id,
            base_watermark,
            snapshot_bytes,
        })
        .await
    }

    /// Replicate an incremental lease journal delta.
    pub async fn replicate_lease_delta(
        &self,
        group_id: String,
        delta: LeaseDeltaRecord,
    ) -> Result<LogIndex> {
        self.replicate_command(MetadataCommand::ReplicateLeaseDelta { group_id, delta })
            .await
    }

    /// Replicate a sliding base watermark advance.
    pub async fn update_watermark(&self, group_id: String, watermark: u64) -> Result<LogIndex> {
        self.replicate_command(MetadataCommand::UpdateWatermark {
            group_id,
            watermark,
        })
        .await
    }

    /// Read raw engine reference for low-level testing.
    pub fn engine(&self) -> Arc<RwLock<RaftEngine>> {
        self.engine.clone()
    }
}
