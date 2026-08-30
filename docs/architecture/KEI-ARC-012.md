# KEI-ARC-012 — Architecture Principles & Decision Record Index (ADR)

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-ARC-012 |
| Title | Architecture Principles & Decision Record Index |
| Version | 1.0 |
| Level | **L1 — Conceptual Architecture** |
| Status | Approved for Engineering |
| Classification | Internal / Engineering Confidential |
| Owner | Chief Architect |
| Required Reviewers | All Principal Engineers, Security Lead, Platform Engineering Lead |
| Depends On | KEI-ARC-001 (Vision), KEI-ARC-010 (Conceptual Architecture), KEI-ARC-011 (NFRs) |
| Feeds | All L2 (KEI-ARC-020…027) and L3 (KEI-DES-030…037) documents |
| Keywords | MUST, MUST NOT, SHOULD, SHOULD NOT, MAY per RFC 2119 |

---

## 2. Purpose, Scope, and Audience

### 2.1 Purpose

This document is the **single source of truth for all binding architectural decisions** in the Polymorphic Event Fabric. It serves two functions:

1. **Principles register** — expands the ten binding principles from KEI-ARC-001 §B.7 into enforceable rules with rationale and verification.
2. **Decision log** — records every architectural decision referenced across KEI-ARC-010 and KEI-ARC-011 as a numbered, status-tracked Architecture Decision Record (ADR), capturing context, decision, and consequences.

### 2.2 Scope

**In scope:** All binding principles and all decisions that (a) constrain more than one subsystem, (b) affect an NFR target, (c) resolve a documented design contradiction, or (d) reverse or supersede a prior decision.

**Out of scope:** Local implementation choices that affect only a single module (those belong in code-level design notes, not here).

### 2.3 Audience

- **Architects** use this to resolve design disputes by reference, not opinion.
- **Engineering leads** use this to know which decisions are fixed and which are open.
- **Reviewers** use this to reject any L2/L3 design that contradicts an accepted ADR without a superseding ADR.

---

## 3. ADR Lifecycle and Status Model

Each decision follows a governed lifecycle. This enforces Principle P4 (explicit semantics over magic) at the governance level.

| Status | Meaning |
|---|---|
| **Proposed** | Under discussion; not yet binding. |
| **Accepted** | Approved and binding. All downstream designs MUST comply. |
| **Superseded** | Replaced by a newer ADR; retained for history. |
| **Deprecated** | No longer applies to new work; existing implementations may persist until migration. |
| **Rejected** | Considered and declined; retained to prevent re-litigation. |

**Governance rule:** An Accepted ADR can only be changed by a new Accepted ADR that explicitly names the superseded record. No silent edits are permitted.

---

## 4. Binding Architecture Principles

These ten principles are carried forward from KEI-ARC-001 §B.7 and expanded here with rationale and enforcement.

| ID | Principle | Rationale | Enforcement |
|---|---|---|---|
| **P1** | **Immutability first.** The physical log is append-only. | Enables replay, audit, virtual DLQ, and safe lakehouse export. | Invariant INV-1; code review + invariant test. |
| **P2** | **Truth/state separation.** Storage truth is immutable; consumption semantics live in replicated overlays. | Resolves the queue/stream dichotomy without data duplication. | Golden Invariant GI-1..GI-3. |
| **P3** | **Bounded everything.** Every mutable structure has a quota, spill path, and shedding policy. | Prevents unbounded lease/bitmap/manifest growth in production. | Invariant INV-5; memory NFRs MEM-001..006. |
| **P4** | **Explicit semantics over magic.** Guarantees are documented modes, not implicit behavior. | Enterprise trust requires testable contracts. | Delivery matrix; verification classes A–D. |
| **P5** | **Progressive durability.** Clients choose latency/durability trade-offs per request. | One size does not fit signaling vs. payments. | ACK_FAST / ACK_DURABLE modes. |
| **P6** | **Compatibility by published subset.** Gateways expose validated compatibility matrices. | Prevents unclosable parity debt. | KEI-DES-035 compatibility matrices. |
| **P7** | **Security by default.** Encryption, ABAC, audit, crypto-shredding are baseline. | Enterprise adoption gate. | SEC/COMP NFRs. |
| **P8** | **Observability is a product feature.** State-plane internals are exposed as metrics. | The state plane is only operable if visible. | OPS NFRs; KEI-ARC-027. |
| **P9** | **Evidence gates phases.** No phase exits on narrative. | De-risks the delivery program. | KEI-OPS-041 test plan. |
| **P10** | **Single artifact, modular subsystems.** One deployable binary of independently testable modules. | Deployment simplicity without monolith coupling. | KEI-ARC-010 §4.2. |

---

## 5. Architecture Decision Records

Decisions are grouped by domain. Each ADR cites the principles it supports and the NFRs it affects.

---

### Domain A — Core Data Model & Invariants

#### ADR-001: Immutable Physical Log as Single Source of Truth
- **Status:** Accepted
- **Principles:** P1, P2 | **NFRs:** DUR-001, DUR-002
- **Context:** Legacy stacks dual-write to a queue and a stream, creating desynchronization bugs. A unified fabric needs one ground truth.
- **Decision:** Every event is written exactly once to an append-only physical log. All consumption modes (stream, queue, DLQ, lakehouse) are projections of this log via state overlays. No subsystem may rewrite or physically delete a record outside retention/crypto-shredding.
- **Consequences:** Eliminates dual-write bugs structurally. Requires a robust overlay/state plane. Replay is free. Retention deletion must use crypto-shredding (see ADR-019).

#### ADR-002: Log-Bitmap Duality for Consumption Semantics
- **Status:** Accepted
- **Principles:** P2 | **NFRs:** SCALE-004, MEM-003
- **Context:** Append logs lack leases/out-of-order ACKs; queues destroy data on ACK. Both are needed on the same data.
- **Decision:** Consumption state is modeled as a replicated Roaring Bitmap overlay per `(tenant, stream, group, shard)` with states `READY`, `LEASED(τ)`, `ACKED`, `EVICTED_DLQ`. The log is never mutated by consumption.
- **Consequences:** Enables dual-mode consumption on one dataset. Introduces a distributed state plane requiring coordination (Domain D). Bitmap memory must be bounded (P3).

#### ADR-003: Virtual DLQ via Flag, Not Physical Copy
- **Status:** Accepted
- **Principles:** P1, P3 | **NFRs:** MEM-004, OPS-006
- **Context:** Physical DLQ topics duplicate payload bytes and complicate replay/retention.
- **Decision:** Poison pills transition to `EVICTED_DLQ` and are indexed in a Sparse Exception Table `⟨TenantID, StreamID, Offset, Metadata⟩`. The payload is not copied. DLQ views read the original log through the flag.
- **Consequences:** Zero-copy dead-lettering. DLQ redrive is a state transition. Requires the Sparse Exception Table to be persisted and queryable.

#### ADR-004: Mandatory DLQ Eviction to Guarantee Watermark Advancement
- **Status:** Accepted
- **Principles:** P3 | **NFRs:** MEM-004
- **Context:** A single stuck offset can hold `W_base` forever, causing an unbounded bitmap window (the "stuck watermark" memory leak identified in audit).
- **Decision:** If `retry_count ≥ max_retries` OR `time_in_flight ≥ max_time_in_flight` OR `lease_policy == FORCE_EVICT`, the offset MUST transition to `EVICTED_DLQ` so `W_base` advances.
- **Consequences:** Guarantees bounded memory. Changes poison-pill behavior from "block forever" to "evict to DLQ." Requires operator-tunable `max_retries` and `max_time_in_flight`.

#### ADR-005: Ordering Unit Is Stream / Entity Key, Not Partition
- **Status:** Accepted
- **Principles:** P4 | **NFRs:** SCALE-001
- **Context:** Static partitions cap concurrency, cause head-of-line blocking, and trigger rebalance storms.
- **Decision:** Strict total order is guaranteed per `stream_id` and, within a shared stream, per `entity_key`. Independent entity keys dispatch concurrently. There are no user-facing physical partitions.
- **Consequences:** Removes partition sizing and rebalance pain. Requires a causal scheduler. Hot keys need isolation, not parallelization (see ADR-006).

#### ADR-006: Hot-Key Handling Is Isolation, Not Parallelization
- **Status:** Accepted
- **Principles:** P4 | **NFRs:** PERF-011
- **Context:** Prior drafts implied hot keys could be striped while preserving full ordering — a contradiction, since a strictly-ordered single key cannot be parallelized.
- **Decision:** A hot entity key is isolated onto a dedicated worker arena. Parallelism within an entity requires either an application-supplied `sub_entity_key` or an explicit relaxed-order mode. PEF MUST NOT claim to parallelize a single strictly-ordered key.
- **Consequences:** Correct, defensible semantics. Applications wanting intra-entity parallelism must opt in with sub-keys.

---

### Domain B — Storage Engine & I/O

#### ADR-010: Multiplexed LSM-WAL Over Per-Stream Files
- **Status:** Accepted
- **Principles:** P3 | **NFRs:** SCALE-001..003, MEM-001
- **Context:** Mapping 1M+ streams to files/directories/Raft groups exhausts file handles and causes heartbeat storms.
- **Decision:** All logical streams multiplex into a shared physical NVMe WAL ring buffer per storage volume, indexed by a sparse 4-tuple `⟨TenantID, StreamID, RangeStartOffset, PhysicalPointer⟩`.
- **Consequences:** O(1) file handles regardless of stream count. Requires sparse block indexing and Prefix Bloom Filters for read amplification control.

#### ADR-011: Two-Tier Storage — NVMe Tier-0 + Object Storage Tier-1
- **Status:** Accepted
- **Principles:** P5 | **NFRs:** PERF-001, REC-001
- **Context:** Local-disk brokers are expensive for retention; direct-to-S3 proxies have a 100–600ms latency floor unsuitable for task queues.
- **Decision:** Tier-0 is local NVMe with synchronous quorum for low-latency durability. Tier-1 is asynchronous object storage for retention and analytics. Tier-0 is an ephemeral ring buffer.
- **Consequences:** Sub-2ms durable writes with cheap cold retention. Nodes are stateless w.r.t. Tier-0. Requires manifest-based recovery (ADR-012).

#### ADR-012: Stateless Node Recovery from Tier-1 Manifest + WAL Delta
- **Status:** Accepted
- **Principles:** P3 | **NFRs:** AVAIL-002
- **Context:** Stateful brokers are slow to replace and hard to rebalance.
- **Decision:** On failure, a replacement node reconstructs active state from the Tier-1 chunk manifest and replays the short active WAL delta from peers, targeting <5 seconds.
- **Consequences:** Fast, elastic recovery. Depends on manifest integrity and peer WAL availability. Recovery-time target is Class B (validated, not design-guaranteed).

#### ADR-013: Batch-Oriented WAL Framing with CRC32C
- **Status:** Accepted
- **Principles:** P4 | **NFRs:** DUR-007, PERF-004
- **Context:** A naive per-record 72-byte header wastes ~7% overhead at 1KB payloads and used weak CRC16.
- **Decision:** WAL uses batch-oriented framing: common fields (producer, schema, transaction, DEK) in a batch header; per-record entries carry only deltas. Integrity uses CRC32C (not CRC16). Records are padded to 4096-byte alignment for `O_DIRECT`.
- **Consequences:** Lower framing overhead, stronger integrity, cleaner alignment. Supersedes the original per-record `WalRecordHeader` design.

#### ADR-014: Single-Pass Tiered Compaction (WAF ≤ 1.35)
- **Status:** Accepted
- **Principles:** P3 | **NFRs:** PERF-020
- **Context:** Multi-level LSM compaction (RocksDB L0–L6) yields WAF 10–30, unacceptable for a streaming engine.
- **Decision:** Compaction is single-pass: memory arena → sealed columnar chunk → object storage. No multi-level merge loops.
- **Consequences:** WAF bounded ≤1.35. Simpler compaction logic. Requires careful small-file aggregation (ADR-016).

#### ADR-015: io_uring + O_DIRECT as Primary I/O Path
- **Status:** Accepted
- **Principles:** P10 | **NFRs:** PORT-002, PERF-021
- **Context:** High-throughput NVMe ingestion needs kernel-bypass async I/O.
- **Decision:** Linux-first I/O via `io_uring` with `O_DIRECT` and registered buffers, with an `epoll` fallback for portability.
- **Consequences:** High throughput, low CPU. Linux dependency accepted. Requires careful alignment and error handling.

---

### Domain C — Consumption State Plane

#### ADR-020: Two Explicit ACK Durability Modes
- **Status:** Accepted
- **Principles:** P4, P5 | **NFRs:** DUR-003, DUR-004
- **Context:** Audit flagged ambiguous ACK durability. Sub-ms fast path and durable ACK are mutually exclusive trade-offs.
- **Decision:** The API exposes `ACK_FAST` (memory apply, async journal replication; bounded loss window) and `ACK_DURABLE` (Raft commit before success). The client selects per request. `ACK_FAST` loss window MUST be documented.
- **Consequences:** Transparent trade-off. Resolves the prior ambiguity. `ACK_DURABLE` carries higher latency.

#### ADR-021: ACK_FAST as Default Queue Mode
- **Status:** Accepted
- **Principles:** P5 | **NFRs:** DUR-003, PERF-011
- **Context:** Choosing a default requires balancing latency against redelivery risk.
- **Decision:** `ACK_FAST` is the default. Workloads needing no-ACK-loss opt into `ACK_DURABLE`. This is recorded because it is a trade-off decision, not an obvious default.
- **Consequences:** Optimizes the common high-throughput idempotent-worker case. Payment/critical workflows MUST explicitly select `ACK_DURABLE`.

#### ADR-022: At-Least-Once Default; No Universal Exactly-Once
- **Status:** Accepted
- **Principles:** P4 | **NFRs:** DUR-001..006
- **Context:** Prior drafts overclaimed "Effectively-Once" and implied broker-side exactly-once.
- **Decision:** Default delivery is at-least-once. Producer idempotence and optional transactional append are provided. Exactly-once end-to-end requires idempotent consumers or transactional sinks and is NOT a broker guarantee.
- **Consequences:** Honest, testable semantics. Removes a dangerous overclaim. Consumer documentation MUST emphasize idempotence.

#### ADR-023: Deterministic Coordinator Sharding by Consistent Hash
- **Status:** Accepted
- **Principles:** P3 | **NFRs:** SCALE-006, AVAIL-003
- **Context:** A single coordinator per consumer group does not scale to high-cardinality streams and large lease counts.
- **Decision:** State is sharded by `hash(tenant_id, stream_id, group_id, bucket)`. Each shard is owned by one coordinator, fenced by a monotonic `coordinator_epoch`. Failover restores from snapshot + lease journal.
- **Consequences:** Bounded per-coordinator load, shard-granularity failover. Requires epoch fencing (ADR-024) and lease journal replication.

#### ADR-024: Epoch Fencing for Split-Brain Lease Safety
- **Status:** Accepted
- **Principles:** P4 | **NFRs:** AVAIL-004
- **Context:** A network partition could allow two coordinators to lease the same offset.
- **Decision:** Lease issuance carries a monotonic `coordinator_epoch`. Successors increment the epoch; stale-epoch requests are rejected. Under unrecoverable partition, the system prefers unavailability of the shard over conflicting leases.
- **Consequences:** Split-brain safety. A liveness trade-off is explicitly accepted (availability of a shard may pause during partition).

#### ADR-025: Hierarchical Timing Wheel for Lease Expiration
- **Status:** Accepted
- **Principles:** P3 | **NFRs:** PERF-011
- **Context:** Millions of concurrent leases need O(1) insertion/cancellation/expiration.
- **Decision:** Lease timeouts are managed by a hierarchical priority timing wheel.
- **Consequences:** Efficient lease management. Timing-wheel state must be recoverable on failover (via lease journal).

---

### Domain D — Consensus & Coordination

#### ADR-030: Two-Tier Raft Topology
- **Status:** Accepted
- **Principles:** P2, P3 | **NFRs:** DUR-001, AVAIL-001
- **Context:** Replicating data and replicating coordination state have different latency/throughput profiles and should not share one Raft group.
- **Decision:** A **Data Plane Raft** (3-node synchronous quorum per storage volume) replicates WAL segment heads. A **Metadata & State Raft** replicates coordinator assignments, manifests, bitmap snapshots, and committed `W_base`.
- **Consequences:** Clean separation of hot-path durability from coordination state. Adds operational complexity of two consensus planes.

#### ADR-031: CP Local Quorum + Causal WAN
- **Status:** Accepted
- **Principles:** P4 | **NFRs:** AVAIL-006, REC-001
- **Context:** The system must state its CAP position honestly.
- **Decision:** Tier-0 writes are CP via local quorum (linearizable for committed records within the local cluster). Wide-area replication is asynchronous and causal. Cross-region linearizability is not provided in v1.
- **Consequences:** Clear consistency model. Multi-region is bounded by Mode A (ADR-033).

---

### Domain E — Columnar ELT & Lakehouse

#### ADR-040: Internalized Columnar ELT (Not "Zero-ETL")
- **Status:** Accepted
- **Principles:** P4 | **NFRs:** PERF-030, PERF-031
- **Context:** "Zero-ETL" was flagged as misleading; shredding rows to Arrow/Parquet is ELT work, just internalized.
- **Decision:** The official term is **Internalized Columnar ELT**. Hot ingress writes rows to a lock-free arena; background workers transpose to Arrow RecordBatches and Parquet, then register with Iceberg. This is async and MUST NOT gate the write path.
- **Consequences:** Accurate, defensible positioning. Adds compaction CPU (~15–25% over a dumb byte pipe). Requires CPU isolation (ADR-041).

#### ADR-041: CPU Core Isolation for Compaction
- **Status:** Accepted
- **Principles:** P3 | **NFRs:** PERF-003
- **Context:** Background Arrow transposition can steal CPU and spike p99 tail latency.
- **Decision:** Compaction and vectorization workers are pinned to isolated CPU cores (`sched_setaffinity`), separate from socket/WAL threads.
- **Consequences:** Bounds compaction interference to ≤5% p99 jitter (PERF-003). Requires core budget planning per node.

#### ADR-042: Adaptive Schema Shredding with 64-Key Cap
- **Status:** Accepted
- **Principles:** P3 | **NFRs:** SCALE (schema), PERF-032
- **Context:** Unbounded schema shredding of polymorphic JSON degrades vectorization and explodes metadata.
- **Decision:** Background workers extract the top 64 consistent primitive keys into typed Arrow arrays. Dynamic/polymorphic/nested fields route to an auxiliary `_unstructured_payload` column.
- **Consequences:** Bounded, predictable vectorization. Polymorphic payloads lose SIMD efficiency (accepted trade-off). Requires schema governance (KEI-DES-033).

#### ADR-043: Shared Tenant Iceberg Tables (Not Per-Stream Tables)
- **Status:** Accepted
- **Principles:** P3 | **NFRs:** PERF-030
- **Context:** One Iceberg table per micro-stream would explode catalog metadata, snapshots, and manifest management at 1M streams.
- **Decision:** Default is one Iceberg table per tenant (`tenant_{id}.events`), partitioned by `event_date`/`stream_bucket`/`schema_version`, with `stream_id` as a column. Dedicated per-stream tables are optional for high-isolation or high-throughput streams.
- **Consequences:** Catalog scalability. Per-stream deletion handled via crypto-shredding + column filtering, not table drop.

#### ADR-044: Default Lakehouse Freshness ≤ 60s (Not ≤ 2s)
- **Status:** Accepted
- **Principles:** P4 | **NFRs:** PERF-030, PERF-031
- **Context:** A universal "queryable within 2 seconds" claim is unachievable under load and would drive excessive S3 API and catalog cost.
- **Decision:** Default Iceberg commit freshness is ≤60s. A fast mode (≤5s) is available for tuned, low-load deployments. Sub-2s is NOT a default.
- **Consequences:** Credible freshness target. Bounds object-storage API cost. Supersedes the earlier "2-second" claim.

#### ADR-045: Small-File Aggregation Before Object Upload
- **Status:** Accepted
- **Principles:** P3 | **NFRs:** PERF-030
- **Context:** Uploading many tiny Parquet files causes small-file explosion in Iceberg/Delta catalogs.
- **Decision:** The compactor aggregates sealed chunks into target 64–128 MB Parquet files before object upload, then commits to the catalog.
- **Consequences:** Healthy lakehouse file sizes. Adds batching latency (acceptable given ADR-044 freshness model).

---

### Domain F — Security & Compliance

#### ADR-050: Envelope Encryption with KMS-Managed DEKs
- **Status:** Accepted
- **Principles:** P7 | **NFRs:** SEC-002, SEC-006
- **Context:** Enterprise data must be encrypted at rest with manageable key lifecycle.
- **Decision:** Envelope encryption: Root → Tenant KEK → Stream/Batch DEK. Data is encrypted with AES-256-GCM (AES-NI) or ChaCha20-Poly1305 fallback. DEKs are KMS-managed and locally cached.
- **Consequences:** Strong at-rest encryption with scalable key hierarchy. Introduces KMS as an external dependency (PORT-004).

#### ADR-051: GDPR/CCPA Deletion via Crypto-Shredding
- **Status:** Accepted
- **Principles:** P7 | **NFRs:** COMP-001, COMP-002, REC-007
- **Context:** The immutable log (P1) conflicts with right-to-erasure; physical deletion is slow and conflicts with retention.
- **Decision:** Erasure destroys the relevant DEK/KEK, rendering ciphertext cryptographically unrecoverable immediately. Physical purge occurs via lifecycle/compaction sweeps. Logical erasure is immediate; physical is eventual.
- **Consequences:** Reconciles immutability with erasure. Backups remain ciphertext but unrecoverable post-destruction. Requires audit proof (COMP-004).

#### ADR-052: ABAC Authorization Scoped to Tenant/Stream
- **Status:** Accepted
- **Principles:** P7 | **NFRs:** SEC-004, SEC-005
- **Context:** Multi-tenant fabric requires fine-grained, policy-driven access control.
- **Decision:** Attribute-Based Access Control scoped to tenant and stream namespaces, covering produce/consume/lease/ack/DLQ/admin operations. Gateway identities (Kafka/SQS/AMQP) map to PEF ABAC principals.
- **Consequences:** Fine-grained isolation. Requires policy engine and identity mapping (KEI-DES-036).

---

### Domain G — Multi-Region & Availability

#### ADR-060: Multi-Region Mode A as the Only v1 Same-Stream Mode
- **Status:** Accepted
- **Principles:** P4 | **NFRs:** REC-001, REC-002
- **Context:** Active-active concurrent writes to the same ordered stream require global consensus or conflict resolution that HLCs alone cannot provide.
- **Decision:** v1 supports **Mode A**: single-writer primary per stream with asynchronous replica. Regional namespaces are an alternative for independent-region writes. Multi-writer same-stream is NOT supported in v1.
- **Consequences:** Avoids unsolvable write conflicts. Clear failover semantics. Active-active same-stream deferred to a future version.

#### ADR-061: Same-AZ 99.95% Availability as v1 Baseline
- **Status:** Accepted
- **Principles:** P4 | **NFRs:** AVAIL-006
- **Context:** Availability must be stated as an operational target tied to a deployment topology, not a universal guarantee.
- **Decision:** The v1 availability baseline is 99.95% monthly for a same-AZ 3-node quorum. Cross-AZ quorum (higher availability, higher latency) is a deployment option with its own profile.
- **Consequences:** Credible, topology-bound SLA. Prevents overclaiming universal availability.

#### ADR-062: Write-Latency ≤ 2ms p99 as a Class D Conditional Target
- **Status:** Accepted
- **Principles:** P4 | **NFRs:** PERF-001
- **Context:** A universal sub-2ms SLA is unachievable across all clouds/workloads/encryption settings.
- **Decision:** ≤2ms p99 Tier-0 write latency is a Class D target, valid under Profile P1 with defined hardware (local NVMe), same-rack quorum, and stated encryption/compression settings. It MUST be quoted with those conditions.
- **Consequences:** Honest latency positioning. Prevents SLA misuse. Conditions documented in KEI-ARC-011 §6.1.

---

### Domain H — Ecosystem & Compatibility

#### ADR-070: Compatibility by Published Subset (Not 100% Parity)
- **Status:** Accepted
- **Principles:** P6 | **NFRs:** (gateway)
- **Context:** Claiming "100% functional parity" with Kafka/SQS/AMQP is unclosable and creates perpetual compatibility debt.
- **Decision:** Each gateway publishes a **Compatibility Matrix** of validated operations. Unsupported operations are explicitly listed. The gateway guarantees the published subset, not full parity.
- **Consequences:** Manageable, testable compatibility. Sets correct customer expectations. Supersedes "100% parity" language.

#### ADR-071: Dual Interface — Kafka Gateway + Native Arrow Flight SDK
- **Status:** Accepted
- **Principles:** P6 | **NFRs:** PERF-032
- **Context:** Developers need a zero-friction migration path (Kafka) and a high-performance native path (Arrow Flight).
- **Decision:** Provide a Kafka wire-protocol ingest gateway for drop-in produce, plus native Arrow Flight/gRPC SDKs for streaming and out-of-order task leasing.
- **Consequences:** Lowers adoption friction and offers a performance ceiling. Two interfaces to maintain (accepted).

---

### Domain I — Engineering & Delivery

#### ADR-080: Rust-Only for v1 (Zig Excluded)
- **Status:** Accepted
- **Principles:** P10 | **NFRs:** PORT-001
- **Context:** The original concept listed "Rust / Zig." Zig's ecosystem and library maturity pose delivery risk for a 36-month program.
- **Decision:** Production v1 is implemented in Rust only. Zig is excluded for v1 and may be revisited post-GA.
- **Consequences:** Reduces toolchain risk and leverages the mature Rust async/Arrow ecosystem. Supersedes the dual-language plan.

#### ADR-081: 36-Month Phased Roadmap with Evidence Gates
- **Status:** Accepted
- **Principles:** P9 | **NFRs:** (program)
- **Context:** An 18-month roadmap for this scope was assessed as unrealistic.
- **Decision:** The program is four 9-month phases (Core Engine → Distributed Durability → Ecosystem Bridge → Enterprise Hardening), each with evidence-gated exit criteria (benchmarks, chaos tests, soak tests).
- **Consequences:** Credible delivery plan. Phase exits require evidence, not narrative. Supersedes the 18-month roadmap.

#### ADR-082: Removal of CXL/RDMA Hardware Disaggregation
- **Status:** Accepted (Supersedes original Paradigm 6)
- **Principles:** P4, P10 | **NFRs:** PORT-005
- **Context:** CXL 3.0 / RDMA zero-broker messaging is not broadly available, breaks multi-tenant isolation, and cannot be productized in the v1 timeframe.
- **Decision:** The CXL/RDMA hardware-disaggregated data plane is removed from the v1 architecture.
- **Consequences:** Keeps the architecture productizable on standard cloud hardware. May be revisited as a research path later.

#### ADR-083: Removal of Active Dataflow / In-Broker Materialized Views from v1
- **Status:** Accepted (Supersedes original Paradigm 7)
- **Principles:** P4 | **NFRs:** (scope)
- **Context:** Embedding a differential-dataflow engine and in-broker SQL turns the broker into a database, exploding scope and risk.
- **Decision:** Active dataflow and in-broker materialized views are removed from v1. Lakehouse query is served via the Iceberg/Arrow path.
- **Consequences:** Focuses v1 on the core fabric. Query use cases served by external engines against Iceberg. Reduces blast radius.

---

## 6. ADR Index & Traceability

| ADR | Title | Domain | Status | Principles | Key NFRs |
|---|---|---|---|---|---|
| ADR-001 | Immutable log as single source of truth | A | Accepted | P1,P2 | DUR-001/002 |
| ADR-002 | Log-Bitmap duality | A | Accepted | P2 | SCALE-004 |
| ADR-003 | Virtual DLQ via flag | A | Accepted | P1,P3 | MEM-004 |
| ADR-004 | Mandatory DLQ eviction | A | Accepted | P3 | MEM-004 |
| ADR-005 | Ordering by stream/entity key | A | Accepted | P4 | SCALE-001 |
| ADR-006 | Hot-key isolation, not parallelization | A | Accepted | P4 | PERF-011 |
| ADR-010 | Multiplexed LSM-WAL | B | Accepted | P3 | SCALE-001..003 |
| ADR-011 | Two-tier storage (NVMe + S3) | B | Accepted | P5 | PERF-001 |
| ADR-012 | Stateless node recovery | B | Accepted | P3 | AVAIL-002 |
| ADR-013 | Batch WAL framing + CRC32C | B | Accepted | P4 | DUR-007 |
| ADR-014 | Single-pass compaction (WAF≤1.35) | B | Accepted | P3 | PERF-020 |
| ADR-015 | io_uring + O_DIRECT | B | Accepted | P10 | PORT-002 |
| ADR-020 | Two ACK durability modes | C | Accepted | P4,P5 | DUR-003/004 |
| ADR-021 | ACK_FAST default | C | Accepted | P5 | DUR-003 |
| ADR-022 | At-least-once default | C | Accepted | P4 | DUR-001..006 |
| ADR-023 | Deterministic coordinator sharding | C | Accepted | P3 | SCALE-006 |
| ADR-024 | Epoch fencing | C | Accepted | P4 | AVAIL-004 |
| ADR-025 | Hierarchical timing wheel | C | Accepted | P3 | PERF-011 |
| ADR-030 | Two-tier Raft topology | D | Accepted | P2,P3 | DUR-001 |
| ADR-031 | CP local + causal WAN | D | Accepted | P4 | REC-001 |
| ADR-040 | Internalized Columnar ELT | E | Accepted | P4 | PERF-030 |
| ADR-041 | CPU isolation for compaction | E | Accepted | P3 | PERF-003 |
| ADR-042 | Adaptive shredding, 64-key cap | E | Accepted | P3 | PERF-032 |
| ADR-043 | Shared tenant Iceberg tables | E | Accepted | P3 | PERF-030 |
| ADR-044 | Freshness ≤60s default | E | Accepted | P4 | PERF-030/031 |
| ADR-045 | Small-file aggregation | E | Accepted | P3 | PERF-030 |
| ADR-050 | Envelope encryption | F | Accepted | P7 | SEC-002/006 |
| ADR-051 | Crypto-shredding for GDPR | F | Accepted | P7 | COMP-001 |
| ADR-052 | ABAC authorization | F | Accepted | P7 | SEC-004 |
| ADR-060 | Multi-region Mode A only | G | Accepted | P4 | REC-001 |
| ADR-061 | Same-AZ 99.95% baseline | G | Accepted | P4 | AVAIL-006 |
| ADR-062 | ≤2ms p99 as Class D target | G | Accepted | P4 | PERF-001 |
| ADR-070 | Compatibility by subset | H | Accepted | P6 | (gateway) |
| ADR-071 | Dual interface | H | Accepted | P6 | PERF-032 |
| ADR-080 | Rust-only for v1 | I | Accepted | P10 | PORT-001 |
| ADR-081 | 36-month evidence-gated roadmap | I | Accepted | P9 | (program) |
| ADR-082 | Remove CXL/RDMA | I | Accepted | P4,P10 | PORT-005 |
| ADR-083 | Remove active dataflow | I | Accepted | P4 | (scope) |

---

## 7. Decision Governance

### 7.1 Proposing a Decision
Any principal engineer may propose an ADR by submitting a draft with Context, Decision, and Consequences, citing affected principles and NFRs.

### 7.2 Approval
An ADR is Accepted when approved by the Chief Architect plus the owning domain's Principal Engineer. Decisions affecting security require Security Lead sign-off; decisions affecting NFR targets require SRE sign-off.

### 7.3 Superseding
An Accepted ADR is changed only by a new Accepted ADR that explicitly names it as superseded. The superseded record is retained with status **Superseded**.

### 7.4 Review Cadence
The ADR index is reviewed at each phase gate (every 9 months) to confirm no decision has been silently violated or become obsolete.

---

## 8. Glossary (Additions)

| Term | Definition |
|---|---|
| ADR | Architecture Decision Record. |
| Principle | A binding rule that all designs must satisfy. |
| Superseded | An ADR replaced by a newer decision. |
| Decision Gate | The approval point at which a Proposed ADR becomes Accepted. |

---

## 9. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial approved ADR index. Consolidates 10 binding principles and 38 ADRs across nine domains. Records the supersession of the original CXL/RDMA paradigm (ADR-082), active dataflow paradigm (ADR-083), 18-month roadmap (ADR-081), dual-language plan (ADR-080), per-record WAL header (ADR-013), "Zero-ETL" terminology (ADR-040), "100% parity" language (ADR-070), and 2-second freshness claim (ADR-044). |