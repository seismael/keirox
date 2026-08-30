# KEI-ARC-020 — Storage Engine Architecture (LSM-WAL & Two-Tier Storage)

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-ARC-020 |
| Title | Storage Engine Architecture — LSM-WAL & Two-Tier Storage |
| Version | 1.0 |
| Level | **L2 — Subsystem Architecture** |
| Pillars Covered | Pillar 1 (Virtual Micro-Stream Fabric), Pillar 4 (Two-Tier Storage Hierarchy) |
| Status | Approved for Engineering |
| Classification | Internal / Engineering Confidential |
| Owner | Principal Engineer (Storage) |
| Required Reviewers | Chief Architect, Principal Engineer (Distributed Systems), SRE Lead |
| Depends On | KEI-ARC-010 (Conceptual Architecture), KEI-ARC-011 (NFRs), KEI-ARC-012 (ADRs) |
| Feeds | KEI-ARC-021 (State Plane), KEI-ARC-023 (Columnar ELT), KEI-DES-030 (WAL Binary Format) |
| Keywords | MUST, MUST NOT, SHOULD, SHOULD NOT, MAY per RFC 2119 |

---

## 2. Purpose, Scope, and Audience

### 2.1 Purpose

This document specifies the internal architecture of the **Storage Engine subsystem** — the component responsible for durable, low-latency, high-cardinality event persistence. It elaborates Pillars 1 and 4 of the Polymorphic Event Fabric:

- **Pillar 1:** Multiplexing 100K–1M+ logical micro-streams onto a shared physical WAL with sparse indexing.
- **Pillar 4:** The two-tier storage hierarchy — Tier-0 NVMe for hot durability and Tier-1 object storage for cold retention and lakehouse analytics.

### 2.2 Scope

**In scope:** write path, read path, segment lifecycle, sparse indexing, stream registry, manifest management, single-pass compaction, Tier-1 offloading, node recovery, capacity backpressure, and the storage threading model.

**Out of scope:**
- Consumption semantics (leases, ACKs, watermarks) — owned by KEI-ARC-021.
- Consensus protocol internals (Raft groups, epoch fencing) — owned by KEI-ARC-022.
- Arrow schema shredding and Iceberg commit logic — owned by KEI-ARC-023.
- Exact byte-level binary layouts — owned by KEI-DES-030.

### 2.3 Position in the Architecture

```
                     ┌──────────────────────────────┐
                     │   Control Plane (quotas,     │
                     │   stream registry, config)   │
                     └──────────────┬───────────────┘
                                    │ stream create/delete, quotas
   Producers ──────►┌──────────────┴───────────────────────────────┐
   (Gateway/SDK)    │              STORAGE ENGINE (this doc)        │◄──── Consensus
                    │  Ingress → Arena → WAL → Index → Compaction   │      (KEI-ARC-022)
                    │            → Tier-1 Offload → Recovery        │      quorum commit
                    └───────┬──────────────────┬───────────────────┘
                            │ read(append-only) │ sealed row segments
                            ▼                   ▼
                    ┌───────────────┐   ┌────────────────┐
                    │ State Plane   │   │ Columnar ELT   │
                    │ KEI-ARC-021   │   │ KEI-ARC-023    │
                    └───────────────┘   └────────────────┘
```

---

## 3. Subsystem Responsibilities and Non-Responsibilities

### 3.1 Responsibilities

| ID | Responsibility |
|---|---|
| R1 | Accept producer records and durably persist them via Tier-0 quorum. |
| R2 | Multiplex many logical streams onto shared physical WAL segments. |
| R3 | Maintain the in-memory stream registry and sparse block index. |
| R4 | Provide append-only read access by `(stream, offset-range)`. |
| R5 | Execute single-pass compaction from row arena to columnar chunks. |
| R6 | Offload sealed columnar chunks asynchronously to Tier-1 object storage. |
| R7 | Maintain and serve the chunk manifest (NVMe + S3). |
| R8 | Recover node state from Tier-1 manifest + WAL delta. |
| R9 | Enforce storage-level backpressure under capacity pressure. |

### 3.2 Non-Responsibilities

| ID | Non-Responsibility | Owned By |
|---|---|---|
| N1 | Consumer lease/ACK state | KEI-ARC-021 |
| N2 | Raft replication protocol | KEI-ARC-022 |
| N3 | Schema shredding to Arrow | KEI-ARC-023 |
| N4 | Iceberg catalog commits | KEI-ARC-023 |
| N5 | Authentication/authorization | KEI-ARC-025 |

---

## 4. Internal Component Decomposition

The storage engine decomposes into eleven cooperating components.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            STORAGE ENGINE                                    │
│                                                                              │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐                    │
│  │ C1. Ingress   │──►│ C2. Row       │──►│ C3. WAL       │──► [Quorum]       │
│  │    Admission  │   │    Arena      │   │    Writer     │                   │
│  └──────────────┘   └──────┬───────┘   └──────┬───────┘                    │
│                            │ sealed segments   │ durable records            │
│                            ▼                   ▼                            │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐                    │
│  │ C8. Compactor │◄──│ C4. Segment   │   │ C5. Sparse    │                   │
│  │    (Arrow)    │   │    Manager    │   │    Indexer    │                   │
│  └──────┬───────┘   └──────────────┘   └──────┬───────┘                    │
│         │ columnar chunks                      │ index entries              │
│         ▼                                      ▼                            │
│  ┌──────────────┐                       ┌──────────────┐                    │
│  │ C9. Tier-1    │──────────────────────►│ C7. Manifest  │                   │
│  │    Offloader  │                       │    Manager    │                   │
│  └──────────────┘                       └──────┬───────┘                    │
│                                                 │                            │
│  ┌──────────────┐   ┌──────────────┐           │                            │
│  │ C6. Stream    │   │ C10. Recovery │◄──────────┘                           │
│  │    Registry   │   │    Manager    │                                      │
│  └──────────────┘   └──────────────┘                                       │
│                                                                              │
│  ┌──────────────────────────────────────────────────────────────────┐       │
│  │ C11. Backpressure Controller (cross-cutting)                      │      │
│  └──────────────────────────────────────────────────────────────────┘       │
└─────────────────────────────────────────────────────────────────────────────┘
```

| Component | Responsibility |
|---|---|
| **C1. Ingress Admission** | Enforce tenant token-bucket quotas; reject or backpressure before allocation. |
| **C2. Row Ingress Arena** | Lock-free memory arena holding raw row payloads pending durability and compaction. |
| **C3. WAL Writer** | io_uring + O_DIRECT batch writer producing page-aligned WAL frames. |
| **C4. Segment Manager** | Owns 64 MB segment lifecycle: preallocation, append, sealing, rotation. |
| **C5. Sparse Indexer** | Maintains the 4-tuple sparse index and per-chunk Bloom filters. |
| **C6. Stream Registry** | In-memory registry mapping stream_id → head offset, active block, fingerprint. |
| **C7. Manifest Manager** | Maintains StreamManifest / ChunkMetadata for NVMe and S3 tiers. |
| **C8. Compactor** | Single-pass row→columnar transposition on isolated cores. |
| **C9. Tier-1 Offloader** | Async multipart uploader to object storage with manifest registration. |
| **C10. Recovery Manager** | Reconstructs state from Tier-1 manifest + WAL delta on node join. |
| **C11. Backpressure Controller** | Monitors NVMe and arena capacity; drives TCP clamping and shedding. |

---

## 5. The Two-Tier Storage Hierarchy

This section defines the topology that all storage behavior derives from (ADR-011).

```
┌────────────────────────────────────────────────────────────────────────┐
│ TIER 0: HOT DURABILITY & LOW-LATENCY INGRESS                          │
│  • io_uring + O_DIRECT on local NVMe                                   │
│  • Synchronous 3-node local Raft quorum over WAL segment heads         │
│  • Treats NVMe as an ephemeral ring buffer                            │
│  • Write latency target: ≤ 2 ms p99 (Class D, Profile P1)             │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ continuous asynchronous block offload
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│ TIER 1: COLD RETENTION, REPLAY & LAKEHOUSE ANALYTICS                  │
│  • Sealed 64–128 MB columnar chunks (Parquet/Arrow) on object storage  │
│  • Metadata registered in Iceberg/Delta catalogs                       │
│  • Standard cloud object-storage pricing                               │
└────────────────────────────────────────────────────────────────────────┘
```

**Normative rules:**
- Tier-0 MUST be treated as ephemeral. No recovery assumption may depend on Tier-0 surviving a node loss.
- Durability is granted at Tier-0 quorum commit; Tier-1 provides retention and analytics, not the durability promise.
- Tier-1 offload MUST be asynchronous and MUST NOT gate the Tier-0 write path (INV-7).

---

## 6. Write Path

### 6.1 End-to-End Write Flow

```
Producer record
   │
   ▼
[C1] Ingress Admission ──(quota exceeded)──► reject / backpressure
   │ (admitted)
   ▼
[C2] Row Ingress Arena (lock-free append; raw row payload)
   │
   ▼
[C3] WAL Writer
   │   • assemble batch frame (common header + record entries)
   │   • CRC32C over header + payload
   │   • pad to 4096-byte page boundary
   ▼
[C4] Segment Manager ──► append to active 64 MB segment via io_uring/O_DIRECT
   │
   ▼
[Quorum] Replicate segment head to 2 followers (synchronous)
   │
   ▼ (quorum commit)
Producer ACK issued  ──►  record is DURABLE (INV-3)
```

**Normative rule (INV-3):** The producer ACK MUST NOT be issued before quorum commit. The arena write and local NVMe write alone do not constitute durability.

### 6.2 Batch Framing

The WAL writer groups records into batch frames (ADR-013). Common fields are amortized into a batch header; per-record entries carry only deltas.

- Integrity: CRC32C (not CRC16) over header and payload.
- Alignment: each batch padded to a 4096-byte boundary for O_DIRECT.
- Producer identity: `producer_id` + `producer_seq` (64-bit) carried for idempotence deduplication (consumed by the State Plane / dedup window).

Exact byte layouts are specified in KEI-DES-030. This document fixes only the framing contract:

> A WAL batch is the unit of CRC integrity, the unit of quorum replication, and the unit of recovery replay.

### 6.3 Idempotence Interaction

The storage engine records `producer_id`/`producer_seq` in every frame but does not itself deduplicate. Deduplication is enforced by the State Plane using the sliding window; the storage engine merely persists the identifiers. This keeps the storage engine append-only and stateless with respect to producer sessions.

---

## 7. Read Path

### 7.1 Stream Sequential Scan (Hot)

For active streams resident in Tier-0:

```
read(stream_id, from_offset, max_bytes)
   │
   ▼
[C6] Stream Registry ──► resolve head_offset + active_block_ptr
   │
   ▼
Direct sequential scan of active WAL segment (zero-copy)
   │
   ▼
Return records from from_offset onward
```

Hot reads MUST be served from Tier-0 without consulting the sparse index (RAF = 1.0 for active streams).

### 7.2 Historical Read via Sparse Index (Cold)

For offsets that have been compacted and/or offloaded:

```
read(stream_id, target_offset)
   │
   ▼
[C7] Manifest Manager ──► locate StreamManifest for stream_id
   │
   ▼
[C5] Sparse Indexer
   │   • Prefix Bloom Filter check (skip non-matching chunks)
   │   • binary search on RangeStartOffset in BTreeMap<u64, ChunkMetadata>
   ▼
Resolve ChunkMetadata ──► physical pointer (NVMe offset or S3 URI + byte range)
   │
   ▼
Read target chunk (local NVMe or S3 GET range)
```

**Normative rule:** The sparse index MUST bound historical lookups to ≤1.05 disk seeks on average (via Bloom filters + sorted chunk keys). This is a Class B benchmark target.

### 7.3 Read Amplification Bounds

| Read Type | RAF |
|---|---|
| Active stream (Tier-0, in memory/NVMe) | 1.0 |
| Historical chunk (S3, via sparse index) | ≤ 1.05 seeks |

---

## 8. Stream Registry and Sparse Indexing

### 8.1 Stream Registry (Pillar 1)

The registry is the in-memory mapping of every logical stream. Nominal footprint is **~224 bytes per active stream** (ADR-010, MEM-001/002):

| Component | Bytes |
|---|---:|
| Packed `StreamRegistryEntry` struct | 32 |
| Hash-table bucket & allocator overhead (mimalloc/jemalloc) | 64 |
| In-memory Prefix Bloom fingerprint | 32 |
| Concurrency locks & head-offset index | 48 |
| Active chunk metadata cache | 48 |
| **Total** | **~224** |

**Normative rules:**
- The registry MUST support 1,000,000 streams per node (validated) and 100,000 streams per node (stable SLA).
- The 224-byte figure is the registry-only nominal; total node memory MUST use the full budget formula in KEI-ARC-010 §9.2.
- Registry operations MUST be O(1) amortized for lookup and insert.

### 8.2 Sparse 4-Tuple Index

Physical records are located via:

```
⟨ TenantID, StreamID, RangeStartOffset, PhysicalPointer ⟩
```

- Chunks are indexed in a `BTreeMap<RangeStartOffset, ChunkMetadata>` per stream manifest.
- Each sealed chunk carries a Prefix Bloom Filter to skip non-matching ranges.
- The index is sparse: one entry per chunk, not per record.

### 8.3 Chunk Metadata and Manifest

`ChunkMetadata` records, for each sealed chunk: stream_id, offset range, storage location (NVMe offset or S3 URI), byte offset/length, record count, Bloom filter bytes, and schema fingerprint.

`StreamManifest` aggregates: stream_id, head_offset, and the ordered map of chunks.

**Normative rule:** The manifest MUST be recoverable from Tier-1 object storage and MUST be versioned/checksummed to detect corruption (feeds KEI-DES-030 and recovery).

---

## 9. Segment Lifecycle

```
[Preallocated 64 MB segment]
        │ active append
        ▼
[Sealing] ── triggered by size (64 MB) OR time (500 ms) OR stream-quiesce
        │
        ▼
[Sealed segment] ──► handed to Compactor (C8) for columnar transposition
        │
        ▼
[New segment preallocated & rotated in]
```

**Normative rules:**
- Segments MUST be preallocated to avoid allocation stalls on the hot path.
- Sealing is triggered by size threshold, time threshold, or explicit quiesce — whichever first.
- A sealed segment becomes immutable and is the unit handed to compaction.

---

## 10. Single-Pass Compaction and Tier-1 Offload

### 10.1 Single-Pass Model (ADR-014)

```
Memory Arena ──► [Columnar transposition] ──► Sealed Arrow/Parquet chunk
                                                      │
                                                      ▼
                                              Async upload to S3
                                                      │
                                                      ▼
                                              Register in manifest + Iceberg
```

- No multi-level LSM merge loops (no L0–L6).
- Records are written once to the WAL and transposed once to columnar form.
- **Write Amplification Factor (WAF) ≤ 1.35** (Class B target).

**Normative rule:** Compaction MUST be single-pass. Any design that re-merges already-compacted chunks MUST be rejected as a violation of ADR-014 unless superseded by a new ADR.

### 10.2 Small-File Aggregation (ADR-045)

The compactor aggregates sealed chunks into target **64–128 MB Parquet files** before object upload to prevent small-file metadata explosion in Iceberg/Delta catalogs.

### 10.3 Tier-1 Offloader

- Asynchronous multipart uploader to S3/GCS/Azure Blob (PORT-003).
- On successful upload, the manifest is updated and the NVMe range becomes eligible for truncation.
- Upload failures trigger exponential backoff with jitter and hash-prefix key partitioning.

**Normative rule:** A Tier-0 range MUST NOT be truncated until the corresponding Tier-1 upload is confirmed and registered in the manifest.

### 10.4 Compaction Isolation (ADR-041)

Compaction and Arrow transposition threads are pinned to isolated CPU cores (`sched_setaffinity`), separate from socket/WAL threads, to bound tail-latency interference to ≤5% p99 jitter (PERF-003).

---

## 11. Node Recovery (Stateless Tier-0)

### 11.1 Recovery Model (ADR-012)

Tier-0 is ephemeral. On node failure or join, a replacement node:

```
1. Fetch StreamManifest set from Tier-1 object storage
2. Reconstruct stream registry + chunk index from manifests
3. Replay the short active WAL delta from cluster peers
4. Rebuild in-memory arena state deterministically
5. Resume traffic
```

**Target:** recovery in **< 5 seconds** (AVAIL-002, Class B).

### 11.2 Deterministic Replay

Because the WAL is append-only and batch-framed with CRC32C, replay is deterministic. A node crash during local compaction loses only in-flight arena state, which is reconstructed from the unsealed WAL segment (red-team scenario 4).

**Normative rule:** Recovery MUST NOT depend on any data that exists only on the failed node's local NVMe.

---

## 12. Capacity Management and Backpressure

This section corrects the earlier unbounded "24–48 hour backlog" claim into a bounded, capacity-derived model.

### 12.1 Bounded Elastic Backlog

The Tier-0 backlog duration is a function of available NVMe capacity and effective ingress rate:

```
backlog_hours = (usable_NVMe_bytes × compression_ratio) /
                (ingress_bytes_per_sec × 3600)
```

There is **no fixed 24–48 hour guarantee**. The actual backlog depends on instance NVMe size, compression, and retention already resident.

### 12.2 Backpressure Ladder (C11)

The Backpressure Controller applies a graduated response:

| Stage | Trigger | Action |
|---|---|---|
| 1 | Arena > 60% | Metrics alert; compaction priority raised. |
| 2 | NVMe buffer > 80% | Progressive TCP window clamping at ingress. |
| 3 | S3 upload sustained failure | Exponential backoff + jitter; hash-prefix repartition. |
| 4 | NVMe > 95% | Emergency shedding of non-critical streams; strict backpressure. |

**Normative rules:**
- Backpressure MUST engage before NVMe corruption is possible (red-team scenario 8).
- Shedding MUST target non-critical streams first and MUST be observable (OPS-007).
- The system MUST expose `nvme_backlog_eta_seconds` and `s3_upload_backlog_bytes` metrics.

---

## 13. Threading and Concurrency Model

```
┌─────────────────────────────────────────────────────────────┐
│  ISOLATED HOT CORES (socket + WAL)                          │
│   • Ingress / admission                                      │
│   • WAL writer (io_uring submit/reap)                        │
│   • Hot read path                                            │
│   (thread-per-core event loop; no compaction)               │
├─────────────────────────────────────────────────────────────┤
│  ISOLATED COMPACTION CORES (lower priority)                 │
│   • Row→Arrow transposition                                  │
│   • Parquet encoding                                         │
│   • Single-pass compaction                                   │
├─────────────────────────────────────────────────────────────┤
│  OFFLOAD / INDEX CORES                                       │
│   • S3 multipart uploader                                    │
│   • Manifest / index maintenance                             │
└─────────────────────────────────────────────────────────────┘
```

**Normative rules:**
- Hot-path threads MUST NOT share cores with compaction threads (PERF-003).
- The row ingress arena MUST be lock-free on the append path.
- Stream registry reads MUST be wait-free or RCU-style to avoid read-path stalls.

---

## 14. Storage-Specific Failure Handling

| Scenario | Defense (this subsystem) |
|---|---|
| Node crash during compaction | Deterministic WAL replay reconstructs arena (see §11.2). |
| S3 throttling (503) | Backoff + jitter + hash-prefix partitioning; NVMe elastic backlog (§12). |
| NVMe full | Backpressure ladder + emergency shedding (§12.2). |
| Disk corruption | CRC32C per batch; corruption detected, not silently returned (DUR-007). |
| Manifest corruption | Versioned, checksummed manifests; restore from backup (REC-006). |
| Historical read latency | Sparse index keeps unacked-task pointers in NVMe (see KEI-ARC-021 cold-task index). |

---

## 15. NFR Traceability (Owned by This Subsystem)

| NFR | Requirement | How This Subsystem Satisfies It |
|---|---|---|
| PERF-001/002 | Tier-0 write latency | io_uring + O_DIRECT + quorum batching (§6). |
| PERF-004 | Framing overhead ≤8% | Batch framing (§6.2, ADR-013). |
| PERF-020/021/022 | Throughput | Multiplexed WAL + isolated cores (§6, §13). |
| SCALE-001/002 | Stream cardinality | LSM-WAL multiplexing + registry (§8.1). |
| SCALE-003 | O(1) file handles | Shared ring-buffer segments (§9). |
| MEM-001/002 | Registry footprint | 224-byte/stream model (§8.1). |
| AVAIL-002 | Node recovery <5s | Manifest + WAL delta replay (§11). |
| DUR-007 | Integrity detection | CRC32C per batch (§6.2). |
| PORT-002 | io_uring/O_DIRECT | Primary I/O path with epoll fallback (§13). |

---

## 16. Interfaces

### 16.1 Provided Interfaces

| Interface | Consumer | Semantics |
|---|---|---|
| `append(batch)` | Gateway / SDK | Durably append a batch; returns after quorum commit. |
| `read(stream, offset_range)` | State Plane | Append-only read of a range. |
| `getManifest(stream)` | Recovery / ELT | Return the stream manifest. |
| `onSegmentSealed(cb)` | Compactor | Callback when a segment is sealed. |
| `backlogMetrics()` | Observability | NVMe ETA, S3 backlog bytes. |

### 16.2 Consumed Interfaces

| Interface | Provider | Purpose |
|---|---|---|
| Quorum commit confirm | KEI-ARC-022 | Gate producer ACK (INV-3). |
| Stream create/delete | Control Plane | Registry mutation. |
| Tenant quotas | Control Plane | Admission decisions (C1). |
| KMS DEK (at rest) | KEI-ARC-025 | Encryption of persisted chunks. |

---

## 17. Open Questions and ADR Dependencies

| Item | Status | Resolution Path |
|---|---|---|
| Segment size (64 MB) vs. time threshold (500 ms) tuning | Open | Benchmark under P1/P2 before Phase-1 exit. |
| Bloom filter false-positive rate target | Open | Derive from ≤1.05-seek RAF budget. |
| Manifest versioning scheme | Open | Specify in KEI-DES-030. |
| Multi-volume striping per node | Open | Evaluate for >1 NVMe instance types. |

All binding decisions are already recorded in KEI-ARC-012 (ADR-010 … ADR-015).

---

## 18. Glossary (Additions)

| Term | Definition |
|---|---|
| LSM-WAL | Log-Structured Multiplexed Write-Ahead Log. |
| Sparse 4-tuple | ⟨TenantID, StreamID, RangeStartOffset, PhysicalPointer⟩ index key. |
| Segment | A preallocated 64 MB immutable WAL unit. |
| Sealed segment | A closed segment handed to compaction. |
| Elastic backlog | The capacity-derived Tier-0 buffering during Tier-1 degradation. |
| Single-pass compaction | One transposition from arena to columnar form; no merge loops. |

---

## 19. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial storage-engine subsystem architecture. Defines 11 components, two-tier hierarchy, write/read paths, single-pass compaction, stateless recovery, bounded backpressure model, and threading isolation. Corrects the prior fixed 24–48h backlog claim to a capacity-derived bound. Aligns to ADR-010…015 and NFRs PERF/SCALE/MEM/AVAIL/DUR/PORT. |