# KEI-LAKE-301 — Lakehouse Iceberg Certification Plan

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-LAKE-301 |
| Title | Lakehouse Iceberg Certification Plan |
| Version | 1.0 |
| Level | Engineering Execution Plan |
| Status | Baseline — Ready for Execution |
| Phase | Phase 3 — Ecosystem Compatibility Gateways & Lakehouse |
| Duration | Weeks 8–32 of Phase 3 |
| Owner | Lakehouse Engineering Lead / Data Platform Lead |
| Governing Plan | KEI-ENG-300 — Phase 3 Engineering Execution Plan |
| Governing Architecture Documents | KEI-ARC-023, KEI-DES-033, KEI-DES-034, KEI-ARC-027 |
| Predecessor | KEI-SPIKE-301 — Ecosystem Gateway & Lakehouse Prototype Plan |
| Next Plan File | KEI-SDK-301 — Native SDK & Developer Experience Plan |

---

## 2. Executive Summary

The lakehouse pillar of Keirox is not a marketing claim. It must be proven through repeatable certification.

This plan defines how Keirox will certify that sealed event data becomes **governed, queryable, fresh, and operationally safe Apache Iceberg tables** without requiring an external ETL pipeline.

The certification validates:

1. **Commit correctness** — commits are idempotent, durable, ledger-backed, and recoverable.
2. **Freshness governance** — event-to-query latency is measured and bounded by mode.
3. **File hygiene** — small files, manifests, snapshots, and orphan files are controlled.
4. **Query engine readiness** — DuckDB, Polars, and Spark can query committed tables correctly.
5. **Schema evolution safety** — historical data remains readable after schema changes.
6. **Operational safety** — conflicts, quarantine, maintenance, and erasure hooks behave safely.

This plan replaces all informal “zero-ETL” and “instant queryability” claims with measurable, evidence-based certification gates.

---

## 3. Purpose and Scope

### 3.1 Purpose

The purpose of this plan is to:

1. Define the Iceberg certification model for Phase 3.
2. Define commit pipeline correctness requirements.
3. Define freshness measurement methodology.
4. Define file and metadata hygiene thresholds.
5. Define query engine conformance tests.
6. Define schema evolution certification tests.
7. Define operational and failure-mode validation.
8. Produce the Phase 3 lakehouse evidence package.

### 3.2 Scope

**In scope:**

- Apache Iceberg commit ledger certification.
- Snapshot creation and validation.
- Manifest management and compaction.
- Orphan file detection and cleanup.
- Snapshot expiration governance.
- Freshness benchmarking.
- File size and small-file governance.
- DuckDB query certification.
- Polars query certification.
- Spark query certification.
- Schema evolution certification.
- Commit conflict and quarantine behavior.
- Committer crash recovery.
- Lakehouse observability validation.

**Out of scope:**

- Full KMS envelope encryption production rollout — Phase 4.
- Multi-region replication — Phase 4.
- Delta Lake certification — future phase.
- Hudi certification — future phase.
- Full SQL query engine optimization — not required for Phase 3.
- In-broker SQL or materialized views — excluded from v1.

---

## 4. Certification Principles

| ID | Principle | Requirement |
|---|---|---|
| LAKE-1 | No commit without ledger | Every Iceberg commit MUST be recorded in the durable commit ledger. |
| LAKE-2 | No duplicate visibility | Restart or retry MUST NOT create duplicate snapshots or duplicate data visibility. |
| LAKE-3 | No orphan deletion of active files | Orphan cleanup MUST be safe, grace-period bounded, and dry-run capable. |
| LAKE-4 | Freshness is conditional | Freshness targets MUST be reported by mode and workload, not as universal SLAs. |
| LAKE-5 | Historical readability is mandatory | Schema evolution MUST NOT break historical reads. |
| LAKE-6 | File hygiene is part of correctness | Small-file explosion is a defect, not an operational inconvenience. |
| LAKE-7 | Query engines are evidence, not assumptions | DuckDB/Polars/Spark support MUST be proven by tests. |
| LAKE-8 | Maintenance must be governed | Snapshot expiration, manifest rewriting, and orphan cleanup MUST be auditable. |

---

## 5. Iceberg Table Model Certification

### 5.1 Default Table Model

The default certified table model is:

```text
catalog:    keirox
namespace:  tenant_{tenant_id}
table:      events
```

Full identifier:

```text
keirox.tenant_{tenant_id}.events
```

### 5.2 Default Partition Specification

```text
event_date    = day(_keirox_ingest_time)
stream_bucket = bucket(128, _keirox_stream_id)
```

### 5.3 Required System Columns

| Column | Type | Purpose |
|---|---|---|
| `_keirox_stream_id` | fixed[16] | Stream identity |
| `_keirox_offset` | long | Logical offset |
| `_keirox_ingest_time` | timestamp_ns | Ingress timestamp |
| `_keirox_entity_key` | string | Entity key if present |
| `_keirox_schema_id` | int | Schema ID |
| `_keirox_schema_version` | int | Schema version |
| `_unstructured_payload` | binary | Unshredded/polymorphic payload |

### 5.4 Optional Dedicated Tables

Dedicated stream tables are not default. They MAY be certified only when:

1. Tenant policy explicitly requests isolation.
2. Stream volume justifies separate lifecycle.
3. Catalog metadata impact is reviewed.
4. Architecture Review Board approves the exception.

---

## 6. Commit Pipeline Certification

### 6.1 Commit Ledger Requirements

The commit ledger MUST prove:

| ID | Requirement |
|---|---|
| COMMIT-001 | Every commit has a unique `commit_id`. |
| COMMIT-002 | Every commit records source chunk IDs. |
| COMMIT-003 | Every commit records schema ID/version/fingerprint. |
| COMMIT-004 | Every commit records file count and byte count. |
| COMMIT-005 | Commit state transitions are durable. |
| COMMIT-006 | Commit replay after restart is idempotent. |
| COMMIT-007 | Committed snapshots are discoverable by ledger `commit_id`. |
| COMMIT-008 | Failed commits are marked FAILED or QUARANTINED. |

### 6.2 Commit State Machine

```text
STAGED
  ↓
COMMITTING
  ↓
COMMITTED
  ↓
REGISTERED
```

Failure paths:

```text
STAGED → FAILED
COMMITTING → FAILED
COMMITTING → QUARANTINED
COMMITTED → REGISTERED
```

### 6.3 Commit Correctness Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| LAKE-T-001 | Normal commit batch | Snapshot created and queryable. |
| LAKE-T-002 | Committer restart before catalog commit | No duplicate snapshot after restart. |
| LAKE-T-003 | Committer restart after catalog commit but before ledger update | Reconciliation marks ledger committed. |
| LAKE-T-004 | Corrupted commit ledger entry | Entry rejected; alert emitted; no partial commit. |
| LAKE-T-005 | Catalog conflict | Retry/rebase or quarantine; no corruption. |
| LAKE-T-006 | S3 upload failure before commit | Batch remains staged; no orphan snapshot. |
| LAKE-T-007 | Partial file set upload | Commit not issued; retry or quarantine. |
| LAKE-T-008 | Replay of committed batch | Idempotent; no duplicate data. |

### 6.4 Commit Performance Targets

| Metric | Mandatory Target | Stretch Target |
|---|---:|---:|
| Commit operation latency p95 | ≤5 seconds | ≤3 seconds |
| Commit operation latency p99 | ≤15 seconds | ≤8 seconds |
| Commit success rate under normal conditions | ≥99.9% | ≥99.99% |
| Commit ledger replay time | ≤30 seconds for 10k commits | ≤10 seconds |
| Quarantine rate under normal load | ≤0.1% | 0% |

---

## 7. Freshness Certification

### 7.1 Freshness Definition

Freshness is defined as:

```text
Freshness = time query engine can read event - time event was ingested
```

Measurement MUST include:

1. Event ingress timestamp.
2. Chunk seal timestamp.
3. Commit batch timestamp.
4. Iceberg snapshot commit timestamp.
5. Query engine read timestamp.

### 7.2 Freshness Modes

| Mode | Target | Conditions |
|---|---:|---|
| Default | ≤60 seconds p95 | Standard commit cadence |
| Fast | ≤5 seconds p95 | Tuned, low-load deployment |
| Cost-optimized | ≤5 minutes p95 | High-volume, cost-sensitive deployment |
| Sub-2-second | Lab/stretch only | Not a default or production SLA |

### 7.3 Freshness Test Profiles

| Profile | Workload | Purpose |
|---|---|---|
| FRESH-P1 | 10 MB/s steady ingest | Default freshness validation |
| FRESH-P2 | 100 MB/s steady ingest | Freshness under baseline load |
| FRESH-P3 | Burst 10× for 5 minutes | Freshness recovery after backlog |
| FRESH-P4 | Fast mode, low load | Fast-mode validation |
| FRESH-P5 | Cost-optimized mode | Cost mode validation |
| FRESH-P6 | S3 throttling simulation | Freshness degradation behavior |

### 7.4 Freshness Acceptance Criteria

| ID | Criterion |
|---|---|
| FRESH-ACC-001 | Default mode p95 freshness ≤60 seconds under FRESH-P1 and FRESH-P2. |
| FRESH-ACC-002 | Fast mode p95 freshness ≤5 seconds under FRESH-P4. |
| FRESH-ACC-003 | Cost-optimized mode p95 freshness ≤5 minutes under FRESH-P5. |
| FRESH-ACC-004 | Freshness recovers within two commit intervals after burst backlog. |
| FRESH-ACC-005 | Freshness degradation during S3 throttling is observable and alerted. |

---

## 8. File Hygiene Certification

### 8.1 File Size Governance

Target Parquet file size:

```text
64 MB to 128 MB
```

| Metric | Mandatory Target | Stretch Target |
|---|---:|---:|
| Files within target size range | ≥80% | ≥90% |
| Files below 8 MB | ≤2% | ≤1% |
| Files below 1 MB | 0% after maintenance | 0% |
| Average file size | ≥48 MB | ≥64 MB |

### 8.2 Manifest and Snapshot Hygiene

| Metric | Mandatory Target | Stretch Target |
|---|---:|---:|
| Active manifests per table before maintenance | ≤100 | ≤50 |
| Snapshot count before expiration | ≤1,000 | ≤500 |
| Deleted-data ratio in manifests triggering rewrite | >20% | >15% |
| Orphan files detected | Measured | Near zero after cleanup |
| Orphan cleanup false positives | 0 active files deleted | 0 |

### 8.3 Maintenance Operations

| Operation | Trigger | Certification Requirement |
|---|---|---|
| Snapshot expiration | Age >7 days OR snapshots >1,000 | Must respect legal hold and minimum snapshots |
| Manifest compaction | Manifest count >100 OR deleted ratio >20% | Must preserve active file references |
| Orphan cleanup | Scheduled or manual | Must support dry-run and grace period |
| File compaction | Small-file threshold exceeded | Must preserve query correctness |

### 8.4 Hygiene Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| HYGIENE-T-001 | Sustained ingest for 24 hours | No small-file explosion |
| HYGIENE-T-002 | Snapshot expiration | Old snapshots removed; active reads unaffected |
| HYGIENE-T-003 | Legal hold active | Expiration blocked; audit event emitted |
| HYGIENE-T-004 | Manifest compaction | Manifest count reduced; no missing files |
| HYGIENE-T-005 | Orphan cleanup dry-run | Candidate list produced; no deletion |
| HYGIENE-T-006 | Orphan cleanup execution | Only eligible orphan files deleted |
| HYGIENE-T-007 | File compaction | Query results unchanged before/after |

---

## 9. Query Engine Certification

### 9.1 Certified Query Engines

| Engine | Phase 3 Target | Priority |
|---|---|---:|
| DuckDB | Certified | P0 |
| Polars | Certified | P0 |
| Spark | Certified | P1 |
| Trino | Design/validation only | P2 |

### 9.2 Query Conformance Tests

Each certified engine MUST pass the following test classes.

| Test Class | Description |
|---|---|
| Table discovery | Engine can list and read Iceberg table metadata. |
| Full scan | `SELECT COUNT(*)` matches ingested record count. |
| Column projection | Selecting subset of columns returns correct values. |
| Predicate filtering | Filters on shredded columns return correct rows. |
| Null handling | Missing fields return SQL NULL. |
| Schema evolution | New columns are readable; old rows return NULL. |
| Unstructured payload | `_unstructured_payload` is readable as binary/string. |
| Partition pruning | Queries on event_date/stream_bucket reduce scanned files. |
| Time-range query | Queries by ingest time return correct window. |
| Stream-specific query | Queries by `_keirox_stream_id` return correct stream rows. |

### 9.3 Query Correctness Requirements

| ID | Requirement |
|---|---|
| QUERY-001 | Query results MUST match source record count for committed data. |
| QUERY-002 | Query results MUST NOT include uncommitted data. |
| QUERY-003 | Query results MUST remain stable after snapshot expiration unless data is deleted. |
| QUERY-004 | Query results MUST NOT change due to manifest compaction. |
| QUERY-005 | Query results MUST NOT change due to orphan cleanup. |
| QUERY-006 | Predicate pushdown MUST NOT produce false negatives for shredded columns. |
| QUERY-007 | Schema evolution MUST NOT corrupt historical rows. |

### 9.4 Query Performance Measurement

Query performance is informational in Phase 3, not a mandatory SLA, but MUST be measured.

| Metric | Measurement |
|---|---|
| Query cold start latency | First query against table |
| Query warm latency | Repeated query after metadata cache |
| Count query latency | `SELECT COUNT(*)` |
| Filter query latency | Predicate on shredded column |
| Scan throughput | Rows/sec and MB/sec |
| Partition pruning effectiveness | Files scanned vs total files |

---

## 10. Schema Evolution Certification

### 10.1 Certified Schema Changes

| Change Type | Certification Status |
|---|---|
| Add nullable column | Certified |
| Safe numeric widening | Certified with policy |
| Rename with alias metadata | Certified if field ID stable |
| Deprecate column | Certified; column remains readable |
| Unsafe type change | Requires new schema version and migration |
| Remove column physically | Not certified in Phase 3 unless governed migration |

### 10.2 Schema Evolution Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| SCHEMA-T-001 | Add new nullable field | New field appears; historical rows NULL |
| SCHEMA-T-002 | Int to long widening | Historical values readable; no truncation |
| SCHEMA-T-003 | Int to string conflict | Conflict handled by policy; no corruption |
| SCHEMA-T-004 | Polymorphic field explosion | Excess fields route to `_unstructured_payload` |
| SCHEMA-T-005 | Field cap exceeded | 64-field cap enforced |
| SCHEMA-T-006 | Schema fingerprint mismatch | Commit quarantined or rejected |
| SCHEMA-T-007 | Iceberg field ID mapping | Stable IDs preserved across evolution |
| SCHEMA-T-008 | Historical query after evolution | Old snapshots remain queryable |

### 10.3 Schema Governance Requirements

| ID | Requirement |
|---|---|
| SCHEMA-GOV-001 | Every commit MUST record schema ID/version/fingerprint. |
| SCHEMA-GOV-002 | Iceberg schema evolution MUST be coordinated with Keirox schema registry. |
| SCHEMA-GOV-003 | Unsafe schema changes MUST NOT be auto-applied. |
| SCHEMA-GOV-004 | Schema drift metrics MUST be observable. |
| SCHEMA-GOV-005 | Schema conflict events MUST be audited. |

---

## 11. Failure and Chaos Certification

### 11.1 Committer Failure Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| LAKE-CHAOS-001 | Kill committer during commit | No duplicate snapshot; ledger recovers |
| LAKE-CHAOS-002 | Kill committer during S3 upload | Partial upload cleaned or ignored; no commit |
| LAKE-CHAOS-003 | Catalog unavailable | Commits queued/retried; alert emitted |
| LAKE-CHAOS-004 | S3 throttling during upload | Backoff with jitter; backlog observable |
| LAKE-CHAOS-005 | Corrupted Parquet file before commit | File rejected; quarantine if repeated |
| LAKE-CHAOS-006 | Corrupted manifest metadata | Detected; restore from prior metadata |
| LAKE-CHAOS-007 | Clock skew during commit | Commit timestamps safe; freshness measurement adjusted |
| LAKE-CHAOS-008 | Concurrent maintenance and commit | No corruption; operations serialized safely |

### 11.2 Invariant Checks

During all lakehouse chaos tests, the following invariants MUST hold:

| Invariant | Check |
|---|---|
| No duplicate data visibility | Committed record count remains exact |
| No active file deletion | Orphan cleanup never removes active file |
| No snapshot regression | Snapshot history remains monotonic |
| No schema corruption | Historical snapshots remain readable |
| No ledger divergence | Ledger and catalog remain reconcilable |
| No silent quarantine | Quarantined commits are observable |

---

## 12. Observability Certification

### 12.1 Required Metrics

| Metric | Type | Purpose |
|---|---|---|
| `keirox_iceberg_snapshot_age_seconds` | Gauge | Freshness |
| `keirox_iceberg_commit_latency_seconds` | Histogram | Commit performance |
| `keirox_iceberg_commit_success_total` | Counter | Commit success |
| `keirox_iceberg_commit_errors_total` | Counter | Commit failures |
| `keirox_iceberg_commit_conflicts_total` | Counter | Catalog conflicts |
| `keirox_iceberg_quarantined_commits_total` | Counter | Quarantine events |
| `keirox_iceberg_pending_files_bytes` | Gauge | Commit backlog |
| `keirox_iceberg_pending_files_count` | Gauge | Commit backlog |
| `keirox_iceberg_orphan_files_count` | Gauge | Orphan hygiene |
| `keirox_iceberg_small_file_count` | Gauge | File hygiene |
| `keirox_iceberg_manifest_count` | Gauge | Manifest hygiene |
| `keirox_iceberg_snapshot_count` | Gauge | Snapshot hygiene |
| `keirox_iceberg_schema_conflicts_total` | Counter | Schema governance |

### 12.2 Required Alerts

| Alert | Condition | Severity |
|---|---|---|
| Freshness SLO breach | Snapshot age > mode target | Warning/Critical |
| Commit backlog growing | Pending files increasing over time | Warning |
| Commit failure storm | Commit error rate > threshold | Critical |
| Quarantine backlog | Quarantined commits unresolved >24h | Critical |
| Small-file explosion | Small-file count above threshold | Warning |
| Orphan cleanup failure | Orphan cleanup error | Warning |
| Legal hold violation attempt | Destructive maintenance blocked | Critical |
| Schema conflict spike | Schema conflict rate above threshold | Warning |

---

## 13. Certification Levels

| Level | Name | Requirement |
|---|---|---|
| L1 | Commit Correctness | All commit correctness tests pass |
| L2 | Freshness Certified | Default and applicable mode freshness targets pass |
| L3 | File Hygiene Certified | File, manifest, snapshot, and orphan thresholds pass |
| L4 | Query Engine Certified | DuckDB/Polars/Spark conformance tests pass |
| L5 | Schema Evolution Certified | Schema evolution tests pass |
| L6 | Failure Resilience Certified | Chaos and recovery tests pass |
| L7 | Operational Readiness Certified | Metrics, alerts, and runbooks validated |

Phase 3 exit requires **L1 through L7**.

---

## 14. Deliverables

| Deliverable | Description | Target Week |
|---|---|---:|
| D-LAKE-001 | Commit ledger certification suite | Week 12 |
| D-LAKE-002 | Freshness measurement harness | Week 14 |
| D-LAKE-003 | File hygiene report generator | Week 16 |
| D-LAKE-004 | DuckDB conformance suite | Week 18 |
| D-LAKE-005 | Polars conformance suite | Week 20 |
| D-LAKE-006 | Spark conformance suite | Week 22 |
| D-LAKE-007 | Schema evolution certification suite | Week 24 |
| D-LAKE-008 | Committer chaos test suite | Week 26 |
| D-LAKE-009 | Maintenance certification suite | Week 28 |
| D-LAKE-010 | Lakehouse operations guide | Week 30 |
| D-LAKE-011 | Phase 3 lakehouse evidence package | Week 32 |

---

## 15. Phase 3 Lakehouse Evidence Package

The evidence package MUST include:

1. Commit correctness report.
2. Commit ledger recovery report.
3. Freshness benchmark report by mode.
4. File hygiene report.
5. Manifest hygiene report.
6. Snapshot hygiene report.
7. Orphan cleanup dry-run and execution report.
8. DuckDB conformance report.
9. Polars conformance report.
10. Spark conformance report.
11. Schema evolution report.
12. Committer chaos report.
13. Maintenance certification report.
14. Observability validation report.
15. Lakehouse operations guide.
16. Known defects and limitations list.
17. Go/no-go recommendation.

---

## 16. Acceptance Criteria Summary

| ID | Requirement | Mandatory |
|---|---|---|
| LAKE-ACC-001 | Commit ledger is durable and idempotent | Yes |
| LAKE-ACC-002 | No duplicate snapshots after restart | Yes |
| LAKE-ACC-003 | Default freshness ≤60 seconds p95 | Yes |
| LAKE-ACC-004 | Fast-mode freshness ≤5 seconds p95 under tuned conditions | Conditional if fast mode enabled |
| LAKE-ACC-005 | DuckDB queries pass | Yes |
| LAKE-ACC-006 | Polars queries pass | Yes |
| LAKE-ACC-007 | Spark queries pass | Yes |
| LAKE-ACC-008 | Schema evolution preserves historical readability | Yes |
| LAKE-ACC-009 | File hygiene thresholds met | Yes |
| LAKE-ACC-010 | Orphan cleanup safe | Yes |
| LAKE-ACC-011 | Committer chaos tests pass | Yes |
| LAKE-ACC-012 | Observability metrics and alerts validated | Yes |

---

## 17. Risks and Mitigations

| Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|
| Iceberg catalog concurrency conflicts | High | Medium | Commit ledger, retry/rebase, quarantine, catalog adapter validation |
| Small-file explosion | High | High | Commit batching, target file size, compaction certification |
| Freshness overpromise | Medium | High | Publish conditional freshness modes; evidence-based reporting |
| Schema evolution breaks readers | High | Medium | Stable field IDs, historical query tests, unsafe-change gating |
| Orphan cleanup deletes active files | Critical | Low | Dry-run, grace period, active manifest cross-check |
| Query engine incompatibility | Medium | Medium | Conformance suite per engine; pin versions; document limitations |
| Committer restart duplicates data | Critical | Medium | Idempotent commit IDs, ledger reconciliation, restart tests |
| S3 throttling delays commits | Medium | High | Backoff with jitter, backlog metrics, hash-prefix keys |
| Legal hold violation | Critical | Low | Maintenance gate checks legal hold before destructive operations |

---

## 18. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial Lakehouse Iceberg Certification Plan. Defines commit correctness, freshness certification, file hygiene, query engine conformance, schema evolution, chaos testing, observability, certification levels, deliverables, and Phase 3 lakehouse evidence package. |