---
id: KEI-INDEX
title: Keirox Polymorphic Event Fabric — Architecture Documentation Index & Routing Map
version: 1.0
phase: Phase 1 Closure — Architecture Baseline
status: Approved
authority: Chief Architect / Architecture Review Board
last_updated: 2026-08-30
---

# Keirox Polymorphic Event Fabric — Architecture Documentation Index

This `INDEX.md` is the authoritative navigation, routing, and validation entry point for all Keirox Polymorphic Event Fabric (PEF) architecture specifications.

It enables human engineers, AI development agents, reviewers, and CI/CD validation tools to:
1. Locate the exact governing specification for any subsystem instantly with zero context waste.
2. Enforce the **Golden Invariant** and system boundaries across all implementation code.
3. Validate requirements end-to-end against the Requirements Traceability Matrix ([`KEI-VAL-051.md`](KEI-VAL-051.md)) and Test Plan ([`KEI-OPS-041.md`](KEI-OPS-041.md)).
4. Eliminate historical or superseded whitepaper overclaims.

---

## 1. Authoritative Document Registry

The approved Phase 1 architecture baseline consists of the following 24 formal specifications + this Index:

### 1.1 Level 0 & Level 1 — Vision, Concepts, NFRs & Principles

| Document ID | File Path | Status | Scope & Key Invariants |
|---|---|---|---|
| **KEI-INDEX** | [`INDEX.md`](INDEX.md) | Approved | Single-entry architecture register, routing map, and precedence ladder. |
| **KEI-ARC-001** | [`KEI-ARC-001.md`](KEI-ARC-001.md) | Approved | System context, stakeholder boundaries, core problems, and unified fabric vision. |
| **KEI-ARC-010** | [`KEI-ARC-010.md`](KEI-ARC-010.md) | Approved | Conceptual architecture, dual-plane separation, and **The Golden Invariant**. |
| **KEI-ARC-011** | [`KEI-ARC-011.md`](KEI-ARC-011.md) | Approved | Measurable NFRs (PERF, DUR, AVAIL, SCALE, MEM, REC, SEC, COMP), Workload Profiles (P1–P6). |
| **KEI-ARC-012** | [`KEI-ARC-012.md`](KEI-ARC-012.md) | Approved | 10 Governing Principles (P1–P10) and 38 binding Architecture Decision Records (ADRs). |

### 1.2 Level 2 — Subsystem Architectures

| Document ID | File Path | Status | Scope & Key Invariants |
|---|---|---|---|
| **KEI-ARC-020** | [`KEI-ARC-020.md`](KEI-ARC-020.md) | Approved | Storage Engine: `io_uring` WAL ring buffer, Tier-0 NVMe, Tier-1 S3, single-pass compaction. |
| **KEI-ARC-021** | [`KEI-ARC-021.md`](KEI-ARC-021.md) | Approved | State Plane: Roaring Bitmap state overlays, lease timing wheels, virtual DLQ, watermark purging. |
| **KEI-ARC-022** | [`KEI-ARC-022.md`](KEI-ARC-022.md) | Approved | Consensus: Dual Raft planes (Data vs. Metadata/State), epoch fencing, quorum commit gates. |
| **KEI-ARC-023** | [`KEI-ARC-023.md`](KEI-ARC-023.md) | Approved | Columnar ELT: Internalized ELT, Arrow vectorizer, adaptive shredding, Iceberg integration. |
| **KEI-ARC-024** | [`KEI-ARC-024.md`](KEI-ARC-024.md) | Approved | Protocol Gateways: Compatibility-by-Subset philosophy, Kafka, SQS, AMQP, Arrow Flight gRPC. |
| **KEI-ARC-025** | [`KEI-ARC-025.md`](KEI-ARC-025.md) | Approved | Security: AES-256-GCM envelope encryption, ABAC PDP, tenant isolation, crypto-shredding. |
| **KEI-ARC-026** | [`KEI-ARC-026.md`](KEI-ARC-026.md) | Approved | Multi-Region: Mode A single-writer primary, HLC causal consistency, region epoch fencing, DR. |
| **KEI-ARC-027** | [`KEI-ARC-027.md`](KEI-ARC-027.md) | Approved | Operability: Observability catalog, progressive backpressure ladder, quotas, rolling upgrades. |

### 1.3 Level 3 — Detailed Design Specifications

| Document ID | File Path | Status | Scope & Key Invariants |
|---|---|---|---|
| **KEI-DES-030** | [`KEI-DES-030.md`](KEI-DES-030.md) | Approved | WAL Binary Framing: 128-byte Batch Headers, 32-byte Record Entries, CRC32C, 4KB page alignment. |
| **KEI-DES-031** | [`KEI-DES-031.md`](KEI-DES-031.md) | Approved | State Plane Data Structures: `Roaring64Map`, Timing Wheel, watermark algorithm, lease journal. |
| **KEI-DES-032** | [`KEI-DES-032.md`](KEI-DES-032.md) | Approved | API & Protocol: Protobuf RPC contracts, ACK modes (FAST/DURABLE), error taxonomy, idempotency. |
| **KEI-DES-033** | [`KEI-DES-033.md`](KEI-DES-033.md) | Approved | Schema Registry: Adaptive inference scoring, 64-field cap, `_unstructured_payload` fallback. |
| **KEI-DES-034** | [`KEI-DES-034.md`](KEI-DES-034.md) | Approved | Iceberg Committer: Atomic catalog commits, commit ledger, snapshot lifecycle, orphan cleanup. |
| **KEI-DES-035** | [`KEI-DES-035.md`](KEI-DES-035.md) | Approved | Gateway Compatibility Matrices: S0–S3 support tiers for Kafka, SQS, and AMQP protocols. |
| **KEI-DES-036** | [`KEI-DES-036.md`](KEI-DES-036.md) | Approved | Encryption & Key Management: KMS adapters, DEK lifecycle, Destroyed-Key Registry, erasure workflow. |

### 1.4 Level 3 — Operations, Validation & Certification

| Document ID | File Path | Status | Scope & Key Invariants |
|---|---|---|---|
| **KEI-OPS-040** | [`KEI-OPS-040.md`](KEI-OPS-040.md) | Approved | Operations Runbooks: 20 operational runbooks (OPS-RB-001..020), DR failover, emergency shedding. |
| **KEI-OPS-041** | [`KEI-OPS-041.md`](KEI-OPS-041.md) | Approved | Test Plan: Benchmarks, durability kill tests (JML=0), chaos matrix (15 scenarios), Jepsen suites. |
| **KEI-VAL-050** | [`KEI-VAL-050.md`](KEI-VAL-050.md) | Approved | Consistency Audit: Independent mathematical & logical cross-document consistency certification. |
| **KEI-VAL-051** | [`KEI-VAL-051.md`](KEI-VAL-051.md) | Approved | Requirements Traceability Matrix (RTM): 113 requirements mapped end-to-end to design and tests. |
| **KEI-VAL-052** | [`KEI-VAL-052.md`](KEI-VAL-052.md) | Approved | Release Readiness Checklist: 5-gate executive sign-off for Phase-1 engineering execution. |

---

## 2. Targeted Ingestion Routing Map

When working on a specific subsystem, agents MUST ingest only the relevant reading path to optimize token context and eliminate hallucinations:

| Engineering Domain | Primary Specifications (Read First) | Supporting Specs & Contracts | Verification Suite |
|---|---|---|---|
| **Storage Engine / WAL** | [`KEI-ARC-020.md`](KEI-ARC-020.md)<br>[`KEI-DES-030.md`](KEI-DES-030.md) | [`KEI-ARC-010.md`](KEI-ARC-010.md)<br>[`KEI-ARC-027.md`](KEI-ARC-027.md) | [`KEI-OPS-041.md`](KEI-OPS-041.md) §6, §7<br>`DUR-T-001..007` |
| **State Plane / Queuing** | [`KEI-ARC-021.md`](KEI-ARC-021.md)<br>[`KEI-DES-031.md`](KEI-DES-031.md) | [`KEI-DES-032.md`](KEI-DES-032.md)<br>[`KEI-ARC-022.md`](KEI-ARC-022.md) | [`KEI-OPS-041.md`](KEI-OPS-041.md) §7.2, §8<br>`STA-T`, `LEA-T` |
| **Consensus / HA** | [`KEI-ARC-022.md`](KEI-ARC-022.md) | [`KEI-ARC-020.md`](KEI-ARC-020.md)<br>[`KEI-DES-031.md`](KEI-DES-031.md) | [`KEI-OPS-041.md`](KEI-OPS-041.md) §9, §10<br>`FO-T`, `CHAOS` |
| **Columnar ELT / Iceberg** | [`KEI-ARC-023.md`](KEI-ARC-023.md)<br>[`KEI-DES-033.md`](KEI-DES-033.md)<br>[`KEI-DES-034.md`](KEI-DES-034.md) | [`KEI-ARC-020.md`](KEI-ARC-020.md)<br>[`KEI-DES-030.md`](KEI-DES-030.md) | [`KEI-OPS-041.md`](KEI-OPS-041.md) §6.4<br>`PERF-T-030..032` |
| **Gateways / SDKs** | [`KEI-ARC-024.md`](KEI-ARC-024.md)<br>[`KEI-DES-032.md`](KEI-DES-032.md)<br>[`KEI-DES-035.md`](KEI-DES-035.md) | [`KEI-ARC-025.md`](KEI-ARC-025.md)<br>[`KEI-ARC-027.md`](KEI-ARC-027.md) | [`KEI-OPS-041.md`](KEI-OPS-041.md) §13<br>`COMPAT` Conformance |
| **Security / Crypto-Shred** | [`KEI-ARC-025.md`](KEI-ARC-025.md)<br>[`KEI-DES-036.md`](KEI-DES-036.md) | [`KEI-DES-030.md`](KEI-DES-030.md)<br>[`KEI-ARC-026.md`](KEI-ARC-026.md) | [`KEI-OPS-041.md`](KEI-OPS-041.md) §14<br>`SEC-T`, `ERASE-T` |
| **Multi-Region / DR** | [`KEI-ARC-026.md`](KEI-ARC-026.md) | [`KEI-ARC-022.md`](KEI-ARC-022.md)<br>[`KEI-DES-036.md`](KEI-DES-036.md) | [`KEI-OPS-041.md`](KEI-OPS-041.md) §9.3<br>`FO-T-020..024` |
| **Operations / SRE** | [`KEI-ARC-027.md`](KEI-ARC-027.md)<br>[`KEI-OPS-040.md`](KEI-OPS-040.md) | [`KEI-OPS-041.md`](KEI-OPS-041.md)<br>[`KEI-VAL-052.md`](KEI-VAL-052.md) | [`KEI-OPS-041.md`](KEI-OPS-041.md) §15<br>`CAP-T` suite |

---

## 3. Normative Precedence & Invariant Hierarchy

When evaluating or resolving technical questions, agents and engineers MUST follow this strict precedence hierarchy:

```text
Level 1: The Golden Invariant (KEI-ARC-010 §3) & 10 Principles (KEI-ARC-012 §3)
   │
   ▼
Level 2: Binding Architecture Decision Records (ADRs 001..083 in KEI-ARC-012)
   │
   ▼
Level 3: Non-Functional Requirements & Profiles (KEI-ARC-011)
   │
   ▼
Level 4: Subsystem Architectures (KEI-ARC-020..027)
   │
   ▼
Level 5: Detailed Design Specifications (KEI-DES-030..036)
   │
   ▼
Level 6: Operations & Validation Specifications (KEI-OPS-040..041, KEI-VAL-050..052)
```

**Interpretation Rule**: Lower-level specifications or implementations MUST NEVER contradict or weaken a higher-level invariant.

---

## 4. Core Invariants Summary

| Invariant | Governing Spec | Invariant Rule |
|---|---|---|
| **The Golden Invariant** | [`KEI-ARC-010`](KEI-ARC-010.md) | Data is written exactly once to an immutable log. Consumption semantics are mutable state overlays. |
| **Log Immutability** | [`KEI-ARC-020`](KEI-ARC-020.md) | Physical WAL is strictly append-only. Zero in-place mutations or physical deletions. |
| **Quorum Gate** | [`KEI-ARC-022`](KEI-ARC-022.md) | Producer ACK MUST be issued only after synchronous local quorum replication ($JML=0$). |
| **Bounded State** | [`KEI-ARC-012`](KEI-ARC-012.md) | Every mutable structure (bitmaps, leases, heaps, journals) MUST have quotas and spill/shedding paths. |
| **Watermark Advance** | [`KEI-ARC-021`](KEI-ARC-021.md) | Mandatory DLQ eviction purges poison pills to prevent watermark stall and unbounded memory growth. |
| **At-Least-Once Default** | [`KEI-ARC-010`](KEI-ARC-010.md) | Queue delivery is at-least-once. Exactly-once external side effects require consumer idempotency. |
| **Compatibility by Subset** | [`KEI-ARC-024`](KEI-ARC-024.md) | Gateways implement published compatibility matrices (S0–S3), never false 100% parity claims. |
| **Fail-Secure Security** | [`KEI-ARC-025`](KEI-ARC-025.md) | Encryption failures (e.g., KMS down) MUST deny access/writes; zero fallback to plaintext. |
| **Crypto-Shredding** | [`KEI-DES-036`](KEI-DES-036.md) | GDPR/CCPA erasure is performed by DEK destruction recorded in the Destroyed-Key Registry. |
| **Mode A Multi-Region** | [`KEI-ARC-026`](KEI-ARC-026.md) | Same-stream WAN replication is single-writer primary only, fenced by monotonic region epochs. |

---

## 5. Banned & Superseded Terminology

Agents MUST NOT reintroduce the following superseded claims from early draft whitepapers:

| ❌ Banned / Superseded Draft Concept | ✅ Correct Authoritative Specification Baseline |
|---|---|
| "Zero-ETL" | **Internalized Columnar ELT** ([`KEI-ARC-023`](KEI-ARC-023.md)) |
| "100% Kafka / SQS / AMQP Parity" | **Compatibility-by-Subset** ([`KEI-ARC-024`](KEI-ARC-024.md), [`KEI-DES-035`](KEI-DES-035.md)) |
| "Universal Exactly-Once" | **Idempotent Produce + At-Least-Once Delivery** ([`KEI-DES-032`](KEI-DES-032.md)) |
| "10M Streams/Node Universal SLA" | **100K–1M+ Streams/Node Bounded Model** (~224 bytes/stream) ([`KEI-ARC-020`](KEI-ARC-020.md)) |
| "Universal Sub-2ms Latency" | **Class D Conditional Target** ($\le 2.0\text{ms}$ p99 under Profile P1 on NVMe) ([`KEI-ARC-011`](KEI-ARC-011.md)) |
| "CXL 3.0 / RDMA Zero-Broker Architecture" | **Excluded from v1 Scope** (ADR-082 in [`KEI-ARC-012`](KEI-ARC-012.md)) |
| "In-Broker Materialized Views / Active Dataflow" | **Excluded from v1 Scope** (ADR-083 in [`KEI-ARC-012`](KEI-ARC-012.md)) |
| "Fixed 24–48 Hour S3 Backlog" | **Capacity-Derived Bounded Backlog & Progressive Ladder** ([`KEI-ARC-027`](KEI-ARC-027.md)) |
| "Per-Stream Iceberg Tables by Default" | **Shared Tenant Tables (`tenant_{id}.events`)** (ADR-043 in [`KEI-DES-034`](KEI-DES-034.md)) |

---

## 6. Pre-Flight Implementation Checklist (Definition of Done)

Before opening a PR or marking any implementation task as complete, verify:

- [ ] **Targeted Spec Alignment**: Implemented types, schemas, and algorithms match the exact L3 specification.
- [ ] **RTM Traceability**: Task traces to at least one Requirement ID in [`KEI-VAL-051.md`](KEI-VAL-051.md).
- [ ] **Golden Invariant Checked**: No physical log mutation or unmanaged state leaks.
- [ ] **Hot-Path Memory Hygiene**: Zero dynamic heap allocations in hot write ingress loops; 64-byte alignment verified.
- [ ] **Fail-Secure Verified**: All edge cases fail closed with explicit domain errors.
- [ ] **Observability Instrumented**: Metrics, tracing spans, and audit hooks emitted per [`KEI-ARC-027.md`](KEI-ARC-027.md).
- [ ] **Validation Mapped**: Unit, benchmark, or chaos tests match [`KEI-OPS-041.md`](KEI-OPS-041.md).

---

## 7. Engineering Execution Suite Cross-Reference

For implementation scheduling, engineering workstreams, prototype plans, formal TLA+ specifications, and risk mitigation, consult [`docs/engineering/README.md`](../engineering/README.md):

- **Master Execution Plan**: [`KEI-ENG-100`](../engineering/KEI-ENG-100.md) (Roadmap M1.0–M1.10)
- **Vertical Prototype Spike**: [`KEI-SPIKE-001`](../engineering/KEI-SPIKE-001.md) (12-Week Execution Spike)
- **Formal State Validation**: [`KEI-FORMAL-001`](../engineering/KEI-FORMAL-001.md) (TLA+ Invariant Models)
- **Performance Harness**: [`KEI-BENCH-001`](../engineering/KEI-BENCH-001.md) (Workload Profiles P1–P6)
- **Delivery & Governance**: [`KEI-ORG-001`](../engineering/KEI-ORG-001.md) (Team Topology & ARB Charter)
- **Risk Management**: [`KEI-RISK-001`](../engineering/KEI-RISK-001.md) (5x5 Matrix & Authorized Pivots)