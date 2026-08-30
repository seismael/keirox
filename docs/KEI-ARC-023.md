# KEI-ARC-023 — Columnar ELT & Lakehouse Integration Architecture

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-ARC-023 |
| Title | Columnar ELT & Lakehouse Integration Architecture |
| Version | 1.0 |
| Level | **L2 — Subsystem Architecture** |
| Pillars Covered | Pillar 3 (Internalized Columnar ELT) |
| Status | Approved for Engineering |
| Classification | Internal / Engineering Confidential |
| Owner | Principal Engineer (Stream Processing / Lakehouse) |
| Required Reviewers | Chief Architect, Principal Engineer (Storage), Data Platform Lead |
| Depends On | KEI-ARC-010 (Conceptual Architecture), KEI-ARC-011 (NFRs), KEI-ARC-012 (ADRs), KEI-ARC-020 (Storage Engine) |
| Feeds | KEI-ARC-024 (Gateways), KEI-DES-033 (Schema Registry & Shredding), KEI-DES-034 (Iceberg Catalog Committer) |
| Keywords | MUST, MUST NOT, SHOULD, SHOULD NOT, MAY per RFC 2119 |

---

## 2. Purpose, Scope, and Audience

### 2.1 Purpose

This document specifies the **Columnar ELT & Lakehouse Integration subsystem** — the component that transforms the immutable row log into queryable columnar lakehouse tables without external ETL pipelines.

It elaborates **Pillar 3 (Internalized Columnar ELT)** and the associated read-path acceleration:

- The dual-representation pipeline (hot row ingress → cold columnar lakehouse).
- Adaptive schema shredding with a bounded column cap.
- Arrow RecordBatch transposition and SIMD predicate pushdown.
- Parquet encoding, small-file aggregation, and Apache Iceberg catalog commits.
- The lakehouse query-freshness model.

**Normative terminology:** This subsystem is **Internalized Columnar ELT** (ADR-040). The term "Zero-ETL" is retired because shredding rows into Arrow/Parquet is ELT work; it is internalized and single-hop, not absent.

### 2.2 Scope

**In scope:** the row-to-columnar transformation pipeline, schema shredding and governance, Arrow/Parquet encoding, SIMD pushdown, Iceberg catalog commits, small-file lifecycle, lakehouse freshness, and compaction CPU isolation.

**Out of scope:**
- Physical WAL persistence, segment lifecycle, and Tier-1 offload mechanics — owned by KEI-ARC-020.
- Consumption state (leases, ACKs, watermarks) — owned by KEI-ARC-021.
- Consensus and replication — owned by KEI-ARC-022.
- Encryption key management — owned by KEI-ARC-025.
- Exact schema-registry wire format — owned by KEI-DES-033.
- Exact Iceberg commit algorithm — owned by KEI-DES-034.

### 2.3 Position in the Architecture

```
   Producers ──►┌─────────────────────────────────────────────────────┐
                │          STORAGE ENGINE (KEI-ARC-020)               │
                │   WAL · segments · Tier-1 offload mechanics         │
                └───────────────────────┬─────────────────────────────┘
                                        │ sealed row segments
                                        ▼
                ┌─────────────────────────────────────────────────────┐
                │      COLUMNAR ELT & LAKEHOUSE (this doc)            │
                │  Shredding → Arrow → SIMD → Parquet → Iceberg       │
                └───────┬─────────────────────────────┬───────────────┘
                        │ Parquet files + commits      │ Arrow read path
                        ▼                             ▼
                ┌───────────────────┐        ┌─────────────────────┐
                │ Object Storage +   │        │ Arrow Flight query  │
                │ Iceberg Catalog    │        │ consumers (SDK)     │
                │  (DuckDB/Spark/    │        │ KEI-ARC-024         │
                │   Polars/Trino)    │        └─────────────────────┘
                └───────────────────┘
                        ▲
                        │ DEK for encrypted Parquet
                ┌───────┴───────────┐
                │ SECURITY (025)     │
                └───────────────────┘
```

**Normative boundary:** This subsystem reads sealed row segments append-only and never mutates the log (INV-1). Its output (Parquet + Iceberg metadata) is a projection of the log (GI-3), and its work is asynchronous and MUST NOT gate the producer write path (INV-7).

---

## 3. Subsystem Responsibilities and Non-Responsibilities

### 3.1 Responsibilities

| ID | Responsibility |
|---|---|
| R1 | Read sealed row segments from the storage engine. |
| R2 | Sample payloads and infer/govern schema. |
| R3 | Shred the top bounded set of primitive keys into typed Arrow columns. |
| R4 | Assemble Arrow RecordBatches. |
| R5 | Execute SIMD predicate pushdown over Arrow buffers on the read path. |
| R6 | Encode RecordBatches to compressed Parquet. |
| R7 | Aggregate chunks into target-size Parquet files. |
| R8 | Commit files to the Iceberg catalog with snapshot/manifest lifecycle. |
| R9 | Enforce lakehouse query-freshness policy. |
| R10 | Isolate compaction CPU and apply ELT backpressure. |

### 3.2 Non-Responsibilities

| ID | Non-Responsibility | Owned By |
|---|---|---|
| N1 | Durable row persistence | KEI-ARC-020 |
| N2 | Consumption state | KEI-ARC-021 |
| N3 | Replication | KEI-ARC-022 |
| N4 | Wire protocol for clients | KEI-ARC-024 / KEI-DES-032 |
| N5 | Key management / encryption primitives | KEI-ARC-025 |

---

## 4. Internal Component Decomposition

```
┌──────────────────────────────────────────────────────────────────────────┐
│                 COLUMNAR ELT & LAKEHOUSE SUBSYSTEM                        │
│                                                                          │
│  ┌──────────────────┐                                                    │
│  │ E1. Segment       │◄── sealed row segments (KEI-ARC-020)             │
│  │     Reader        │                                                   │
│  └────────┬─────────┘                                                    │
│           ▼                                                              │
│  ┌──────────────────┐   ┌──────────────────┐                            │
│  │ E2. Schema        │──►│ E3. Schema        │                          │
│  │     Sampler       │   │     Registry      │                          │
│  └────────┬─────────┘   │     Client        │                          │
│           │             └──────────────────┘                            │
│           ▼                                                              │
│  ┌──────────────────┐                                                    │
│  │ E4. Adaptive      │  top-64 keys → typed columns                     │
│  │     Shredder      │  polymorphic → _unstructured_payload             │
│  └────────┬─────────┘                                                    │
│           ▼                                                              │
│  ┌──────────────────┐                                                    │
│  │ E5. Arrow         │  typed, contiguous RecordBatches                  │
│  │     RecordBatch   │                                                   │
│  │     Builder       │                                                   │
│  └──────┬─────────────────────────────┐                                  │
│         │                             │                                  │
│         ▼ (write/lakehouse path)      ▼ (read path)                      │
│  ┌──────────────────┐         ┌──────────────────┐                       │
│  │ E7. Parquet       │         │ E6. SIMD          │                    │
│  │     Encoder       │         │     Predicate     │                    │
│  └────────┬─────────┘         │     Engine        │                    │
│           ▼                    └──────────────────┘                      │
│  ┌──────────────────┐                                                    │
│  │ E8. Small-File    │  aggregate to 64–128 MB                          │
│  │     Aggregator    │                                                   │
│  └────────┬─────────┘                                                    │
│           ▼                                                              │
│  ┌──────────────────┐                                                    │
│  │ E9. Iceberg       │  snapshots, manifests, expiry, orphan GC         │
│  │     Catalog       │                                                   │
│  │     Committer     │                                                   │
│  └──────────────────┘                                                    │
│                                                                          │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │ E10. Compaction Scheduler (core pinning + backpressure)           │  │
│  │ E11. Freshness Controller (default ≤60s / fast ≤5s)               │  │
│  └──────────────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────────────┘
```

| Component | Responsibility |
|---|---|
| **E1. Segment Reader** | Reads sealed row segments handed off by the storage engine. |
| **E2. Schema Sampler** | Samples payloads per stream to infer stable primitive keys. |
| **E3. Schema Registry Client** | Resolves and versions schemas; enforces compatibility modes. |
| **E4. Adaptive Shredder** | Extracts the bounded top-N primitive keys into typed columns. |
| **E5. Arrow RecordBatch Builder** | Assembles typed, contiguous Arrow arrays. |
| **E6. SIMD Predicate Engine** | Vectorized filtering over Arrow buffers on the read path. |
| **E7. Parquet Encoder** | Encodes RecordBatches to Parquet with compression. |
| **E8. Small-File Aggregator** | Aggregates chunks to target file size before upload. |
| **E9. Iceberg Catalog Committer** | Registers files, manages snapshots, manifests, expiry, GC. |
| **E10. Compaction Scheduler** | Pins compaction to isolated cores; drives backpressure. |
| **E11. Freshness Controller** | Selects default vs. fast commit cadence. |

---

## 5. The Dual-Representation Pipeline

The subsystem realizes the Golden Invariant's lakehouse projection through a dual-representation pipeline: a hot row path for low-latency durability and an asynchronous columnar path for analytics.

```
HOT ROW PATH (durability; owned by KEI-ARC-020)
   Producer row ──► lock-free ingress arena ──► WAL quorum ──► ACK
                                                     │
                                                     ▼ (asynchronous; never gates ACK)
COLUMNAR PATH (analytics; this subsystem)
   Sealed row segment ──► Schema Sampler ──► Adaptive Shredder
                       ──► Arrow RecordBatch ──► Parquet Encoder
                       ──► Small-File Aggregator ──► S3 upload ──► Iceberg commit
```

**Normative rules:**
- The columnar path MUST be asynchronous (INV-7). A slow committer MUST NOT increase producer ACK latency.
- The row path and columnar path share the same immutable log as their source of truth (GI-2). There is no second copy of the raw event; the Parquet/Iceberg layer is a columnar projection.

---

## 6. Lock-Free Row Ingress Arena (Handoff Point)

The ingress arena is the boundary between the storage engine and this subsystem.

- Producers append raw row payloads (JSON, Protobuf, FlatBuffers, or Arrow) into a **lock-free memory arena** for sub-millisecond queuing.
- The arena avoids batch-assembly wait times on the hot write path.
- Sealed arena regions become **sealed row segments** handed to the Columnar ELT subsystem.

**Normative rule:** Arena append MUST be lock-free on the producer path. Columnar transformation reads sealed arena regions after they are durably committed.

---

## 7. Adaptive Schema Shredding (ADR-042)

### 7.1 Shredding Model

The Adaptive Shredder ingests raw rows without requiring upfront schema definitions:

```
{ "tenant":"A1", "amount":104.5, "user_id":7, "meta":{...} }
        │
        ▼  (sample + infer)
Stable primitive keys  ──►  typed Arrow columns   (tenant, amount, user_id)
Polymorphic/nested     ──►  _unstructured_payload (binary/JSON column)
```

### 7.2 The 64-Key Cap

- Background workers sample incoming payloads within each stream namespace.
- The **top 64 consistent primitive keys** are shredded into typed, contiguous Arrow arrays.
- Polymorphic, deeply nested, or sparse fields route to an auxiliary `_unstructured_payload` column.

**Normative rules:**
- Shredding MUST be capped at 64 primitive keys per stream namespace.
- Excess or morphing fields MUST route to `_unstructured_payload`, never expand the shredded column set unboundedly.
- The cap protects against wide-schema drift and polymorphic poisoning (red-team scenario 3).

### 7.3 Schema Governance

Schema evolution is governed by the Schema Registry (elaborated in KEI-DES-033):

- New columns are added as nullable.
- Safe numeric widening is allowed.
- Unsafe type changes create a new schema version.
- Old data remains readable via schema fingerprint stored in chunk metadata.

**Normative rule:** Schema changes MUST be backward-compatible for readers, or MUST increment the schema version so historical chunks remain decodable.

---

## 8. Arrow RecordBatch Transposition

The Arrow RecordBatch Builder converts shredded columns into typed, contiguous Arrow arrays.

- Columns are laid out contiguously to enable SIMD and cache-friendly scans.
- Each RecordBatch carries the schema fingerprint so readers can resolve types.
- RecordBatches are the unit handed to both the SIMD read path and the Parquet encoder.

**Normative rule:** The in-memory analytics representation MUST be Apache Arrow, so that downstream engines (DuckDB, Polars, Spark, PyTorch) can consume it zero-copy.

---

## 9. SIMD-Accelerated Predicate Pushdown

On the read path, the SIMD Predicate Engine filters data before network transmission, eliminating consumer deserialization overhead.

```
Consumer query:  WHERE amount > 100 AND tenant = 'A1'
        │
        ▼
SIMD Predicate Engine executes AVX-512 / ARM Neon vector instructions
directly over memory-mapped Arrow buffers in CPU cache
        │
        ▼
Only matching rows are serialized and transferred
```

**Normative rules:**
- Predicate pushdown MUST operate over Arrow buffers, not over opaque row bytes.
- The engine MUST support AVX-512 and ARM Neon vector paths.
- Pushdown MUST degrade gracefully: if a predicate references `_unstructured_payload`, the engine falls back to non-vectorized evaluation for that column.

**Performance target:** Arrow Flight clients using pushdown SHOULD achieve ≤1/3 the CPU consumption of an equivalent JVM Kafka consumer for vectorized workloads (PERF-032, Class B).

---

## 10. Parquet Encoding and Compression

The Parquet Encoder converts Arrow RecordBatches to Parquet for Tier-1 storage.

- Compression: zstd (default) or lz4, selected per workload profile.
- Encoding preserves the shredded typed columns and the `_unstructured_payload` column.
- Each Parquet file embeds the schema fingerprint and chunk offset metadata.

**Normative rule:** Parquet files MUST be self-describing and MUST carry the schema fingerprint so that readers can join them to the Iceberg table schema.

---

## 11. Small-File Aggregation (ADR-045)

To prevent small-file metadata explosion in Iceberg/Delta catalogs:

- The Small-File Aggregator accumulates sealed chunks until reaching a **target 64–128 MB Parquet file**.
- Only target-size files are uploaded to object storage and committed to the catalog.
- Aggregation is per `(tenant, stream_bucket, event_date, schema_version)` grouping.

**Normative rule:** The subsystem MUST NOT upload per-record or per-segment Parquet files directly to the catalog. Aggregation to target file size is mandatory before commit.

---

## 12. Iceberg Catalog Committer (ADR-043)

### 12.1 Table Model — Shared Tenant Tables

Default is **one Iceberg table per tenant**, not per stream:

```
Table: tenant_{tenant_id}.events
Partitioned by: event_date / stream_bucket / schema_version
Columns include: tenant_id, stream_id, entity_key, event_timestamp,
                 schema_id, <shredded columns>, _unstructured_payload,
                 _keirox_offset, _keirox_ingest_time
```

**Normative rules:**
- The default table model MUST be shared tenant tables (ADR-043). This bounds catalog metadata at high stream cardinality.
- Dedicated per-stream tables are optional and reserved for high-isolation or high-throughput streams.
- Per-stream deletion MUST be handled via crypto-shredding (KEI-ARC-025) plus column filtering, not by dropping a per-stream table.

### 12.2 Commit Lifecycle

The Iceberg Catalog Committer:

1. Receives target-size Parquet files from the aggregator.
2. Registers data files in a new Iceberg snapshot.
3. Maintains the manifest list and manifest files.
4. Applies snapshot expiry and manifest compaction.
5. Runs orphan-file garbage collection after a safety window.

### 12.3 Concurrent Commit Safety

**Normative rule:** The committer MUST use the catalog's concurrency control (optimistic concurrency or catalog lock) so that concurrent commits from multiple storage nodes do not corrupt the table. Commit retries MUST be idempotent.

### 12.4 Specification Delegation

The exact commit algorithm, snapshot expiry policy, and manifest compaction thresholds are specified in KEI-DES-034.

---

## 13. Lakehouse Query Freshness Model (ADR-044)

Freshness is the time from event ingress to queryable-in-Iceberg.

| Mode | Target | Conditions | Class |
|---|---|---|---|
| Default | ≤ 60 s | Standard commit cadence | D |
| Fast mode | ≤ 5 s | Tuned, low-load deployment | D |

**Normative rules:**
- The default freshness target is **≤60 s** (ADR-044). Sub-2-second freshness is NOT a default and MUST NOT be advertised as one.
- Fast mode (≤5 s) increases object-storage API and catalog load and MUST be explicitly enabled per tenant.
- The Freshness Controller selects commit cadence to meet the configured mode.

---

## 14. CPU Isolation and ELT Backpressure

### 14.1 CPU Core Isolation (ADR-041)

Compaction and Arrow transposition threads are pinned to isolated CPU cores via `sched_setaffinity`, separate from socket/WAL threads.

```
┌────────────────────────────────────────────────────────────┐
│ HOT CORES (socket + WAL)          [owned by KEI-ARC-020]   │
├────────────────────────────────────────────────────────────┤
│ COMPACTION CORES (shredding, Arrow, Parquet)  [this doc]   │
│   lower priority; sched_setaffinity pinned                  │
└────────────────────────────────────────────────────────────┘
```

**Normative rule:** Compaction CPU MUST be isolated so that background ELT interference on the write path is bounded to ≤5% p99 jitter (PERF-003, Class B).

### 14.2 ELT Backpressure

If compaction or Iceberg commits fall behind:

| Stage | Trigger | Action |
|---|---|---|
| 1 | Compaction queue > threshold | Raise compaction priority; metrics alert. |
| 2 | Arena residency > 80% | Coordinate with KEI-ARC-020 Backpressure Controller for TCP clamping. |
| 3 | Sustained Iceberg commit failure | Buffer Parquet locally; retry with backoff; alert. |

**Normative rule:** ELT backpressure MUST coordinate with the storage-engine Backpressure Controller (KEI-ARC-020 C11) so that a single, consistent ingress throttling policy applies.

---

## 15. ELT-Specific Failure Handling

| Scenario | Defense (this subsystem) |
|---|---|
| Wide schema drift / polymorphic poisoning | 64-key cap + `_unstructured_payload` (§7.2). |
| Compaction CPU jitter | Core pinning (§14.1). |
| Small-file explosion | Target-size aggregation (§11). |
| Iceberg commit contention | Catalog concurrency control + idempotent retry (§12.3). |
| Snapshot/manifest bloat | Snapshot expiry + manifest compaction (§12.2). |
| Schema type conflict | Schema versioning; old chunks remain decodable (§7.3). |
| Committer lag | ELT backpressure + freshness-mode selection (§14.2, §13). |

---

## 16. NFR Traceability (Owned by This Subsystem)

| NFR | Requirement | How This Subsystem Satisfies It |
|---|---|---|
| PERF-003 | Compaction interference ≤5% p99 jitter | CPU core isolation (§14.1). |
| PERF-030 | Default lakehouse freshness ≤60s | Freshness Controller default mode (§13). |
| PERF-031 | Fast-mode freshness ≤5s | Freshness Controller fast mode (§13). |
| PERF-032 | Arrow Flight CPU ≤1/3 JVM Kafka | SIMD predicate pushdown (§9). |
| SCALE (schema) | Bounded shredding | 64-key cap (§7.2). |
| DUR / integrity | Correct projection | Schema fingerprint + immutable source (§8, §10). |

---

## 17. Interfaces

### 17.1 Provided Interfaces

| Interface | Consumer | Semantics |
|---|---|---|
| `transformSegment(segment)` | E10 Scheduler | Shred + build Arrow + encode Parquet. |
| `pushdownQuery(predicate, range)` | KEI-ARC-024 / SDK | SIMD-filtered Arrow read. |
| `commitFiles(files)` | E9 Committer | Register Parquet in Iceberg snapshot. |
| `getFreshnessMode(tenant)` | Control Plane | Return configured freshness mode. |

### 17.2 Consumed Interfaces

| Interface | Provider | Purpose |
|---|---|---|
| `onSegmentSealed(cb)` | KEI-ARC-020 | Trigger columnar transformation. |
| Schema registry | KEI-DES-033 | Schema resolution/versioning. |
| DEK for Parquet | KEI-ARC-025 | Encrypt Parquet at rest. |
| Object storage upload | KEI-ARC-020 (C9) | Persist target-size files. |
| Iceberg catalog | External | Snapshot/manifest store. |

---

## 18. Open Questions and ADR Dependencies

| Item | Status | Resolution Path |
|---|---|---|
| Schema sampling rate and confidence threshold | Open | Benchmark under P5 before Phase-3 exit. |
| Nested-field shredding policy (opt-in) | Open | Specify in KEI-DES-033. |
| Iceberg commit concurrency mechanism (lock vs. OCC) | Open | Select per catalog backend in KEI-DES-034. |
| Snapshot expiry horizon and orphan GC window | Open | Define in KEI-DES-034. |
| Fast-mode cost model (API + catalog load) | Open | Validate under P5. |

Binding decisions already recorded: ADR-040, ADR-041, ADR-042, ADR-043, ADR-044, ADR-045.

---

## 19. Glossary (Additions)

| Term | Definition |
|---|---|
| Internalized Columnar ELT | In-broker row→Arrow→Parquet transformation and Iceberg registration; not "zero-ETL." |
| Adaptive Schema Shredding | Extracting the top bounded set of primitive keys into typed Arrow columns. |
| `_unstructured_payload` | Auxiliary column for polymorphic/nested/sparse fields beyond the shred cap. |
| SIMD Predicate Pushdown | Vectorized filtering over Arrow buffers before network transfer. |
| Small-File Aggregation | Combining chunks into target-size Parquet files before catalog commit. |
| Shared Tenant Table | The default one-Iceberg-table-per-tenant model. |
| Freshness | Time from event ingress to queryable-in-Iceberg. |

---

## 20. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial columnar ELT & lakehouse architecture. Defines the dual-representation pipeline, 64-key adaptive shredding with `_unstructured_payload` fallback, Arrow transposition, SIMD pushdown, Parquet encoding, small-file aggregation, shared-tenant Iceberg commits, freshness model (≤60s default), and CPU-isolated backpressure. Retires the "Zero-ETL" and "2-second freshness" claims per ADR-040 and ADR-044. |