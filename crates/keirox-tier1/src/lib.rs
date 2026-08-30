//! Tier-1 continuous S3/GCS cloud object storage streaming, multipart uploader, and manifest registry for Keirox per `KEI-ARC-020`.

pub mod backlog;
pub mod manifest;
pub mod partitioner;
pub mod storage;
pub mod uploader;

pub use backlog::{ElasticBacklogManager, PendingChunk};
pub use manifest::{ChunkManifestEntry, ManifestRegistry};
pub use partitioner::HashPrefixPartitioner;
pub use storage::{MockObjectStorage, ObjectStorageClient};
pub use uploader::MultipartUploader;
