//! 3-node distributed cluster coordinator and fault injection runtime for Phase 2 validation.

use crate::engine::SingleNodeRuntime;
use async_trait::async_trait;
use keirox_consensus::{ChannelMesh, ClusterConfig, DataPlaneRaftGroup, MetadataRaftGroup, NodeId};
use keirox_coordinator::{
    ConsistentHashRing, CoordinatorEpoch, CoordinatorNode, EpochFencedToken, ShardId,
};
use keirox_core::error::{KeiroxError, Result};
use keirox_core::model::{Offset, StreamId, TenantId};
use keirox_tier1::{
    ChunkManifestEntry, HashPrefixPartitioner, ManifestRegistry, MockObjectStorage,
    MultipartUploader,
};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Individual cluster node runtime hosting storage, consensus, coordinator, and S3 uploader.
pub struct ClusterNode {
    /// Node identifier.
    pub node_id: NodeId,
    /// Local single node storage and state runtime.
    pub local_runtime: SingleNodeRuntime,
    /// Data plane Raft consensus group.
    pub data_raft: DataPlaneRaftGroup,
    /// Metadata Raft consensus group.
    pub metadata_raft: MetadataRaftGroup,
    /// Coordinator node for assigned shards.
    pub coordinator: CoordinatorNode,
    /// Manifest registry.
    pub manifest_registry: Arc<RwLock<ManifestRegistry>>,
    /// Active status.
    pub is_alive: bool,
}

/// 3-Node distributed cluster runtime orchestrator.
pub struct ClusterRuntime {
    mesh: ChannelMesh,
    nodes: HashMap<NodeId, ClusterNode>,
    storage: Arc<MockObjectStorage>,
    partitioner: HashPrefixPartitioner,
    leader_node_id: NodeId,
}

impl ClusterRuntime {
    /// Initialize a 3-node distributed cluster runtime with a shared in-memory network mesh and mock S3.
    pub fn init_three_node(base_dir: &Path) -> Result<Self> {
        let mesh = ChannelMesh::new();
        let storage = Arc::new(MockObjectStorage::new());
        let partitioner = HashPrefixPartitioner::new("keirox-lakehouse-test");

        let node_ids = [NodeId(1), NodeId(2), NodeId(3)];
        let mut nodes = HashMap::new();

        let mut hash_ring = ConsistentHashRing::new(64);
        for &id in &node_ids {
            hash_ring.add_node(id);
        }

        for &id in &node_ids {
            let node_dir = base_dir.join(format!("node_{}", id.0));
            std::fs::create_dir_all(&node_dir)?;

            let local_runtime = SingleNodeRuntime::init(&node_dir)?;

            let peer_ids: Vec<u64> = node_ids
                .iter()
                .filter(|&&other| other != id)
                .map(|p| p.0)
                .collect();

            let config = ClusterConfig::three_node(id, [peer_ids[0], peer_ids[1]]);
            let transport = Arc::new(mesh.create_transport(id));

            let mut rx = mesh.register(id, 256);

            let data_raft = DataPlaneRaftGroup::new(config.clone(), transport.clone());
            let metadata_raft = MetadataRaftGroup::new(config, transport);
            let coordinator = CoordinatorNode::new(id, hash_ring.clone());
            let manifest_registry = Arc::new(RwLock::new(ManifestRegistry::new()));

            let engine_handle = data_raft.engine();
            tokio::spawn(async move {
                while let Some(msg) = rx.recv().await {
                    match msg {
                        keirox_consensus::RaftMessage::Vote(req, reply) => {
                            let resp = {
                                let mut engine = engine_handle.write().await;
                                engine.handle_vote_request(req)
                            };
                            let _ = reply.send(Ok(resp));
                        }
                        keirox_consensus::RaftMessage::AppendEntries(req, reply) => {
                            let resp = {
                                let mut engine = engine_handle.write().await;
                                engine.handle_append_entries(req)
                            };
                            let _ = reply.send(Ok(resp));
                        }
                        keirox_consensus::RaftMessage::InstallSnapshot(_req, reply) => {
                            let _ = reply.send(Ok(keirox_consensus::InstallSnapshotResponse {
                                term: keirox_consensus::Term(0),
                                success: true,
                            }));
                        }
                    }
                }
            });

            nodes.insert(
                id,
                ClusterNode {
                    node_id: id,
                    local_runtime,
                    data_raft,
                    metadata_raft,
                    coordinator,
                    manifest_registry,
                    is_alive: true,
                },
            );
        }

        Ok(Self {
            mesh,
            nodes,
            storage,
            partitioner,
            leader_node_id: NodeId(1),
        })
    }

    /// Form cluster quorum and elect Node 1 as initial leader.
    pub async fn form_cluster(&mut self) -> Result<()> {
        let leader_node = self
            .nodes
            .get(&self.leader_node_id)
            .ok_or_else(|| KeiroxError::Internal("Leader node not found".into()))?;

        // Node 1 campaigns and wins election
        let won = leader_node.data_raft.campaign().await?;
        if !won {
            return Err(KeiroxError::Consensus(
                "Failed to elect Node 1 as cluster leader".into(),
            ));
        }

        // Host shards across all 3 nodes
        for node in self.nodes.values() {
            for s in 0..1024 {
                node.coordinator
                    .host_shard(ShardId(s), CoordinatorEpoch(1))
                    .await;
            }
        }

        Ok(())
    }

    /// Produce records with synchronous 3-node quorum replication before local commit.
    pub async fn produce_cluster(
        &mut self,
        _tenant_id: TenantId,
        stream_id: StreamId,
        records: Vec<Vec<u8>>,
    ) -> Result<Offset> {
        let leader = self
            .nodes
            .get_mut(&self.leader_node_id)
            .ok_or_else(|| KeiroxError::Internal("Leader node offline".into()))?;

        if !leader.is_alive {
            return Err(KeiroxError::QuorumUnavailable(
                "Leader node is offline".into(),
            ));
        }

        // Serialize record payload for quorum replication
        let batch_payload = serde_json::to_vec(&records)
            .map_err(|e| KeiroxError::Internal(format!("Serialization error: {e}")))?;

        // Synchronously replicate batch to 3-node quorum
        let _quorum_idx = leader.data_raft.append_batch_quorum(batch_payload).await?;

        // Once quorum acknowledges, commit to leader's local NVMe WAL
        let resp =
            leader
                .local_runtime
                .produce(stream_id, keirox_api::AckMode::Durable, &records)?;

        Ok(resp.base_offset)
    }

    /// Lease an offset for a consumer group using consistent hashing and epoch fencing.
    pub async fn lease_cluster(
        &self,
        group_id: &str,
        offset: u64,
        ttl_ms: u64,
        now_us: u64,
    ) -> Result<EpochFencedToken> {
        // Resolve designated node from consistent hash ring
        let node_id = {
            let any_node = self.nodes.values().next().unwrap();
            let (_, target) = any_node
                .coordinator
                .resolve_shard_for_group(group_id)
                .await
                .ok_or_else(|| KeiroxError::Internal("Hash ring resolution failed".into()))?;
            target
        };

        let target_node = self.nodes.get(&node_id).ok_or_else(|| {
            KeiroxError::Consensus(format!("Coordinator node {node_id} not found"))
        })?;

        if !target_node.is_alive {
            return Err(KeiroxError::Consensus(format!(
                "Coordinator node {node_id} is offline; failover required"
            )));
        }

        target_node
            .coordinator
            .lease_offset(group_id, offset, ttl_ms, now_us)
            .await
    }

    /// Acknowledge a leased offset with epoch fencing verification.
    pub async fn ack_cluster(&self, group_id: &str, token: EpochFencedToken) -> Result<()> {
        let node_id = {
            let any_node = self.nodes.values().next().unwrap();
            let (_, target) = any_node
                .coordinator
                .resolve_shard_for_group(group_id)
                .await
                .ok_or_else(|| KeiroxError::Internal("Hash ring resolution failed".into()))?;
            target
        };

        let target_node = self.nodes.get(&node_id).ok_or_else(|| {
            KeiroxError::Consensus(format!("Coordinator node {node_id} not found"))
        })?;

        if !target_node.is_alive {
            return Err(KeiroxError::Consensus(format!(
                "Coordinator node {node_id} is offline"
            )));
        }

        target_node.coordinator.ack_offset(group_id, token).await
    }

    /// Negative-acknowledge (NACK) an offset and requeue.
    pub async fn nack_cluster(&self, group_id: &str, token: EpochFencedToken) -> Result<()> {
        let node_id = {
            let any_node = self.nodes.values().next().unwrap();
            let (_, target) = any_node
                .coordinator
                .resolve_shard_for_group(group_id)
                .await
                .ok_or_else(|| KeiroxError::Internal("Hash ring resolution failed".into()))?;
            target
        };

        let target_node = self.nodes.get(&node_id).ok_or_else(|| {
            KeiroxError::Consensus(format!("Coordinator node {node_id} not found"))
        })?;

        target_node.coordinator.nack_offset(group_id, token).await
    }

    /// Seal segment chunk, upload to S3 with hash-prefix key, and register manifest in Metadata Raft.
    pub async fn seal_and_stream_tier1(
        &self,
        tenant_id: TenantId,
        stream_id: StreamId,
        start_offset: u64,
        end_offset: u64,
        chunk_data: bytes::Bytes,
    ) -> Result<String> {
        let size_bytes = chunk_data.len() as u64;
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&chunk_data);
        let crc32 = hasher.finalize();

        let s3_uri =
            self.partitioner
                .format_chunk_uri(&tenant_id.0, &stream_id.0, start_offset, end_offset);

        let uploader = MultipartUploader::new(self.storage.clone());
        uploader.upload_chunk(&s3_uri, chunk_data).await?;

        let entry = ChunkManifestEntry {
            stream_id: stream_id.0,
            start_offset,
            end_offset,
            s3_uri: s3_uri.clone(),
            size_bytes,
            crc32,
            sealed_at_ns: 1_700_000_000_000_000_000,
        };

        for node in self.nodes.values() {
            node.manifest_registry.write().await.register(entry.clone());
        }

        Ok(s3_uri)
    }

    /// Crash a node simulating `kill -9` or sudden power failure.
    pub fn crash_node(&mut self, node_id: NodeId) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.is_alive = false;
            self.mesh.isolate_node(node_id);
        }
    }

    /// Recover and replace a failed node in <5 seconds.
    pub async fn recover_and_replace_node(
        &mut self,
        new_node_id: NodeId,
        failed_node_id: NodeId,
        base_dir: &Path,
    ) -> Result<()> {
        let new_node_dir = base_dir.join(format!("node_{}", new_node_id.0));
        std::fs::create_dir_all(&new_node_dir)?;

        let local_runtime = SingleNodeRuntime::init(&new_node_dir)?;

        let surviving_peers: Vec<u64> = self
            .nodes
            .keys()
            .filter(|&&id| id != failed_node_id)
            .map(|n| n.0)
            .collect();

        let config = ClusterConfig::three_node(
            new_node_id,
            [
                surviving_peers[0],
                surviving_peers
                    .get(1)
                    .copied()
                    .unwrap_or(surviving_peers[0]),
            ],
        );
        let transport = Arc::new(self.mesh.create_transport(new_node_id));

        let mut rx = self.mesh.register(new_node_id, 256);

        let data_raft = DataPlaneRaftGroup::new(config.clone(), transport.clone());
        let metadata_raft = MetadataRaftGroup::new(config, transport);

        let engine_handle = data_raft.engine();
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    keirox_consensus::RaftMessage::Vote(req, reply) => {
                        let resp = {
                            let mut engine = engine_handle.write().await;
                            engine.handle_vote_request(req)
                        };
                        let _ = reply.send(Ok(resp));
                    }
                    keirox_consensus::RaftMessage::AppendEntries(req, reply) => {
                        let resp = {
                            let mut engine = engine_handle.write().await;
                            engine.handle_append_entries(req)
                        };
                        let _ = reply.send(Ok(resp));
                    }
                    keirox_consensus::RaftMessage::InstallSnapshot(_req, reply) => {
                        let _ = reply.send(Ok(keirox_consensus::InstallSnapshotResponse {
                            term: keirox_consensus::Term(0),
                            success: true,
                        }));
                    }
                }
            }
        });

        let mut hash_ring = ConsistentHashRing::new(64);
        for &id in self.nodes.keys() {
            if id != failed_node_id {
                hash_ring.add_node(id);
            }
        }
        hash_ring.add_node(new_node_id);

        let coordinator = CoordinatorNode::new(new_node_id, hash_ring);

        // Reconstruct manifest registry from healthy peer
        let peer = self
            .nodes
            .values()
            .find(|n| n.is_alive && n.node_id != failed_node_id)
            .ok_or_else(|| {
                KeiroxError::Consensus("No healthy peer available for recovery".into())
            })?;

        let manifest_clone = {
            let reg = peer.manifest_registry.read().await;
            reg.clone()
        };

        self.nodes.remove(&failed_node_id);
        self.nodes.insert(
            new_node_id,
            ClusterNode {
                node_id: new_node_id,
                local_runtime,
                data_raft,
                metadata_raft,
                coordinator,
                manifest_registry: Arc::new(RwLock::new(manifest_clone)),
                is_alive: true,
            },
        );

        self.mesh.heal();

        Ok(())
    }

    /// Partition the cluster into two network sets.
    pub fn partition_cluster(&self, group_a: &[NodeId], group_b: &[NodeId]) {
        self.mesh.partition(group_a, group_b);
    }

    /// Heal all cluster partitions.
    pub fn heal_partitions(&self) {
        self.mesh.heal();
    }
}

/// Thread-safe cluster adapter implementing `ClusterIngress` and `ClusterClientTransport`.
#[derive(Clone)]
pub struct SharedClusterHandle {
    inner: Arc<RwLock<ClusterRuntime>>,
}

impl SharedClusterHandle {
    /// Create a new shared cluster handle.
    pub fn new(runtime: ClusterRuntime) -> Self {
        Self {
            inner: Arc::new(RwLock::new(runtime)),
        }
    }

    /// Inner cluster runtime handle.
    pub fn inner(&self) -> Arc<RwLock<ClusterRuntime>> {
        self.inner.clone()
    }
}

#[async_trait]
impl keirox_gateway::ClusterIngress for SharedClusterHandle {
    async fn produce(
        &self,
        tenant_id: TenantId,
        stream_id: StreamId,
        records: Vec<Vec<u8>>,
    ) -> Result<u64> {
        let mut guard = self.inner.write().await;
        guard.produce_cluster(tenant_id, stream_id, records).await
    }
}

#[async_trait]
impl keirox_sdk::ClusterClientTransport for SharedClusterHandle {
    async fn produce(
        &self,
        tenant_id: TenantId,
        stream_id: StreamId,
        records: Vec<Vec<u8>>,
    ) -> Result<u64> {
        let mut guard = self.inner.write().await;
        guard.produce_cluster(tenant_id, stream_id, records).await
    }

    async fn lease(
        &self,
        group_id: &str,
        offset: u64,
        ttl_ms: u64,
        now_us: u64,
    ) -> Result<EpochFencedToken> {
        let guard = self.inner.read().await;
        guard.lease_cluster(group_id, offset, ttl_ms, now_us).await
    }

    async fn ack(&self, group_id: &str, token: EpochFencedToken) -> Result<()> {
        let guard = self.inner.read().await;
        guard.ack_cluster(group_id, token).await
    }

    async fn nack(&self, group_id: &str, token: EpochFencedToken) -> Result<()> {
        let guard = self.inner.read().await;
        guard.nack_cluster(group_id, token).await
    }
}
