# KEI-OPS-041 — Validation, Benchmark & Chaos Test Plan

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-OPS-041 |
| Title | Validation, Benchmark & Chaos Test Plan |
| Version | 1.0 |
| Level | **L3 — Validation & Certification Specification** |
| Subsystem Covered | Cross-Cutting Validation, Benchmarking, Chaos Engineering, Release Certification |
| Status | Approved for Engineering |
| Classification | Internal / Engineering Confidential |
| Owner | QA Lead / Reliability Engineering Lead |
| Required Reviewers | Chief Architect, Principal Engineer (Storage), Principal Engineer (Distributed Systems), SRE Lead, Security Lead |
| Depends On | KEI-ARC-011 (NFRs), KEI-ARC-012 (ADRs), KEI-ARC-020..027 (Subsystem Architectures), KEI-DES-030..036 (Detailed Design Specifications), KEI-OPS-040 (Operations Runbooks) |
| Consumed By | QA engineers, reliability engineers, release managers, performance engineers, chaos engineers, compliance auditors |
| Keywords | MUST, MUST NOT, SHOULD, SHOULD NOT, MAY per RFC 2119 |

---

## 2. Purpose, Scope, and Audience

### 2.1 Purpose

This document specifies the **complete validation, benchmarking, chaos testing, and release certification program** for the Polymorphic Event Fabric. It defines how every non-functional requirement, architectural invariant, and design contract is empirically proven before production release.

It operationalizes Principle P9:

> **Evidence gates phases.** No phase exits on narrative; exit requires benchmarks, chaos tests, and soak tests.

### 2.2 Scope

**In scope:**

- Canonical workload profiles.
- Performance benchmark methodology and acceptance thresholds.
- Durability and correctness tests.
- Failover and recovery tests.
- Chaos engineering scenarios.
- Jepsen-style consistency validation.
- Long-running soak tests.
- Compatibility conformance tests.
- Security and compliance validation.
- NFR verification gates.
- Release certification criteria.

**Out of scope:**

- Unit test strategy (owned by engineering teams).
- Integration test environment provisioning.
- Customer acceptance testing.
- Operational runbooks — owned by KEI-OPS-040.

### 2.3 Audience

- QA and reliability engineers.
- Performance engineers.
- Chaos engineers.
- Release managers.
- Security and compliance auditors.
- Principal engineers reviewing evidence.

---

## 3. Validation Design Principles

| ID | Principle | Required Behavior |
|---|---|---|
| VL-1 | **Every NFR has a test.** | No NFR may remain unverified. |
| VL-2 | **Every invariant has a checker.** | Architectural invariants MUST be continuously validated during tests. |
| VL-3 | **Evidence is reproducible.** | All tests MUST be scripted, versioned, and repeatable. |
| VL-4 | **Tests disclose conditions.** | Hardware, kernel, topology, encryption, and compression MUST be reported. |
| VL-5 | **Class D targets are conditional.** | Workload-dependent targets MUST be quoted with their profile. |
| VL-6 | **Failures are findings, not blockers.** | Test failures produce engineering actions, not suppressed results. |
| VL-7 | **Soak tests detect drift.** | Long-duration tests MUST detect leaks, fragmentation, and degradation. |
| VL-8 | **Certification requires evidence.** | No release ships without passing the certification gate. |

---

## 4. Canonical Workload Profiles

These profiles are defined in KEI-ARC-011 §5 and are referenced throughout this document.

| Profile | Name | Definition | Primary Purpose |
|---|---|---|---|
| **P1** | Baseline Sustained | 100,000 msgs/s @ 1 KB (100 MB/s), 30-day retention, steady state. | Primary latency and throughput baseline. |
| **P2** | Burst | 10× P1 (1,000,000 msgs/s) for 5 minutes, then drain. | Backpressure and backlog behavior. |
| **P3** | High Cardinality | 1,000,000 active streams, low per-stream throughput. | Registry memory and file-handle scaling. |
| **P4** | Queue-Churn | High lease acquisition/ACK/NACK churn; 1M concurrent leases; out-of-order ACKs. | State-plane and timing-wheel stress. |
| **P5** | Analytics / Lakehouse | Heavy compaction + concurrent Iceberg query; SIMD filter pushdown. | ELT interference and query freshness. |
| **P6** | Degraded | S3 throttling (503) + compaction backpressure + one node down. | Failure-mode resilience. |

### 4.1 Profile Disclosure Requirement

**Normative rule:** Every benchmark result MUST disclose:

- Profile identifier.
- Hardware model and NVMe type.
- Kernel version and io_uring configuration.
- Network topology and replication factor.
- Encryption on/off.
- Compression algorithm and level.
- Test duration.
- Client count and distribution.

---

## 5. Test Categories Overview

| Category | Purpose | Primary NFRs |
|---|---|---|
| Performance Benchmarks | Latency, throughput, overhead | PERF-* |
| Durability Tests | Zero loss of committed records | DUR-* |
| Delivery Semantics Tests | At-least-once, idempotence, ACK modes | DUR-003/004 |
| Ordering Tests | Per-stream and per-entity-key ordering | SCALE, PERF |
| State Plane Tests | Bitmaps, watermarks, leases, DLQ | MEM-*, SCALE-004/005 |
| Failover Tests | Node, coordinator, leader recovery | AVAIL-* |
| Multi-Region Tests | RPO/RTO, epoch fencing | REC-* |
| Chaos Tests | Partitions, stalls, skew, kills | AVAIL-004, DUR-001 |
| Jepsen-Style Tests | Consistency under adversarial conditions | DUR, AVAIL |
| Soak Tests | Leak and drift detection | MEM-*, PERF |
| Compatibility Tests | Gateway conformance | KEI-DES-035 |
| Security Tests | AuthN/AuthZ, encryption, erasure | SEC-*, COMP-* |
| Capacity Tests | Backpressure, shedding, quotas | OPS-* |

---

## 6. Performance Benchmark Suite

### 6.1 Write Path Latency

| Test ID | Requirement | Profile | Threshold | Class |
|---|---|---|---|---|
| PERF-T-001 | Tier-0 durable write latency | P1 | ≤2.0 ms p99; ≤5.0 ms p999 | D |
| PERF-T-002 | Tier-0 write latency under burst | P2 | ≤5.0 ms p99 | D |
| PERF-T-003 | Compaction interference on write path | P1, P5 | ≤5% p99 jitter vs. compaction-off | B |
| PERF-T-004 | Batch framing overhead at 1 KB records | P1 | ≤8% payload overhead | B |

**Methodology:**

- Sustained load for minimum 30 minutes.
- Report p50, p99, p999.
- Compare compaction-on vs. compaction-off for PERF-T-003.
- Encryption enabled and disabled runs reported separately.

### 6.2 Read / Consume Path Latency

| Test ID | Requirement | Profile | Threshold | Class |
|---|---|---|---|---|
| PERF-T-010 | Active-stream read first-byte latency | P1 | ≤2.0 ms p99 | D |
| PERF-T-011 | Lease acquisition for hot task | P4 | ≤1.0 ms p99 fast path | D |
| PERF-T-012 | Cold-task lease pointer resolution | P6 | ≤10 ms p99 | D |
| PERF-T-013 | SIMD pushdown bytes-on-wire reduction | P5 | ≥3× lower for selective predicates | B |

### 6.3 Throughput

| Test ID | Requirement | Profile | Threshold | Class |
|---|---|---|---|---|
| PERF-T-020 | Sustained single-node ingress | P1 | ≥100 MB/s | B |
| PERF-T-021 | Single-NVMe sequential write micro-benchmark | P1 | ≥1.2 GB/s; ≤1.2 ms p99 @ 200K ops/s | B |
| PERF-T-022 | Cluster linear scaling efficiency | P1 | ≥0.85 efficiency to N nodes | B |

### 6.4 Lakehouse / ELT Freshness

| Test ID | Requirement | Profile | Threshold | Class |
|---|---|---|---|---|
| PERF-T-030 | Default Iceberg query freshness | P5 | ≤60 s | D |
| PERF-T-031 | Fast-mode Iceberg query freshness | P5 | ≤5 s | D |
| PERF-T-032 | Arrow Flight client CPU vs. JVM Kafka consumer | P5 | ≤1/3 CPU | B |

---

## 7. Durability and Correctness Tests

### 7.1 Durability Tests

| Test ID | Requirement | Method | Threshold |
|---|---|---|---|
| DUR-T-001 | Zero loss of quorum-committed records (JML=0) | `kill -9` leader after ACK; verify all ACKed records present | Zero loss |
| DUR-T-002 | ACK issued only after quorum commit | Instrument write path; verify ordering | Invariant holds |
| DUR-T-003 | ACK_FAST bounded loss window | Kill coordinator during batch window | Loss ≤ batch interval |
| DUR-T-004 | ACK_DURABLE zero loss | Kill coordinator after success response | Zero loss |
| DUR-T-005 | Idempotent-produce deduplication | Replay same producer_seq within window | Zero duplicate appends |
| DUR-T-006 | Transactional append atomicity | Kill during commit/abort | All-or-nothing visibility |
| DUR-T-007 | Record integrity detection | Corrupt batch on disk | Corruption detected, not returned |

### 7.2 Delivery Semantics Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| SEM-T-001 | Worker crashes after processing, before ACK | Message redelivered. |
| SEM-T-002 | Duplicate ACK after already-ACKed offset | Success returned idempotently. |
| SEM-T-003 | Stale lease ACK after lease reassigned | Rejected with `STALE_LEASE`. |
| SEM-T-004 | ACK_FAST coordinator failover | Bounded redelivery; no data loss. |
| SEM-T-005 | ACK_DURABLE coordinator failover | No ACK loss after success. |
| SEM-T-006 | NACK with retry limit exceeded | Offset evicted to virtual DLQ. |
| SEM-T-007 | Redrive from DLQ | Offset requeued; audit event emitted. |

### 7.3 Ordering Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| ORD-T-001 | Sequential produce to same stream | Offsets strictly monotonic. |
| ORD-T-002 | Concurrent produce to independent entity keys | Per-key order preserved; cross-key concurrent. |
| ORD-T-003 | Kafka partition order via gateway | Per-partition order preserved. |
| ORD-T-004 | SQS FIFO MessageGroupId order | Per-group order preserved. |
| ORD-T-005 | Hot-key striping with sub-key | Per-sub-key order preserved. |

---

## 8. State Plane Tests

### 8.1 Bitmap and Watermark Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| STA-T-001 | Dense ACK run compression | RLE container used; memory bounded. |
| STA-T-002 | Sparse lease distribution | Array container used. |
| STA-T-003 | Mixed ACK/NACK region | Bitset container used. |
| STA-T-004 | Stuck offset blocks watermark | Mandatory DLQ eviction advances `W_base`. |
| STA-T-005 | Offsets below `W_base` purged | Memory reclaimed. |
| STA-T-006 | Bitmap exceeds spill threshold | Inactive containers spill to NVMe SSTable. |
| STA-T-007 | Spilled container accessed | Loaded transparently; correct state returned. |

### 8.2 Lease and Timing Wheel Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| LEA-T-001 | Lease grant and ACK within TTL | Offset terminal; timer invalidated. |
| LEA-T-002 | Lease timeout without ACK | Offset returns to READY; retry count incremented. |
| LEA-T-003 | Lease renewal extends TTL | Expiry advanced; old timer ignored. |
| LEA-T-004 | Retry limit exceeded | Offset evicted to DLQ. |
| LEA-T-005 | 1M concurrent leases | Timing wheel O(1) behavior; memory bounded. |
| LEA-T-006 | Lease token mismatch | Operation rejected. |
| LEA-T-007 | Coordinator epoch mismatch | Operation rejected with `STALE_EPOCH`. |

---

## 9. Failover and Recovery Tests

### 9.1 Storage Node Failover

| Test ID | Scenario | Target |
|---|---|---|
| FO-T-001 | Single storage node `kill -9` | Recovery <5s; zero committed data loss. |
| FO-T-002 | NVMe failure simulation | Node replaced; data reconstructed from Tier-1 + peer WAL delta. |
| FO-T-003 | Two-node failure in 3-node quorum | Writes pause safely; no corruption; recovery on restore. |

### 9.2 Coordinator Failover

| Test ID | Scenario | Target |
|---|---|---|
| FO-T-010 | Coordinator shard leader kill | Failover <3.5s; no double lease. |
| FO-T-011 | Coordinator failover during lease storm | No lease duplication; watermark consistent. |
| FO-T-012 | Coordinator failover with spilled bitmaps | State restored correctly. |

### 9.3 Multi-Region Failover

| Test ID | Scenario | Target |
|---|---|---|
| FO-T-020 | Planned region failover | RTO ≤1min; RPO ≤5s. |
| FO-T-021 | Unplanned region failover | RTO ≤5min; RPO ≤60s degraded. |
| FO-T-022 | Region epoch fencing | Old primary writes rejected. |
| FO-T-023 | Split-brain region partition | Orphaned writes quarantined; no corruption. |
| FO-T-024 | Failover with destroyed keys | Erased data not exposed. |

---

## 10. Chaos Engineering Scenarios

### 10.1 Chaos Test Matrix

| Chaos ID | Scenario | Injection | Expected Defense |
|---|---|---|---|
| CHAOS-001 | Network partition between storage nodes | Drop packets between nodes | Raft quorum safety; no JML violation. |
| CHAOS-002 | Network partition isolating coordinator | Isolate coordinator shard | Epoch fencing; no double lease. |
| CHAOS-003 | Asymmetric partition (partial visibility) | One-way packet loss | Majority side continues; minority fenced. |
| CHAOS-004 | Disk stall on leader | Inject I/O latency | Leader steps down or request times out safely. |
| CHAOS-005 | Clock skew injection | Offset node clocks by ±5s | Lease expiry safe; HLC causal order preserved. |
| CHAOS-006 | S3 throttling (503) | Inject S3 errors | Backpressure ladder engages; no data loss. |
| CHAOS-007 | KMS outage | Block KMS API | Cached DEKs used; new writes fail closed. |
| CHAOS-008 | Process kill during compaction | `kill -9` compactor | Deterministic WAL replay reconstructs state. |
| CHAOS-009 | Process kill during Iceberg commit | Kill committer mid-commit | Idempotent retry or orphan cleanup. |
| CHAOS-010 | Memory pressure on state plane | Limit cgroup memory | Bitmap spill engages; no OOM crash. |
| CHAOS-011 | CPU starvation on compaction cores | CPU quota limit | Hot path latency preserved. |
| CHAOS-012 | Node reboot during rolling upgrade | Reboot mid-upgrade | Cluster remains available; upgrade resumes. |
| CHAOS-013 | Object storage bucket permission loss | Revoke S3 permissions | Alerts; uploads queued; no corruption. |
| CHAOS-014 | Metadata Raft leader kill | Kill metadata leader | New leader elected; state continues. |
| CHAOS-015 | Simultaneous multi-failure | Combine two failures | System degrades safely; no invariant violation. |

### 10.2 Chaos Test Invariant Checks

During every chaos test, the following invariants MUST be continuously checked:

| Invariant | Check |
|---|---|
| No loss of quorum-committed records | Verify all ACKed records present after recovery. |
| No double lease | At most one active lease per offset. |
| No watermark regression | `W_base` never decreases. |
| No leased offset is ACKED or DLQ | State machine consistency. |
| No plaintext fallback | Encryption invariant holds. |
| No destroyed data accessible | Destroyed-key registry enforced. |
| No silent corruption | CRC validation catches corruption. |

---

## 11. Jepsen-Style Consistency Validation

### 11.1 Purpose

Jepsen-style tests validate consistency guarantees under adversarial conditions: network partitions, clock drift, process crashes, and disk failures.

### 11.2 Test Topology

- 5-node cluster.
- Nemesis injection: partitions, kills, clock skew, disk stalls.
- Concurrent clients performing produce, consume, lease, ACK operations.
- Linearizability checker for committed writes.
- State machine checker for lease/ACK transitions.

### 11.3 Consistency Models Validated

| Model | Scope | Validation |
|---|---|---|
| Linearizable committed writes | Local quorum | No ACKed write lost; no duplicate committed write. |
| At-least-once delivery | Queue consumption | Redelivery allowed; no silent loss. |
| Monotonic stream offsets | Per stream | Offsets never regress. |
| Causal cross-region order | Multi-region | HLC order preserved. |
| Epoch fencing | Coordinators and regions | Stale operations rejected. |

### 11.4 Jepsen Acceptance Criteria

| Criterion | Requirement |
|---|---|
| Committed write loss | Zero. |
| Duplicate committed writes from idempotent producer | Zero within dedup window. |
| Double lease observed | Zero. |
| State machine violation | Zero. |
| Unreported error | Zero. |
| Test harness crash | Zero. |

**Normative rule:** A Jepsen failure MUST block release certification until root-caused and fixed.

---

## 12. Soak Tests

### 12.1 Purpose

Soak tests detect memory leaks, fragmentation, file-handle leaks, bitmap growth, and performance drift over extended duration.

### 12.2 Soak Test Matrix

| Soak ID | Duration | Profile | Focus |
|---|---|---|---|
| SOAK-001 | 72 hours | P1 | Baseline stability, memory, latency drift. |
| SOAK-002 | 72 hours | P3 | High-cardinality stream registry stability. |
| SOAK-003 | 72 hours | P4 | Lease churn, bitmap fragmentation, timing wheel. |
| SOAK-004 | 72 hours | P5 | Compaction, Iceberg commits, small-file growth. |
| SOAK-005 | 168 hours (7 days) | P1 | Long-term drift, retention lifecycle. |

### 12.3 Soak Acceptance Criteria

| Criterion | Requirement |
|---|---|
| Memory growth | No unbounded growth after steady state. |
| File-handle count | Stable; no leak. |
| p99 latency drift | ≤10% degradation over test duration. |
| Watermark advancement | Continuous; no stuck offsets. |
| Bitmap memory | Bounded; spill operates correctly. |
| Error rate | No upward trend. |
| OOM or crash | Zero. |

---

## 13. Compatibility Conformance Tests

### 13.1 Purpose

Compatibility conformance validates that gateways implement the certified subsets defined in KEI-DES-035.

### 13.2 Gateway Conformance Matrix

| Gateway | Client Set | Required Coverage |
|---|---|---|
| Kafka Ingest | librdkafka, Kafka Java, Sarama, kafka-go, aiokafka | Produce, metadata, idempotence. |
| Kafka Stream Consumer | Kafka Java, librdkafka | Fetch, offsets, consumer groups. |
| SQS Standard | AWS SDK Java/Python/Go/JS | Send, receive, delete, visibility. |
| SQS FIFO | AWS SDK Java/Python | Group ordering, deduplication. |
| AMQP Direct | RabbitMQ Java, Pika, amqp091-go | Declare, publish, consume, ack/nack. |

### 13.3 Conformance Acceptance Criteria

| Criterion | Requirement |
|---|---|
| S1 operations | All pass. |
| S2 operations | Pass with documented limitations. |
| S0 operations | Return explicit unsupported errors. |
| Version negotiation | Certified versions discovered correctly. |
| Auth failures | Protocol-native errors returned. |
| Idempotence | Duplicate operations safe. |
| Long-running soak | 72-hour gateway stability. |

---

## 14. Security and Compliance Validation

### 14.1 Security Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| SEC-T-001 | Unauthenticated request | Rejected. |
| SEC-T-002 | Expired token | Rejected. |
| SEC-T-003 | Cross-tenant access attempt | Denied; audit logged. |
| SEC-T-004 | ABAC policy denial | Denied with reason. |
| SEC-T-005 | Plaintext fallback attempt | Prohibited; fail closed. |
| SEC-T-006 | DEK cache eviction | Key material zeroized. |
| SEC-T-007 | AAD mismatch on decryption | Decryption fails. |
| SEC-T-008 | Nonce reuse attempt | Prevented by random generation. |
| SEC-T-009 | Secrets in logs scan | No secrets detected. |
| SEC-T-010 | TLS version downgrade | Rejected. |

### 14.2 Crypto-Shredding Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| ERASE-T-001 | Stream erasure | Stream DEK destroyed; data unreadable. |
| ERASE-T-002 | Tenant erasure | Tenant KEK destroyed; all tenant data unreadable. |
| ERASE-T-003 | Erasure propagation to replica | All regions confirm destruction. |
| ERASE-T-004 | Backup restore after erasure | Destroyed data not restored. |
| ERASE-T-005 | Query after erasure | Access error returned. |
| ERASE-T-006 | Erasure under legal hold | Blocked; audit logged. |
| ERASE-T-007 | Erasure audit evidence | Complete proof generated. |

### 14.3 Compliance Readiness Validation

| Control Area | Validation Method |
|---|---|
| Access control | ABAC policy tests. |
| Encryption at rest | Storage inspection; no plaintext found. |
| Encryption in transit | TLS configuration scan. |
| Audit logging | Event completeness review. |
| Retention enforcement | Lifecycle tests. |
| Erasure proof | Crypto-shredding evidence review. |
| Secret management | No hardcoded secrets scan. |
| Incident response | Tabletop exercise. |

---

## 15. Capacity and Backpressure Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| CAP-T-001 | Tenant exceeds ingress quota | Throttled with retriable error. |
| CAP-T-002 | Arena >80% | Compaction priority raised. |
| CAP-T-003 | NVMe >80% | TCP clamping engaged. |
| CAP-T-004 | NVMe >90% | Protocol throttling engaged. |
| CAP-T-005 | NVMe >95% | Priority shedding engaged. |
| CAP-T-006 | NVMe >98% | Hard reject engaged; no corruption. |
| CAP-T-007 | S3 outage 24h simulation | Backlog bounded; backpressure prevents overflow. |
| CAP-T-008 | Bitmap memory quota exceeded | Spill engages; no OOM. |
| CAP-T-009 | Lease quota exceeded | Lease requests throttled. |

---

## 16. NFR Verification Gates

Each NFR from KEI-ARC-011 MUST be mapped to at least one test and pass before release.

### 16.1 Verification Gate Matrix

| NFR Group | Owning Subsystem | Primary Test Suite | Gate |
|---|---|---|---|
| PERF write/read | KEI-ARC-020 | Performance benchmarks | Pass |
| DUR | KEI-ARC-022 | Durability tests, chaos | Pass |
| AVAIL | KEI-ARC-022 | Failover tests, chaos | Pass |
| SCALE | KEI-ARC-020/021 | High-cardinality benchmarks | Pass |
| MEM | KEI-ARC-021 | State plane tests, soak | Pass |
| REC | KEI-ARC-026 | Multi-region tests, DR drills | Pass |
| SEC | KEI-ARC-025 | Security tests | Pass |
| COMP | KEI-ARC-025 | Crypto-shredding tests | Pass |
| OPS | KEI-ARC-027 | Capacity tests, observability checks | Pass |
| PORT | KEI-ARC-020 | Portability tests | Pass |
| COMPAT | KEI-ARC-024 | Gateway conformance | Pass |

**Normative rule:** No release candidate may advance if any verification gate is failing.

---

## 17. Release Certification Criteria

### 17.1 Phase Exit Evidence

Each engineering phase from KEI-ARC-012 ADR-081 MUST produce evidence:

| Phase | Required Evidence |
|---|---|
| Phase 1: Core Engine | P1/P3 benchmarks; 72h soak; state machine invariant checks. |
| Phase 2: Distributed Durability | JML=0 kill tests; failover <3.5s; WAF ≤1.35. |
| Phase 3: Ecosystem Bridge | Gateway conformance; Iceberg freshness; Arrow Flight CPU. |
| Phase 4: Enterprise Hardening | Jepsen pass; crypto-shredding verified; SOC2/ISO readiness evidence. |

### 17.2 Production Release Gate

A production release MUST satisfy all of the following:

1. All Class A invariants hold under chaos tests.
2. All Class B benchmarks pass without regression.
3. All Class D targets pass under declared profiles.
4. Jepsen-style consistency tests pass.
5. 72-hour soak tests pass.
6. Gateway conformance tests pass.
7. Security tests pass.
8. Crypto-shredding tests pass.
9. DR drills pass.
10. No unresolved SEV-1 or SEV-2 defects.
11. Observability dashboards validated.
12. Runbooks tested.
13. Release notes and compatibility matrices published.

### 17.3 Regression Policy

**Normative rule:** Any benchmark regression beyond threshold MUST either be fixed or explicitly approved via ADR exception with documented trade-off.

---

## 18. Test Environment Requirements

### 18.1 Reference Hardware Profile

For Class D headline claims, the reference environment MUST be disclosed. Recommended baseline:

| Component | Minimum Specification |
|---|---|
| Nodes | 3 storage nodes + 1 test client node |
| CPU | Modern x86_64 with AVX-512 or ARM with NEON |
| Memory | ≥64 GB per node |
| NVMe | Local NVMe, ≥2 TB per node |
| Network | ≥10 Gbps, same rack or same AZ |
| Object storage | S3-compatible bucket with versioning |
| KMS | Supported KMS provider |
| OS | Linux kernel with io_uring support |

### 18.2 Isolation Requirements

- Benchmark environment MUST be isolated from unrelated workloads.
- Chaos environment MUST be separate from certification benchmark environment.
- Security tests MUST run in an environment where secret scanning and audit review are possible.

---

## 19. Reporting and Evidence Artifacts

Every test execution MUST produce:

| Artifact | Content |
|---|---|
| Test manifest | Test IDs, versions, environment, profile. |
| Raw metrics | Latency histograms, throughput, error counts. |
| Invariant checker report | All invariant checks and results. |
| Failure report | Any failures, root cause, linked issue. |
| Evidence bundle | Logs, traces, dashboards, and command outputs. |
| Sign-off record | Responsible engineer and reviewer. |

**Normative rule:** Evidence bundles MUST be retained for compliance audit and release traceability.

---

## 20. NFR Traceability

| NFR | Requirement | Validated By |
|---|---|---|
| PERF-001/002 | Write latency | PERF-T-001/002 |
| PERF-003 | Compaction interference | PERF-T-003 |
| PERF-020/021/022 | Throughput | PERF-T-020/021/022 |
| PERF-030/031 | Lakehouse freshness | PERF-T-030/031 |
| DUR-001..007 | Durability | DUR-T-001..007, CHAOS suite |
| AVAIL-001..006 | Availability | FO-T suite, CHAOS suite |
| SCALE-001..006 | Scalability | P3 benchmarks, STA/LEA tests |
| MEM-001..006 | Memory | STA-T suite, SOAK suite |
| REC-001..007 | Recoverability | FO-T-020..024, ERASE-T suite |
| SEC-001..008 | Security | SEC-T suite |
| COMP-001..006 | Compliance | ERASE-T suite, compliance validation |
| OPS-001..007 | Operability | CAP-T suite, observability checks |

---

## 21. Open Questions

| Item | Status | Resolution Path |
|---|---|---|
| Jepsen test harness selection | Open | Evaluate internal vs. external Jepsen engagement. |
| Reference hardware certification | Open | Define certified cloud instance list. |
| Long-term 7-day soak automation | Open | Integrate with CI/staging pipeline. |
| Customer-facing benchmark disclosure policy | Open | Coordinate with product and legal. |
| Chaos automation cadence | Open | Define recurring chaos schedule. |

---

## 22. Glossary

| Term | Definition |
|---|---|
| Workload Profile | A canonical load definition used for benchmarking. |
| Verification Class | A/B/C/D classification of how an NFR is validated. |
| Soak Test | Long-duration test detecting leaks and drift. |
| Chaos Test | Adversarial test injecting failures to validate defenses. |
| Jepsen-Style Test | Consistency validation under partitions, crashes, and skew. |
| Release Gate | Mandatory pass criteria before shipping. |
| Evidence Bundle | Retained artifacts proving test execution and results. |

---

## 23. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial validation, benchmark, and chaos test plan. Defines workload profiles, performance/durability/state-plane/failover/chaos/Jepsen/soak/compatibility/security/capacity test suites, NFR verification gates, release certification criteria, environment requirements, and evidence artifacts. Implements Principle P9. |