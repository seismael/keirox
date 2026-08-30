//! In-memory Schema Registry for polymorphic event validation and versioning per `KEI-ARC-024`.

use crate::compatibility::{CompatibilityLevel, SchemaDefinition, SchemaValidator};
use keirox_core::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Monotonic Schema Version identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SchemaVersion(pub u32);

/// Versioned schema registration entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaEntry {
    /// Schema ID.
    pub id: u32,
    /// Schema subject / stream name.
    pub subject: String,
    /// Monotonic version.
    pub version: SchemaVersion,
    /// Canonical schema definition.
    pub definition: SchemaDefinition,
    /// Active compatibility level for subsequent evolutions.
    pub compatibility_level: CompatibilityLevel,
}

/// Central Schema Registry tracking subjects, versions, and evolution rules.
#[derive(Debug, Default)]
pub struct SchemaRegistry {
    next_id: AtomicU32,
    subjects: RwLock<HashMap<String, Vec<SchemaEntry>>>,
    subject_compatibility: RwLock<HashMap<String, CompatibilityLevel>>,
}

impl SchemaRegistry {
    /// Create a new schema registry instance.
    #[must_use]
    pub fn new() -> Self {
        Self {
            next_id: AtomicU32::new(1),
            subjects: RwLock::new(HashMap::new()),
            subject_compatibility: RwLock::new(HashMap::new()),
        }
    }

    /// Set compatibility level for a subject.
    pub async fn set_compatibility(&self, subject: &str, level: CompatibilityLevel) {
        self.subject_compatibility
            .write()
            .await
            .insert(subject.to_string(), level);
    }

    /// Register a new schema for a subject, validating compatibility against historical versions.
    pub async fn register(
        &self,
        subject: &str,
        definition: SchemaDefinition,
    ) -> Result<SchemaEntry> {
        let mut subjects = self.subjects.write().await;
        let versions = subjects.entry(subject.to_string()).or_default();

        let default_level = CompatibilityLevel::Backward;
        let compatibility_level = self
            .subject_compatibility
            .read()
            .await
            .get(subject)
            .copied()
            .unwrap_or(default_level);

        if let Some(latest) = versions.last() {
            // Check if identical to latest -> return existing
            if latest.definition == definition {
                return Ok(latest.clone());
            }

            // Validate schema evolution against compatibility rules
            SchemaValidator::validate_evolution(
                &latest.definition,
                &definition,
                compatibility_level,
            )?;
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let version = SchemaVersion(versions.len() as u32 + 1);

        let entry = SchemaEntry {
            id,
            subject: subject.to_string(),
            version,
            definition,
            compatibility_level,
        };

        versions.push(entry.clone());
        Ok(entry)
    }

    /// Look up latest schema for a subject.
    pub async fn get_latest(&self, subject: &str) -> Option<SchemaEntry> {
        let subjects = self.subjects.read().await;
        subjects.get(subject).and_then(|v| v.last().cloned())
    }

    /// Look up specific schema version for a subject.
    pub async fn get_version(&self, subject: &str, version: SchemaVersion) -> Option<SchemaEntry> {
        let subjects = self.subjects.read().await;
        subjects
            .get(subject)
            .and_then(|v| v.iter().find(|e| e.version == version).cloned())
    }
}

/// Shared reference to Schema Registry.
pub type SharedSchemaRegistry = Arc<SchemaRegistry>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compatibility::FieldType;

    #[tokio::test]
    async fn test_schema_registration_and_evolution_governance() {
        let registry = SchemaRegistry::new();

        let mut v1_def = SchemaDefinition::new();
        v1_def.add_field("user_id", FieldType::Int64, true);
        v1_def.add_field("amount", FieldType::Float64, false);

        let e1 = registry.register("orders", v1_def.clone()).await.unwrap();
        assert_eq!(e1.version, SchemaVersion(1));

        let mut v2_def = v1_def.clone();
        v2_def.add_field("notes", FieldType::Utf8, false); // Valid optional addition

        let e2 = registry.register("orders", v2_def).await.unwrap();
        assert_eq!(e2.version, SchemaVersion(2));

        let latest = registry.get_latest("orders").await.unwrap();
        assert_eq!(latest.version, SchemaVersion(2));
    }
}
