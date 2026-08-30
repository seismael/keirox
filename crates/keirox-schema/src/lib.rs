//! # Keirox Schema Registry & Adaptive Shredding Governance
//!
//! Provides schema evolution enforcement, Avro/JSON schema registration, and
//! adaptive 64-field columnar shredding governance per `KEI-ARC-024` and `KEI-DES-033`.

#![deny(missing_docs)]

pub mod compatibility;
pub mod registry;
pub mod shredding_policy;

pub use compatibility::{CompatibilityLevel, FieldType, SchemaDefinition, SchemaValidator};
pub use registry::{SchemaEntry, SchemaRegistry, SchemaVersion, SharedSchemaRegistry};
pub use shredding_policy::{
    AdaptiveShreddingPolicy, MAX_SHREDDED_COLUMNS, UNSTRUCTURED_PAYLOAD_COLUMN,
};
