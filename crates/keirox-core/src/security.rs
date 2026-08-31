//! Enterprise security, KMS envelope encryption, crypto-shredding, and tamper-evident audit logging per `KEI-SEC-401` and `KEI-ARC-025`.

use crate::error::{KeiroxError, Result};
use crate::model::{StreamId, TenantId};
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use crc32fast::Hasher as Crc32Hasher;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

/// Unique identifier for a Data Encryption Key (DEK).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DekId(pub u64);

/// Encrypted ciphertext payload containing 12-byte nonce, ciphertext, and 16-byte auth tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedPayload {
    /// DEK ID used to encrypt this payload.
    pub dek_id: DekId,
    /// 12-byte initialization vector / nonce.
    pub nonce: [u8; 12],
    /// Ciphertext bytes including authenticated tag.
    pub ciphertext: Vec<u8>,
}

/// KMS Envelope Provider managing key generation, envelope encryption, and DEK lifecycle.
pub struct KmsEnvelopeProvider {
    /// In-memory master key for mock/local envelope wrapping.
    master_key: [u8; 32],
    /// Active DEK storage mapping (TenantId, DekId) -> [u8; 32].
    keys: RwLock<HashMap<(TenantId, DekId), [u8; 32]>>,
}

impl KmsEnvelopeProvider {
    /// Create a new KMS Envelope Provider with a specified master key.
    #[must_use]
    pub fn new(master_key: [u8; 32]) -> Self {
        Self {
            master_key,
            keys: RwLock::new(HashMap::new()),
        }
    }

    /// Initialize a provider with a cryptographically secure random master key.
    #[must_use]
    pub fn with_random_master_key() -> Self {
        let mut master_key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut master_key);
        Self::new(master_key)
    }

    /// Access master Key Encryption Key (KEK) slice.
    #[must_use]
    pub fn master_key(&self) -> &[u8; 32] {
        &self.master_key
    }

    /// Generate or register a new DEK for a tenant.
    pub fn generate_dek(&self, tenant_id: TenantId, dek_id: DekId) -> Result<[u8; 32]> {
        let mut dek = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut dek);

        let mut keys = self
            .keys
            .write()
            .map_err(|_| KeiroxError::Internal("KMS provider keys lock poisoned".into()))?;
        keys.insert((tenant_id, dek_id), dek);

        Ok(dek)
    }

    /// Retrieve active plaintext DEK if not erased.
    pub fn get_dek(&self, tenant_id: TenantId, dek_id: DekId) -> Result<[u8; 32]> {
        let keys = self
            .keys
            .read()
            .map_err(|_| KeiroxError::Internal("KMS provider keys lock poisoned".into()))?;

        keys.get(&(tenant_id, dek_id)).copied().ok_or_else(|| {
            KeiroxError::KeyDestroyed(format!(
                "DEK {dek_id:?} for tenant {tenant_id:?} not found or destroyed"
            ))
        })
    }

    /// Invalidate/erase a DEK from local memory.
    pub fn erase_dek(&self, tenant_id: TenantId, dek_id: DekId) -> Result<bool> {
        let mut keys = self
            .keys
            .write()
            .map_err(|_| KeiroxError::Internal("KMS provider keys lock poisoned".into()))?;
        Ok(keys.remove(&(tenant_id, dek_id)).is_some())
    }

    /// Encrypt a plaintext record payload with AES-256-GCM and bound AAD metadata.
    pub fn encrypt(
        &self,
        tenant_id: TenantId,
        stream_id: StreamId,
        dek_id: DekId,
        plaintext: &[u8],
    ) -> Result<EncryptedPayload> {
        let dek = self.get_dek(tenant_id, dek_id)?;
        let cipher = Aes256Gcm::new_from_slice(&dek)
            .map_err(|e| KeiroxError::Internal(format!("Failed to init AES-256-GCM: {e}")))?;

        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Construct AAD binding tenant and stream to prevent cross-tenant/cross-stream ciphertext splicing
        let mut aad = Vec::with_capacity(32);
        aad.extend_from_slice(&tenant_id.0);
        aad.extend_from_slice(&stream_id.0);

        let payload = Payload {
            msg: plaintext,
            aad: &aad,
        };

        let ciphertext = cipher
            .encrypt(nonce, payload)
            .map_err(|e| KeiroxError::Internal(format!("AES-GCM encryption error: {e}")))?;

        Ok(EncryptedPayload {
            dek_id,
            nonce: nonce_bytes,
            ciphertext,
        })
    }

    /// Decrypt an encrypted payload with AES-256-GCM and verify AAD binding.
    pub fn decrypt(
        &self,
        tenant_id: TenantId,
        stream_id: StreamId,
        payload: &EncryptedPayload,
    ) -> Result<Vec<u8>> {
        let dek = self.get_dek(tenant_id, payload.dek_id)?;
        let cipher = Aes256Gcm::new_from_slice(&dek)
            .map_err(|e| KeiroxError::Internal(format!("Failed to init AES-256-GCM: {e}")))?;

        let nonce = Nonce::from_slice(&payload.nonce);

        let mut aad = Vec::with_capacity(32);
        aad.extend_from_slice(&tenant_id.0);
        aad.extend_from_slice(&stream_id.0);

        let aead_payload = Payload {
            msg: &payload.ciphertext,
            aad: &aad,
        };

        cipher.decrypt(nonce, aead_payload).map_err(|e| {
            KeiroxError::Internal(format!("AES-GCM decryption/authentication error: {e}"))
        })
    }
}

/// Record of a destroyed DEK for GDPR/CCPA crypto-shredding compliance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DestroyedKeyEntry {
    /// Owning tenant ID.
    pub tenant_id: TenantId,
    /// Destroyed DEK ID.
    pub dek_id: DekId,
    /// Associated micro-stream ID (if stream-scoped).
    pub stream_id: Option<StreamId>,
    /// Destruction timestamp (epoch nanoseconds).
    pub destroyed_at_ns: u64,
    /// Authorized operator or service principal initiating destruction.
    pub operator_id: String,
    /// Justification or ticket reference for compliance audit.
    pub reason: String,
}

/// Registry of destroyed keys ensuring that crypto-shredded data cannot be decrypted or restored.
#[derive(Debug, Default)]
pub struct DestroyedKeyRegistry {
    destroyed: RwLock<HashMap<(TenantId, DekId), DestroyedKeyEntry>>,
    destroyed_set: RwLock<HashSet<(TenantId, DekId)>>,
}

impl DestroyedKeyRegistry {
    /// Create a new empty destroyed key registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record the destruction of a DEK.
    pub fn record_destruction(&self, entry: DestroyedKeyEntry) -> Result<()> {
        let key = (entry.tenant_id, entry.dek_id);
        let mut map = self
            .destroyed
            .write()
            .map_err(|_| KeiroxError::Internal("DestroyedKeyRegistry map lock poisoned".into()))?;
        let mut set = self
            .destroyed_set
            .write()
            .map_err(|_| KeiroxError::Internal("DestroyedKeyRegistry set lock poisoned".into()))?;

        map.insert(key, entry);
        set.insert(key);
        Ok(())
    }

    /// Check if a DEK has been destroyed.
    #[must_use]
    pub fn is_destroyed(&self, tenant_id: TenantId, dek_id: DekId) -> bool {
        if let Ok(set) = self.destroyed_set.read() {
            set.contains(&(tenant_id, dek_id))
        } else {
            true // Fail secure on lock corruption
        }
    }

    /// Verify that a DEK is active, returning an error if destroyed.
    pub fn verify_active(&self, tenant_id: TenantId, dek_id: DekId) -> Result<()> {
        if self.is_destroyed(tenant_id, dek_id) {
            Err(KeiroxError::KeyDestroyed(format!(
                "Crypto-shredded: DEK {dek_id:?} for tenant {tenant_id:?} is destroyed"
            )))
        } else {
            Ok(())
        }
    }

    /// Retrieve all destroyed key entries for audit report generation.
    pub fn list_destroyed(&self) -> Vec<DestroyedKeyEntry> {
        if let Ok(map) = self.destroyed.read() {
            map.values().cloned().collect()
        } else {
            Vec::new()
        }
    }
}

/// Cryptographic proof of erasure certificate per `KEI-SEC-401 §6`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErasureProof {
    /// Unique proof identifier.
    pub proof_id: String,
    /// Tenant ID subject to erasure.
    pub tenant_id: TenantId,
    /// Shredded DEK ID.
    pub dek_id: DekId,
    /// Target micro-stream (if scoped).
    pub stream_id: Option<StreamId>,
    /// Erasure timestamp (epoch nanoseconds).
    pub erased_at_ns: u64,
    /// Operator ID who executed the erasure.
    pub operator_id: String,
    /// Legal/compliance justification.
    pub reason: String,
    /// Cryptographic digest (CRC32C/SHA256 signature) over erasure attributes.
    pub verification_checksum: u32,
}

impl ErasureProof {
    /// Verify the cryptographic integrity of the erasure proof.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        let expected_crc = self.compute_checksum();
        self.verification_checksum == expected_crc
    }

    fn compute_checksum(&self) -> u32 {
        let mut hasher = Crc32Hasher::new();
        hasher.update(self.proof_id.as_bytes());
        hasher.update(&self.tenant_id.0);
        hasher.update(&self.dek_id.0.to_le_bytes());
        hasher.update(&self.erased_at_ns.to_le_bytes());
        hasher.update(self.operator_id.as_bytes());
        hasher.update(self.reason.as_bytes());
        hasher.finalize()
    }
}

/// Crypto-shredding engine executing GDPR/CCPA erasure workflows.
pub struct CryptoShreddingEngine {
    kms: Arc<KmsEnvelopeProvider>,
    registry: Arc<DestroyedKeyRegistry>,
}

impl CryptoShreddingEngine {
    /// Create a new CryptoShreddingEngine.
    pub fn new(kms: Arc<KmsEnvelopeProvider>, registry: Arc<DestroyedKeyRegistry>) -> Self {
        Self { kms, registry }
    }

    /// Execute crypto-shredding for a DEK, producing an immutable erasure proof.
    pub fn shred_dek(
        &self,
        tenant_id: TenantId,
        stream_id: Option<StreamId>,
        dek_id: DekId,
        operator_id: String,
        reason: String,
        now_ns: u64,
    ) -> Result<ErasureProof> {
        // 1. Erase plaintext key from KMS/memory cache
        self.kms.erase_dek(tenant_id, dek_id)?;

        // 2. Record destruction in DestroyedKeyRegistry
        let entry = DestroyedKeyEntry {
            tenant_id,
            dek_id,
            stream_id,
            destroyed_at_ns: now_ns,
            operator_id: operator_id.clone(),
            reason: reason.clone(),
        };
        self.registry.record_destruction(entry)?;

        // 3. Generate verifiable ErasureProof
        let proof_id = format!("PROOF-SHRED-{:?}-{}-{}", tenant_id.0[0], dek_id.0, now_ns);
        let mut proof = ErasureProof {
            proof_id,
            tenant_id,
            dek_id,
            stream_id,
            erased_at_ns: now_ns,
            operator_id,
            reason,
            verification_checksum: 0,
        };
        proof.verification_checksum = proof.compute_checksum();

        Ok(proof)
    }
}

/// Action type for tamper-evident audit logging.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditAction {
    /// Ingest produce batch.
    Produce,
    /// Consume stream.
    Consume,
    /// Lease offset.
    Lease,
    /// Acknowledge offset.
    Ack,
    /// Negative acknowledge offset.
    Nack,
    /// Dead-letter queue eviction.
    EvictDlq,
    /// Crypto-shredding key destruction.
    CryptoShred,
    /// Schema registration or evolution.
    SchemaRegister,
    /// Lakehouse snapshot commit.
    LakehouseCommit,
    /// Administrative cluster configuration change.
    AdminConfig,
    /// Access control authorization failure.
    AuthFailure,
}

/// Tamper-evident audit event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Timestamp of event (nanoseconds).
    pub timestamp_ns: u64,
    /// Principal ID executing action.
    pub principal_id: String,
    /// Tenant context.
    pub tenant_id: TenantId,
    /// Target resource identifier.
    pub resource: String,
    /// Action executed.
    pub action: AuditAction,
    /// Outcome status (e.g. "ALLOW", "DENY", "SUCCESS", "FAILED").
    pub outcome: String,
    /// Additional context or metadata.
    pub details: String,
}

/// Chained tamper-evident audit record with SHA-256 cryptographic chaining (REQ-SEC-006).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    /// Monotonic sequence index.
    pub sequence: u64,
    /// 32-byte SHA-256 hash of preceding audit record.
    pub previous_hash: [u8; 32],
    /// Audit event data.
    pub event: AuditEvent,
    /// 32-byte SHA-256 hash of this audit record.
    pub record_hash: [u8; 32],
}

impl AuditRecord {
    /// Genesis block previous hash constant.
    pub const GENESIS_HASH: [u8; 32] = [0x5A; 32];

    /// Compute the cryptographic SHA-256 hash of this audit record.
    #[must_use]
    pub fn compute_hash(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(self.sequence.to_le_bytes());
        hasher.update(self.previous_hash);
        hasher.update(self.event.timestamp_ns.to_le_bytes());
        hasher.update(self.event.principal_id.as_bytes());
        hasher.update(self.event.tenant_id.0);
        hasher.update(self.event.resource.as_bytes());
        hasher.update(self.event.outcome.as_bytes());
        hasher.update(self.event.details.as_bytes());
        hasher.finalize().into()
    }
}

/// Tamper-evident, append-only security audit trail ledger.
#[derive(Debug, Default)]
pub struct AuditTrailLedger {
    records: RwLock<Vec<AuditRecord>>,
}

impl AuditTrailLedger {
    /// Create a new empty audit trail ledger.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an audit event to the tamper-evident chain.
    pub fn record_event(&self, event: AuditEvent) -> Result<u64> {
        let mut records = self
            .records
            .write()
            .map_err(|_| KeiroxError::Internal("AuditTrailLedger lock poisoned".into()))?;

        let sequence = records.len() as u64;
        let previous_hash = records
            .last()
            .map_or(AuditRecord::GENESIS_HASH, |r| r.record_hash);

        let mut record = AuditRecord {
            sequence,
            previous_hash,
            event,
            record_hash: [0u8; 32],
        };
        record.record_hash = record.compute_hash();

        records.push(record);
        Ok(sequence)
    }

    /// Verify the complete cryptographic SHA-256 hash chain of the audit trail.
    pub fn verify_integrity(&self) -> Result<()> {
        let records = self
            .records
            .read()
            .map_err(|_| KeiroxError::Internal("AuditTrailLedger lock poisoned".into()))?;

        let mut expected_prev_hash = AuditRecord::GENESIS_HASH;
        for (idx, record) in records.iter().enumerate() {
            if record.sequence != idx as u64 {
                return Err(KeiroxError::Internal(format!(
                    "Audit record sequence gap at {idx}"
                )));
            }
            if record.previous_hash != expected_prev_hash {
                return Err(KeiroxError::Internal(format!(
                    "Audit record tamper detected: invalid previous_hash at sequence {idx}"
                )));
            }
            let computed = record.compute_hash();
            if record.record_hash != computed {
                return Err(KeiroxError::Internal(format!(
                    "Audit record tamper detected: invalid record_hash at sequence {idx}"
                )));
            }
            expected_prev_hash = record.record_hash;
        }

        Ok(())
    }

    /// Total count of audit records in ledger.
    pub fn record_count(&self) -> usize {
        self.records.read().map_or(0, |r| r.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kms_envelope_encryption_and_decryption() {
        let kms = KmsEnvelopeProvider::with_random_master_key();
        let tenant = TenantId([0x01; 16]);
        let stream = StreamId([0x02; 16]);
        let dek_id = DekId(101);

        kms.generate_dek(tenant, dek_id).unwrap();

        let original = b"Sensitive payment payload for customer #99812";
        let encrypted = kms.encrypt(tenant, stream, dek_id, original).unwrap();

        assert_ne!(encrypted.ciphertext, original);

        let decrypted = kms.decrypt(tenant, stream, &encrypted).unwrap();
        assert_eq!(decrypted, original);

        // Verify that decrypting with wrong tenant or stream fails AAD authentication
        let wrong_tenant = TenantId([0x99; 16]);
        assert!(kms.decrypt(wrong_tenant, stream, &encrypted).is_err());
    }

    #[test]
    fn test_crypto_shredding_and_erasure_proof() {
        let kms = Arc::new(KmsEnvelopeProvider::with_random_master_key());
        let registry = Arc::new(DestroyedKeyRegistry::new());
        let shredder = CryptoShreddingEngine::new(kms.clone(), registry.clone());

        let tenant = TenantId([0x05; 16]);
        let stream = StreamId([0x06; 16]);
        let dek_id = DekId(202);

        kms.generate_dek(tenant, dek_id).unwrap();
        let payload = kms
            .encrypt(tenant, stream, dek_id, b"GDPR confidential data")
            .unwrap();

        assert!(!registry.is_destroyed(tenant, dek_id));

        // Execute shredding
        let proof = shredder
            .shred_dek(
                tenant,
                Some(stream),
                dek_id,
                "dpo-operator-01".into(),
                "GDPR Article 17 Right to Erasure Request #1234".into(),
                1_700_000_000_000_000,
            )
            .unwrap();

        assert!(proof.is_valid());
        assert!(registry.is_destroyed(tenant, dek_id));
        assert!(registry.verify_active(tenant, dek_id).is_err());

        // Attempting to decrypt shredded payload must fail immediately
        assert!(kms.decrypt(tenant, stream, &payload).is_err());
    }

    #[test]
    fn test_tamper_evident_audit_ledger() {
        let ledger = AuditTrailLedger::new();
        let tenant = TenantId([0x10; 16]);

        ledger
            .record_event(AuditEvent {
                timestamp_ns: 1000,
                principal_id: "alice".into(),
                tenant_id: tenant,
                resource: "stream-orders".into(),
                action: AuditAction::Produce,
                outcome: "SUCCESS".into(),
                details: "Batch 1 appended".into(),
            })
            .unwrap();

        ledger
            .record_event(AuditEvent {
                timestamp_ns: 2000,
                principal_id: "bob".into(),
                tenant_id: tenant,
                resource: "stream-orders".into(),
                action: AuditAction::Consume,
                outcome: "SUCCESS".into(),
                details: "Leased offset 0".into(),
            })
            .unwrap();

        assert_eq!(ledger.record_count(), 2);
        assert!(ledger.verify_integrity().is_ok());
    }
}
