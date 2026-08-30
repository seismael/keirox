//! Resilient cloud object chunk uploader with exponential backoff and jitter per `KEI-ARC-020`.

use crate::storage::ObjectStorageClient;
use bytes::Bytes;
use keirox_core::error::{KeiroxError, Result};
use rand::Rng;
use std::sync::Arc;
use std::time::Duration;

/// Uploader executing resilient chunk uploads to Tier-1 object storage.
#[derive(Clone)]
pub struct MultipartUploader {
    storage: Arc<dyn ObjectStorageClient>,
    max_retries: usize,
    initial_backoff: Duration,
    max_backoff: Duration,
}

impl MultipartUploader {
    /// Create a new multipart uploader with default retry policy.
    pub fn new(storage: Arc<dyn ObjectStorageClient>) -> Self {
        Self {
            storage,
            max_retries: 5,
            initial_backoff: Duration::from_millis(50),
            max_backoff: Duration::from_secs(5),
        }
    }

    /// Upload chunk data with exponential backoff and full jitter.
    pub async fn upload_chunk(&self, uri: &str, data: Bytes) -> Result<()> {
        let mut attempts = 0;
        let mut current_backoff = self.initial_backoff;

        loop {
            attempts += 1;
            match self.storage.put_object(uri, data.clone()).await {
                Ok(()) => return Ok(()),
                Err(err) => {
                    if attempts > self.max_retries {
                        return Err(KeiroxError::Tier1Storage(format!(
                            "Failed to upload chunk to {uri} after {attempts} attempts: {err}"
                        )));
                    }

                    // Full jitter backoff: random duration between 0 and current_backoff
                    let jitter_ms =
                        rand::thread_rng().gen_range(0..=current_backoff.as_millis() as u64);
                    tokio::time::sleep(Duration::from_millis(jitter_ms)).await;

                    current_backoff = (current_backoff * 2).min(self.max_backoff);
                }
            }
        }
    }
}
