# KEI-DES-033 — Schema Registry & Adaptive Shredding Specification

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-DES-033 |
| Title | Schema Registry & Adaptive Shredding Specification |
| Version | 1.0 |
| Level | **L3 — Detailed Design Specification** |
| Subsystem Covered | Columnar ELT — Schema Governance and Shredding |
| Status | Approved for Engineering |
| Classification | Internal / Engineering Confidential |
| Owner | Principal Engineer (Stream Processing / Lakehouse) |
| Required Reviewers | Chief Architect, Principal Engineer (Storage), Data Platform Lead, Security Lead |
| Depends On | KEI-ARC-023 (Columnar ELT), KEI-ARC-025 (Security), KEI-DES-030 (WAL Binary Format), KEI-DES-031 (State Plane), KEI-DES-032 (API & Protocol) |
| Consumed By | Arrow vectorizer, Parquet encoder, Iceberg committer, SDK teams, gateway teams, lakehouse query engines |
| Keywords | MUST, MUST NOT, SHOULD, SHOULD NOT, MAY per RFC 2119 |

---

## 2. Purpose, Scope, and Audience

### 2.1 Purpose

This document specifies the **Schema Registry and Adaptive Schema Shredding subsystem** of the Polymorphic Event Fabric. It defines how schemas are registered, versioned, resolved, evolved, and used during Internalized Columnar ELT.

It implements:

- ADR-040: Internalized Columnar ELT.
- ADR-042: Adaptive schema shredding with a 64-key cap.
- ADR-043: Shared tenant Iceberg table compatibility.
- KEI-ARC-023 schema governance requirements.

### 2.2 Scope

**In scope:**

- Schema registry data model.
- Schema registration and versioning.
- Compatibility modes.
- Schema fingerprinting.
- Stream schema policies.
- Adaptive schema inference.
- Field promotion and demotion.
- Type inference and conflict resolution.
- `_unstructured_payload` handling.
- Nested field policy.
- Arrow schema generation.
- Iceberg schema evolution coordination.
- Registry persistence, caching, and failover.
- Validation rules and failure handling.

**Out of scope:**

- WAL binary layout — owned by KEI-DES-030.
- Iceberg commit algorithm — owned by KEI-DES-034.
- Client wire protocol for produce/consume — owned by KEI-DES-032.
- Encryption key management — owned by KEI-DES-036.

### 2.3 Audience

- ELT implementation engineers.
- Schema registry engineers.
- Lakehouse integration engineers.
- SDK and gateway engineers.
- Data governance and compliance stakeholders.
- Test engineers validating schema evolution.

---

## 3. Design Principles

| ID | Principle | Rationale |
|---|---|---|
| SR-1 | **Raw truth remains immutable.** Schema evolution MUST NOT rewrite the immutable WAL. |
| SR-2 | **Schema is metadata, not payload mutation.** Shredding projects payload into columnar form; it does not alter source truth. |
| SR-3 | **Bounded shredding.** Maximum 64 shredded primitive keys per stream namespace. |
| SR-4 | **Graceful polymorphism.** Unknown, polymorphic, or conflicting fields route to `_unstructured_payload`. |
| SR-5 | **Backward-readable history.** Historical chunks MUST remain readable via schema fingerprint. |
| SR-6 | **Registry failover is non-destructive.** Registry unavailability MUST NOT cause data loss. |
| SR-7 | **Schema changes are audited.** Every schema registration, evolution, and deprecation MUST emit an audit event. |

---

## 4. Schema Modes

PEF supports five schema modes per stream or tenant default.

| Mode | Description | Ingress Validation | Shredding Behavior |
|---|---|---|---|
| `RAW` | Payload treated as opaque bytes. | None. | No shredding unless explicitly enabled. |
| `INFERRED` | Broker infers schema from sampled payloads. | Optional warning only. | Adaptive shredding with inference. |
| `REGISTERED` | Producer supplies schema ID. | Schema validation enabled. | Shredding uses registered schema. |
| `STRICT` | Registered schema is enforced. | Invalid records rejected. | Shredding uses registered schema. |
| `PERMISSIVE` | Registered schema is preferred but tolerant. | Invalid fields routed to `_unstructured_payload`. | Shredding uses best-effort schema. |

**Normative rules:**

- Default mode for new streams SHOULD be `INFERRED` unless tenant policy overrides.
- `STRICT` mode MUST reject records that fail registered schema validation before durable append.
- `PERMISSIVE` mode MUST accept records and preserve malformed or unexpected fields in `_unstructured_payload`.

---

## 5. Schema Registry Data Model

### 5.1 Schema Identifier

```rust
#[repr(C, align(32))]
pub struct SchemaId {
    pub tenant_id: u64,
    pub schema_id: u32,
    pub version: u32,
    pub reserved: [u8; 8],
}
```

Total size: 32 bytes.

### 5.2 Schema Metadata

```rust
pub struct SchemaMetadata {
    pub schema_id: SchemaId,
    pub name: String,
    pub description: String,
    pub created_at_ms: u64,
    pub created_by: String,
    pub status: SchemaStatus,
    pub compatibility_mode: CompatibilityMode,
    pub fingerprint_sha256: [u8; 32],
    pub fingerprint_xxh3_64: u64,
    pub fields: Vec<FieldSpec>,
    pub metadata: Map<String, String>,
}
```

### 5.3 Schema Status

| Status | Meaning |
|---|---|
| `DRAFT` | Schema is being defined; not active for production writes. |
| `ACTIVE` | Schema is active and resolvable. |
| `DEPRECATED` | Schema is readable but SHOULD NOT accept new writes. |
| `TOMBSTONED` | Schema is logically deleted; metadata retained for audit. |

### 5.4 Compatibility Mode

| Mode | Meaning |
|---|---|
| `NONE` | No compatibility validation. |
| `BACKWARD` | New schema can read old data. Default. |
| `FORWARD` | Old schema can read new data. |
| `FULL` | Both backward and forward compatible. |
| `STRICT_TYPE` | Type changes require explicit migration. |

**Normative rule:** The default compatibility mode MUST be `BACKWARD` to preserve historical lakehouse readability.

---

## 6. Field Specification

### 6.1 FieldSpec

```rust
pub struct FieldSpec {
    pub field_id: u32,
    pub name: String,
    pub path: String,
    pub logical_type: LogicalType,
    pub physical_type: ArrowType,
    pub nullable: bool,
    pub required: bool,
    pub default_value: Option<Value>,
    pub sensitivity: SensitivityClass,
    pub shredding_policy: ShreddingPolicy,
    pub metadata: Map<String, String>,
}
```

### 6.2 Logical Types

| Logical Type | Arrow Physical Type | Notes |
|---|---|---|
| `BOOLEAN` | `Boolean` | JSON boolean. |
| `INTEGER` | `Int64` | Default integer representation. |
| `FLOAT` | `Float64` | JSON number. |
| `DECIMAL` | `Decimal128` | Optional exact numeric. |
| `STRING` | `Utf8` | JSON string. |
| `BINARY` | `Binary` | Base64 or raw bytes. |
| `TIMESTAMP` | `Timestamp(ns)` | ISO-8601 or epoch if stable. |
| `DATE` | `Date32` | Optional. |
| `UUID` | `FixedSizeBinary(16)` or `Utf8` | Optional. |
| `JSON` | `Binary` | Stored in `_unstructured_payload` or nested column. |

### 6.3 Sensitivity Classes

| Class | Meaning |
|---|---|
| `PUBLIC` | No special handling. |
| `INTERNAL` | Standard enterprise access controls. |
| `CONFIDENTIAL` | Restricted ABAC policies. |
| `PII` | Privacy-sensitive; eligible for erasure policies. |
| `PCI` | Payment-sensitive; restricted audit. |

**Normative rule:** Sensitivity labels are metadata in v1. Enforcement of field-level redaction is out of scope unless explicitly enabled by a future ADR.

---

## 7. Schema Fingerprinting

### 7.1 Canonical Schema Form

Before fingerprinting, schema fields MUST be canonicalized:

1. Sort fields by `path`, then `field_id`.
2. Normalize logical types.
3. Remove descriptions and non-semantic metadata.
4. Preserve nullability, type, and field IDs.
5. Encode canonical form as deterministic JSON or Avro schema JSON.

### 7.2 Fingerprint Algorithms

| Fingerprint | Use |
|---|---|
| SHA-256 | Registry integrity, audit, governance. |
| XXH3-64 | Chunk metadata and fast runtime lookup. |

**Normative rules:**

- `SSTableChunkHeader.schema_fingerprint` MUST use the XXH3-64 fingerprint.
- The schema registry MUST store SHA-256 as the authoritative fingerprint.
- Fingerprint collision handling MUST verify SHA-256 before accepting a schema match.

---

## 8. Stream Schema Policy

### 8.1 Policy Structure

```rust
pub struct StreamSchemaPolicy {
    pub tenant_id: u64,
    pub stream_id: u128,
    pub schema_mode: SchemaMode,
    pub active_schema_id: Option<u32>,
    pub compatibility_mode: CompatibilityMode,
    pub max_shredded_fields: u16,
    pub allow_nested_shredding: bool,
    pub max_unstructured_bytes: u32,
    pub inference_enabled: bool,
    pub inference_sampling_ratio: f32,
    pub updated_at_ms: u64,
    pub updated_by: String,
}
```

### 8.2 Default Policy

| Field | Default |
|---|---:|
| `schema_mode` | `INFERRED` |
| `compatibility_mode` | `BACKWARD` |
| `max_shredded_fields` | 64 |
| `allow_nested_shredding` | `false` |
| `max_unstructured_bytes` | 1 MB |
| `inference_enabled` | `true` |
| `inference_sampling_ratio` | 0.01 |

**Normative rule:** `max_shredded_fields` MUST NOT exceed 64 unless a future ADR explicitly changes the global cap.

---

## 9. Adaptive Schema Inference

### 9.1 Inference Pipeline

```text
Sealed row segment / sample stream
        │
        ▼
Payload decoder (JSON, Protobuf, FlatBuffers, raw)
        │
        ▼
Field path extractor
        │
        ▼
Candidate field tracker
        │
        ▼
Type consistency analyzer
        │
        ▼
Frequency/stability scorer
        │
        ▼
Promotion/demotion controller
        │
        ▼
Active schema candidate
```

### 9.2 Candidate Field Tracking

For each stream namespace, the shredder maintains bounded candidate statistics:

```rust
pub struct CandidateFieldStats {
    pub path: String,
    pub observed_count: u64,
    pub type_counts: Map<LogicalType, u64>,
    pub first_seen_offset: u64,
    pub last_seen_offset: u64,
    pub stability_score: f64,
    pub frequency_score: f64,
    pub promotion_score: f64,
    pub state: CandidateState,
}
```

### 9.3 Candidate States

| State | Meaning |
|---|---|
| `OBSERVED` | Field seen but not stable enough. |
| `PROMOTED` | Field is part of active shredded schema. |
| `DEMOTE_PENDING` | Field is unstable or infrequent; demotion candidate. |
| `REJECTED` | Field rejected due to polymorphism, depth, or quota. |
| `QUARANTINED` | Field causes repeated type conflicts or excessive cardinality. |

### 9.4 Promotion Score

A simplified scoring model:

```text
frequency_score   = observed_count / total_records_in_window
type_consistency  = max_type_count / observed_count
stability_score   = appearances_in_windows / total_windows
promotion_score   = frequency_score × type_consistency × stability_score
```

### 9.5 Promotion Rules

A field MAY be promoted when:

```text
promotion_score >= promotion_threshold
AND observed_count >= min_observations
AND type_consistency >= type_threshold
AND shredded_field_count < max_shredded_fields
AND field_depth <= max_field_depth
```

Defaults:

| Parameter | Default |
|---|---:|
| `promotion_threshold` | 0.70 |
| `min_observations` | 1,000 |
| `type_threshold` | 0.95 |
| `max_field_depth` | 1 for v1 unless nested shredding enabled |

### 9.6 Demotion Rules

A promoted field SHOULD be demoted when:

```text
rolling_frequency_score < demotion_threshold
OR type_consistency < type_threshold
OR field_conflict_rate > conflict_threshold
```

Defaults:

| Parameter | Default |
|---|---:|
| `demotion_threshold` | 0.05 |
| `conflict_threshold` | 0.10 |

### 9.7 Anti-Churn Rules

To prevent schema oscillation:

- Promoted fields MUST have a minimum tenure before demotion, default 24 hours.
- Demoted fields MUST have a cooldown before re-promotion, default 6 hours.
- Schema changes MUST be batched, not per-record.
- Promotion and demotion decisions MUST be logged and auditable.

---

## 10. Type Inference and Conflict Resolution

### 10.1 JSON to Arrow Type Mapping

| JSON Type | Default Arrow Type |
|---|---|
| `null` | Null / missing value |
| `boolean` | Boolean |
| integer number | Int64 |
| floating number | Float64 |
| string | Utf8 |
| ISO-8601 timestamp string | Timestamp(ns), if stable |
| base64 binary string | Binary, if stable |
| object | `_unstructured_payload` unless nested shredding enabled |
| array | `_unstructured_payload` unless array shredding enabled |

### 10.2 Safe Numeric Widening

Allowed automatic widenings:

```text
Int8  → Int16 → Int32 → Int64
Int64 → Float64 only if policy permits
Float32 → Float64
```

**Normative rule:** `Int64 → Float64` MUST be disabled by default if exact integer preservation is required.

### 10.3 Type Conflict Matrix

| Existing Type | New Type | Default Action |
|---|---|---|
| Int64 | Int64 | OK |
| Int64 | Float64 | Widen if allowed; otherwise conflict |
| Float64 | Int64 | Accept as Float64 |
| String | Int64 | Conflict |
| String | Timestamp | Promote to Timestamp only if stable |
| Boolean | String | Conflict |
| Object | Scalar | Conflict |
| Array | Scalar | Conflict |

### 10.4 Conflict Handling by Schema Mode

| Mode | Conflict Behavior |
|---|---|
| `RAW` | Ignore; payload remains raw. |
| `INFERRED` | Route conflicting field to `_unstructured_payload`. |
| `REGISTERED` | Validate against registered schema; conflict may produce warning. |
| `STRICT` | Reject record at ingress if validation fails. |
| `PERMISSIVE` | Accept record; conflicting field routed to `_unstructured_payload`. |

---

## 11. `_unstructured_payload` Handling

### 11.1 Purpose

`_unstructured_payload` stores fields that are:

- Not promoted to shredded columns.
- Polymorphic.
- Deeply nested.
- Sparse below promotion threshold.
- In type conflict.
- Beyond the 64-field cap.

### 11.2 Storage Format

```text
_unstructured_payload = compressed JSON or MessagePack binary
```

Recommended default:

```text
MessagePack with zstd compression for compactness
```

Fallback:

```text
UTF-8 JSON when debuggability is prioritized
```

### 11.3 Unstructured Entry Metadata

```rust
pub struct UnstructuredPayloadMeta {
    pub encoding: u8,          // 0 = JSON, 1 = MessagePack
    pub compression: u8,       // 0 = none, 1 = zstd
    pub original_bytes: u32,
    pub stored_bytes: u32,
    pub reason_flags: u16,
}
```

### 11.4 Reason Flags

| Bit | Reason |
|---|---|
| 0 | Field not promoted |
| 1 | Type conflict |
| 2 | Nested object |
| 3 | Array |
| 4 | Schema cap exceeded |
| 5 | Validation failed in permissive mode |
| 6 | Payload truncated |

### 11.5 Size Limit

If unstructured payload exceeds `max_unstructured_bytes`:

1. The original record remains immutable in the WAL.
2. The lakehouse projection stores a truncation marker.
3. The metadata flag `PAYLOAD_TRUNCATED` is set.
4. Query engines MAY access the original record via offset-based raw log fetch if authorized.

**Normative rule:** The lakehouse projection MUST NOT silently lose data without a truncation marker.

---

## 12. Arrow Schema Generation

### 12.1 System Columns

Every shredded Arrow schema MUST include the following system columns:

| Column | Type | Description |
|---|---|---|
| `_keirox_stream_id` | `FixedSizeBinary(16)` | Stream UUID. |
| `_keirox_offset` | `UInt64` | Logical offset. |
| `_keirox_ingest_time` | `Timestamp(ns)` | Ingress timestamp. |
| `_keirox_entity_key` | `Utf8` | Entity key if present. |
| `_keirox_schema_id` | `UInt32` | Schema ID. |
| `_keirox_schema_version` | `UInt32` | Schema version. |
| `_unstructured_payload` | `Binary` | Dynamic/unshredded fields. |

### 12.2 Field ID Assignment

- Field IDs MUST be stable within a schema lineage.
- New fields MUST receive monotonically increasing field IDs.
- Removed fields MUST NOT reuse field IDs.
- System columns MUST occupy reserved field IDs `0..999`.

### 12.3 Arrow Metadata

Arrow schema metadata MUST include:

```text
keirox.tenant_id
keirox.stream_id
keirox.schema_id
keirox.schema_version
keirox.schema_fingerprint_xxh3_64
keirox.schema_fingerprint_sha256
keirox.compatibility_mode
keirox.created_at_ms
```

---

## 13. Iceberg Schema Evolution Coordination

### 13.1 Mapping to Shared Tenant Table

The default Iceberg table is:

```text
tenant_{tenant_id}.events
```

Shredded columns are added to this table as nullable columns.

### 13.2 Evolution Rules

| Schema Change | Iceberg Action |
|---|---|
| New nullable field | Add column. |
| Removed field | Mark deprecated; do not drop immediately. |
| Safe widening | Use Iceberg type evolution if supported. |
| Unsafe type change | Add new column and deprecate old column. |
| Field rename | Add alias metadata; preserve field ID. |
| Nested field | Add only if nested shredding enabled. |

### 13.3 Commit Coordination

- The Iceberg committer MUST receive the schema version and fingerprint with each Parquet file set.
- If the target Iceberg table schema does not include a required column, the committer MUST request schema evolution before committing.
- If schema evolution fails, chunks MUST be quarantined and alerted, not silently dropped.

**Normative rule:** Parquet files MUST NOT be committed with a schema that is incompatible with the active Iceberg table unless the incompatible columns are routed to `_unstructured_payload`.

---

## 14. Schema Registry API

### 14.1 gRPC Service

```protobuf
service SchemaRegistryService {
  rpc RegisterSchema(RegisterSchemaRequest) returns (RegisterSchemaResponse);
  rpc GetSchema(GetSchemaRequest) returns (GetSchemaResponse);
  rpc GetLatestSchema(GetLatestSchemaRequest) returns (GetLatestSchemaResponse);
  rpc ListSchemaVersions(ListSchemaVersionsRequest) returns (ListSchemaVersionsResponse);
  rpc ResolveByFingerprint(ResolveByFingerprintRequest) returns (ResolveByFingerprintResponse);
  rpc SetStreamSchemaPolicy(SetStreamSchemaPolicyRequest) returns (SetStreamSchemaPolicyResponse);
  rpc GetStreamSchemaPolicy(GetStreamSchemaPolicyRequest) returns (GetStreamSchemaPolicyResponse);
  rpc DeprecateSchema(DeprecateSchemaRequest) returns (DeprecateSchemaResponse);
}
```

### 14.2 RegisterSchemaRequest

```protobuf
message RegisterSchemaRequest {
  uint64 tenant_id = 1;
  string name = 2;
  string description = 3;
  repeated FieldSpec fields = 4;
  CompatibilityMode compatibility_mode = 5;
  bool dry_run = 6;
  string idempotency_key = 7;
}
```

### 14.3 RegisterSchemaResponse

```protobuf
message RegisterSchemaResponse {
  uint32 schema_id = 1;
  uint32 version = 2;
  bytes fingerprint_sha256 = 3;
  uint64 fingerprint_xxh3_64 = 4;
  SchemaStatus status = 5;
  repeated CompatibilityViolation violations = 6;
}
```

### 14.4 Error Conditions

| Condition | Error |
|---|---|
| Duplicate fingerprint | Return existing schema ID/version idempotently. |
| Compatibility violation | Return `FAILED_PRECONDITION` with violations. |
| Invalid field definition | Return `INVALID_ARGUMENT`. |
| Unauthorized | Return `PERMISSION_DENIED`. |
| Registry unavailable | Return `UNAVAILABLE` with retry hint. |

---

## 15. Producer Schema Validation Flow

### 15.1 Validation Modes

```text
Producer sends payload + optional schema_id
        │
        ▼
Gateway resolves stream schema policy
        │
        ├── RAW         → accept, no validation
        ├── INFERRED    → accept, optional soft validation
        ├── REGISTERED  → validate, warn or reject per policy
        ├── STRICT      → reject on validation failure
        └── PERMISSIVE  → accept, route failures to unstructured
```

### 15.2 Validation Timing

**Normative rules:**

- In `STRICT` mode, validation MUST occur before durable append.
- In `PERMISSIVE` mode, validation MAY occur asynchronously for lakehouse projection, but ingress MUST NOT fail due to schema mismatch.
- In `INFERRED` mode, invalid or unexpected fields MUST NOT cause data loss.

---

## 16. Shredding Algorithm

### 16.1 High-Level Algorithm

```rust
fn shred_segment(
    segment: &SealedRowSegment,
    policy: &StreamSchemaPolicy,
    registry: &SchemaRegistryCache,
) -> Result<ArrowRecordBatch, EltError> {
    let active_schema = resolve_active_schema(registry, policy)?;

    let mut builder = ArrowBatchBuilder::new(active_schema.arrow_schema());

    for record in segment.records() {
        let decoded = decode_payload(record, policy)?;

        let system_columns = extract_system_columns(record, &decoded);
        builder.append_system_columns(system_columns);

        for field in active_schema.fields() {
            match decoded.get(&field.path) {
                Some(value) if type_matches(value, field.logical_type) => {
                    builder.append_typed(field.field_id, value)?;
                }
                Some(value) if policy.schema_mode == SchemaMode::Permissive => {
                    builder.append_null(field.field_id);
                    builder.append_unstructured(field.path, value, Reason::TypeConflict);
                }
                Some(_) if policy.schema_mode == SchemaMode::Strict => {
                    return Err(EltError::SchemaViolation);
                }
                _ => {
                    builder.append_null(field.field_id);
                }
            }
        }

        for (path, value) in decoded.remaining_fields() {
            builder.append_unstructured(path, value, Reason::NotPromoted);
        }
    }

    builder.finish_with_metadata(active_schema.fingerprint())
}
```

### 16.2 Batch Shredding Rules

- Shredding MUST operate on sealed segments or bounded sample windows.
- Shredding MUST NOT block the producer write path.
- Shredding MUST be idempotent for the same sealed segment and schema version.
- Shredding MUST emit metrics for unstructured ratio and conflict rate.

---

## 17. Schema Versioning and Historical Data

### 17.1 Historical Readability

Each chunk stores:

```text
schema_id
schema_version
schema_fingerprint_xxh3_64
```

Readers MUST resolve the schema fingerprint before decoding.

### 17.2 Schema Evolution Horizon

| Event | Behavior |
|---|---|
| New schema version | Applies from promotion/registration offset forward. |
| Old chunks | Remain readable via old schema version. |
| New column added | Historical rows read as null unless re-compacted. |
| Column deprecated | Historical rows remain readable. |
| Unsafe type change | New column added; old column deprecated. |

### 17.3 Optional Re-Shredding

Re-shredding historical data MAY occur during compaction if:

- Tenant opts in.
- Storage and CPU budget allow.
- Data is not crypto-shredded.
- Target schema is backward compatible.

**Normative rule:** Re-shredding MUST NOT be required for query correctness.

---

## 18. Registry Persistence and Caching

### 18.1 Persistence

Schema registry entries MUST be persisted in the Metadata & State Raft plane.

Persisted artifacts:

```text
SchemaMetadata records
StreamSchemaPolicy records
SchemaVersion lineage table
Fingerprint index
Audit events
```

### 18.2 Local Cache

Each ELT worker and gateway MUST maintain a local schema cache:

```rust
pub struct SchemaCache {
    pub schemas: LruCache<SchemaId, SchemaMetadata>,
    pub fingerprints: HashMap<u64, SchemaId>,
    pub policies: LruCache<u128, StreamSchemaPolicy>,
    pub last_sync_lsn: u64,
}
```

### 18.3 Cache Rules

- Cache TTL SHOULD be 60 seconds unless event-driven invalidation is available.
- Cache misses MUST fetch from registry or last-known-good snapshot.
- If registry is unavailable, last-known-good schema MUST be used.
- If no schema is available, payload MUST be treated as `RAW`, not dropped.

---

## 19. Security and Governance

### 19.1 Authorization

| Operation | Required Permission |
|---|---|
| Register schema | `schema_write` on tenant |
| Get schema | `schema_read` on tenant |
| Set stream policy | `admin` on stream or tenant |
| Deprecate schema | `schema_admin` on tenant |
| Read unstructured payload | `consume` on stream and ABAC approval |

### 19.2 Audit Requirements

The following events MUST be audited:

- Schema registration.
- Schema version creation.
- Compatibility violation.
- Stream policy change.
- Schema deprecation.
- Schema cache invalidation.
- Repeated unstructured payload overflow.

### 19.3 Privacy

- Schema field names MAY reveal business semantics and SHOULD be protected as tenant metadata.
- Fields tagged `PII` or `PCI` MUST propagate sensitivity labels to lakehouse metadata.
- Crypto-shredding MUST render both payload ciphertext and shredded projections inaccessible.

---

## 20. Failure Handling

| Scenario | Required Behavior |
|---|---|
| Registry unavailable | Use last-known-good schema cache; treat unknown payloads as RAW. |
| Schema cache corrupt | Refetch from Raft snapshot; fail closed for STRICT writes if unresolved. |
| Fingerprint collision | Validate SHA-256; reject ambiguous match. |
| Schema conflict during Iceberg commit | Quarantine chunk set; alert; retry after evolution. |
| Polymorphic field explosion | Enforce 64-field cap; route excess to `_unstructured_payload`. |
| Inference CPU overload | Reduce sampling ratio; delay promotions; alert. |
| Invalid STRICT record | Reject produce with protocol-appropriate validation error. |
| Oversized unstructured payload | Store truncation marker; preserve original WAL record. |

---

## 21. Observability

| Metric | Type | Description |
|---|---|---|
| `keirox_schema_count` | Gauge | Registered schemas per tenant. |
| `keirox_schema_active_versions` | Gauge | Active schema versions. |
| `keirox_schema_cache_miss_total` | Counter | Schema cache misses. |
| `keirox_schema_registry_errors_total` | Counter | Registry failures. |
| `keirox_shredding_cpu_ratio` | Gauge | CPU used by shredding workers. |
| `keirox_promoted_fields_total` | Gauge | Current shredded field count per stream. |
| `keirox_field_promotions_total` | Counter | Field promotions over time. |
| `keirox_field_demotions_total` | Counter | Field demotions over time. |
| `keirox_type_conflicts_total` | Counter | Type conflicts detected. |
| `keirox_unstructured_payload_ratio` | Gauge | Fraction of payload bytes routed to `_unstructured_payload`. |
| `keirox_unstructured_truncations_total` | Counter | Truncated unstructured payloads. |

---

## 22. NFR Traceability

| NFR | Requirement | How This Specification Satisfies It |
|---|---|---|
| PERF-032 | Arrow client CPU efficiency | Typed shredded columns reduce consumer SerDe. |
| SCALE | High stream cardinality | Schema cache and bounded candidate tracking. |
| DUR | Projection integrity | Schema fingerprint in chunks and registry. |
| OPS | Schema observability | Metrics and audit events. |
| SEC | Metadata protection | ABAC and audit for registry operations. |
| COMP | Schema governance | Compatibility modes and version lineage. |

---

## 23. Interfaces

### 23.1 Provided Interfaces

| Interface | Consumer | Semantics |
|---|---|---|
| `registerSchema(...)` | Admin / SDK | Register new schema version. |
| `getSchema(...)` | ELT / Gateway | Resolve schema by ID/version. |
| `resolveByFingerprint(...)` | Reader / Iceberg committer | Resolve schema from chunk fingerprint. |
| `getStreamPolicy(...)` | Gateway / ELT | Return stream schema policy. |
| `setStreamPolicy(...)` | Admin | Update schema mode and limits. |
| `deprecateSchema(...)` | Admin | Deprecate schema version. |

### 23.2 Consumed Interfaces

| Interface | Provider | Purpose |
|---|---|---|
| Metadata Raft persistence | KEI-ARC-022 | Durable schema registry storage. |
| ABAC authorization | KEI-ARC-025 | Access control. |
| Audit sink | KEI-ARC-025 | Governance events. |
| Sealed row segments | KEI-ARC-020 | Input to shredding. |
| Iceberg committer | KEI-DES-034 | Schema evolution coordination. |

---

## 24. Open Questions

| Item | Status | Resolution Path |
|---|---|---|
| Decimal precision defaults | Open | Evaluate financial workload requirements. |
| Nested shredding opt-in model | Open | Requires ADR before enabling. |
| Inference confidence thresholds | Open | Benchmark under Profile P5. |
| Field ID allocation across Iceberg tables | Open | Coordinate with KEI-DES-034. |
| Schema registry backend format | Open | Evaluate protobuf vs. Avro for registry persistence. |
| Re-shredding cost model | Open | FinOps and compaction capacity analysis. |

---

## 25. Glossary

| Term | Definition |
|---|---|
| Adaptive Shredding | Background extraction of stable primitive fields into typed Arrow columns. |
| Candidate Field | A field path observed during inference but not yet promoted. |
| Promoted Field | A field included in the active shredded schema. |
| Schema Fingerprint | Canonical hash of a schema definition. |
| `_unstructured_payload` | Auxiliary column for dynamic, conflicting, or unshredded fields. |
| Compatibility Mode | Rule governing whether schema changes are allowed. |
| Stream Schema Policy | Per-stream schema mode and shredding limits. |

---

## 26. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial schema registry and adaptive shredding specification. Defines schema modes, registry model, fingerprinting, inference scoring, promotion/demotion, type conflict handling, `_unstructured_payload`, Arrow schema generation, Iceberg evolution coordination, caching, security, and failure handling. Implements ADR-040/042. |