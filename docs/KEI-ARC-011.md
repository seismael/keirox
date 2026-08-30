# KEI-ARC-011 — Quality Attributes & Non-Functional Requirements

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-ARC-011 |
| Title | Quality Attributes & Non-Functional Requirements |
| Version | 1.0 |
| Level | **L1 — Conceptual Architecture** |
| Status | Approved for Engineering |
| Classification | Internal / Engineering Confidential |
| Owner | Chief Architect |
| Required Reviewers | Principal Engineer (Storage), Principal Engineer (Distributed Systems), SRE Lead, Security Lead, FinOps Lead |
| Depends On | KEI-ARC-001 (Vision), KEI-ARC-010 (Conceptual Architecture) |
| Feeds | KEI-ARC-020…027 (L2 subsystems), KEI-OPS-041 (Validation & Test Plan) |
| Keywords | MUST, MUST NOT, SHOULD, SHOULD NOT, MAY per RFC 2119 |

---

## 2. Purpose, Scope, and Audience

### 2.1 Purpose

This document converts the conceptual semantics defined in KEI-ARC-010 into **measurable, testable, and verifiable non-functional requirements (NFRs)**. Every target in this document is bound to:

- A **canonical workload profile** (§5), because a performance number without a workload definition is meaningless.
- A **verification method** (§6.5), because a target that cannot be tested is not a requirement.
- A **verification class** (§4.3), which distinguishes design-guaranteed invariants from workload-dependent targets.

### 2.2 Scope

**In scope:** performance, durability, availability, scalability, memory/resource, recoverability, security, compliance, operability, and portability requirements, plus the benchmark methodology and traceability matrix.

**Out of scope:** implementation algorithms (L3), feature/functional requirements (managed separately), and commercial pricing.

### 2.3 Normative Reading Rule

Any NFR in this document that is marked **Class D (workload-dependent)** MUST be quoted together with its workload profile and conditions. Quoting a Class D target without its profile is a documentation violation.

---

## 3. Relationship to Other Documents

| Document | Relationship |
|---|---|
| KEI-ARC-010 | Source of the semantic contracts this document quantifies. |
| KEI-ARC-012 | Records ADRs where an NFR target represents a binding trade-off decision. |
| KEI-ARC-020…027 | Each L2 subsystem MUST map its design to the NFRs it owns. |
| KEI-OPS-041 | The Validation & Test Plan implements the verification methods defined here. |

**Traceability rule:** Every NFR has exactly one owning L2 document and at least one verification method. Orphan NFRs are not permitted.

---

## 4. Quality Attribute Framework

### 4.1 Attribute Categories

| Category | Concern | Primary Stakeholders |
|---|---|---|
| Performance | Latency and throughput | Application teams, SRE |
| Durability | Data loss bounds | Platform architects, compliance |
| Availability | Uptime and failover | SRE, operations |
| Scalability | Cardinality and growth | Platform architects, FinOps |
| Resource | Memory, CPU, file handles | SRE, capacity planning |
| Recoverability | RPO/RTO and restore | SRE, DR owners |
| Security | Encryption, authN/authZ | Security, compliance |
| Compliance | Deletion, audit, retention | Legal, compliance |
| Operability | Observability, upgrades | SRE |
| Portability | Deployment targets | Platform engineering |

### 4.2 NFR Identifier Scheme

`<CATEGORY>-<3-digit number>` — e.g., `PERF-001`, `DUR-002`, `SEC-004`.

### 4.3 Verification Classes (Normative)

Every NFR carries exactly one verification class. This enforces Principle P4 (explicit semantics over magic) and P9 (evidence gates phases).

| Class | Meaning | Enforcement |
|---|---|---|
| **A** | Guaranteed by design invariant; structurally or mathematically enforced. | Code review + invariant test. |
| **B** | Validated by automated benchmark or chaos test; must pass release gate. | CI/release benchmark suite. |
| **C** | Validated by independent certification. | Jepsen, SOC2/ISO27001 audit. |
| **D** | Workload-dependent target; published with explicit conditions. | Conditional benchmark. |

---

## 5. Canonical Workload Profiles (Normative)

All Class B and Class D NFRs reference one or more of these profiles. A benchmark MUST declare which profile it executes.

| Profile | Name | Definition | Purpose |
|---|---|---|---|
| **P1** | Baseline Sustained | 100,000 msgs/s @ 1 KB (100 MB/s), 30-day retention, steady state. | Primary TCO and latency baseline. |
| **P2** | Burst | 10× P1 (1,000,000 msgs/s) for 5 minutes, then drain. | Backpressure and backlog behavior. |
| **P3** | High Cardinality | 1,000,000 active streams, low per-stream throughput (~100 msgs/s aggregate each). | Registry memory and file-handle scaling. |
| **P4** | Queue-Churn | High lease acquisition/ACK/NACK churn; 1M concurrent leases; out-of-order ACKs. | State-plane and timing-wheel stress. |
| **P5** | Analytics / Lakehouse | Heavy compaction + concurrent Iceberg query; SIMD filter pushdown. | ELT interference and query freshness. |
| **P6** | Degraded | S3 throttling (503) + compaction backpressure + one node down. | Failure-mode resilience. |

**Normative rule:** The headline write-latency and durability targets are defined under **Profile P1** unless stated otherwise.

---

## 6. Performance Requirements

### 6.1 Write Path Latency

| ID | Requirement | Target | Profile | Verify | Class |
|---|---|---|---|---|---|
| PERF-001 | Tier-0 durable write latency (producer ACK after quorum commit). | ≤ 2.0 ms p99; ≤ 5.0 ms p999 | P1 | Benchmark | D |
| PERF-002 | Tier-0 write latency under burst. | ≤ 5.0 ms p99 | P2 | Benchmark | D |
| PERF-003 | Background compaction interference on write path. | ≤ 5% p99 jitter vs. compaction-off baseline | P1, P5 | A/B benchmark | B |
| PERF-004 | Single-record framing overhead amortized in batch mode. | ≤ 8% payload overhead at 1 KB records | P1 | Benchmark | B |

**Conditions for PERF-001/002:** local NVMe (io_uring + O_DIRECT), 3-node same-rack/same-AZ quorum, no encryption or encryption with AES-NI, workload within admission quota. These are Class D — MUST be quoted with conditions.

### 6.2 Read / Consume Path Latency

| ID | Requirement | Target | Profile | Verify | Class |
|---|---|---|---|---|---|
| PERF-010 | Active-stream read (Tier-0, in-cache). | ≤ 2.0 ms p99 first-byte | P1 | Benchmark | D |
| PERF-011 | Lease acquisition for hot (Tier-0) task. | ≤ 1.0 ms p99 fast path | P4 | Benchmark | D |
| PERF-012 | Cold-task lease (S3-resident task metadata via sparse queue index). | ≤ 10 ms p99 pointer resolution; payload fetch bounded by S3 GET | P6 | Benchmark | D |
| PERF-013 | SIMD predicate-pushdown select-then-transfer vs. full transfer. | ≥ 3× lower bytes-on-wire for selective predicates | P5 | Benchmark | B |

### 6.3 Throughput

| ID | Requirement | Target | Profile | Verify | Class |
|---|---|---|---|---|---|
| PERF-020 | Sustained single-node ingress. | ≥ 100 MB/s (100K msgs/s @ 1 KB) | P1 | Benchmark | B |
| PERF-021 | Sustained single-NVMe sequential write (micro-kernel). | ≥ 1.2 GB/s; ≤ 1.2 ms p99 @ 200K ops/s | P1 | Micro-benchmark | B |
| PERF-022 | Cluster linear scaling. | Near-linear throughput to N storage nodes (≥ 0.85 efficiency) | P1 | Scale benchmark | B |

### 6.4 Lakehouse / ELT Freshness

| ID | Requirement | Target | Profile | Verify | Class |
|---|---|---|---|---|---|
| PERF-030 | Default Iceberg query freshness (event → queryable). | ≤ 60 s | P5 | Benchmark | D |
| PERF-031 | Fast-mode Iceberg query freshness (tuned, low-load). | ≤ 5 s | P5 | Benchmark | D |
| PERF-032 | Arrow Flight client CPU vs. equivalent JVM Kafka consumer. | ≤ 1/3 CPU for equivalent vectorized throughput | P5 | Benchmark | B |

**Normative note:** PERF-030/031 replace any prior “instant / 2-second” universal claim. Sub-2-second freshness is achievable only under low-load, tuned configurations and MUST NOT be advertised as a default.

---

## 7. Durability Requirements

| ID | Requirement | Target | Profile | Verify | Class |
|---|---|---|---|---|---|
| DUR-001 | Loss of quorum-committed records. | Zero (JML = 0) | All | Chaos test | A |
| DUR-002 | Producer ACK issued only after quorum commit. | Enforced by write path ordering | All | Invariant test | A |
| DUR-003 | `ACK_FAST` acknowledgment loss window on coordinator failover. | Bounded by `min(journal_batch_interval, max_unflushed_bytes)`; documented to client | P4 | Chaos test | D |
| DUR-004 | `ACK_DURABLE` acknowledgment loss after success. | Zero | P4 | Chaos test | A |
| DUR-005 | Idempotent-produce duplicate suppression within dedup window. | Zero duplicate appends | All | Benchmark + invariant | A |
| DUR-006 | Transactional append atomicity. | All-or-nothing visibility on commit/abort | All | Chaos test | A |
| DUR-007 | Record integrity detection. | CRC32C per batch + payload; corruption detected, not silently returned | All | Fault injection | A |

**Normative rule:** DUR-001 and DUR-002 are the durability backbone and derive from Invariant INV-3 in KEI-ARC-010. They are Class A (design-guaranteed) and MUST hold under all chaos scenarios in KEI-OPS-041.

---

## 8. Availability Requirements

| ID | Requirement | Target | Profile | Verify | Class |
|---|---|---|---|---|---|
| AVAIL-001 | Storage-node failure → service continuity. | No loss of committed writes; degraded-capacity operation | All | Chaos test | A |
| AVAIL-002 | Storage-node replacement / recovery time. | ≤ 5 s to resume from Tier-1 manifest + WAL delta | P1 | Chaos test | B |
| AVAIL-003 | Coordinator-shard failover time. | ≤ 3.5 s lease reassignment | P4 | Chaos test | B |
| AVAIL-004 | Split-brain lease safety. | No double-lease of the same offset to two live workers (epoch fencing) | P6 | Chaos test | A |
| AVAIL-005 | Rolling upgrade availability. | Zero downtime for produce/consume during N→N+1 upgrade | P1 | Upgrade test | B |
| AVAIL-006 | Single-AZ quorum availability target. | 99.95% monthly (same-AZ deployment) | P1 | Operational SLA | D |

**Normative rule:** AVAIL-004 is a safety property, not a liveness property. Under an unrecoverable partition the system SHOULD prefer unavailability of the affected shard over issuing conflicting leases.

---

## 9. Scalability & Cardinality Requirements

| ID | Requirement | Target | Profile | Verify | Class |
|---|---|---|---|---|---|
| SCALE-001 | Virtual streams per storage node. | 100,000 stable; 1,000,000 validated | P3 | Scale benchmark | B |
| SCALE-002 | Cluster-wide virtual streams. | 10,000,000 across ~10 nodes via sharding | P3 | Scale benchmark | B |
| SCALE-003 | File-handle footprint vs. stream count. | O(1) — fixed ring-buffer segment set regardless of stream count | P3 | Invariant + benchmark | A |
| SCALE-004 | Consumer groups per stream. | ≥ 100 without per-group physical duplication | P4 | Benchmark | B |
| SCALE-005 | Concurrent active leases cluster-wide. | ≥ 1,000,000 | P4 | Benchmark | B |
| SCALE-006 | State-shard horizontal scaling. | Coordinator load bounded per shard; rebalance without downtime | P4 | Scale test | B |

---

## 10. Memory & Resource Requirements

These NFRs enforce Principle P3 (bounded everything) and Invariant INV-5.

| ID | Requirement | Target | Profile | Verify | Class |
|---|---|---|---|---|---|
| MEM-001 | Stream registry footprint. | ≤ 224 bytes/stream nominal (model in KEI-ARC-010 §6) | P3 | Memory benchmark | B |
| MEM-002 | 1,000,000 streams registry total. | ≤ ~224 MB nominal, excluding consumer state | P3 | Memory benchmark | B |
| MEM-003 | Per-state-shard bitmap memory. | Bounded by `max_bitmap_memory`; spill to NVMe SSTable on exceed | P4 | Memory benchmark + fault | A |
| MEM-004 | Watermark advancement under stuck offsets. | Guaranteed via mandatory DLQ eviction; no unbounded window | P4 | Invariant test | A |
| MEM-005 | Node memory budget adherence. | No OOM under P4/P5 within published node memory budget | P4, P5 | Soak test | B |
| MEM-006 | Active lease map memory. | Bounded per shard; spill or throttle on exceed | P4 | Memory benchmark | A |

**Normative rule:** MEM-001/002 are nominal registry figures. Total node memory MUST be reported using the full budget formula in KEI-ARC-010 §9.2 (registry + manifests + consumer state + leases + dedup + arena + compaction + network + observability + headroom).

---

## 11. Recoverability Requirements (RPO/RTO)

| ID | Requirement | Target | Profile | Verify | Class |
|---|---|---|---|---|---|
| REC-001 | RPO, normal network (Mode A replication). | ≤ 5 s | All | DR test | D |
| REC-002 | RPO, degraded network. | ≤ 60 s | P6 | DR test | D |
| REC-003 | RTO, planned failover. | ≤ 1 min | All | DR test | B |
| REC-004 | RTO, unplanned failover. | ≤ 5 min | All | DR test | B |
| REC-005 | Point-in-time restore from backup. | Supported to a new cluster with checksum validation | All | Restore test | B |
| REC-006 | Backup scope completeness. | Manifests, Iceberg metadata, Raft snapshots, schema registry, quotas, optional WAL tails | All | Audit | A |
| REC-007 | Crypto-shredding vs. backups. | Destroyed key renders all backup ciphertext unrecoverable; audit proof emitted | All | Security test | A |

**Normative rule:** REC-001/002 apply to Multi-Region Mode A (single-writer primary + async replica). Same-stream active-active is out of scope for v1 (KEI-ARC-010 §10.2).

---

## 12. Security Requirements

| ID | Requirement | Target | Profile | Verify | Class |
|---|---|---|---|---|---|
| SEC-001 | Encryption in transit. | TLS 1.3; mTLS supported | All | Security audit | A |
| SEC-002 | Encryption at rest. | AES-256-GCM (AES-NI) with ChaCha20-Poly1305 fallback | All | Security audit | A |
| SEC-003 | Authentication. | SASL/SCRAM-SHA-512 and OAuth2/OIDC | All | Security audit | A |
| SEC-004 | Authorization. | ABAC scoped to tenant and stream namespaces | All | Security audit | A |
| SEC-005 | Tenant isolation. | No cross-tenant read/write/lease observable under adversarial test | All | Pen test | C |
| SEC-006 | Key management. | KMS envelope encryption; Root → Tenant KEK → Stream/Batch DEK | All | Security audit | A |
| SEC-007 | Secrets handling. | No secrets in logs, manifests, or error messages | All | Audit | A |
| SEC-008 | Audit logging. | Security-relevant events logged with tamper-evident store | All | Audit | A |

---

## 13. Compliance Requirements

| ID | Requirement | Target | Profile | Verify | Class |
|---|---|---|---|---|---|
| COMP-001 | Right-to-erasure (GDPR/CCPA). | Crypto-shredding renders data cryptographically unrecoverable on request | All | Security test | A |
| COMP-002 | Erasure latency. | Logical erasure immediate on key destruction; physical purge via lifecycle | All | Security test | A |
| COMP-003 | Retention enforcement. | Per-stream/tenant retention honored; expired data inaccessible | All | Lifecycle test | A |
| COMP-004 | Audit trail for deletion. | Deletion events recorded with operator, timestamp, key ID | All | Audit | A |
| COMP-005 | Compliance posture. | SOC2 Type II / ISO27001 readiness controls present | All | Independent audit | C |
| COMP-006 | Data residency. | Region-locked storage and key scopes supported | All | Config audit | A |

---

## 14. Operability & Observability Requirements

| ID | Requirement | Target | Profile | Verify | Class |
|---|---|---|---|---|---|
| OPS-001 | Metric coverage. | Watermark lag, lease age, ACK replication lag, bitmap spill, S3 backlog, compaction lag exposed | All | Audit | A |
| OPS-002 | Tracing. | OpenTelemetry distributed tracing across produce→commit→consume | All | Audit | A |
| OPS-003 | Quota enforcement. | Per-tenant ingress/stream/lease/bitmap quotas enforced with backpressure | P2 | Fault injection | A |
| OPS-004 | Backpressure behavior. | Progressive TCP window clamping before Tier-0 overflow; no corruption | P6 | Fault injection | A |
| OPS-005 | Rolling upgrade safety. | N/N-1 mixed-version supported; feature-flag gated | All | Upgrade test | B |
| OPS-006 | DLQ operability. | List, inspect, redrive, purge with audit | P4 | Functional test | A |
| OPS-007 | Capacity forecasting. | NVMe backlog ETA and S3 upload backlog surfaced as metrics | P6 | Audit | A |

---

## 15. Portability Requirements

| ID | Requirement | Target | Profile | Verify | Class |
|---|---|---|---|---|---|
| PORT-001 | Implementation language. | Rust for production v1 (Zig excluded by ADR) | All | Build audit | A |
| PORT-002 | OS / I/O. | Linux-first (io_uring + O_DIRECT) with epoll fallback | All | Portability test | A |
| PORT-003 | Object storage. | S3-compatible (AWS S3, GCS, Azure Blob) behind abstraction | All | Integration test | B |
| PORT-004 | KMS providers. | AWS KMS and HashiCorp Vault behind abstraction | All | Integration test | B |
| PORT-005 | Deployment. | Single static binary; container-friendly; no mandatory external daemons | All | Packaging audit | A |

---

## 16. Benchmark & Validation Methodology

### 16.1 Measurement Discipline

- All latency figures MUST report **p50, p99, and p999**, not averages.
- Benchmarks MUST run for a minimum soak duration (72 h for memory-leak claims; see KEI-OPS-041).
- Benchmarks MUST disclose hardware, kernel, NVMe model, network topology, encryption on/off, and compression ratio.
- Any benchmark run with compaction disabled MUST be labeled as such and MUST NOT be used for headline claims.

### 16.2 Verification Method Catalog

| Method | Used For | Tooling |
|---|---|---|
| Micro-benchmark | Raw I/O, framing overhead | Custom Rust harness, fio |
| A/B benchmark | Compaction interference | Same cluster, toggled compaction |
| Scale benchmark | Cardinality, throughput scaling | Load generators (P1–P5) |
| Chaos test | Failover, durability, split-brain | kill -9, partition injection, clock skew |
| Fault injection | S3 throttle, disk stall, KMS latency | Service mesh fault injection |
| Security audit / pen test | SEC/COMP requirements | Internal + external review |
| Independent certification | Jepsen, SOC2 | Third-party |

### 16.3 Release-Gate Rule

A release MUST NOT ship if any **Class A** invariant fails, and MUST NOT ship if any **Class B** benchmark regresses beyond its threshold without an approved ADR exception.

---

## 17. NFR Traceability Matrix (Summary)

| NFR Group | Owning L2 Document | Primary Verification |
|---|---|---|
| PERF (write/read/throughput) | KEI-ARC-020 (Storage) | Benchmark (P1/P2) |
| PERF (lakehouse freshness) | KEI-ARC-023 (ELT) | Benchmark (P5) |
| DUR | KEI-ARC-022 (Consensus) | Chaos test |
| AVAIL | KEI-ARC-022 (Consensus) | Chaos + upgrade test |
| SCALE | KEI-ARC-020 + KEI-ARC-021 | Scale benchmark (P3/P4) |
| MEM | KEI-ARC-021 (State Plane) | Memory benchmark (P3/P4) |
| REC | KEI-ARC-026 (Multi-Region/DR) | DR test |
| SEC / COMP | KEI-ARC-025 (Security) | Audit + pen test |
| OPS | KEI-ARC-027 (Operability) | Audit + fault injection |
| PORT | KEI-ARC-020 / KEI-ARC-024 | Portability + integration test |

Full per-NFR traceability is maintained in KEI-OPS-041.

---

## 18. Decisions Deferred to ADR Index

The following NFR-defining trade-offs are recorded in KEI-ARC-012:

- ADR: Write-latency target of ≤2 ms p99 (Class D) vs. stronger availability via cross-AZ quorum.
- ADR: `ACK_FAST` as default queue mode vs. `ACK_DURABLE` default.
- ADR: Default Iceberg freshness of ≤60 s (not ≤2 s) to bound S3 API and catalog cost.
- ADR: Same-AZ 99.95% availability target as the v1 baseline.
- ADR: Rust-only for v1 (PORT-001).

---

## 19. Glossary (Additions)

| Term | Definition |
|---|---|
| NFR | Non-functional requirement. |
| Verification Class | A/B/C/D classification of how an NFR is guaranteed or validated. |
| Workload Profile | A canonical, named load definition (P1–P6) against which NFRs are measured. |
| JML | Justified Maximum Loss; here, zero loss of quorum-committed records. |
| Soak Test | A long-duration continuous test used to detect leaks and drift. |
| Release Gate | The pass/fail barrier a build must clear before shipping. |

---

## 20. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial approved NFR baseline. Introduces workload profiles P1–P6, verification classes A–D, and full NFR set with traceability. Corrects prior overclaims: lakehouse freshness (≤60 s default), write latency as Class D conditional target, and availability as an operational SLA rather than a universal guarantee. |