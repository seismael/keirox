# A. KEI-ADF-000 — Architecture Documentation Framework & Register

## A.1 Documentation Hierarchy

The suite follows a four-level hierarchy (aligned with ISO/IEC/IEEE 42010 and C4 conventions):

| Level | Purpose | Audience |
|---|---|---|
| **L0 — Vision & Context** | Why the system exists; business context; scope; principles; system boundary. | Executives, architects, investors, security review boards |
| **L1 — Conceptual Architecture** | What the system is; core invariants; quality attributes; decisions. | Architects, principal engineers, reviewers |
| **L2 — Subsystem Architecture** | How each subsystem is structured and interacts. | Engineering leads, subsystem owners |
| **L3 — Detailed Design Specs** | Exact formats, algorithms, protocols, APIs, runbooks. | Implementing engineers, test engineers |

## A.2 Document Register

The master document register, reading paths, and normative precedence rules are defined in [`docs/INDEX.md`](INDEX.md).

| ID | Title | Level | Status |
|---|---|---|---|
| KEI-INDEX | Architecture Documentation Index & Routing Map | Framework | Approved |
| KEI-ARC-001 | Architecture Vision & System Context | L0 | Approved |
| KEI-ARC-010 | Conceptual Architecture & Golden Invariant | L1 | Approved |
| KEI-ARC-011 | Quality Attributes & Non-Functional Requirements | L1 | Approved |
| KEI-ARC-012 | Architecture Principles & Decision Record Index (ADR) | L1 | Approved |
| KEI-ARC-020 | Storage Engine Architecture (LSM-WAL & Tiering) | L2 | Approved |
| KEI-ARC-021 | Consumption State Plane Architecture (Bitmaps, Leases, DLQ) | L2 | Approved |
| KEI-ARC-022 | Consensus, Coordination & High Availability Architecture | L2 | Approved |
| KEI-ARC-023 | Columnar ELT & Lakehouse Integration Architecture | L2 | Approved |
| KEI-ARC-024 | Protocol Gateways & SDK Architecture | L2 | Approved |
| KEI-ARC-025 | Security, Privacy & Compliance Architecture | L2 | Approved |
| KEI-ARC-026 | Multi-Region Replication & Disaster Recovery Architecture | L2 | Approved |
| KEI-ARC-027 | Operability, Observability & Capacity Architecture | L2 | Approved |
| KEI-DES-030 | WAL Binary Format & On-Disk Layout Specification | L3 | Approved |
| KEI-DES-031 | State Plane Data Structures & Algorithms Specification | L3 | Approved |
| KEI-DES-032 | Producer/Consumer/Lease/ACK API & Protocol Specification | L3 | Approved |
| KEI-DES-033 | Schema Registry & Adaptive Shredding Specification | L3 | Approved |
| KEI-DES-034 | Iceberg Catalog Committer Specification | L3 | Approved |
| KEI-DES-035 | Gateway Wire-Protocol Compatibility Matrices | L3 | Approved |
| KEI-DES-036 | Encryption, Key Management & Crypto-Shredding Specification | L3 | Approved |
| KEI-OPS-040 | Operations Runbooks, Upgrade & DR Procedures | L3 | Approved |
| KEI-OPS-041 | Validation, Benchmark & Chaos Test Plan | L3 | Approved |
| KEI-VAL-050 | Final Cross-Document Consistency Audit | Closure | Approved |
| KEI-VAL-051 | End-To-End Requirements Traceability Matrix | Closure | Approved |
| KEI-VAL-052 | Architecture Release Readiness Checklist | Closure | Approved |

**Rule:** Lower-level documents MUST NOT contradict higher-level documents. Conflicts are resolved upward via ADRs in KEI-ARC-012.

---

# B. KEI-ARC-001 — Architecture Vision & System Context

## B.1 Document Control

| Field | Value |
|---|---|
| Document ID | KEI-ARC-001 |
| Version | 1.0 |
| Status | **Approved for Engineering** |
| Classification | Internal / Engineering Confidential |
| Owner | Chief Architect |
| Required Reviewers | Principal Engineer (Storage), Principal Engineer (Distributed Systems), Security Lead, Platform Operations Lead |
| Supersedes | “Next-Generation Real-Time Event & Work Fabric” whitepaper (draft), “Polymorphic Event Fabric” whitepaper (draft) |
| Keywords | MUST, SHOULD, MAY per RFC 2119 |

## B.2 Purpose, Scope, and Audience

**Purpose.** This document defines *why* Keirox exists, *what* boundary it draws around the system, *which* problems it will and will not solve, and *which* principles govern all downstream design decisions.

**Scope.** It covers the Keirox Polymorphic Event Fabric as a product architecture: a unified, durable, multi-tenant event ingestion, dispatch, and lakehouse-materialization fabric.

**Out of scope.** Implementation details (see L2/L3 documents), organizational staffing plans, and pricing/packaging decisions.

**Audience.** Executives and investors (sections B.4–B.6), architects (all), security/compliance reviewers (B.9–B.11), engineering leads (B.12–B.14).

## B.3 How to Read This Suite

1. Read **KEI-ARC-001** for boundary and intent.
2. Read **KEI-ARC-010/011/012** for the conceptual model, measurable quality targets, and binding decisions.
3. Read the **L2 documents** for the subsystem you own or review.
4. Implement only from **L3 specifications**; L0–L2 documents are not implementation contracts.

## B.4 Business Context & Problem Statement

Enterprises today operate fragmented messaging topologies — the **Frankenstein Infrastructure Tax**:

- **Kafka/Redpanda** for durable replayable logs (partition-bound, head-of-line blocking, rebalance friction).
- **RabbitMQ/SQS** for transactional work queues (destructive ACKs, no replay, no analytics).
- **Redis Streams** for low-latency signaling (RAM-bound, durability trade-offs).
- **Flink/Kafka Connect + S3/Iceberg** for lakehouse ingestion (multi-hop ETL, dual-write risk, network egress tax).

Consequences: duplicated compute and operations, dual-write desynchronization bugs, partition-sizing pain at high tenant cardinality, repeated SerDe CPU tax, and inter-system egress cost.

**Problem statement (normative):** *No existing production system allows the same immutable dataset to be consumed simultaneously as a replayable stream, an out-of-order leased work queue, a dead-letter view, and a queryable lakehouse table, at high tenant cardinality, with enterprise durability and compliance.*

## B.5 Vision Statement

> **Keirox writes every event exactly once to an immutable physical log, and lets each consumer choose its semantics — stream, queue, DLQ view, or columnar table — through a replicated, mutable state overlay.**

Strategic outcomes:

1. **Consolidation:** replace the core ingestion/buffering/dispatch layer of 3–5 systems with one fabric.
2. **Cardinality:** support 100K–1M+ virtual streams per node without partition topology management.
3. **Lakehouse-native:** internalized columnar ELT delivers near-real-time Iceberg tables without external pipelines.
4. **Enterprise trust:** explicit, testable delivery/ordering/durability semantics; GDPR crypto-shredding; SOC2/ISO27001-ready controls.
5. **Credible economics:** 20–40% net greenfield infrastructure savings; 10–25% net migration savings (scenario-dependent).

## B.6 Goals and Non-Goals

| Goals | Non-Goals (explicit exclusions) |
|---|---|
| Unify stream replay and leased task queues on one log | General-purpose OLTP database or in-broker SQL engine (v1) |
| Out-of-order ACKs, leases, retries, virtual DLQ | Sub-100µs ephemeral caching (Redis territory) |
| Entity-key ordering without static partitions | Complex multi-hop AMQP exchange topologies (v1) |
| High-cardinality multi-tenant micro-streams | Full Flink stateful stream-processing replacement |
| Durable <2ms p99 Tier-0 writes *under defined profiles* | Universal sub-2ms SLA across all clouds/workloads |
| Idempotent produce + optional transactional append | Universal exactly-once side effects without consumer cooperation |
| Kafka/SQS/AMQP compatibility *subsets* | 100% wire-protocol parity with any incumbent |
| Single-writer-per-stream multi-region DR | Active-active concurrent writes to the same ordered stream (v1) |
| KMS envelope encryption + crypto-shredding | CXL/RDMA hardware-disaggregated data planes |

## B.7 Architectural Principles (binding)

| # | Principle | Rationale |
|---|---|---|
| P1 | **Immutability first.** The physical log is append-only and never rewritten. | Enables replay, audit, virtual DLQ, and safe lakehouse export. |
| P2 | **Truth/state separation.** Storage truth is immutable; consumption semantics live in replicated overlays. | Resolves the queue/stream dichotomy without data duplication. |
| P3 | **Bounded everything.** Every memory structure has a quota, spill path, and shedding policy. | Prevents unbounded lease/bitmap/manifest growth in production. |
| P4 | **Explicit semantics over magic.** Guarantees are documented modes (`ACK_FAST`, `ACK_DURABLE`, read modes), not marketing claims. | Enterprise trust requires testable contracts. |
| P5 | **Progressive durability.** Clients choose latency/durability trade-offs per request. | One size does not fit signaling vs. payments workloads. |
| P6 | **Compatibility by published subset.** Gateways expose validated compatibility matrices, never parity promises. | Prevents unclosable compatibility debt. |
| P7 | **Security by default.** Encryption in transit/at rest, ABAC, audit, crypto-shredding are baseline, not add-ons. | Enterprise adoption gate. |
| P8 | **Observability is a product feature.** Watermark lag, lease age, ACK replication lag, bitmap spill, S3 backlog are first-class metrics. | The state plane is only operable if it is visible. |
| P9 | **Evidence gates phases.** No phase exits on narrative; exit requires benchmarks, chaos tests, and soak tests. | De-risks the 36-month program. |
| P10 | **Single artifact, modular subsystems.** One deployable binary composed of independently testable crates/modules. | Deployment simplicity without internal monolith coupling. |

## B.8 System Context (C4 Level 1)

```
                         ┌──────────────────────────────┐
                         │        Identity Provider     │ (OIDC / mTLS PKI)
                         └──────────────┬───────────────┘
                                        │ authn
 ┌───────────────────┐   produce        ▼                consume/lease     ┌───────────────────┐
 │ Kafka-compatible  │──────────►┌──────────────┐◄─────────────────────────│ Stream consumers, │
 │ producers, CDC    │          │              │                          │ task workers,     │
 │ (Debezium, etc.)  │          │   KEIROX     │─────────────────────────►│ DLQ operators,    │
 └───────────────────┘          │  CLUSTER     │   Arrow Flight / gRPC    │ AI agent runners  │
 ┌───────────────────┐          │              │                          └───────────────────┘
 │ Native SDK apps   │─────────►│ • Gateways   │
 │ (Rust/Go/Py/Java) │          │ • Storage    │   sealed Parquet +     ┌───────────────────┐
 └───────────────────┘          │   nodes      │── Iceberg commits ────►│ Object Storage    │
                                │ • Coordina-  │                        │ (S3/GCS) +        │
 ┌───────────────────┐  admin   │   tors       │◄── manifests ──────────│ Iceberg Catalog   │
 │ Ops / Admin /     │─────────►│ • Compactors │                        └────────┬──────────┘
 │ FinOps consoles   │          └──────┬───────┘                                 │ query
 └───────────────────┘                 │                                         ▼
                                ┌──────▼───────┐   key ops        ┌───────────────────────────┐
                                │ Observability│                  │ Query engines: DuckDB,    │
                                │ (OTel/Prom)  │                  │ Spark, Polars, Trino      │
                                └──────────────┘                  └───────────────────────────┘
                                        ▲
                                 ┌──────┴───────┐
                                 │  KMS (AWS /  │
                                 │  Vault)      │
                                 └──────────────┘
```

**System boundary.** Everything inside “KEIROX CLUSTER” is in scope. Object storage, KMS, IdP, catalog services, and query engines are **external dependencies** consumed via well-defined interfaces.

## B.9 Stakeholders and Primary Concerns

| Stakeholder | Primary Concerns | Addressed In |
|---|---|---|
| CTO / Platform leadership | Consolidation value, migration risk, roadmap | B.4–B.6, B.13 |
| FinOps | TCO, egress, S3 API costs | KEI-ARC-011, KEI-ARC-027 |
| Application teams | Delivery semantics, ordering, SDK friction | KEI-ARC-010, KEI-DES-032 |
| Data/AI teams | Lakehouse freshness, schema evolution, query access | KEI-ARC-023, KEI-DES-033/034 |
| Security & Compliance | Encryption, deletion, audit, tenancy isolation | KEI-ARC-025, KEI-DES-036 |
| SRE / Operations | Upgrades, DR, observability, capacity | KEI-ARC-027, KEI-OPS-040 |
| QA / Reliability | Testability, chaos behavior, Jepsen evidence | KEI-OPS-041 |

## B.10 Quality Attribute Summary (targets; full spec in KEI-ARC-011)

| Attribute | Target (v1) |
|---|---|
| Write latency | <2ms p99 Tier-0, defined hardware/workload profile, quorum durability |
| Durability | Zero loss of quorum-committed records (JML = 0) |
| Delivery | At-least-once default; idempotent produce; optional transactional append |
| Ordering | Strict per stream / per entity key; concurrent across independent keys |
| Availability | Node failover <5s; coordinator failover <3.5s (targets, tested) |
| Scalability | 100K–1M streams/node; 10M cluster-wide via sharding |
| Lakehouse freshness | Default <60s; fast mode <5s (tuned, low-load) |
| Security | TLS 1.3/mTLS, SASL/OIDC, ABAC, envelope encryption, crypto-shredding |
| Compliance posture | GDPR/CCPA deletion via crypto-shredding; SOC2/ISO27001-ready controls |
| Recoverability | RPO ≤5s normal / ≤60s degraded; RTO ≤5min (Mode A replication) |

## B.11 Constraints and Assumptions

**Constraints.**
- C1: Linux-first I/O strategy (`io_uring`, `O_DIRECT`), with `epoll` fallback.
- C2: Implementation language: Rust (production); no Zig in v1 (ecosystem maturity risk accepted as a decision, see ADR index).
- C3: Durable storage tiers: local NVMe (Tier-0) + S3-compatible object storage (Tier-1).
- C4: External KMS and OIDC/PKI required for enterprise mode.
- C5: 36-month phased program with evidence-gated exits (P9).

**Assumptions.**
- A1: Baseline workload 100K msgs/s @1KB (100 MB/s), 30-day retention for TCO modeling.
- A2: Target beachhead: multi-tenant SaaS, IoT/gaming, AI agent platforms (high stream cardinality).
- A3: Enterprises accept compatibility-subset gateways plus native SDKs as migration path.
- A4: Cryptographic deletion is acceptable evidence for right-to-erasure, subject to customer policy review.

## B.12 Top-Level Risks and Mitigations

| Risk | Severity | Mitigation | Owner Doc |
|---|---|---|---|
| Distributed state-plane correctness (leases/ACKs under failover) | Critical | Replicated lease journal, epoch fencing, ACK durability modes, Jepsen gate | KEI-ARC-021/022, KEI-OPS-041 |
| Kafka ecosystem gravity | High | Ingest gateway subset + Connect bridge; native SDK differentiation | KEI-ARC-024, KEI-DES-035 |
| Compaction interference with hot path | High | CPU pinning, admission control, backpressure, jitter budgets | KEI-ARC-020/027 |
| Lakehouse metadata explosion | Medium | Shared tenant tables, commit batching, manifest compaction | KEI-ARC-023, KEI-DES-034 |
| Memory model overruns at scale | High | Quotas, spill paths, per-node memory budget, 1M-stream gate | KEI-ARC-011/021 |
| Security/compliance blockers | Critical | Baseline encryption/ABAC/audit from Phase 1; crypto-shredding by Phase 4 gate | KEI-ARC-025 |
| Schedule overcommit | High | 36-month phased plan, evidence gates, scope exclusions (B.6) | Program |

## B.13 Delivery Strategy Summary

Five evidence-gated phases (full criteria in KEI-ARC-011 and KEI-OPS-041):

1. **Phase 1 (M1–9):** Single-node core — multiplexed WAL, bitmap state plane, timing wheel, virtual DLQ, Arrow vectorizer.
2. **Phase 2 (M10–18):** Distributed durability — data-plane Raft, coordinator sharding, lease journal, S3 offload, crash recovery.
3. **Phase 3 (M19–27):** Ecosystem bridge — Kafka gateway subset, Arrow Flight SDKs, Iceberg committer, schema registry.
4. **Phase 4 (M28–36):** Enterprise hardening — KMS crypto-shredding, Mode A multi-region, backup/restore, rolling upgrades, SQS/AMQP subsets, chaos/Jepsen certification.
5. **Phase 5 (M37–42):** Productization, Distribution & Day-2 Operations — Operator, Helm, Terraform, Migration bridge, Telemetry, and GA Launch.

## B.14 Glossary (authoritative subset)

| Term | Definition |
|---|---|
| PEF | Polymorphic Event Fabric — the Keirox architecture family. |
| Golden Invariant | Data written once to an immutable log; semantics defined by consumer state overlays. |
| State Overlay | Replicated Roaring Bitmap + lease metadata per (tenant, stream, group, shard). |
| W_base | Sliding base watermark; lowest non-terminal offset boundary. |
| Virtual DLQ | Offsets flagged `EVICTED_DLQ` and indexed in the Sparse Exception Table; no payload copy. |
| State Shard | Horizontal unit of consumer-state ownership: hash(tenant, stream, group, bucket). |
| Coordinator Epoch | Monotonic generation used to fence stale coordinators. |
| ACK_FAST / ACK_DURABLE | Client-selectable acknowledgment durability modes. |
| Internalized Columnar ELT | In-broker row→Arrow→Parquet transformation and Iceberg registration. |
| Crypto-Shredding | Right-to-erasure via KMS key destruction rendering ciphertext unrecoverable. |
| Tier-0 / Tier-1 | Local NVMe hot durability tier / object-storage cold retention tier. |

## B.15 References

- RFC 2119 (requirement language); ISO/IEC/IEEE 42010 (architecture description); C4 model conventions.
- Apache Arrow, Parquet, Iceberg specifications; Roaring Bitmap literature; Raft consensus (Ongaro & Ousterhout); Hybrid Logical Clocks (Kulkarni et al.).
- Predecessor whitepapers (superseded) and final audit resolution record (internal).

## B.16 Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial approved baseline; supersedes draft whitepapers; incorporates audit resolutions (ACK modes, state sharding, bounded claims, security baseline, 36-month roadmap). |