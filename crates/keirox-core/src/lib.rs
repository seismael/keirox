//! # Keirox Core
//!
//! Domain models, error taxonomy, and core invariants for the Keirox Polymorphic Event Fabric.
//!
//! ## Core Architectural Invariant
//!
//! **The Golden Invariant (KEI-ARC-010 §3)**:
//! Data is written exactly once to an immutable physical log. Consumption semantics
//! (streaming, queuing, dead-lettering, columnar views) are pure state overlays.

#![deny(missing_docs)]
#![deny(unsafe_code)]

/// Authorization, principal context, and ABAC policy evaluation.
pub mod auth;
/// Diagnostic error codes and operational telemetry taxonomy.
pub mod diagnostics;
/// Concrete error definitions for Keirox operations.
pub mod error;
/// Model definitions for streams, offsets, and records.
pub mod model;
/// Enterprise security, KMS envelope encryption, crypto-shredding, and audit trail.
pub mod security;
/// Domain interfaces and traits.
pub mod traits;

pub use auth::{AbacPolicyEngine, Action, PolicyEffect, PolicyRule, PrincipalContext, Resource};
pub use diagnostics::{DiagnosticCode, DiagnosticEvent, SubsystemTag};
pub use error::{KeiroxError, Result};
pub use model::{Offset, StreamId, TenantId};
pub use security::{
    AuditAction, AuditEvent, AuditRecord, AuditTrailLedger, CryptoShreddingEngine, DekId,
    DestroyedKeyEntry, DestroyedKeyRegistry, EncryptedPayload, ErasureProof, KmsEnvelopeProvider,
};
pub use traits::{StateOverlayEngine, WalEngine};
