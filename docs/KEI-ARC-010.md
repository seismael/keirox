# KEI-ARC-010 — Conceptual Architecture & The Golden Invariant

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-ARC-010 |
| Title | Conceptual Architecture & The Golden Invariant |
| Version | 1.0 |
| Level | **L1 — Conceptual Architecture** |
| Status | Approved for Engineering |
| Classification | Internal / Engineering Confidential |
| Owner | Chief Architect |
| Required Reviewers | Principal Engineer (Storage), Principal Engineer (Distributed Systems), Principal Engineer (Stream Processing), Security Lead |
| Depends On | KEI-ARC-001 (Vision & Context), KEI-ARC-012 (Principles & ADR Index) |
| Feeds | KEI-ARC-020 … KEI-ARC-027 (all L2 subsystem documents) |
| Keywords | MUST, MUST NOT, SHOULD, SHOULD NOT, MAY per RFC 2119 |

---

## 2. Purpose, Scope, and Audience

### 2.1 Purpose

This document defines **what** the Polymorphic Event Fabric (PEF) is, independent of **how** any individual subsystem is implemented. It establishes:

- The normative **Golden Invariant** from which all other design rules derive.
- The **conceptual decomposition** of the system into planes and pillars.
- The **end-to-end data lifecycle** across all consumption modes.
- The **semantic contracts** for delivery, ordering, durability, and consistency.

### 2.2 Scope

**In scope:** logical structure, data lifecycle, consumption semantics, ordering model, delivery/idempotence/transaction semantics, consistency model, and multi-tenancy hierarchy.

**Out of scope:** binary formats (see KEI-DES-030/031), wire protocols (see KEI-DES-032/035), specific algorithms and data-structure internals (see KEI-DES-031), and measurable NFR targets (see KEI-ARC-011).

### 2.3 Relationship to Other Documents

| Document | Relationship |
|---|---|
| KEI-ARC-001 | Defines *why*; this document defines *what*. |
| KEI-ARC-011 | Converts the semantics defined here into measurable NFR targets. |
| KEI-ARC-012 | Records the binding decisions (ADRs) that justify the choices made here. |
| KEI-ARC-020…027 | Each L2 document elaborates one pillar defined in §4. |

**Normative rule:** Any L2/L3 document that contradicts the Golden Invariant (§3) or the semantic contracts (§7–§10) is invalid and MUST be reconciled via a new ADR.

---

## 3. The Golden Invariant (Normative)

> **Data is written exactly once to an immutable physical log. The operational semantics of that data — whether consumed as a strict sequential stream, a leased out-of-order task queue, a virtual dead-letter view, or an Apache Iceberg columnar table — are defined entirely by the consumer's replicated, mutable state overlay.**

### 3.1 Formal Decomposition of the Invariant

The invariant splits into three binding sub-rules:

- **GI-1 (Immutability):** The physical log is append-only. Once a record is committed to the quorum, it is never rewritten, reordered, or physically deleted by the consumption layer. Physical removal occurs only via retention lifecycle and crypto-shredding (KEI-ARC-025).
- **GI-2 (Single Truth):** Every byte of every event exists in exactly one durable place. There is no separate queue store, no separate stream store, and no separate analytics store. All views are projections.
- **GI-3 (Semantic Projection):** Consumption behavior is a pure function of the immutable log plus a per-consumer mutable overlay:

```
View = f(ImmutableLog, StateOverlay_consumer)
```

### 3.2 Consequences of the Invariant

Because of GI-1..GI-3, the following become architecturally true and MUST be preserved by every subsystem:

1. **Replay is free.** Re-consuming data never mutates storage.
2. **Queue and stream are not different storage.** They are different overlays over identical storage.
3. **The lakehouse is a view.** Iceberg tables are a projection of the log, not a second copy.
4. **Dead-lettering is a flag, not a move.** A poison-pill is re-flagged in the overlay; the payload is never duplicated.
5. **Dual-write bugs are structurally eliminated.** There is exactly one write path to one log.

These five consequences are the architectural moat and MUST NOT be violated by future features.

---

## 4. Conceptual Decomposition — The Six Pillars

PEF decomposes conceptually into six pillars. Each pillar maps to one or more L2 subsystem documents.

| # | Pillar | Conceptual Responsibility | L2 Owner |
|---|---|---|---|
| 1 | **Virtual Micro-Stream Fabric (LSM-WAL)** | Multiplex 100K–1M+ logical streams onto shared physical WAL; sparse indexing. | KEI-ARC-020 |
| 2 | **Log-Bitmap Duality (State Plane)** | Immutable log + replicated Roaring Bitmap overlays for stream/queue/DLQ. | KEI-ARC-021 |
| 3 | **Internalized Columnar ELT** | Row → Arrow → Parquet transformation; adaptive schema shredding. | KEI-ARC-023 |
| 4 | **Two-Tier Storage Hierarchy** | Tier-0 low-latency NVMe quorum + Tier-1 async object storage offload. | KEI-ARC-020 |
| 5 | **Distributed State Plane (Coordination)** | Deterministic coordinator sharding, consensus, epoch fencing, failover. | KEI-ARC-022 |
| 6 | **Enterprise Compliance Plane** | Envelope encryption, KMS crypto-shredding, ABAC, audit. | KEI-ARC-025 |

### 4.1 Cross-Cutting Planes

In addition to the six pillars, two cross-cutting planes span all of them:

- **Control Plane:** metadata catalog, stream/group registry, quota/admission control, cluster membership. Feeds KEI-ARC-022 and KEI-ARC-027.
- **Protocol Plane:** Kafka ingest gateway, native Arrow Flight/gRPC SDKs, SQS/AMQP translation. Feeds KEI-ARC-024.

### 4.2 Pillar Independence Principle

Each pillar MUST be independently testable and independently replaceable behind a stable internal interface. This enforces Principle P5 (single artifact, modular subsystems) and prevents monolithic coupling.

---

## 5. End-to-End Data Lifecycle

This section traces a single event from production to every consumption mode. It is the canonical conceptual flow that all subsystem documents must remain consistent with.

### 5.1 Lifecycle Overview

```
PRODUCER (row event or Arrow batch)
   │  produce(stream_id, entity_key, payload)
   ▼
[1] INGRESS & ADMISSION CONTROL
   │  tenant token-bucket, schema mode check
   ▼
[2] ROW INGRESS ARENA (lock-free, hot path)
   │
   ▼
[3] TIER-0 WRITE-AHEAD LOG (multiplexed NVMe, io_uring)
   │  synchronous 3-node Raft quorum commit  ──► durable, producer ACKed
   │
   ├────────────────────────────┬──────────────────────────────┐
   ▼                            ▼                              ▼
[4a] STREAM CONSUMPTION    [4b] QUEUE CONSUMPTION        [4c] COLUMNAR ELT
   offset-based sequential     lease → ack/nack → DLQ        row → Arrow → Parquet
   replay via offset cursor    via Roaring Bitmap overlay     (background, async)
   │                            │                              │
   │                            │                              ▼
   │                            │                         [5] TIER-1 S3 OFFLOAD
   │                            │                              sealed Parquet chunks
   │                            │                              + Iceberg catalog commit
   │                            │                              │
   ▼                            ▼                              ▼
        QUERY / ANALYTICS (DuckDB, Spark, Polars, Arrow Flight)
```

### 5.2 Lifecycle Stages (Normative Descriptions)

| Stage | Name | Durability | Notes |
|---|---|---|---|
| 1 | Ingress & Admission | None yet | Enforce tenant quotas; reject or backpressure before memory allocation. |
| 2 | Row Ingress Arena | In-memory only | Sub-millisecond queuing; no batch-assembly stall on hot path. |
| 3 | Tier-0 WAL Commit | **Durable (quorum)** | The moment of durability. Producer ACK is issued only after quorum commit. |
| 4a | Stream Consumption | Read-only | Offset cursor over immutable log. |
| 4b | Queue Consumption | Overlay-mutating | Lease/ACK transitions mutate only the state overlay, never the log. |
| 4c | Columnar ELT | Async | Does not block the hot path; governed by compaction backpressure. |
| 5 | Tier-1 Offload | Durable (object store) | Sealed chunks + Iceberg metadata; enables retention and analytics. |

**Normative rule (Lifecycle Ordering):** Durability (stage 3) MUST precede any consumer-visible acknowledgment. Columnar ELT (stage 4c) and Tier-1 offload (stage 5) MUST be asynchronous and MUST NOT gate the producer write path.

### 5.3 The Two Durability Tiers (Conceptual)

- **Tier-0 (Hot):** Local NVMe, synchronous quorum, bounded elastic backlog. Optimized for write latency and fast recovery. Treated as an ephemeral ring buffer.
- **Tier-1 (Cold):** Object storage, asynchronous, columnar. Optimized for retention cost and analytical query. Treated as the long-term source of truth for historical ranges.

A node is **conceptually stateless** with respect to Tier-0: on failure, a replacement reconstructs active state from the Tier-1 manifest plus a short WAL delta from peers.

---

## 6. Multi-Tenancy Hierarchy (Conceptual)

PEF is multi-tenant by construction. The containment hierarchy is normative and is referenced by every downstream document.

```
Tenant
  └── Stream (virtual micro-stream; 100K–1M+ per node)
        ├── Physical mapping: multiplexed into shared WAL (no 1:1 file)
        ├── Ordering unit: per stream_id (or per entity_key within a shared stream)
        └── Consumer Group (0..N)
              └── State Shard = hash(tenant_id, stream_id, group_id, bucket)
                    ├── Roaring Bitmap overlays
                    ├── Lease table
                    ├── Timing-wheel subset
                    └── Watermark (W_base)
```

### 6.1 Key Entities

| Entity | Definition | Cardinality |
|---|---|---|
| **Tenant** | Isolation, quota, and encryption boundary. | Many per cluster |
| **Stream** | The unit of durable ordering and retention. | 100K–1M+ per node |
| **Consumer Group** | A named set of consumers sharing a state overlay. | 0..N per stream |
| **State Shard** | The unit of state-plane ownership and failover. | Derived by hash |
| **Coordinator** | The node owning a set of state shards. | Sharded deterministically |

### 6.2 Sharding Rationale

State is sharded by `hash(tenant, stream, group, bucket)` rather than by group alone so that:

- A single high-cardinality stream does not overload one coordinator.
- Failover granularity is a shard, not an entire group.
- Memory and lease load are bounded per shard.

This resolves the consumer-group scalability concern identified in prior audits.

---

## 7. Consumption Semantics Model

This is the core of the Log-Bitmap Duality. All four consumption modes are projections of the same immutable log.

### 7.1 The State Machine

Each offset within a state shard has exactly one state at any instant:

```
State(i) ∈ { READY, LEASED(τ), ACKED, EVICTED_DLQ }
```

`READY` is the implicit default for any offset that is not leased, acked, or evicted.

```
                        lease(τ)
              ┌─────────────────────────────┐
              │                             ▼
          ┌────────┐                  ┌──────────┐
          │ READY  │                  │  LEASED  │
          └────────┘                  └────┬─────┘
              ▲                            │
              │   timeout(τ) / NACK        │
              └────────────────────────────┤
                                           │ ACK
                                           ▼
                                     ┌──────────┐
                                     │  ACKED   │
                                     └──────────┘

   READY or LEASED  ──(retry_count ≥ R_max  OR  time_in_flight ≥ max)──►  EVICTED_DLQ
```

### 7.2 State Transition Table

| From | Event | To | Side Effect |
|---|---|---|---|
| READY | lease(τ) | LEASED | Insert timing-wheel timer; record worker + retry_count. |
| LEASED | ACK | ACKED | Set acked bit; cancel timer. |
| LEASED | NACK | READY | Cancel timer; requeue for retry. |
| LEASED | timeout(τ) | READY | Cancel timer; requeue at head for prioritized retry. |
| READY/LEASED | retry_count ≥ R_max | EVICTED_DLQ | Insert into Sparse Exception Table; advance W_base. |
| READY/LEASED | time_in_flight ≥ max | EVICTED_DLQ | Insert into Sparse Exception Table; advance W_base. |

### 7.3 The Sliding Base Watermark

Memory is bounded by advancing a base watermark past all terminal offsets:

```
W_base = max { k ∈ ℕ | ∀ i < k, State(i) = ACKED ∨ State(i) = EVICTED_DLQ }
```

**Normative rules:**

- All state for offsets `< W_base` MUST be purged from memory.
- `W_base` MUST advance even when individual offsets are stuck. This is guaranteed by the mandatory DLQ-eviction rule (§7.4).
- An implementation MUST NOT allow a non-terminal offset to block `W_base` indefinitely.

### 7.4 Mandatory DLQ Eviction (Anti-Stuck Guarantee)

To guarantee that `W_base` always advances, PEF enforces:

```
IF retry_count ≥ max_retries
   OR time_in_flight ≥ max_time_in_flight
   OR lease_policy == FORCE_EVICT
THEN State(offset) = EVICTED_DLQ
     INSERT (tenant, stream, offset, failure_metadata) INTO Sparse Exception Table
     ALLOW W_base TO ADVANCE
```

This rule is what converts a potentially unbounded lease window into a bounded one, and it is a direct consequence of Principle P3 (bounded everything).

### 7.5 The Four Consumption Modes

| Mode | Mechanism | Mutates Log? | Mutates Overlay? |
|---|---|---|---|
| **Stream Replay** | Sequential offset cursor over immutable log. | No | Only offset commit. |
| **Task Queue** | Lease → ACK/NACK over Roaring Bitmap overlay. | No | Yes (lease/ACK bits). |
| **Virtual DLQ View** | Read offsets where `State == EVICTED_DLQ` via Sparse Exception Table. | No | No (read-only). |
| **Lakehouse Table** | Iceberg/Parquet projection of sealed chunks. | No | No (read-only). |

**Normative rule:** No consumption mode may physically delete or rewrite a record. DLQ and retention are the only mechanisms that remove visibility, and retention removal is governed by KEI-ARC-025 (crypto-shredding) and lifecycle policy.

---

## 8. Ordering & Concurrency Model

### 8.1 Ordering Unit

PEF decouples ordering from physical partitions. The ordering guarantee is defined per **stream** and, within a shared stream, per **entity key**.

- **Strict total order** is guaranteed per `stream_id`.
- **Strict order per `entity_key`** is guaranteed within a shared stream.
- **Independent entity keys** are dispatched concurrently across workers with no ordering constraint between them.

This replaces the legacy model where concurrency was capped by a static partition count.

### 8.2 Dynamic Causal Scheduling (Conceptual)

```
Ingress:   [Order_A #1] [Order_B #1 (fails)] [Order_A #2] [Order_C #1]
Workers:
  Worker-1: blocked on Order_B #1 (isolated; does not stall others)
  Worker-2: Order_A #1 → Order_A #2   (strictly sequential per entity key)
  Worker-3: Order_C #1                (concurrent; independent key)
```

Order_A is never blocked by the failure of the independent Order_B. This is the head-of-line-blocking elimination that the architecture promises.

### 8.3 Hot-Key Handling (Corrected Semantics)

Prior audits flagged a contradiction: *strict ordering for a single full entity key cannot be parallelized across workers without relaxing ordering.* PEF therefore defines hot-key handling as follows:

- **Isolation:** A hot entity key is isolated onto a dedicated worker arena to prevent CPU-core starvation of other keys.
- **Sub-key parallelism (opt-in):** If the application supplies a `sub_entity_key`, ordering is guaranteed per sub-key, enabling parallelism within an entity.
- **Relaxed-order mode (opt-in):** An application may explicitly relax ordering for a stream to gain parallelism.

**Normative rule:** PEF MUST NOT claim to parallelize a single strictly-ordered key. Parallelism for a hot key requires either a sub-key or an explicit relaxed-order declaration.

---

## 9. Delivery, Idempotence & Transaction Semantics

PEF defines explicit, selectable semantic modes rather than implicit magic. This enforces Principle P4 (explicit semantics over magic).

### 9.1 Delivery Guarantee Matrix

| Mode | Guarantee | Requirement |
|---|---|---|
| `AT_LEAST_ONCE` | Default. Records may be redelivered after crash, lease timeout, or coordinator failover. | Consumers SHOULD be idempotent or tolerate duplicates. |
| `EFFECTIVELY_ONCE_PRODUCE` | Producer-side. Idempotent producers prevent duplicate appends within the deduplication window. | Producer supplies `producer_id` + `producer_seq`. |
| `TRANSACTIONAL_APPEND` | Optional. Multi-record / multi-stream appends become visible atomically on commit. | Producer uses transaction API. |
| `EXACTLY_ONCE_PROCESSING` | Not automatic. Requires idempotent consumers or transactional sinks plus durable ACK/offset commits. | Application responsibility. |

**Normative rule:** PEF MUST NOT advertise universal exactly-once side-effect execution. Exactly-once end-to-end requires consumer cooperation.

### 9.2 Acknowledgment Durability Modes

| Mode | Behavior | Trade-off |
|---|---|---|
| `ACK_FAST` | Coordinator applies ACK to memory immediately; replicates to state Raft asynchronously. | Sub-ms latency; bounded ACK-loss window on coordinator failover → possible redelivery. |
| `ACK_DURABLE` | ACK is committed to the state Raft log before success is returned. | Higher latency; no known ACK loss after success. |

**Normative rule:** The client API MUST expose the ACK mode, and the system MUST document the `ACK_FAST` loss window explicitly.

### 9.3 Producer Idempotence

- Deduplication key: `(producer_id, producer_epoch, producer_seq)`.
- `producer_seq` is a 64-bit monotonic value with explicit session/epoch semantics.
- Deduplication window is bounded by sequence distance, time, and memory.
- Duplicate produce within the window returns the original offset without re-appending.

### 9.4 Optional Transactional Append

- `BEGIN_TXN` → `APPEND`(s) → `COMMIT_TXN` / `ABORT_TXN`.
- Internal control records: `TXN_PREPARE`, `TXN_COMMIT`, `TXN_ABORT`.
- `READ_COMMITTED` consumers see only committed records; queue dispatch marks records READY only after commit.
- Scope: atomic append across streams within a single tenant. Cross-tenant transactions are out of scope for v1.

---

## 10. Consistency Model

### 10.1 Classification

PEF is **CP with local quorum** on the hot path and **causal** semantics for wide-area replication:

- **Tier-0 writes** commit via synchronous local Raft quorum → linearizable within the local cluster for committed records.
- **Multi-region replication** is asynchronous and causal; it does not provide synchronous cross-region linearizability in v1.

### 10.2 Multi-Region Mode (v1)

To avoid unsolvable concurrent-write conflicts, v1 supports:

- **Mode A (recommended):** Single-writer primary per stream with asynchronous replica. Clear failover and ordering.
- **Regional namespaces:** Each region writes its own stream namespace; consumers merge analytically.
- **Multi-writer same-stream:** NOT supported in v1 for strictly ordered streams.

This resolves the prior ambiguity around active-active same-stream writes.

### 10.3 Coordinator Consistency

- State shards are owned by exactly one coordinator at a time, guarded by a monotonic `coordinator_epoch`.
- On failover, the successor increments the epoch, restores state from snapshot + lease journal, and fences stale requests.
- Split-brain is mitigated by epoch fencing plus Raft quorum membership.

---

## 11. Cross-Cutting Concern Summary

These concerns are defined here at the conceptual level and elaborated in their respective L2 documents.

| Concern | Conceptual Position | Elaborated In |
|---|---|---|
| Security & encryption | Envelope encryption; tenant/stream DEKs; crypto-shredding. | KEI-ARC-025, KEI-DES-036 |
| Quotas & admission | Per-tenant token buckets; backpressure before allocation. | KEI-ARC-027 |
| Observability | Watermark lag, lease age, ACK replication lag, bitmap spill, S3 backlog. | KEI-ARC-027 |
| Schema governance | Registry; adaptive shredding cap; `_unstructured_payload` fallback. | KEI-ARC-023, KEI-DES-033 |
| Lakehouse commit | Shared tenant tables; commit batching; manifest compaction. | KEI-ARC-023, KEI-DES-034 |
| Backup / DR | Manifest + snapshot backup; restore validation. | KEI-ARC-026, KEI-OPS-040 |

---

## 12. Conceptual Invariants & Constraints

The following invariants MUST hold across all subsystems. They are the checklist against which every L2/L3 design is reviewed.

| ID | Invariant |
|---|---|
| INV-1 | The physical log is append-only; consumption never rewrites it. |
| INV-2 | Every event exists in exactly one durable place. |
| INV-3 | Durability precedes consumer-visible acknowledgment. |
| INV-4 | `W_base` always advances; no stuck offset may block it indefinitely. |
| INV-5 | All mutable state has a bounded memory quota and a spill/shed path. |
| INV-6 | A state shard is owned by exactly one coordinator, fenced by epoch. |
| INV-7 | Columnar ELT and Tier-1 offload are asynchronous and never gate the write path. |
| INV-8 | No consumption mode physically deletes a record outside retention/crypto-shredding. |
| INV-9 | Ordering is guaranteed per stream / per entity key; hot-key parallelism requires sub-key or relaxed mode. |
| INV-10 | Exactly-once end-to-end is a consumer contract, not a broker guarantee. |

---

## 13. Decisions Deferred to ADR Index

The following are recognized decision points whose rationale and alternatives are recorded in KEI-ARC-012 (ADR Index):

- ADR candidate: Rust-only for v1 (exclude Zig).
- ADR candidate: Shared tenant Iceberg tables vs. per-stream tables.
- ADR candidate: ACK_FAST default vs. ACK_DURABLE default.
- ADR candidate: Multi-region Mode A as the only v1 same-stream replication mode.
- ADR candidate: State-shard hash inputs and bucket count.

---

## 14. Glossary (Additions)

| Term | Definition |
|---|---|
| Golden Invariant | The rule that data is written once to an immutable log and semantics come from consumer overlays. |
| State Overlay | The per-consumer replicated Roaring Bitmap + lease metadata that projects the log. |
| State Shard | The unit of state-plane ownership: hash(tenant, stream, group, bucket). |
| W_base | Sliding base watermark below which all state is purged. |
| Sparse Exception Table | Index of EVICTED_DLQ offsets and their failure metadata. |
| ACK_FAST / ACK_DURABLE | Selectable acknowledgment durability modes. |
| Tier-0 / Tier-1 | Hot NVMe quorum tier / cold object-storage tier. |
| Coordinator Epoch | Monotonic generation used to fence stale coordinators. |

---

## 15. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial approved conceptual baseline. Defines Golden Invariant, six pillars, lifecycle, consumption semantics, ordering model, delivery/idempotence/transaction semantics, consistency model, and ten cross-subsystem invariants. Incorporates all prior audit resolutions (ACK modes, state sharding, DLQ eviction guarantee, hot-key correction, multi-region Mode A). |