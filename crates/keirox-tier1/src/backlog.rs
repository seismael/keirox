//! Elastic NVMe backlog management and progressive backpressure gating per `KEI-ARC-020` and `KEI-ARC-027`.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Queued chunk pending confirmed S3 upload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingChunk {
    /// Stream identifier.
    pub stream_id: [u8; 16],
    /// Segment file path on local NVMe.
    pub segment_path: String,
    /// Start logical offset.
    pub start_offset: u64,
    /// End logical offset.
    pub end_offset: u64,
    /// Payload size in bytes.
    pub size_bytes: u64,
}

/// Elastic backlog manager ensuring local NVMe is never truncated before confirmed S3 upload.
#[derive(Clone)]
pub struct ElasticBacklogManager {
    pending_queue: Arc<RwLock<VecDeque<PendingChunk>>>,
    backlog_bytes: Arc<AtomicU64>,
    backpressure_threshold_bytes: u64,
    backpressure_active: Arc<AtomicBool>,
}

impl ElasticBacklogManager {
    /// Create a new backlog manager with a backpressure byte threshold.
    #[must_use]
    pub fn new(backpressure_threshold_bytes: u64) -> Self {
        Self {
            pending_queue: Arc::new(RwLock::new(VecDeque::new())),
            backlog_bytes: Arc::new(AtomicU64::new(0)),
            backpressure_threshold_bytes,
            backpressure_active: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Enqueue a sealed chunk pending S3 upload.
    pub async fn enqueue_pending(&self, chunk: PendingChunk) {
        let size = chunk.size_bytes;
        let mut queue = self.pending_queue.write().await;
        queue.push_back(chunk);

        let new_backlog = self.backlog_bytes.fetch_add(size, Ordering::SeqCst) + size;
        if new_backlog >= self.backpressure_threshold_bytes {
            self.backpressure_active.store(true, Ordering::SeqCst);
        }
    }

    /// Mark the oldest pending chunk as confirmed uploaded to S3, returning it for safe local truncation.
    pub async fn confirm_upload_completed(&self) -> Option<PendingChunk> {
        let mut queue = self.pending_queue.write().await;
        let chunk = queue.pop_front()?;

        let prev = self
            .backlog_bytes
            .fetch_sub(chunk.size_bytes, Ordering::SeqCst);
        let current = prev.saturating_sub(chunk.size_bytes);

        if current < self.backpressure_threshold_bytes {
            self.backpressure_active.store(false, Ordering::SeqCst);
        }

        Some(chunk)
    }

    /// Total backlog size in bytes.
    #[must_use]
    pub fn backlog_bytes(&self) -> u64 {
        self.backlog_bytes.load(Ordering::Relaxed)
    }

    /// True if progressive backpressure is active due to S3 upload backlog.
    #[must_use]
    pub fn is_backpressure_active(&self) -> bool {
        self.backpressure_active.load(Ordering::Relaxed)
    }
}
