# KEI-VAL-051 — End-To-End Requirements Traceability Matrix

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-VAL-051 |
| Title | End-To-End Requirements Traceability Matrix |
| Version | 1.0 |
| Level | **Closure & Certification** |
| Status | Approved for Final Release Readiness |
| Classification | Internal / Engineering Confidential |
| Owner | Chief Architect / QA Lead |
| Required Reviewers | Principal Engineers, Security Lead, SRE Lead, Release Manager, Compliance Lead |
| Depends On | KEI-ARC-001..027, KEI-DES-030..036, KEI-OPS-040..041, KEI-VAL-050 |
| Purpose | Provides exhaustive traceability from business/architectural requirements through design, operations, and validation evidence. |

---

## 2. Purpose

This document is the **single authoritative Requirements Traceability Matrix (RTM)** for the Keirox Polymorphic Event Fabric architecture suite.

It verifies that every major requirement is:

1. Derived from the approved vision and conceptual architecture.
2. Assigned to an owning subsystem.
3. Specified in an L2/L3 document.
4. Operationally supported by runbooks.
5. Validated by a defined test or certification gate.
6. Explicitly marked as covered, deferred, or excluded.

**Normative rule:** No requirement may enter engineering implementation unless it appears in this RTM with an owner, design reference, and verification path.

---

## 3. Traceability Method

### 3.1 Requirement ID Schema

| Prefix | Domain |
|---|---|
| `REQ-GI` | Golden Invariant & Core Model |
| `REQ-STOR` | Storage Engine & Tiering |
| `REQ-STATE` | Consumption State Plane |
| `REQ-SEM` | Delivery, Ordering & Semantics |
| `REQ-CONS` | Consensus & High Availability |
| `REQ-ELT` | Columnar ELT & Lakehouse |
| `REQ-GATE` | Protocol Gateways & SDKs |
| `REQ-SEC` | Security, Privacy & Compliance |
| `REQ-MR` | Multi-Region & Disaster Recovery |
| `REQ-OPS` | Operability, Observability & Capacity |
| `REQ-BUS` | Business / Adoption / TCO |

### 3.2 Status Definitions

| Status | Meaning |
|---|---|
| **Covered** | Fully specified, owned, and testable. |
| **Deferred** | Explicitly out of scope for v1; requires future ADR. |
| **Excluded** | Rejected by architecture decision. |
| **Conditional** | Covered under stated workload, profile, or policy constraints. |

---

# 4. Golden Invariant & Core Model Requirements

| Req ID | Requirement | Source | Owning Subsystem | Design Reference | Verification | Status |
|---|---|---|---|---|---|---|
| REQ-GI-001 | Data MUST be written exactly once to an immutable physical log. | KEI-ARC-010 | Storage Engine | KEI-ARC-020, KEI-DES-030 | DUR-T-001, CHAOS suite | Covered |
| REQ-GI-002 | Consumption semantics MUST be projections of the immutable log via mutable overlays. | KEI-ARC-010 | State Plane | KEI-ARC-021, KEI-DES-031 | STA-T suite | Covered |
| REQ-GI-003 | Consumption operations MUST NOT mutate the physical log. | KEI-ARC-010 | State Plane | KEI-DES-031 | Invariant checker | Covered |
| REQ-GI-004 | The same dataset MUST be consumable as stream, queue, virtual DLQ, and lakehouse table. | KEI-ARC-010 | All subsystems | KEI-ARC-020..024 | End-to-end integration test | Covered |
| REQ-GI-005 | Every event MUST have exactly one durable source of truth. | KEI-ARC-010 | Storage / Consensus | KEI-ARC-020, KEI-ARC-022 | DUR-T-001 | Covered |
| REQ-GI-006 | All mutable state MUST be bounded by quotas, spill, or shedding. | KEI-ARC-012 P3 | State Plane / Ops | KEI-DES-031, KEI-ARC-027 | CAP-T, SOAK tests | Covered |

---

# 5. Storage Engine & Tiering Requirements

| Req ID | Requirement | Source | Owning Subsystem | Design Reference | Verification | Status |
|---|---|---|---|---|---|---|
| REQ-STOR-001 | Support 100K–1M+ virtual streams per node. | KEI-ARC-020 | Storage Engine | KEI-ARC-020, KEI-DES-030 | P3 benchmark, SOAK-002 | Covered |
| REQ-STOR-002 | Multiplex logical streams into shared physical WAL with O(1) file handles. | KEI-ARC-020 | Storage Engine | KEI-DES-030 | SCALE tests | Covered |
| REQ-STOR-003 | Use batch-oriented WAL framing with CRC32C integrity. | KEI-DES-030 | Storage Engine | KEI-DES-030 | DUR-T-007 | Covered |
| REQ-STOR-004 | Tier-0 writes MUST be durable after synchronous quorum commit. | KEI-ARC-020 | Storage / Consensus | KEI-ARC-022 | DUR-T-001/002 | Covered |
| REQ-STOR-005 | Tier-0 write latency target ≤2ms p99 under Profile P1. | KEI-ARC-011 | Storage Engine | KEI-ARC-020 | PERF-T-001 | Conditional |
| REQ-STOR-006 | Tier-1 offload MUST be asynchronous and MUST NOT gate producer ACK. | KEI-ARC-020 | Storage / ELT | KEI-ARC-020, KEI-ARC-023 | PERF tests | Covered |
| REQ-STOR-007 | Node replacement recovery target <5 seconds. | KEI-ARC-020 | Storage / Consensus | KEI-ARC-022, KEI-OPS-040 | FO-T-001 | Covered |
| REQ-STOR-008 | Single-pass compaction MUST maintain WAF ≤1.35. | KEI-ARC-020 | Storage / ELT | KEI-ARC-023 | PERF-T-020, WAF audit | Covered |
| REQ-STOR-009 | Sparse index historical reads SHOULD average ≤1.05 seeks. | KEI-ARC-020 | Storage Engine | KEI-DES-030 | PERF-T-012 | Conditional |
| REQ-STOR-010 | Backlog duration MUST be capacity-derived, not fixed 24–48h. | Audit resolution | Storage / Ops | KEI-ARC-027, KEI-OPS-040 | CAP-T-007 | Covered |
| REQ-STOR-011 | Backpressure ladder MUST protect NVMe from corruption. | KEI-ARC-027 | Ops / Storage | KEI-OPS-040 | CAP-T-003..006 | Covered |

---

# 6. Consumption State Plane Requirements

| Req ID | Requirement | Source | Owning Subsystem | Design Reference | Verification | Status |
|---|---|---|---|---|---|---|
| REQ-STATE-001 | State machine MUST support READY, LEASED, ACKED, EVICTED_DLQ. | KEI-ARC-021 | State Plane | KEI-DES-031 | STA-T suite | Covered |
| REQ-STATE-002 | Consumer state MUST use Roaring Bitmap overlays. | KEI-ARC-021 | State Plane | KEI-DES-031 | STA-T-001..003 | Covered |
| REQ-STATE-003 | `W_base` MUST purge terminal offsets below watermark. | KEI-ARC-021 | State Plane | KEI-DES-031 | STA-T-005 | Covered |
| REQ-STATE-004 | Mandatory DLQ eviction MUST prevent stuck watermark. | KEI-ARC-021 | State Plane | KEI-DES-031 | STA-T-004, OPS-RB-019 | Covered |
| REQ-STATE-005 | Leases MUST expire via O(1) hierarchical timing wheel. | KEI-ARC-021 | State Plane | KEI-DES-031 | LEA-T suite | Covered |
| REQ-STATE-006 | Out-of-order ACKs MUST be supported without head-of-line blocking. | KEI-ARC-021 | State Plane | KEI-DES-031 | SEM-T suite | Covered |
| REQ-STATE-007 | Virtual DLQ MUST be zero-copy and index-based. | KEI-ARC-021 | State Plane | KEI-DES-031 | STA-T, ERASE tests | Covered |
| REQ-STATE-008 | ACK_FAST and ACK_DURABLE modes MUST be explicit. | KEI-ARC-021 | State Plane | KEI-DES-031 | DUR-T-003/004 | Covered |
| REQ-STATE-009 | Consumer state MUST shard by tenant/stream/group/bucket. | KEI-ARC-021 | State Plane / Consensus | KEI-DES-031 | SCALE tests | Covered |
| REQ-STATE-010 | Bitmap memory MUST be bounded and spillable. | KEI-ARC-021 | State Plane | KEI-DES-031 | STA-T-006/007 | Covered |
| REQ-STATE-011 | Coordinator epochs MUST fence stale lease operations. | KEI-ARC-021 | State Plane / Consensus | KEI-DES-031 | LEA-T-007, CHAOS-002 | Covered |
| REQ-STATE-012 | State snapshots and lease journals MUST support failover reconstruction. | KEI-ARC-021 | State Plane / Consensus | KEI-DES-031 | FO-T-010..012 | Covered |

---

# 7. Delivery, Ordering & Semantics Requirements

| Req ID | Requirement | Source | Owning Subsystem | Design Reference | Verification | Status |
|---|---|---|---|---|---|---|
| REQ-SEM-001 | Default queue delivery MUST be at-least-once. | KEI-ARC-010 | State Plane | KEI-DES-031 | SEM-T suite | Covered |
| REQ-SEM-002 | Producer idempotence MUST prevent duplicate appends inside dedup window. | KEI-DES-031 | Storage / State | KEI-DES-030/031 | DUR-T-005 | Covered |
| REQ-SEM-003 | Optional transactional append MUST be atomic. | KEI-DES-031 | Storage / Consensus | KEI-DES-030/031 | DUR-T-006 | Covered |
| REQ-SEM-004 | Ordering MUST be guaranteed per stream or entity key. | KEI-ARC-010 | State Plane / Gateways | KEI-DES-031/032 | ORD-T suite | Covered |
| REQ-SEM-005 | Independent entity keys MAY be processed concurrently. | KEI-ARC-010 | State Plane | KEI-DES-031 | ORD-T-002 | Covered |
| REQ-SEM-006 | Strict single-key ordering MUST NOT be parallelized without sub-key or relaxed mode. | Audit resolution | State Plane | KEI-DES-031 | ORD-T-005 | Covered |
| REQ-SEM-007 | Exactly-once external side effects MUST require idempotent consumers. | KEI-ARC-012 | SDK / Application | KEI-DES-032 | Documentation + SEM-T | Covered |
| REQ-SEM-008 | Timed-out leases SHOULD be prioritized for retry. | KEI-ARC-021 | State Plane | KEI-DES-031 | LEA-T-002 | Covered |
| REQ-SEM-009 | Duplicate ACKs MUST be idempotent. | KEI-DES-031 | State Plane | KEI-DES-031 | SEM-T-002 | Covered |
| REQ-SEM-010 | Stale lease operations MUST be rejected. | KEI-DES-031 | State Plane | KEI-DES-031 | SEM-T-003 | Covered |

---

# 8. Consensus & High Availability Requirements

| Req ID | Requirement | Source | Owning Subsystem | Design Reference | Verification | Status |
|---|---|---|---|---|---|---|
| REQ-CONS-001 | Data plane MUST use synchronous local quorum replication. | KEI-ARC-022 | Consensus | KEI-ARC-022 | DUR-T-001 | Covered |
| REQ-CONS-002 | Metadata/state plane MUST replicate coordinator assignments, manifests, journals, snapshots, and committed watermarks. | KEI-ARC-022 | Consensus | KEI-ARC-022 | FO-T suite | Covered |
| REQ-CONS-003 | JML for quorum-committed records MUST be zero. | KEI-ARC-011 | Consensus | KEI-ARC-022 | DUR-T-001, Jepsen | Covered |
| REQ-CONS-004 | Producer ACK MUST be issued only after quorum commit. | KEI-ARC-022 | Consensus / Storage | KEI-DES-030 | DUR-T-002 | Covered |
| REQ-CONS-005 | Coordinator shard failover target <3.5 seconds. | KEI-ARC-022 | State / Consensus | KEI-DES-031, KEI-OPS-040 | FO-T-010 | Covered |
| REQ-CONS-006 | Split-brain MUST prefer unavailability over conflicting leases. | KEI-ARC-022 | Consensus / State | KEI-DES-031 | CHAOS-002/003 | Covered |
| REQ-CONS-007 | Membership changes MUST NOT cause data loss. | KEI-ARC-022 | Consensus | KEI-OPS-040 | FO-T suite | Covered |
| REQ-CONS-008 | Two consensus planes MUST be isolated to protect hot-path latency. | KEI-ARC-022 | Consensus | KEI-ARC-022 | PERF tests | Covered |

---

# 9. Columnar ELT & Lakehouse Requirements

| Req ID | Requirement | Source | Owning Subsystem | Design Reference | Verification | Status |
|---|---|---|---|---|---|---|
| REQ-ELT-001 | ELT MUST be internalized, not marketed as zero-ETL. | ADR-040 | ELT | KEI-ARC-023 | Documentation audit | Covered |
| REQ-ELT-002 | ELT MUST be asynchronous and MUST NOT block producer ACK. | KEI-ARC-023 | ELT / Storage | KEI-ARC-023 | PERF tests | Covered |
| REQ-ELT-003 | Schema registry MUST support versioned schemas and fingerprints. | KEI-DES-033 | Schema Registry | KEI-DES-033 | Schema tests | Covered |
| REQ-ELT-004 | Adaptive shredding MUST cap primitive columns at 64 by default. | ADR-042 | ELT | KEI-DES-033 | STA schema tests | Covered |
| REQ-ELT-005 | Unshredded/polymorphic fields MUST route to `_unstructured_payload`. | ADR-042 | ELT | KEI-DES-033 | Schema tests | Covered |
| REQ-ELT-006 | Arrow batches MUST support SIMD predicate pushdown where applicable. | KEI-ARC-023 | ELT / SDK | KEI-DES-032/033 | PERF-T-013 | Covered |
| REQ-ELT-007 | Parquet target file size MUST be 64–128 MB. | ADR-045 | ELT / Iceberg | KEI-DES-034 | Lakehouse tests | Covered |
| REQ-ELT-008 | Default Iceberg table model MUST be shared tenant table. | ADR-043 | Iceberg Committer | KEI-DES-034 | Lakehouse tests | Covered |
| REQ-ELT-009 | Default lakehouse freshness target ≤60s; fast mode ≤5s. | ADR-044 | ELT / Iceberg | KEI-DES-034 | PERF-T-030/031 | Conditional |
| REQ-ELT-010 | Iceberg commits MUST be idempotent and ledger-backed. | KEI-DES-034 | Iceberg Committer | KEI-DES-034 | CHAOS-009 | Covered |
| REQ-ELT-011 | Snapshot expiration, manifest compaction, and orphan cleanup MUST be governed. | KEI-DES-034 | Iceberg Committer / Ops | KEI-DES-034, KEI-OPS-040 | OPS-RB-020 | Covered |
| REQ-ELT-012 | Schema evolution MUST preserve historical readability. | KEI-DES-033 | Schema / Iceberg | KEI-DES-033/034 | Schema evolution tests | Covered |

---

# 10. Protocol Gateways & SDK Requirements

| Req ID | Requirement | Source | Owning Subsystem | Design Reference | Verification | Status |
|---|---|---|---|---|---|---|
| REQ-GATE-001 | Kafka gateway MUST support a published compatibility subset, not full parity. | ADR-070 | Protocol Plane | KEI-DES-035 | Compatibility suite | Covered |
| REQ-GATE-002 | Native API MUST expose Arrow Flight/gRPC streaming and queue operations. | ADR-071 | Protocol Plane | KEI-DES-032 | Native SDK tests | Covered |
| REQ-GATE-003 | SQS gateway MUST translate core queue operations to PEF lease/ACK. | KEI-ARC-024 | Protocol Plane | KEI-DES-035 | SQS conformance | Covered |
| REQ-GATE-004 | AMQP gateway MUST support direct/default exchange subset only. | KEI-ARC-024 | Protocol Plane | KEI-DES-035 | AMQP conformance | Covered |
| REQ-GATE-005 | Unsupported operations MUST return explicit protocol-native errors. | KEI-DES-035 | Protocol Plane | KEI-DES-035 | Negative tests | Covered |
| REQ-GATE-006 | Gateway identities MUST map to PEF ABAC principals. | KEI-ARC-025 | Security / Protocol | KEI-DES-035 | SEC-T suite | Covered |
| REQ-GATE-007 | Protocol throttling MUST be retriable and observable. | KEI-ARC-027 | Protocol / Ops | KEI-DES-035, KEI-OPS-041 | CAP-T suite | Covered |
| REQ-GATE-008 | Kafka virtual partitions MUST preserve per-partition ordering. | KEI-DES-035 | Protocol Plane | KEI-DES-035 | ORD-T-003 | Covered |
| REQ-GATE-009 | SQS FIFO `MessageGroupId` MUST map to entity-key ordering. | KEI-DES-035 | Protocol Plane | KEI-DES-035 | ORD-T-004 | Covered |
| REQ-GATE-010 | Kafka transactions MUST be excluded from v1 compatibility. | KEI-DES-035 | Protocol Plane | KEI-DES-035 | Negative tests | Covered |
| REQ-GATE-011 | AMQP complex exchange topologies MUST be excluded from v1. | KEI-DES-035 | Protocol Plane | KEI-DES-035 | Negative tests | Covered |
| REQ-GATE-012 | Delayed messages/timers MUST be excluded from v1 unless future ADR approved. | KEI-DES-035 | Protocol Plane | KEI-DES-035 | Open question | Deferred |

---

# 11. Security, Privacy & Compliance Requirements

| Req ID | Requirement | Source | Owning Subsystem | Design Reference | Verification | Status |
|---|---|---|---|---|---|---|
| REQ-SEC-001 | All external and internal traffic MUST use TLS 1.3/mTLS. | KEI-ARC-025 | Security | KEI-ARC-025 | SEC-T-010 | Covered |
| REQ-SEC-002 | Customer data MUST be encrypted at rest. | KEI-ARC-025 | Security / Storage | KEI-DES-036 | SEC-T-005 | Covered |
| REQ-SEC-003 | Authorization MUST use default-deny ABAC. | KEI-ARC-025 | Security | KEI-ARC-025 | SEC-T-004 | Covered |
| REQ-SEC-004 | Tenant isolation MUST be enforced by namespace, policy, and key hierarchy. | KEI-ARC-025 | Security | KEI-ARC-025/036 | SEC-T-003 | Covered |
| REQ-SEC-005 | Secrets MUST NOT appear in logs, errors, manifests, or snapshots. | KEI-ARC-025 | Security | KEI-DES-036 | SEC-T-009 | Covered |
| REQ-SEC-006 | Audit logs MUST be tamper-evident. | KEI-ARC-025 | Security | KEI-ARC-025 | Audit validation | Covered |
| REQ-SEC-007 | Envelope encryption MUST use Root → Tenant KEK → Stream/Batch DEK. | ADR-050 | Security | KEI-DES-036 | SEC-T suite | Covered |
| REQ-SEC-008 | DEKs MUST be cached securely and zeroized on eviction. | KEI-DES-036 | Security | KEI-DES-036 | SEC-T-006 | Covered |
| REQ-SEC-009 | Crypto-shredding MUST render target data cryptographically unrecoverable. | ADR-051 | Security | KEI-DES-036 | ERASE-T suite | Covered |
| REQ-SEC-010 | Destroyed keys MUST be recorded in a replicated destroyed-key registry. | KEI-DES-036 | Security / DR | KEI-DES-036 | ERASE-T-003 | Covered |
| REQ-SEC-011 | Backup restore MUST NOT resurrect destroyed data. | KEI-DES-036 | Security / DR | KEI-DES-036, KEI-OPS-040 | ERASE-T-004 | Covered |
| REQ-SEC-012 | Legal hold MUST suspend destructive lifecycle operations. | KEI-DES-034/036 | Security / Ops | KEI-OPS-040 | ERASE-T-006 | Covered |
| REQ-SEC-013 | KMS unavailability MUST fail secure, never plaintext. | KEI-DES-036 | Security | KEI-DES-036 | CHAOS-007 | Covered |
| REQ-SEC-014 | Key compromise MUST trigger incident response and key rotation/destruction. | KEI-OPS-040 | Security / Ops | KEI-OPS-040 | OPS-RB-013 | Covered |

---

# 12. Multi-Region & Disaster Recovery Requirements

| Req ID | Requirement | Source | Owning Subsystem | Design Reference | Verification | Status |
|---|---|---|---|---|---|---|
| REQ-MR-001 | v1 same-stream replication MUST use Mode A single-writer primary. | ADR-060 | Multi-Region | KEI-ARC-026 | FO-T-020..024 | Covered |
| REQ-MR-002 | Cross-region ordering MUST use HLC causal tags. | KEI-ARC-026 | Multi-Region | KEI-ARC-026 | Jepsen-style tests | Covered |
| REQ-MR-003 | Multi-writer same-stream active-active MUST be excluded from v1. | ADR-060 | Multi-Region | KEI-ARC-026 | Negative tests | Covered |
| REQ-MR-004 | RPO target ≤5s normal, ≤60s degraded. | KEI-ARC-011 | Multi-Region | KEI-ARC-026 | FO-T-020/021 | Conditional |
| REQ-MR-005 | RTO target ≤1min planned, ≤5min unplanned. | KEI-ARC-011 | Multi-Region | KEI-ARC-026 | FO-T-020/021 | Conditional |
| REQ-MR-006 | Region failover MUST use `region_epoch` fencing. | KEI-ARC-026 | Multi-Region | KEI-ARC-026 | FO-T-022 | Covered |
| REQ-MR-007 | Split-brain orphaned writes MUST be quarantined. | KEI-ARC-026 | Multi-Region | KEI-ARC-026 | FO-T-023 | Covered |
| REQ-MR-008 | Backup scope MUST include manifests, snapshots, schema registry, WAL tails, and destroyed-key registry. | KEI-ARC-026 | DR | KEI-OPS-040 | OPS-RB-009 | Covered |
| REQ-MR-009 | PITR MUST reconstruct state to target timestamp without post-target leakage. | KEI-ARC-026 | DR | KEI-OPS-040 | OPS-RB-011 | Covered |
| REQ-MR-010 | Data residency MUST block unauthorized cross-region replication. | KEI-ARC-025/026 | Security / DR | KEI-ARC-026 | Residency tests | Covered |

---

# 13. Operability, Observability & Capacity Requirements

| Req ID | Requirement | Source | Owning Subsystem | Design Reference | Verification | Status |
|---|---|---|---|---|---|---|
| REQ-OPS-001 | All bounded resources MUST expose metrics. | KEI-ARC-027 | Operability | KEI-ARC-027 | Observability audit | Covered |
| REQ-OPS-002 | Distributed tracing MUST propagate across subsystem boundaries. | KEI-ARC-027 | Operability | KEI-ARC-027 | Trace tests | Covered |
| REQ-OPS-003 | Tenant quotas MUST be enforced before resource allocation. | KEI-ARC-027 | Protocol / Ops | KEI-ARC-027 | CAP-T-001 | Covered |
| REQ-OPS-004 | Backpressure MUST progress through alert → clamp → throttle → shed → reject. | KEI-ARC-027 | Ops / Storage | KEI-OPS-040 | CAP-T suite | Covered |
| REQ-OPS-005 | Rolling upgrades MUST support N/N-1 mixed-version operation. | KEI-ARC-027 | Lifecycle | KEI-OPS-040 | OPS-RB-003 | Covered |
| REQ-OPS-006 | Feature flags MUST support scoped rollout and kill-switch. | KEI-ARC-027 | Lifecycle | KEI-OPS-040 | OPS-RB-004 | Covered |
| REQ-OPS-007 | Capacity forecasting MUST advise scaling before exhaustion. | KEI-ARC-027 | Ops / FinOps | KEI-ARC-027 | CAP tests | Covered |
| REQ-OPS-008 | Emergency shedding MUST preserve critical streams and committed data. | KEI-ARC-027 | Ops | KEI-OPS-040 | OPS-RB-017 | Covered |
| REQ-OPS-009 | Runbooks MUST include abort criteria and verification checks. | KEI-OPS-040 | Ops | KEI-OPS-040 | DR drills | Covered |
| REQ-OPS-010 | Destructive operations MUST require authorization and audit. | KEI-OPS-040 | Ops / Security | KEI-OPS-040 | Two-person rule tests | Covered |

---

# 14. Business, Adoption & TCO Requirements

| Req ID | Requirement | Source | Owning Subsystem | Design Reference | Verification | Status |
|---|---|---|---|---|---|---|
| REQ-BUS-001 | Architecture MUST reduce fragmented streaming/queue/ELT infrastructure. | KEI-ARC-001 | Product / Architecture | KEI-ARC-001 | Solution review | Covered |
| REQ-BUS-002 | Migration MUST be possible through Kafka gateway subset. | KEI-ARC-024 | Protocol Plane | KEI-DES-035 | Compatibility tests | Covered |
| REQ-BUS-003 | Native SDK MUST provide higher-performance path than compatibility gateways. | KEI-ARC-024 | SDK | KEI-DES-032 | PERF-T-032 | Covered |
| REQ-BUS-004 | TCO claims MUST be scenario-dependent, not universal. | Audit resolution | Product / FinOps | KEI-ARC-001, KEI-ARC-011 | TCO model review | Covered |
| REQ-BUS-005 | High-cardinality multi-tenant streams MUST be a primary design target. | KEI-ARC-001 | Storage / State | KEI-ARC-020/021 | P3 benchmark | Covered |
| REQ-BUS-006 | Lakehouse integration MUST be native but not block operational messaging. | KEI-ARC-023 | ELT | KEI-ARC-023 | PERF tests | Covered |
| REQ-BUS-007 | Enterprise compliance MUST support GDPR/CCPA erasure. | KEI-ARC-025 | Security | KEI-DES-036 | ERASE-T suite | Covered |
| REQ-BUS-008 | Non-goals MUST be explicit to prevent scope creep. | KEI-ARC-001 | Architecture | KEI-ARC-012 | KEI-VAL-050 audit | Covered |

---

# 15. Explicit Deferrals and Exclusions

To ensure no ambiguity, the following items are explicitly **not** v1 requirements. They are controlled by ADR governance and may be revisited later.

| ID | Item | Status | Reason |
|---|---|---|---|
| DEF-001 | CXL 3.0 / RDMA zero-broker data plane | Excluded | Non-portable, weak multi-tenant isolation, not productizable in v1. |
| DEF-002 | In-broker materialized views / active dataflow | Excluded | Scope explosion; broker-database anti-pattern. |
| DEF-003 | Universal exactly-once side effects | Excluded | Requires consumer-side idempotence/transactional sink. |
| DEF-004 | Kafka transactions full parity | Deferred | High compatibility complexity; v1 supports idempotent non-transactional produce. |
| DEF-005 | AMQP publisher confirms | Deferred | Requires additional delivery-receipt semantics. |
| DEF-006 | Delayed messages / scheduled delivery | Deferred | Requires state-plane timer extension. |
| DEF-007 | Multi-writer same-stream active-active | Excluded from v1 | Requires global conflict resolution beyond HLC. |
| DEF-008 | Nested/recursive schema shredding by default | Deferred | Polymorphism and metadata explosion risk. |
| DEF-009 | Per-stream Iceberg tables by default | Excluded | Catalog explosion risk at high cardinality. |
| DEF-010 | 100% protocol parity with Kafka/SQS/AMQP | Excluded | Compatibility-by-subset is the governed model. |

---

# 16. Coverage Summary

| Domain | Requirements | Covered | Conditional | Deferred/Excluded | Gaps |
|---|---:|---:|---:|---:|---:|
| Golden Invariant | 6 | 6 | 0 | 0 | 0 |
| Storage Engine | 11 | 9 | 2 | 0 | 0 |
| State Plane | 12 | 12 | 0 | 0 | 0 |
| Semantics | 10 | 10 | 0 | 0 | 0 |
| Consensus | 8 | 8 | 0 | 0 | 0 |
| ELT / Lakehouse | 12 | 11 | 1 | 0 | 0 |
| Gateways / SDK | 12 | 10 | 0 | 2 | 0 |
| Security / Compliance | 14 | 14 | 0 | 0 | 0 |
| Multi-Region / DR | 10 | 8 | 2 | 0 | 0 |
| Operability | 10 | 10 | 0 | 0 | 0 |
| Business / Adoption | 8 | 8 | 0 | 0 | 0 |
| **Total** | **113** | **106** | **5** | **2** | **0** |

**Normative interpretation:**  
- “Conditional” requirements are fully specified but depend on workload profile, deployment topology, or policy mode.  
- “Deferred/Excluded” items are deliberately out of scope and governed by ADRs.  
- There are **zero unowned or untested requirement gaps**.

---

# 17. RTM Change Control

Any future change MUST follow this process:

1. Create or update requirement ID.
2. Update owning subsystem.
3. Update L2/L3 design reference.
4. Update operational runbook if production behavior changes.
5. Update validation test in KEI-OPS-041.
6. Record ADR if the change is architectural.
7. Re-run KEI-VAL-050 consistency audit if the change affects invariants.

**Normative rule:** No engineering change is considered approved unless this RTM is updated.

---

# 18. Final Traceability Verdict

The Keirox Polymorphic Event Fabric architecture suite now provides:

- Complete requirement-to-design traceability.
- Complete design-to-operation traceability.
- Complete operation-to-validation traceability.
- Explicit deferrals and exclusions.
- No unowned requirements.
- No orphaned tests.
- No unresolved contradictions within the approved architecture baseline.

**Verdict:** The RTM is complete and suitable for final release readiness certification.

---

## 19. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial end-to-end requirements traceability matrix. Maps 113 requirements across core model, storage, state plane, semantics, consensus, ELT, gateways, security, multi-region, operability, and business domains. Confirms zero unresolved requirement gaps. |