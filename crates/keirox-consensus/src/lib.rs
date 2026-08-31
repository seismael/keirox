//! Multi-Raft distributed consensus and quorum replication engine for Keirox per `KEI-ARC-022`.

#![deny(unsafe_code)]

pub mod data_plane;
pub mod engine;
/// Hybrid Logical Clock for WAN causal ordering.
pub mod hlc;
pub mod log;
pub mod membership;
pub mod metadata_plane;
/// Multi-Region Mode A asynchronous WAN replication.
pub mod multi_region;
pub mod rpc;
pub mod transport;
pub mod types;

pub use data_plane::DataPlaneRaftGroup;
pub use engine::{HardState, RaftEngine};
pub use hlc::{HlcTimestamp, HybridLogicalClock};
pub use log::{LeaseDeltaRecord, LogPayload, MetadataCommand, RaftLog, RaftLogEntry};
pub use membership::{MembershipManager, NodeStatus};
pub use metadata_plane::MetadataRaftGroup;
pub use multi_region::{
    MultiRegionReplicator, RegionEpoch, RegionId, RegionRole, ReplicationBatch,
};
pub use rpc::{
    AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest, InstallSnapshotResponse,
    VoteRequest, VoteResponse,
};
pub use transport::{ChannelMesh, MeshTransport, RaftMessage, RaftTransport};
pub use types::{ClusterConfig, LogIndex, NodeId, PeerEndpoint, ReplicaRole, Term};
