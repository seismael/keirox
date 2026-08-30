# KEI-DES-036 — Encryption, Key Management & Crypto-Shredding Specification

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-DES-036 |
| Title | Encryption, Key Management & Crypto-Shredding Specification |
| Version | 1.0 |
| Level | **L3 — Detailed Design Specification** |
| Subsystem Covered | Security Plane — Encryption & Key Lifecycle |
| Status | Approved for Engineering |
| Classification | Internal / Engineering Confidential |
| Owner | Security Architect / Security Lead |
| Required Reviewers | Chief Architect, Principal Engineer (Storage), Compliance Lead, DR Owner |
| Depends On | KEI-ARC-025 (Security Architecture), KEI-ARC-026 (Multi-Region/DR), KEI-DES-030 (WAL Binary Format), KEI-DES-034 (Iceberg Committer) |
| Consumed By | Storage engine, state plane, lakehouse committer, protocol gateways, multi-region replicator, backup/restore executor, compliance auditors |
| Keywords | MUST, MUST NOT, SHOULD, SHOULD NOT, MAY per RFC 2119 |

---

## 2. Purpose, Scope, and Audience

### 2.1 Purpose

This document specifies the **complete encryption, key management, and cryptographic erasure subsystem** of the Polymorphic Event Fabric. It defines:

- The envelope encryption architecture.
- The key hierarchy and lifecycle.
- KMS provider abstraction and adapter contracts.
- Data Encryption Key (DEK) creation, caching, rotation, and destruction.
- Crypto-shredding workflows for GDPR/CCPA compliance.
- Interaction with backups, cross-region replicas, and lakehouse metadata.
- Security failure behavior and fail-secure requirements.

This document implements:

- ADR-050: Envelope Encryption with KMS-Managed DEKs.
- ADR-051: GDPR/CCPA Deletion via Crypto-Shredding.
- KEI-ARC-025 Security Architecture requirements.

### 2.2 Scope

**In scope:**

- Envelope encryption key hierarchy.
- KMS adapter abstraction.
- DEK lifecycle management.
- WAL batch encryption format.
- Parquet file encryption format.
- State plane and manifest encryption.
- Key rotation procedures.
- Crypto-shredding erasure workflow.
- Destroyed-key registry.
- Backup interaction with encryption.
- Cross-region key replication.
- Security failure modes and fail-secure behavior.

**Out of scope:**

- Authentication and authorization mechanics — owned by KEI-ARC-025.
- TLS/mTLS transport encryption configuration — owned by KEI-ARC-025.
- Audit logging mechanics — owned by KEI-ARC-025.
- WAL binary structure — owned by KEI-DES-030.
- Iceberg commit mechanics — owned by KEI-DES-034.

### 2.3 Audience

- Security implementation engineers.
- Storage engine engineers implementing encryption hooks.
- Compliance and privacy engineers.
- DR and backup engineers.
- KMS integration engineers.
- Penetration testers and security auditors.

---

## 3. Design Principles

| ID | Principle | Rationale |
|---|---|---|
| EN-1 | **Encryption at rest is mandatory for all customer data.** | No plaintext customer payloads may persist on NVMe or object storage. |
| EN-2 | **Envelope encryption is the only supported model.** | Direct root-key encryption does not scale; envelope encryption enables per-stream erasure. |
| EN-3 | **Key destruction is logical erasure.** | Destroying a DEK renders all ciphertext encrypted under it cryptographically unrecoverable. |
| EN-4 | **Fail secure.** | Encryption failures MUST deny access rather than fall back to plaintext. |
| EN-5 | **Keys are never persisted in plaintext.** | DEKs exist only in process memory and are wrapped by KEKs at rest. |
| EN-6 | **Crypto-shredding is auditable.** | Every key destruction event MUST produce tamper-evident audit evidence. |
| EN-7 | **Backups respect erasure.** | Restoring a backup MUST NOT resurrect destroyed data. |
| EN-8 | **Key operations are rate-limited and cached.** | KMS API calls are expensive; DEK caching prevents hot-path latency. |

---

## 4. Envelope Encryption Key Hierarchy

### 4.1 Hierarchy Model

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         EXTERNAL KMS / HSM                              │
│                    (Root Key — never leaves KMS)                       │
└───────────────────────────────┬─────────────────────────────────────────┘
                                │ wraps/unwraps
                                ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    TENANT KEY ENCRYPTION KEY (KEK)                     │
│                    One per tenant; rotatable                           │
│                    Wrapped by Root Key                                 │
└───────────────────────────────┬─────────────────────────────────────────┘
                                │ wraps/unwraps
                                ▼
┌─────────────────────────────────────────────────────────────────────────┐
│              DATA ENCRYPTION KEYS (DEKs)                                │
│                                                                         │
│   ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐    │
│   │ Stream DEK       │  │ Stream-Batch DEK │  │ Manifest DEK     │    │
│   │ (per-stream)     │  │ (per-tenant/day) │  │ (per-tenant)     │    │
│   └──────────────────┘  └──────────────────┘  └──────────────────┘    │
│                                                                         │
│   Encrypts: WAL batches, Parquet files, manifests, state snapshots     │
└─────────────────────────────────────────────────────────────────────────┘
```

### 4.2 Key Types

| Key Type | Scope | Lifetime | Rotation Policy | Destruction Trigger |
|---|---|---|---|---|
| Root Key | KMS trust anchor | Long-lived | Per KMS provider policy | Never destroyed except KMS decommission. |
| Tenant KEK | One per tenant | Long-lived; rotatable | Annual default; on-demand | Tenant deletion. |
| Stream DEK | One per regulated/high-isolation stream | Medium-lived | On key rotation or erasure | Stream erasure. |
| Stream-Batch DEK | One per tenant/date/bucket | Short-lived | Daily default | Batch erasure or tenant deletion. |
| Manifest DEK | One per tenant | Medium-lived | On key rotation | Tenant deletion. |

### 4.3 DEK Granularity Decision

| Granularity | Use Case | Pros | Cons |
|---|---|---|---|
| Per-stream DEK | Regulated streams, high-isolation tenants, GDPR-sensitive streams | Precise stream-level erasure | Higher KMS API cost; more keys to manage |
| Stream-Batch DEK | High-cardinality streams, standard tenants | Lower KMS cost; fewer keys | Erasure granularity is batch-level, not stream-level |

**Normative rules:**

- Default DEK granularity MUST be **Stream-Batch DEK** for standard tenants.
- Per-stream DEK MUST be used when:
  - Tenant compliance policy requires stream-level erasure.
  - Stream is tagged with `PII`, `PCI`, or `CONFIDENTIAL` sensitivity.
  - Tenant explicitly enables per-stream encryption.
- The DEK granularity MUST be recorded in the stream schema policy (KEI-DES-033).

---

## 5. KMS Adapter Abstraction

### 5.1 Supported KMS Providers

| Provider | Support Level | Authentication |
|---|---|---|
| AWS KMS | Full | IAM Role / STS |
| GCP Cloud KMS | Full | Workload Identity |
| Azure Key Vault | Full | Managed Identity |
| HashiCorp Vault | Full | AppRole / Kubernetes Auth |
| FIPS-certified HSM | Conditional | Provider-specific |

### 5.2 KMS Adapter Interface

```rust
pub trait KmsAdapter: Send + Sync {
    /// Generate a new DEK, returning plaintext DEK and wrapped DEK.
    fn generate_dek(&self, kek_id: &str, context: &EncryptionContext) -> Result<GenerateDekResponse, KmsError>;

    /// Unwrap (decrypt) a wrapped DEK using the KEK.
    fn unwrap_dek(&self, wrapped_dek: &[u8], kek_id: &str, context: &EncryptionContext) -> Result<PlaintextDek, KmsError>;

    /// Destroy a key permanently. Irreversible.
    fn destroy_key(&self, key_id: &str) -> Result<DestructionReceipt, KmsError>;

    /// Schedule key deletion with a waiting period (provider-specific).
    fn schedule_key_deletion(&self, key_id: &str, waiting_period_days: u32) -> Result<DeletionSchedule, KmsError>;

    /// Rotate a KEK. Old KEK version remains available for decryption.
    fn rotate_kek(&self, kek_id: &str) -> Result<RotationResult, KmsError>;

    /// Get key metadata without exposing key material.
    fn describe_key(&self, key_id: &str) -> Result<KeyMetadata, KmsError>;

    /// Check if a key is destroyed or scheduled for deletion.
    fn is_key_destroyed(&self, key_id: &str) -> Result<bool, KmsError>;
}
```

### 5.3 Encryption Context

Every KMS operation MUST include an encryption context for audit and binding:

```rust
pub struct EncryptionContext {
    pub tenant_id: u64,
    pub stream_id: Option<u128>,
    pub purpose: KeyPurpose,
    pub created_at_ms: u64,
    pub region: String,
}

pub enum KeyPurpose {
    WalEncryption,
    ParquetEncryption,
    ManifestEncryption,
    StateSnapshotEncryption,
    BackupEncryption,
}
```

**Normative rule:** The encryption context MUST be validated on unwrap. If the context does not match the expected values, decryption MUST fail.

### 5.4 KMS Failure Behavior

| KMS State | Behavior |
|---|---|
| KMS available | Normal operation. |
| KMS temporarily unavailable | Use cached DEKs within TTL; queue new DEK requests. |
| KMS unavailable and no cached DEK | New writes requiring new DEKs MUST fail closed. Reads with cached DEKs MAY continue. |
| KMS permanently unavailable | Alert critical; system enters degraded mode; no new data encrypted. |

**Normative rule:** The system MUST NEVER fall back to plaintext when KMS is unavailable. Fail closed is mandatory.

---

## 6. Data Encryption Key (DEK) Lifecycle

### 6.1 DEK Creation

```text
1. ELT/Storage engine requests new DEK for (tenant, stream, purpose)
2. KMS Adapter generates DEK via KMS GenerateDataKey API
3. KMS returns:
   a. Plaintext DEK (ephemeral, in-memory only)
   b. Wrapped DEK (encrypted under KEK)
4. System stores wrapped DEK in metadata
5. System caches plaintext DEK in DEK Cache with TTL
6. System NEVER persists plaintext DEK to disk
```

### 6.2 DEK Cache

```rust
pub struct DekCache {
    pub entries: LruCache<DekCacheKey, CachedDek>,
    pub max_entries: usize,
    pub default_ttl_ms: u64,
}

pub struct DekCacheKey {
    pub tenant_id: u64,
    pub stream_id: Option<u128>,
    pub dek_id: u64,
    pub purpose: KeyPurpose,
}

pub struct CachedDek {
    pub plaintext_dek: Zeroizing<[u8; 32]>,  // Zeroized on drop
    pub wrapped_dek: Vec<u8>,
    pub expires_at_ms: u64,
    pub last_used_ms: u64,
}
```

**Normative rules:**

- DEK cache TTL default: 300 seconds (5 minutes).
- DEK cache entries MUST be zeroized on eviction.
- DEK cache MUST NOT be serialized to disk or included in snapshots.
- DEK cache miss MUST trigger KMS unwrap with retry.
- DEK cache size MUST be bounded (default: 10,000 entries).

### 6.3 DEK Rotation

DEK rotation creates a new DEK for future writes while preserving the old DEK for reading historical data.

```text
1. Generate new DEK for (tenant, stream, purpose)
2. Update stream/segment metadata to reference new DEK ID
3. Old DEK remains available for decryption of historical data
4. New writes use new DEK
5. Historical data is NOT re-encrypted (unless compaction rewrites it)
```

**Normative rules:**

- DEK rotation MUST NOT require rewriting all historical data.
- Each encrypted artifact MUST record which DEK version was used.
- Old DEKs MUST remain available until all data encrypted under them is either re-encrypted or erased.

### 6.4 DEK Destruction

DEK destruction is the core mechanism for crypto-shredding.

```text
1. Erasure request received and authorized
2. System identifies all DEKs associated with target (stream/tenant/batch)
3. For each DEK:
   a. Remove from DEK Cache
   b. Command KMS to destroy key material
   c. Record destruction receipt
   d. Add DEK ID to Destroyed-Key Registry
4. Emit audit event
5. Propagate destruction to all regions
```

**Normative rules:**

- DEK destruction MUST be irreversible.
- DEK destruction MUST complete before erasure is reported as successful.
- DEK destruction MUST be recorded in the Destroyed-Key Registry.
- DEK destruction MUST propagate to all regions before erasure is considered complete.

---

## 7. WAL Batch Encryption Format

### 7.1 Encrypted Batch Layout

Per KEI-DES-030 §9, encrypted WAL batches use the following layout:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Batch Header (128 bytes) — UNENCRYPTED                                      │
│   • magic, format_version, flags                                            │
│   • dek_id, dek_version                                                     │
│   • batch_physical_seq_start, batch_record_count                            │
│   • batch_crc32c                                                            │
├─────────────────────────────────────────────────────────────────────────────┤
│ Record Entries (N × 32 bytes) — UNENCRYPTED                                │
│   • Metadata only (offsets, lengths, timestamps)                            │
│   • No payload content                                                      │
├─────────────────────────────────────────────────────────────────────────────┤
│ Encrypted Payload Block                                                     │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │ AES-256-GCM Nonce (12 bytes)                                        │   │
│   ├─────────────────────────────────────────────────────────────────────┤   │
│   │ Ciphertext (variable length)                                        │   │
│   ├─────────────────────────────────────────────────────────────────────┤   │
│   │ AES-256-GCM Authentication Tag (16 bytes)                           │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────────────────┤
│ CRC32C Trailer (over ciphertext, not plaintext)                            │
├─────────────────────────────────────────────────────────────────────────────┤
│ Padding to 4096-byte boundary                                               │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 7.2 Authenticated Additional Data (AAD)

The AES-GCM AAD MUST include:

```text
tenant_id
stream_id (if single-stream batch)
batch_physical_seq_start
format_version
dek_id
```

**Normative rule:** Decryption MUST fail if AAD validation fails. This prevents ciphertext substitution attacks.

### 7.3 Nonce Generation

```text
nonce = random_96_bits()
```

**Normative rules:**

- Nonces MUST be unique per (DEK, batch) pair.
- Nonces MUST be cryptographically random.
- Nonce reuse with the same key MUST NEVER occur.
- Nonces MUST be stored in the encrypted batch (not derived).

---

## 8. Parquet File Encryption

### 8.1 Encryption Mode

Parquet files use **file-level encryption** with AES-256-GCM.

```text
Parquet File Layout (Encrypted):
┌─────────────────────────────────────────────────────────────────┐
│ Parquet Magic Bytes ("PAR1")                                    │
├─────────────────────────────────────────────────────────────────┤
│ Encrypted Row Groups                                            │
│   • Column chunks encrypted with file DEK                       │
│   • AES-256-GCM per column chunk                                │
├─────────────────────────────────────────────────────────────────┤
│ Encrypted Footer                                                │
│   • Schema, statistics, offsets encrypted                       │
│   • Footer decryption required before query planning            │
├─────────────────────────────────────────────────────────────────┤
│ Footer Length (4 bytes) — UNENCRYPTED                           │
├─────────────────────────────────────────────────────────────────┤
│ Parquet Magic Bytes ("PAR1")                                    │
└─────────────────────────────────────────────────────────────────┘
```

### 8.2 Parquet Encryption Metadata

Each encrypted Parquet file MUST record:

```text
dek_id
dek_version
encryption_algorithm (AES-256-GCM)
nonce (per column chunk)
aad_context
```

### 8.3 Query Engine Integration

Query engines (DuckDB, Spark, Trino, Polars) MUST:

1. Retrieve the DEK from the DEK Cache or KMS.
2. Decrypt the Parquet footer.
3. Decrypt column chunks on demand.
4. Respect destroyed-key registry (fail if DEK is destroyed).

**Normative rule:** If the DEK is destroyed, query engines MUST return an access error, not attempt to read ciphertext.

---

## 9. State Plane and Manifest Encryption

### 9.1 State Snapshot Encryption

State snapshots (KEI-DES-031) are encrypted using the Manifest DEK.

```text
State Snapshot Layout:
┌─────────────────────────────────────────────────────────────────┐
│ StateSnapshotHeader (4096 bytes) — UNENCRYPTED                  │
│   • magic, format_version, snapshot_id                          │
│   • dek_id, dek_version                                         │
│   • snapshot_crc32c                                             │
├─────────────────────────────────────────────────────────────────┤
│ Encrypted Snapshot Body                                         │
│   • Serialized bitmaps                                          │
│   • Active lease array                                          │
│   • Retry heap                                                  │
│   • Sparse Exception Table                                      │
└─────────────────────────────────────────────────────────────────┘
```

### 9.2 Stream Manifest Encryption

Stream manifests are encrypted using the Manifest DEK.

```text
Stream Manifest Layout:
┌─────────────────────────────────────────────────────────────────┐
│ Manifest Header — UNENCRYPTED                                   │
│   • magic, format_version                                       │
│   • dek_id, dek_version                                         │
├─────────────────────────────────────────────────────────────────┤
│ Encrypted Manifest Body                                         │
│   • Stream metadata                                             │
│   • Chunk metadata list                                         │
│   • Head offset                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 9.3 Lease Journal Encryption

Lease journal frames (KEI-DES-031) are encrypted using the Manifest DEK.

**Normative rule:** All state plane artifacts that contain tenant data or metadata MUST be encrypted at rest.

---

## 10. Crypto-Shredding Erasure Workflow

### 10.1 Erasure Granularity

| Granularity | DEK(s) Destroyed | Effect |
|---|---|---|
| Stream erasure | Stream DEK (if per-stream) or Stream-Batch DEKs for that stream | All data for that stream becomes unrecoverable. |
| Tenant erasure | Tenant KEK | All data for that tenant becomes unrecoverable. |
| Batch erasure | Stream-Batch DEK | All data in that batch becomes unrecoverable. |

### 10.2 Erasure Request

```rust
pub struct ErasureRequest {
    pub request_id: Uuid,
    pub tenant_id: u64,
    pub stream_id: Option<u128>,
    pub granularity: ErasureGranularity,
    pub requested_by: String,
    pub legal_approval_reference: String,
    pub requested_at_ms: u64,
}

pub enum ErasureGranularity {
    Stream,
    Tenant,
    Batch,
}
```

### 10.3 Erasure Workflow

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                        CRYPTO-SHREDDING WORKFLOW                            │
└─────────────────────────────────────────────────────────────────────────────┘

1. RECEIVE ERASURE REQUEST
   │
   ├── Validate requester authorization (ABAC)
   ├── Validate legal approval reference
   ├── Check for legal hold (BLOCK if held)
   └── Create erasure ticket
   │
   ▼
2. IDENTIFY TARGET KEYS
   │
   ├── Stream erasure: identify Stream DEK or Stream-Batch DEKs
   ├── Tenant erasure: identify Tenant KEK
   └── Batch erasure: identify Stream-Batch DEK
   │
   ▼
3. FREEZE NEW WRITES
   │
   ├── Block new appends to target stream(s)
   └── Block new commits to target table(s)
   │
   ▼
4. DESTROY KEYS
   │
   ├── Remove DEKs from DEK Cache (all regions)
   ├── Command KMS to destroy key material
   ├── Receive destruction receipts
   └── Record DEK IDs in Destroyed-Key Registry
   │
   ▼
5. WRITE ERASURE TOMBSTONE
   │
   ├── Write tombstone to metadata (Stream/Tenant/Batch)
   ├── Record erasure ticket ID, timestamp, operator
   └── Replicate tombstone to all regions
   │
   ▼
6. PROPAGATE TO ALL REGIONS
   │
   ├── Verify destroyed-key registry in all regions
   ├── Verify tombstone in all regions
   └── Confirm no cached DEKs remain
   │
   ▼
7. PHYSICAL CLEANUP (ASYNC)
   │
   ├── Mark chunks as tombstoned in manifests
   ├── Exclude tombstoned data from new Iceberg commits
   ├── Schedule physical deletion during compaction
   └── Expire snapshots referencing tombstoned data
   │
   ▼
8. AUDIT AND REPORT
   │
   ├── Emit tamper-evident audit event
   ├── Generate erasure proof report
   └── Notify compliance stakeholders
   │
   ▼
9. ERASURE COMPLETE
```

### 10.4 Erasure Tombstone

```rust
pub struct ErasureTombstone {
    pub tombstone_id: Uuid,
    pub erasure_ticket_id: Uuid,
    pub tenant_id: u64,
    pub stream_id: Option<u128>,
    pub granularity: ErasureGranularity,
    pub destroyed_key_ids: Vec<String>,
    pub destroyed_at_ms: u64,
    pub operator_principal: String,
    pub legal_approval_reference: String,
    pub region_propagation_status: RegionPropagationStatus,
}
```

### 10.5 Legal Hold Interaction

**Normative rules:**

- If a legal hold is active on the target stream/tenant, erasure MUST be blocked.
- Legal hold MUST be explicitly released before erasure can proceed.
- Legal hold release MUST be audited.
- Erasure attempts against legally held data MUST be logged as security events.

---

## 11. Destroyed-Key Registry

### 11.1 Purpose

The Destroyed-Key Registry tracks all destroyed keys to prevent:

- Accidental restoration of destroyed data.
- Query attempts against destroyed data.
- Backup restoration of destroyed data.

### 11.2 Registry Entry

```rust
pub struct DestroyedKeyEntry {
    pub key_id: String,
    pub key_type: KeyType,
    pub tenant_id: u64,
    pub stream_id: Option<u128>,
    pub destroyed_at_ms: u64,
    pub destruction_receipt: DestructionReceipt,
    pub erasure_ticket_id: Uuid,
    pub region_propagation_complete: bool,
}
```

### 11.3 Registry Replication

**Normative rules:**

- The Destroyed-Key Registry MUST be replicated to all regions.
- The Destroyed-Key Registry MUST be persisted in the Metadata Raft plane.
- The Destroyed-Key Registry MUST be checked before:
  - Any DEK unwrap operation.
  - Any backup restore operation.
  - Any cross-region failover.

### 11.4 Registry Query

```rust
pub trait DestroyedKeyRegistry: Send + Sync {
    fn is_destroyed(&self, key_id: &str) -> Result<bool, RegistryError>;
    fn record_destruction(&self, entry: DestroyedKeyEntry) -> Result<(), RegistryError>;
    fn list_destroyed_for_tenant(&self, tenant_id: u64) -> Result<Vec<DestroyedKeyEntry>, RegistryError>;
    fn list_destroyed_for_stream(&self, stream_id: u128) -> Result<Vec<DestroyedKeyEntry>, RegistryError>;
}
```

---

## 12. Backup Interaction with Encryption

### 12.1 Backup Encryption

All backups MUST be encrypted:

| Backup Artifact | Encryption |
|---|---|
| Tier-1 Parquet files | Stream/Batch DEK (inherent) |
| Stream manifests | Manifest DEK |
| Raft snapshots | Manifest DEK |
| State snapshots | Manifest DEK |
| WAL tails | Stream/Batch DEK |
| Schema registry | Manifest DEK |
| Destroyed-Key Registry | Manifest DEK |

### 12.2 Restore with Destroyed Keys

**Normative rule:** Restoring a backup MUST check the Destroyed-Key Registry before exposing any data.

```text
Restore Procedure:
1. Load backup artifacts
2. For each artifact, identify DEK ID
3. Check Destroyed-Key Registry
4. If DEK is destroyed:
   a. DO NOT restore that artifact
   b. Log restoration block
   c. Continue with remaining artifacts
5. If DEK is not destroyed:
   a. Unwrap DEK
   b. Decrypt artifact
   c. Restore to cluster
```

### 12.3 Backup Key Escrow

For disaster recovery, backup encryption keys MUST be escrowed:

- Tenant KEKs MUST be replicated to the DR region.
- Manifest DEKs MUST be included in backup metadata.
- Stream/Batch DEKs MUST be recoverable from wrapped form.

**Normative rule:** Key escrow MUST NOT violate erasure. If a key is destroyed, the escrowed copy MUST also be destroyed.

---

## 13. Cross-Region Key Replication

### 13.1 Key Replication Policy

| Key Type | Replication Policy |
|---|---|
| Root Key | Managed by KMS provider; multi-region key recommended. |
| Tenant KEK | MUST be replicated to DR region. |
| Stream/Batch DEKs | Wrapped DEKs replicated with data; plaintext DEKs not replicated. |
| Manifest DEK | MUST be replicated to DR region. |

### 13.2 Cross-Region Erasure Propagation

When a key is destroyed in the primary region:

```text
1. Primary region destroys key via KMS
2. Primary region records destruction in Destroyed-Key Registry
3. Primary region propagates destruction event to replica regions
4. Replica regions verify key destruction in their KMS
5. Replica regions update their Destroyed-Key Registry
6. Erasure is marked complete when all regions confirm
```

**Normative rule:** Erasure MUST NOT be reported as complete until all regions have confirmed key destruction.

### 13.3 Cross-Region Failover with Erasure

During region failover:

```text
1. Replica region checks Destroyed-Key Registry
2. Any data protected by destroyed keys MUST NOT be exposed
3. Failover proceeds only for non-erased data
4. Erasure tombstones are enforced in the new primary
```

---

## 14. Key Rotation Procedures

### 14.1 Tenant KEK Rotation

```text
1. Generate new KEK version in KMS
2. New DEKs are wrapped under new KEK version
3. Old KEK version remains available for unwrapping old DEKs
4. Over time, old DEKs are re-wrapped under new KEK (optional)
5. Old KEK version is retired after all old DEKs are re-wrapped or destroyed
```

### 14.2 DEK Rotation

```text
1. Generate new DEK for target (stream/batch)
2. Update metadata to reference new DEK ID
3. New writes use new DEK
4. Old DEK remains available for reading historical data
5. Old DEK is destroyed when:
   a. All data encrypted under it is re-encrypted, OR
   b. All data encrypted under it is erased, OR
   c. Retention period expires and data is physically deleted
```

### 14.3 Emergency Key Rotation

In case of suspected key compromise:

```text
1. Immediately destroy compromised DEK
2. Generate replacement DEK
3. Re-encrypt active data with new DEK (if retention allows)
4. Mark historical data as compromised (if applicable)
5. Emit critical security alert
6. Initiate incident response workflow
```

**Normative rule:** Emergency key rotation MUST be treated as a security incident and MUST trigger audit and alerting.

---

## 15. Security Failure Modes

### 15.1 Failure Matrix

| Failure | Detection | Required Behavior |
|---|---|---|
| KMS unavailable | KMS API timeout | Use cached DEKs; fail closed for new DEK requests. |
| DEK cache miss | Cache lookup failure | Fetch from KMS; if KMS unavailable, deny access. |
| KMS returns wrong key | AAD validation failure | Deny decryption; emit security alert. |
| Key destroyed but data requested | Destroyed-Key Registry check | Deny access; return erasure error. |
| Backup restore with destroyed key | Destroyed-Key Registry check | Block restoration of destroyed data. |
| Cross-region key mismatch | Region propagation check | Block failover for affected data; alert. |
| Plaintext fallback attempt | Code invariant violation | MUST NOT occur; compile-time prevention. |
| DEK leaked in logs | Log scanning | MUST NOT occur; DEKs are never logged. |
| Nonce reuse | Cryptographic invariant | MUST NOT occur; random nonce generation. |

### 15.2 Fail-Secure Rules

**Normative rules:**

- The system MUST NEVER fall back to plaintext encryption.
- The system MUST NEVER log or expose plaintext DEKs.
- The system MUST NEVER allow backup restore of destroyed data.
- The system MUST NEVER allow cross-region failover to expose destroyed data.
- The system MUST deny access when encryption cannot be guaranteed.

---

## 16. Compliance Integration

### 16.1 GDPR Right-to-Erasure

The crypto-shredding workflow satisfies GDPR Article 17 (Right to Erasure) by:

1. Rendering personal data cryptographically unrecoverable immediately.
2. Providing auditable proof of key destruction.
3. Propagating erasure to all regions and backups.
4. Physically deleting ciphertext during routine lifecycle operations.

### 16.2 CCPA Deletion Rights

The same mechanism satisfies CCPA deletion requirements.

### 16.3 Compliance Evidence

For each erasure event, the system MUST produce:

- Erasure ticket ID.
- Requester identity and authorization.
- Legal approval reference.
- Destroyed key IDs.
- KMS destruction receipts.
- Region propagation confirmation.
- Timestamp.
- Audit trail reference.

### 16.4 Legal Acceptance Caveat

**Normative statement:** Crypto-shredding is the system's technical erasure mechanism. Whether it satisfies a specific regulatory or contractual obligation depends on the customer's legal review and jurisdiction. Keirox provides cryptographic erasure with audit evidence; compliance acceptance is customer-specific.

---

## 17. Observability

### 17.1 Metrics

| Metric | Type | Description |
|---|---|---|
| `keirox_kms_requests_total` | Counter | KMS API calls by operation. |
| `keirox_kms_errors_total` | Counter | KMS failures by error type. |
| `keirox_dek_cache_hit_ratio` | Gauge | DEK cache hit rate. |
| `keirox_dek_cache_size` | Gauge | Current DEK cache entries. |
| `keirox_dek_unwrap_latency_seconds` | Histogram | DEK unwrap latency. |
| `keirox_crypto_shred_count` | Counter | Erasure events by granularity. |
| `keirox_destroyed_key_registry_size` | Gauge | Destroyed keys tracked. |
| `keirox_encryption_errors_total` | Counter | Encryption/decryption failures. |
| `keirox_key_rotation_count` | Counter | Key rotations by type. |

### 17.2 Alerts

| Alert | Condition | Severity |
|---|---|---|
| KMS unavailable | KMS errors > threshold | Critical |
| DEK cache exhaustion | Cache full | Warning |
| Erasure propagation incomplete | Region propagation stalled | Critical |
| Destroyed key access attempt | Query against destroyed key | Critical |
| Plaintext fallback detected | Security invariant violation | Critical |
| Key rotation overdue | Rotation past due date | Warning |

---

## 18. NFR Traceability

| NFR | Requirement | How This Specification Satisfies It |
|---|---|---|
| SEC-002 | Encryption at rest | AES-256-GCM envelope encryption (§7, §8, §9). |
| SEC-006 | Key management | KMS adapter, DEK lifecycle, rotation (§5, §6, §14). |
| COMP-001 | Right-to-erasure | Crypto-shredding workflow (§10). |
| COMP-002 | Erasure latency | Immediate logical erasure via key destruction (§10.3). |
| COMP-004 | Deletion audit | Erasure tickets, receipts, audit trail (§10.3, §16.3). |
| REC-007 | Crypto-shred backups unrecoverable | Destroyed-Key Registry check on restore (§12.2). |
| AVAIL | Fail-secure behavior | Fail closed on KMS unavailability (§15). |

---

## 19. Interfaces

### 19.1 Provided Interfaces

| Interface | Consumer | Semantics |
|---|---|---|
| `getDek(dek_id)` | Storage / ELT / Query | Retrieve or unwrap DEK. |
| `generateDek(context)` | Storage / ELT | Generate new DEK. |
| `destroyKey(key_id)` | Erasure Orchestrator | Destroy key material. |
| `isKeyDestroyed(key_id)` | Restore / Query | Check destroyed status. |
| `rotateKek(kek_id)` | Admin | Rotate tenant KEK. |
| `recordErasure(tombstone)` | Erasure Orchestrator | Record erasure tombstone. |

### 19.2 Consumed Interfaces

| Interface | Provider | Purpose |
|---|---|---|
| KMS API | AWS KMS / GCP KMS / Azure KV / Vault | Key generation, wrapping, destruction. |
| Metadata Raft | KEI-ARC-022 | Destroyed-Key Registry persistence. |
| Audit sink | KEI-ARC-025 | Erasure audit events. |
| ABAC authorization | KEI-ARC-025 | Erasure request authorization. |

---

## 20. Open Questions

| Item | Status | Resolution Path |
|---|---|---|
| Parquet modular encryption vs. file-level encryption | Open | Evaluate query engine compatibility. |
| Default DEK cache TTL tuning | Open | Benchmark under P1/P5 profiles. |
| KMS multi-region key replication latency | Open | Measure under Mode A failover. |
| HSM FIPS mode requirements | Open | Determine enterprise target segment. |
| Emergency key rotation automation | Open | Define incident response integration. |

---

## 21. Glossary

| Term | Definition |
|---|---|
| Envelope Encryption | Encryption model where data keys are encrypted by higher-level keys. |
| KEK | Key Encryption Key; encrypts DEKs. |
| DEK | Data Encryption Key; encrypts actual data. |
| Crypto-Shredding | Erasure by destroying encryption keys. |
| Destroyed-Key Registry | Registry tracking all destroyed keys. |
| Erasure Tombstone | Metadata marker indicating data has been cryptographically erased. |
| AAD | Authenticated Additional Data for AES-GCM. |
| Nonce | Number used once; unique per encryption operation. |
| Fail Secure | Failure behavior that denies access rather than reducing security. |

---

## 22. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial encryption, key management, and crypto-shredding specification. Defines envelope encryption hierarchy, KMS adapter, DEK lifecycle, WAL/Parquet/state encryption formats, crypto-shredding workflow, destroyed-key registry, backup interaction, cross-region key replication, key rotation, security failure modes, and compliance integration. Implements ADR-050/051. |