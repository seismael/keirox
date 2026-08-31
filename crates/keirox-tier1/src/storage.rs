//! Cloud object storage client interface and thread-safe mock implementation per `KEI-ARC-020`.

use async_trait::async_trait;
use bytes::Bytes;
use keirox_core::error::{KeiroxError, Result};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Cloud object storage client interface (S3 / GCS / MinIO).
#[async_trait]
pub trait ObjectStorageClient: Send + Sync {
    /// Upload an object to the specified URI.
    async fn put_object(&self, uri: &str, data: Bytes) -> Result<()>;

    /// Fetch an object from the specified URI.
    async fn get_object(&self, uri: &str) -> Result<Bytes>;

    /// Delete an object at the specified URI.
    async fn delete_object(&self, uri: &str) -> Result<()>;

    /// Check if object exists.
    async fn exists(&self, uri: &str) -> Result<bool>;

    /// List all object URIs matching prefix.
    async fn list_objects(&self, prefix: &str) -> Result<Vec<String>>;
}

/// In-memory thread-safe mock object storage for testing and simulation.
#[derive(Debug, Clone, Default)]
pub struct MockObjectStorage {
    objects: Arc<RwLock<HashMap<String, Bytes>>>,
}

impl MockObjectStorage {
    /// Create a new mock object storage.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Total objects stored.
    pub fn object_count(&self) -> usize {
        self.objects.read().map(|g| g.len()).unwrap_or(0)
    }

    /// Total bytes stored.
    pub fn total_bytes(&self) -> usize {
        self.objects
            .read()
            .map(|g| g.values().map(|b| b.len()).sum())
            .unwrap_or(0)
    }
}

#[async_trait]
impl ObjectStorageClient for MockObjectStorage {
    async fn put_object(&self, uri: &str, data: Bytes) -> Result<()> {
        let mut guard = self
            .objects
            .write()
            .map_err(|_| KeiroxError::Tier1Storage("Object storage lock poisoned".into()))?;
        guard.insert(uri.to_string(), data);
        Ok(())
    }

    async fn get_object(&self, uri: &str) -> Result<Bytes> {
        let guard = self
            .objects
            .read()
            .map_err(|_| KeiroxError::Tier1Storage("Object storage lock poisoned".into()))?;
        guard
            .get(uri)
            .cloned()
            .ok_or_else(|| KeiroxError::Tier1Storage(format!("Object not found at URI {uri}")))
    }

    async fn delete_object(&self, uri: &str) -> Result<()> {
        let mut guard = self
            .objects
            .write()
            .map_err(|_| KeiroxError::Tier1Storage("Object storage lock poisoned".into()))?;
        guard.remove(uri);
        Ok(())
    }

    async fn exists(&self, uri: &str) -> Result<bool> {
        let guard = self
            .objects
            .read()
            .map_err(|_| KeiroxError::Tier1Storage("Object storage lock poisoned".into()))?;
        Ok(guard.contains_key(uri))
    }

    async fn list_objects(&self, prefix: &str) -> Result<Vec<String>> {
        let guard = self
            .objects
            .read()
            .map_err(|_| KeiroxError::Tier1Storage("Object storage lock poisoned".into()))?;
        let list = guard
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();
        Ok(list)
    }
}

/// S3-compatible object storage implementation for Tier-1 offloading.
pub struct S3ObjectStorage {
    client: aws_sdk_s3::Client,
    bucket: String,
}

impl S3ObjectStorage {
    /// Create a new S3ObjectStorage using the default AWS configuration.
    pub async fn new(bucket: String) -> Self {
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let client = aws_sdk_s3::Client::new(&config);
        Self { client, bucket }
    }
}

#[async_trait]
impl ObjectStorageClient for S3ObjectStorage {
    async fn put_object(&self, uri: &str, data: Bytes) -> Result<()> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(uri)
            .body(data.into())
            .send()
            .await
            .map_err(|e| KeiroxError::Tier1Storage(format!("S3 PutObject error: {}", e)))?;
        Ok(())
    }

    async fn get_object(&self, uri: &str) -> Result<Bytes> {
        let resp = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(uri)
            .send()
            .await
            .map_err(|e| KeiroxError::Tier1Storage(format!("S3 GetObject error: {}", e)))?;

        let data = resp
            .body
            .collect()
            .await
            .map_err(|e| KeiroxError::Tier1Storage(format!("S3 read error: {}", e)))?
            .into_bytes();

        Ok(data)
    }

    async fn delete_object(&self, uri: &str) -> Result<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(uri)
            .send()
            .await
            .map_err(|e| KeiroxError::Tier1Storage(format!("S3 DeleteObject error: {}", e)))?;
        Ok(())
    }

    async fn exists(&self, uri: &str) -> Result<bool> {
        let resp = self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(uri)
            .send()
            .await;
        match resp {
            Ok(_) => Ok(true),
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("NotFound") || err_str.contains("404") {
                    Ok(false)
                } else {
                    Err(KeiroxError::Tier1Storage(format!(
                        "S3 HeadObject error: {}",
                        e
                    )))
                }
            }
        }
    }

    async fn list_objects(&self, prefix: &str) -> Result<Vec<String>> {
        let resp = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(prefix)
            .send()
            .await
            .map_err(|e| KeiroxError::Tier1Storage(format!("S3 ListObjects error: {}", e)))?;

        let mut keys = Vec::new();
        for obj in resp.contents() {
            if let Some(key) = obj.key() {
                keys.push(key.to_string());
            }
        }
        Ok(keys)
    }
}
