# KEI-DES-034 — Iceberg Catalog Committer Specification

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-DES-034 |
| Title | Iceberg Catalog Committer Specification |
| Version | 1.0 |
| Level | **L3 — Detailed Design Specification** |
| Subsystem Covered | Columnar ELT — Lakehouse Catalog Commit |
| Status | Approved for Engineering |
| Classification | Internal / Engineering Confidential |
| Owner | Principal Engineer (Stream Processing / Lakehouse) |
| Required Reviewers | Chief Architect, Principal Engineer (Storage), Data Platform Lead, Security Lead, FinOps Lead |
| Depends On | KEI-ARC-023 (Columnar ELT), KEI-ARC-025 (Security), KEI-ARC-026 (DR), KEI-DES-030 (WAL), KEI-DES-033 (Schema Registry), KEI-DES-036 (Encryption & Key Management) |
| Consumed By | ELT compactor, lakehouse query engines, DR reconciler, operations runbooks, validation suite |
| Keywords | MUST, MUST NOT, SHOULD, SHOULD NOT, MAY per RFC 2119 |

---

## 2. Purpose, Scope, and Audience

### 2.1 Purpose

This document specifies the **Apache Iceberg Catalog Committer** subsystem of the Polymorphic Event Fabric. It defines how sealed, aggregated Parquet files produced by the Internalized Columnar ELT pipeline are registered into Iceberg tables with safe concurrency, idempotent commits, snapshot lifecycle control, manifest hygiene, orphan-file cleanup, schema evolution coordination, and compliance-aware erasure.

It implements:

- ADR-040: Internalized Columnar ELT.
- ADR-043: Shared tenant Iceberg tables.
- ADR-044: Default lakehouse freshness ≤60 seconds, fast mode ≤5 seconds.
- ADR-045: Small-file aggregation before object upload.
- KEI-ARC-023 lakehouse integration requirements.

### 2.2 Scope

**In scope:**

- Iceberg table model and naming.
- Partition specification.
- Parquet file contract for lakehouse registration.
- Commit batching and freshness control.
- Atomic catalog commit protocol.
- Commit ledger and idempotence.
- Snapshot lifecycle and expiration.
- Manifest compaction.
- Orphan file cleanup.
- Schema evolution coordination with KEI-DES-033.
- Erasure, legal hold, and destroyed-key interaction.
- Failure handling and reconciliation.
- Security, audit, and observability requirements.

**Out of scope:**

- Row shredding and schema inference — owned by KEI-DES-033.
- WAL persistence — owned by KEI-DES-030.
- Object storage upload mechanics — owned by KEI-ARC-020.
- Query engine internals such as DuckDB, Spark, Trino, or Polars.
- Encryption key management internals — owned by KEI-DES-036.

### 2.3 Audience

- Lakehouse integration engineers.
- ELT compaction engineers.
- Catalog and metadata platform engineers.
- SRE and DR engineers.
- Security and compliance engineers.
- Test engineers validating lakehouse correctness.

---

## 3. Design Principles

| ID | Principle | Rationale |
|---|---|---|
| IC-1 | **No file becomes queryable without a successful catalog commit.** | Prevents orphaned or partially visible data. |
| IC-2 | **No source chunk is considered lakehouse-registered until the commit ledger confirms.** | Prevents premature Tier-0 truncation. |
| IC-3 | **Commits are idempotent.** | Crash recovery and retries MUST NOT create duplicate visibility. |
| IC-4 | **Catalog concurrency is atomic.** | Concurrent committers MUST NOT corrupt table metadata. |
| IC-5 | **Freshness is policy-controlled, not absolute.** | Commit cadence balances latency, S3 API cost, and catalog load. |
| IC-6 | **Small files are prevented at the source.** | The committer receives target-size Parquet files, not raw micro-files. |
| IC-7 | **Metadata hygiene is continuous.** | Snapshots, manifests, and orphan files MUST be maintained automatically. |
| IC-8 | **Erasure is coordinated with cryptography.** | Iceberg commit metadata MUST respect destroyed-key and legal-hold state. |

---

## 4. Committer Position in the ELT Pipeline

```
Sealed row segments
        │
        ▼
Adaptive shredding / Arrow RecordBatches
        │
        ▼
Parquet encoding
        │
        ▼
Small-file aggregation to 64–128 MB target files
        │
        ▼
Object storage upload
        │
        ▼
┌─────────────────────────────────────────────────────┐
│          ICEBERG CATALOG COMMITTER (this doc)       │
│                                                     │
│  File validation → Commit batching → Catalog commit │
│  Commit ledger → Snapshot lifecycle → Maintenance   │
└──────────────┬──────────────────────┬───────────────┘
               │                      │
               ▼                      ▼
       Iceberg Catalog         Commit Ledger / Metadata Raft
       REST / Glue / JDBC      Durable commit proof
```

### 4.1 Inputs

| Input | Source | Contract |
|---|---|---|
| Target-size Parquet files | ELT aggregator | 64–128 MB target; schema fingerprint embedded. |
| File statistics | Parquet encoder | Record count, column stats, checksums. |
| Schema version | KEI-DES-033 | Active schema ID/version/fingerprint. |
| Freshness policy | Control plane | Default or fast commit mode. |
| Erasure tombstones | KEI-DES-036 / KEI-ARC-025 | Destroyed-key and legal-hold state. |

### 4.2 Outputs

| Output | Consumer | Contract |
|---|---|---|
| Iceberg snapshots | Query engines | Consistent queryable table state. |
| Commit ledger entries | Storage engine / DR | Proof of lakehouse registration. |
| Maintenance actions | Operations | Manifest compaction, snapshot expiry, orphan cleanup. |
| Quarantine events | SRE | Failed or unrecoverable commit batches. |

---

## 5. Iceberg Table Model

### 5.1 Default Shared Tenant Table

Per ADR-043, the default table is one shared table per tenant:

```text
catalog:    keirox
namespace:  tenant_{tenant_id}
table:      events
```

Full identifier:

```text
keirox.tenant_{tenant_id}.events
```

### 5.2 Optional Dedicated Tables

Dedicated tables MAY be created for:

- Regulated streams requiring isolated lifecycle.
- Very high-throughput streams.
- Streams with unique retention or residency constraints.

Dedicated table identifier:

```text
keirox.tenant_{tenant_id}.stream_{stream_bucket_or_stream_name}
```

**Normative rule:** Per-stream tables MUST NOT be the default. They require explicit policy approval because they can explode catalog metadata at high stream cardinality.

### 5.3 Table Schema

The base Iceberg schema MUST include the system columns defined in KEI-DES-033:

| Column | Iceberg Type | Notes |
|---|---|---|
| `_keirox_stream_id` | `fixed[16]` | Stream UUID. |
| `_keirox_offset` | `long` | Logical offset. |
| `_keirox_ingest_time` | `timestamp_ns` | Ingress timestamp. |
| `_keirox_entity_key` | `string` | Optional. |
| `_keirox_schema_id` | `int` | Schema ID. |
| `_keirox_schema_version` | `int` | Schema version. |
| `_unstructured_payload` | `binary` | Dynamic/unshredded fields. |

Shredded business columns are added as nullable Iceberg columns.

### 5.4 Partition Specification

Default partition spec:

```text
event_date    = day(_keirox_ingest_time)
stream_bucket = bucket(128, _keirox_stream_id)
```

**Normative rules:**

- Partitioning MUST avoid unbounded partition cardinality.
- `stream_id` MUST NOT be used as an identity partition column by default.
- Bucket count SHOULD be tunable per tenant based on scale.
- Partition evolution MAY be used to change bucket count without rewriting all historical data.

### 5.5 Table Properties

Default table properties:

```text
write.format.default                  = parquet
write.parquet.compression-codec       = zstd
write.target-file-size-bytes          = 134217728
write.parquet.row-group-size-bytes    = 67108864
write.metadata.delete-after-commit.enabled = true
write.metadata.previous-versions-max  = 100
commit.retry.num-retries              = 5
commit.retry.min-wait-ms              = 100
commit.retry.max-wait-ms              = 5000
```

---

## 6. Parquet File Contract

### 6.1 File Requirements

A Parquet file is eligible for Iceberg commit only if:

1. File size is within target range, default 64–128 MB.
2. File checksum is valid.
3. Schema fingerprint is present and resolvable.
4. Tenant and stream metadata are present.
5. Encryption metadata is present when encryption is required.
6. File has been successfully uploaded to object storage.
7. File is not associated with a tombstoned or legally held stream unless policy allows.

### 6.2 File Naming

Object key layout:

```text
s3://{lakehouse_bucket}/
  {hash_prefix}/
  tenant_{tenant_id}/
  events/
  data/
  event_date={YYYY-MM-DD}/
  stream_bucket={bucket_id}/
  {commit_batch_id}/
  {file_uuid}.parquet
```

Where:

```text
hash_prefix = first 2 hex digits of xxh3_64(tenant_id + table_id + date)
file_uuid   = UUIDv7
```

**Normative rules:**

- File names MUST be globally unique.
- File names MUST NOT contain tenant-sensitive business identifiers.
- The hash prefix SHOULD be used to distribute S3 request load.

### 6.3 File Sorting

Within each Parquet file, records SHOULD be sorted by:

```text
_keirox_stream_id ASC
_keirox_offset ASC
```

This improves predicate pushdown, range reads, and stream-level locality.

### 6.4 File Statistics

The committer MUST extract and register:

```text
record_count
file_size_in_bytes
column_sizes
value_counts
null_value_counts
lower_bounds
upper_bounds
```

**Security rule:** Lower and upper bounds MUST be suppressed for columns tagged `PII`, `PCI`, or `CONFIDENTIAL` unless an explicit policy allows bounded statistics.

---

## 7. Commit Pipeline

### 7.1 High-Level Commit Flow

```text
1. Receive eligible Parquet file set
2. Validate files, schema, encryption, erasure state
3. Assign commit_id
4. Stage commit in Commit Ledger
5. Build Iceberg DataFile metadata
6. Create or append manifest file
7. Create new Iceberg snapshot
8. Submit atomic catalog commit
9. On success, mark ledger COMMITTED
10. Emit registration confirmation
11. Trigger maintenance if thresholds are crossed
```

### 7.2 Commit Identifier

Each commit batch uses a globally unique identifier:

```text
commit_id = UUIDv7
```

The commit ID MUST be embedded in the Iceberg snapshot summary:

```text
keirox.commit_id = {commit_id}
```

### 7.3 Commit Batch Structure

```rust
pub struct IcebergCommitRequest {
    pub tenant_id: u64,
    pub table_id: String,
    pub commit_id: Uuid,
    pub freshness_mode: FreshnessMode,
    pub schema_id: u32,
    pub schema_version: u32,
    pub schema_fingerprint_xxh3: u64,
    pub schema_fingerprint_sha256: [u8; 32],
    pub source_chunk_ids: Vec<u64>,
    pub files: Vec<CommittedFile>,
    pub created_at_ms: u64,
}
```

### 7.4 Committed File Structure

```rust
pub struct CommittedFile {
    pub file_path: String,
    pub file_format: FileFormat,
    pub record_count: u64,
    pub file_size_bytes: u64,
    pub partition_values: PartitionValues,
    pub column_stats: ColumnStats,
    pub checksum_sha256: [u8; 32],
    pub encryption_dek_id: Option<u64>,
    pub source_chunk_ids: Vec<u64>,
}
```

---

## 8. Commit Ledger and Idempotence

### 8.1 Purpose

The Commit Ledger provides durable proof of which file sets have been registered in Iceberg. It enables crash recovery, reconciliation, and prevention of duplicate commits.

### 8.2 Ledger Entry

```rust
pub struct CommitLedgerEntry {
    pub commit_id: Uuid,
    pub tenant_id: u64,
    pub table_id: String,
    pub status: CommitStatus,
    pub snapshot_id: Option<u64>,
    pub schema_id: u32,
    pub schema_version: u32,
    pub file_count: u32,
    pub record_count: u64,
    pub total_bytes: u64,
    pub source_chunk_ids: Vec<u64>,
    pub staged_at_ms: u64,
    pub committed_at_ms: Option<u64>,
    pub retry_count: u32,
    pub last_error: Option<String>,
}
```

### 8.3 Commit Status

| Status | Meaning |
|---|---|
| `STAGED` | Files validated and commit prepared. |
| `COMMITTING` | Catalog commit in progress. |
| `COMMITTED` | Catalog commit confirmed. |
| `FAILED` | Commit failed after retries. |
| `QUARANTINED` | Files isolated for operator review. |
| `ORPHANED` | Files uploaded but never committed; cleanup eligible. |

### 8.4 Ledger Replication

**Normative rules:**

- The Commit Ledger MUST be replicated via the Metadata & State Raft plane.
- A commit MUST NOT be reported as successful until both the Iceberg catalog commit and ledger commit are confirmed.
- If the catalog commit succeeds but ledger commit fails, reconciliation MUST detect the commit from Iceberg snapshot summary and mark the ledger committed.

### 8.5 Idempotence Rules

- A `commit_id` MUST NOT be committed twice.
- If a retry observes an existing Iceberg snapshot with the same `keirox.commit_id`, the committer MUST treat the commit as successful.
- Source chunk registration MUST be idempotent.
- File uploads MUST use unique names so retries do not overwrite existing files.

---

## 9. Catalog Abstraction and Concurrency

### 9.1 Supported Catalog Backends

| Catalog | Support Level | Notes |
|---|---|---|
| Iceberg REST Catalog | Preferred | Atomic commit, multi-engine friendly. |
| AWS Glue Catalog | Supported | Requires optimistic locking behavior. |
| JDBC Catalog | Supported | Requires transactional commit table. |
| Hadoop Catalog on S3 | Not recommended | Unsafe for concurrent commits without external lock. |

**Normative rule:** Production deployments MUST use a catalog backend that provides atomic table metadata commits. HadoopCatalog on object storage MUST NOT be used for concurrent writers unless protected by an external distributed lock.

### 9.2 Atomic Commit Protocol

```text
1. Read current table metadata and base snapshot_id
2. Build new snapshot from base snapshot
3. Append manifest list / manifest files
4. Submit commit with base snapshot expectation
5. If catalog reports conflict:
     a. Refresh table metadata
     b. Rebase additive changes onto latest snapshot
     c. Retry
6. If retry limit exceeded:
     a. Mark commit FAILED
     b. Quarantine if unrecoverable
```

### 9.3 Conflict Handling

| Conflict Type | Resolution |
|---|---|
| Concurrent additive append | Rebase and retry. |
| Concurrent schema evolution | Validate compatibility; retry if compatible. |
| Concurrent partition spec change | Revalidate file partition values; retry if compatible. |
| Table deleted during commit | Abort and quarantine. |
| Catalog unavailable | Retry with backoff; fail if unavailable beyond SLA. |
| Repeated conflicts | Increase commit batch size and reduce commit frequency. |

### 9.4 Retry Policy

```text
max_retries = 5
initial_backoff = 100 ms
max_backoff = 5,000 ms
backoff_multiplier = 2
jitter = full jitter
```

---

## 10. Snapshot Lifecycle

### 10.1 Snapshot Creation

Each successful commit creates an Iceberg snapshot with:

```text
operation = append
```

### 10.2 Snapshot Summary Fields

The snapshot summary MUST include:

```text
keirox.commit_id
keirox.tenant_id
keirox.table_id
keirox.schema_id
keirox.schema_version
keirox.schema_fingerprint_xxh3_64
keirox.freshness_mode
keirox.file_count
keirox.record_count
keirox.total_bytes
keirox.source_chunk_min_id
keirox.source_chunk_max_id
```

### 10.3 Snapshot Retention Policy

Default policy:

| Parameter | Default |
|---:|---|
| Minimum retained snapshots | 20 |
| Maximum snapshot age | 7 days |
| Maximum snapshots | 1,000 |
| Legal hold override | Suspend expiration |

**Normative rules:**

- Snapshot expiration MUST NOT remove snapshots required by legal hold.
- Snapshot expiration MUST NOT remove snapshots needed for active DR/PITR windows.
- Snapshot expiration MUST be audited.

### 10.4 Snapshot Expiration Procedure

```text
1. Identify snapshots older than retention horizon
2. Exclude snapshots under legal hold
3. Exclude snapshots required by DR/PITR policy
4. Expire snapshots
5. Remove unreferenced metadata files
6. Record audit event
```

---

## 11. Manifest Management

### 11.1 Manifest Creation

The committer SHOULD append new manifest files rather than rewriting existing manifests on every commit.

### 11.2 Manifest Compaction Triggers

Manifest compaction SHOULD run when:

```text
manifest_count > max_manifest_count
OR deleted_entry_ratio > 0.20
OR manifest_metadata_size > threshold
OR maintenance schedule triggered
```

Defaults:

| Parameter | Default |
|---:|---|
| `max_manifest_count` | 100 |
| `deleted_entry_ratio` | 0.20 |
| `maintenance_interval` | 6 hours |

### 11.3 Manifest Compaction Rules

- Manifest compaction MUST preserve all active data file references.
- Manifest compaction MUST NOT delete files.
- Manifest compaction MUST be audited.
- Manifest compaction MAY be deferred under high commit load.

---

## 12. Orphan File Cleanup

### 12.1 Definition

An orphan file is an object in the table storage path that is not referenced by any active Iceberg snapshot and is not protected by a pending commit.

Common orphan sources:

1. File uploaded but commit failed.
2. Commit succeeded but ledger failed before confirmation.
3. Snapshot expiration removed references.
4. Quarantined commit abandoned.

### 12.2 Orphan Grace Period

Default:

```text
orphan_grace_period = 24 hours
```

**Normative rule:** The orphan cleaner MUST NOT delete files younger than the grace period unless they are explicitly marked orphaned by the Commit Ledger.

### 12.3 Orphan Cleanup Procedure

```text
1. List objects under table data path
2. Compare against active snapshot file references
3. Compare against pending commit ledger entries
4. Exclude objects younger than grace period
5. Exclude objects under legal hold or active quarantine
6. Delete eligible orphan objects
7. Emit metrics and audit events
```

### 12.4 Safety Rules

- Orphan cleanup MUST be dry-run capable.
- Orphan cleanup MUST be rate-limited.
- Orphan cleanup MUST NOT run if catalog metadata is unhealthy.
- Orphan cleanup MUST NOT delete metadata files required by active snapshots.

---

## 13. Schema Evolution Integration

### 13.1 Commit-Time Schema Validation

Before committing a file set, the committer MUST:

1. Resolve schema fingerprint via KEI-DES-033.
2. Confirm the schema is `ACTIVE` or compatible.
3. Map Keirox field IDs to Iceberg field IDs.
4. Verify all shredded columns exist in target Iceberg schema.

### 13.2 Schema Evolution Procedure

If the file set contains columns not present in the Iceberg table schema:

```text
1. Validate compatibility mode (default BACKWARD)
2. Add new columns as nullable
3. Preserve stable field IDs
4. Commit schema change atomically with data commit if catalog supports it
5. If atomic schema+data commit is unsupported:
     a. Commit schema evolution first
     b. Commit data snapshot second
     c. Record both in commit ledger
```

### 13.3 Unsafe Schema Changes

Unsafe changes include:

- Removing a required column.
- Changing a type incompatibly.
- Renaming a field without alias support.
- Changing field ID mapping.

**Normative rule:** Unsafe schema changes MUST NOT be auto-applied. They MUST create a new column or require explicit administrative migration.

### 13.4 Evolution Failure Behavior

If schema evolution fails:

1. Do not commit incompatible files.
2. If possible, reroute excess fields to `_unstructured_payload`.
3. If rerouting is not possible, quarantine the file set.
4. Emit alert and audit event.

---

## 14. Freshness Controller

### 14.1 Freshness Modes

| Mode | Target Freshness | Commit Interval | Commit Size Target |
|---|---:|---:|---:|
| Default | ≤60 seconds | 60 seconds | 128 MB |
| Fast | ≤5 seconds | 5 seconds | 16 MB |
| Cost-optimized | ≤5 minutes | 5 minutes | 256 MB |

### 14.2 Commit Trigger Logic

A commit is triggered when:

```text
now - last_commit_time >= max_commit_interval
OR pending_bytes >= max_commit_bytes
OR pending_files >= max_pending_files
```

Defaults for default mode:

```text
max_commit_interval = 60 seconds
max_commit_bytes = 128 MB
max_pending_files = 32
```

### 14.3 Adaptive Behavior

| Condition | Action |
|---|---|
| Catalog commit latency rising | Increase batch size, reduce commit frequency. |
| S3 API throttling | Increase batch size, add jitter, alert. |
| Snapshot count rising too fast | Increase commit interval. |
| Query freshness SLO at risk | Decrease commit interval if cost policy allows. |
| Tenant cost limit exceeded | Switch to cost-optimized mode. |

**Normative rule:** Fast mode MUST NOT be enabled by default for all tenants because it increases catalog and object-storage API load.

---

## 15. Erasure, Legal Hold, and Compliance

### 15.1 Interaction with Crypto-Shredding

When a stream or tenant erasure request is received:

1. KEI-DES-036 destroys the relevant DEK/KEK.
2. A destroyed-key tombstone is recorded.
3. The committer MUST block new commits for the tombstoned stream.
4. Existing Iceberg file references remain as ciphertext references unless physical metadata purge is required.
5. Query access MUST fail or return no accessible rows because decryption keys are destroyed.

### 15.2 Compliance Modes

| Mode | Behavior |
|---|---|
| Standard | Crypto-shredding is sufficient; metadata may remain temporarily. |
| Strict metadata purge | Committer removes file references through manifest rewrite and snapshot expiration. |
| Legal hold | No expiration, orphan cleanup, or physical purge. |

### 15.3 Legal Hold Rules

**Normative rules:**

- Legal hold MUST suspend snapshot expiration.
- Legal hold MUST suspend orphan file deletion.
- Legal hold MUST suspend manifest rewrites that remove referenced files.
- Legal hold changes MUST be audited.

### 15.4 Metadata Privacy

Iceberg metadata and snapshot summaries MUST NOT contain:

- Customer payload data.
- Sensitive entity keys unless explicitly permitted.
- PII/PCI column lower/upper bounds.
- Raw unstructured payload content.

---

## 16. Failure Handling and Recovery

### 16.1 Failure Matrix

| Failure | Detection | Recovery |
|---|---|---|
| File upload failed | Storage uploader error | Retry upload; do not stage commit. |
| File checksum mismatch | Validation stage | Quarantine file; alert. |
| Catalog unavailable | Commit timeout | Retry with backoff; keep ledger staged. |
| Catalog conflict | Commit rejection | Rebase and retry. |
| Schema evolution failure | Pre-commit validation | Quarantine or reroute to `_unstructured_payload`. |
| Commit succeeded, ledger failed | Reconciliation | Read snapshot summary and mark ledger committed. |
| Commit failed after file upload | Commit ledger | Mark files orphaned after grace period. |
| Snapshot expiration failure | Maintenance error | Retry later; alert if repeated. |
| Orphan cleaner uncertainty | Reconciliation mismatch | Dry-run only; require operator approval. |
| Legal hold conflict | Policy check | Block destructive maintenance. |

### 16.2 Quarantine

Quarantined files are moved or marked under:

```text
s3://{lakehouse_bucket}/{hash_prefix}/tenant_{tenant_id}/events/_quarantine/
```

Quarantine entries MUST include:

```text
commit_id
reason_code
source_chunk_ids
file_checksum
created_timestamp
operator_notes
```

### 16.3 Reconciliation Procedure

```text
1. Read Commit Ledger entries in STAGED or COMMITTING state
2. Query latest Iceberg snapshots
3. Match snapshots by keirox.commit_id
4. If commit_id exists:
     mark ledger COMMITTED
5. If commit_id absent after retry window:
     mark ledger FAILED or ORPHANED
6. Emit reconciliation metrics
```

---

## 17. Security and Authorization

### 17.1 Committer Service Principal

The committer runs as a system service principal with least privileges:

```text
lakehouse.table.read
lakehouse.table.write
lakehouse.catalog.commit
object.read
object.write
object.delete_orphan
kms.decrypt
kms.key_read
audit.write
```

### 17.2 Authorization Rules

- The committer MUST NOT write across tenant boundaries.
- The committer MUST validate residency constraints before committing.
- The committer MUST reject files from unauthorized streams.
- The committer MUST validate destroyed-key tombstones before commit.

### 17.3 Audit Events

The following events MUST be audited:

- Commit staged.
- Commit succeeded.
- Commit failed.
- Commit quarantined.
- Snapshot expired.
- Manifest compacted.
- Orphan files deleted.
- Legal hold enforced.
- Erasure tombstone applied.
- Schema evolution committed.

---

## 18. Observability

### 18.1 Metrics

| Metric | Type | Description |
|---|---|---|
| `keirox_iceberg_snapshot_age_seconds` | Gauge | Time since latest successful snapshot per tenant table. |
| `keirox_iceberg_commit_latency_seconds` | Histogram | Commit duration. |
| `keirox_iceberg_commit_success_total` | Counter | Successful commits. |
| `keirox_iceberg_commit_errors_total` | Counter | Failed commits by reason. |
| `keirox_iceberg_commit_conflicts_total` | Counter | Catalog commit conflicts. |
| `keirox_iceberg_pending_files_bytes` | Gauge | Bytes awaiting commit. |
| `keirox_iceberg_pending_files_count` | Gauge | Files awaiting commit. |
| `keirox_iceberg_quarantined_files_total` | Counter | Files quarantined. |
| `keirox_iceberg_orphan_files_count` | Gauge | Detected orphan files. |
| `keirox_iceberg_orphan_cleanup_total` | Counter | Orphan files deleted. |
| `keirox_iceberg_manifest_count` | Gauge | Active manifests per table. |
| `keirox_iceberg_snapshot_count` | Gauge | Retained snapshots per table. |

### 18.2 Alerts

| Alert | Condition | Severity |
|---|---|---|
| Lakehouse freshness SLO breach | Snapshot age > freshness target | Warning/Critical |
| Commit conflict storm | Conflict rate > threshold | Warning |
| Catalog unavailable | Repeated commit failures | Critical |
| Orphan backlog growing | Orphan count increasing over time | Warning |
| Quarantine backlog | Quarantined files unresolved > 24h | Critical |
| Legal hold violation attempt | Blocked destructive maintenance | Critical |

---

## 19. NFR Traceability

| NFR | Requirement | How This Specification Satisfies It |
|---|---|---|
| PERF-030 | Default lakehouse freshness ≤60s | Freshness Controller default mode (§14). |
| PERF-031 | Fast-mode freshness ≤5s | Fast-mode commit policy (§14). |
| DUR | No premature data registration loss | Commit ledger and reconciliation (§8, §16). |
| OPS | Lakehouse observability | Metrics and alerts (§18). |
| SEC | Metadata privacy and authorization | ABAC, audit, stats suppression (§15, §17). |
| COMP | Erasure and legal hold | Crypto-shredding coordination and legal-hold suspension (§15). |
| AVAIL | Commit recovery | Idempotence, retries, quarantine, reconciliation (§8, §9, §16). |

---

## 20. Interfaces

### 20.1 Provided Interfaces

| Interface | Consumer | Semantics |
|---|---|---|
| `commitFileSet(request)` | ELT aggregator | Register a validated Parquet file set. |
| `getCommitStatus(commit_id)` | Control plane | Return commit ledger status. |
| `getTableStatus(table_id)` | Observability / admin | Return snapshot age, pending bytes, manifest count. |
| `runMaintenance(table_id)` | Scheduler | Run manifest compaction, expiration, orphan scan. |
| `expireSnapshots(policy)` | Admin | Expire snapshots according to policy. |
| `rewriteManifests(table_id)` | Admin | Compact manifests. |
| `removeOrphanFiles(grace_period)` | Admin | Remove eligible orphan files. |
| `applyErasureTombstone(tombstone)` | Security plane | Block commits and coordinate metadata handling. |

### 20.2 Consumed Interfaces

| Interface | Provider | Purpose |
|---|---|---|
| Parquet file metadata | ELT pipeline | Commit inputs. |
| Schema registry | KEI-DES-033 | Schema resolution and evolution. |
| Object storage | S3/GCS/Azure Blob | File persistence. |
| Iceberg catalog | REST/Glue/JDBC | Atomic table metadata commits. |
| Metadata Raft | KEI-ARC-022 | Commit ledger durability. |
| KMS / destroyed-key registry | KEI-DES-036 | Erasure and encryption state. |
| Audit sink | KEI-ARC-025 | Governance events. |

---

## 21. Open Questions

| Item | Status | Resolution Path |
|---|---|---|
| Default catalog backend | Open | Evaluate REST catalog vs. Glue for target clouds. |
| Partition bucket count | Open | Benchmark query and commit overhead per tenant scale. |
| Parquet modular encryption for metadata | Open | Evaluate for strict compliance deployments. |
| Row-level delete strategy for unencrypted mode | Open | Requires ADR; compliance mode should mandate encryption. |
| Commit ledger retention window | Open | Align with DR/PITR policy. |
| Iceberg v3 feature adoption | Open | Evaluate when ecosystem support matures. |

---

## 22. Glossary

| Term | Definition |
|---|---|
| Commit ID | Unique identifier for a Keirox Iceberg commit batch. |
| Commit Ledger | Durable log of staged, committed, failed, and orphaned commit batches. |
| Snapshot | An immutable Iceberg table version. |
| Manifest | Iceberg metadata file listing data files. |
| Orphan File | An object not referenced by any active snapshot or pending commit. |
| Freshness Mode | Policy controlling commit interval and lakehouse visibility latency. |
| Quarantine | Protected area for files that cannot be safely committed. |
| Legal Hold | Compliance state suspending destructive lifecycle operations. |

---

## 23. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial Iceberg Catalog Committer specification. Defines shared tenant table model, partitioning, Parquet contract, commit batching, atomic catalog commit, commit ledger, idempotence, snapshot lifecycle, manifest compaction, orphan cleanup, schema evolution, erasure/legal-hold coordination, failure recovery, security, and observability. Implements ADR-040/043/044/045. |