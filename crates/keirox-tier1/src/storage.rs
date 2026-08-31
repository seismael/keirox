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
