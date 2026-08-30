# KEI-SPIKE-201 — Distributed Consensus & Coordinator Sharding Prototype Plan

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-SPIKE-201 |
| Title | Distributed Consensus & Coordinator Sharding Prototype Plan |
| Version | 1.0 |
| Level | Engineering Execution Plan |
| Status | Baseline — Ready for Execution |
| Phase | Phase 2 Engineering Bridge |
| Duration | 90 days / 12 weeks |
| Owner | Distributed Systems Lead |
| Governing Plan | KEI-ENG-200 — Phase 2 Engineering Execution Plan |
| Governing Architecture Documents | KEI-ARC-020, KEI-ARC-021, KEI-ARC-022, KEI-DES-030, KEI-DES-031 |
| Predecessor | KEI-SPIKE-101 (Phase 1 Single-Node Prototype) |
| Next Plan File | KEI-FORMAL-201 — Distributed Consensus Verification Plan |

---

## 2. Executive Summary

This document defines the plan for building the **Distributed Consensus & Coordinator Sharding Prototype** — the first executable proof that the Phase 1 single-node engine can be clustered into a fault-tolerant distributed system.

Phase 1 proved the Golden Invariant on a single node. This prototype must prove that:

1. WAL segment heads can be replicated synchronously across 3 nodes via Raft.
2. Coordinator shards can be deterministically assigned, fenced, and failed over.
3. Bitmap state and lease deltas can be replicated without corruption.
4. Tier-1 S3 streaming can operate continuously under cluster load.
5. A failed node can be replaced and recover in <5 seconds.
6. Zero data loss occurs under `kill -9` failure scenarios.

The prototype is a **3-node cluster** running on the Phase 1 single-node engine, extended with Raft consensus, coordinator sharding, and S3 streaming.

---

## 3. Prototype Mission

The prototype must answer the following question:

> Can the single-node Keirox engine be clustered into a 3-node fault-tolerant system where WAL heads replicate synchronously, coordinator shards fail over safely, bitmap state remains consistent, S3 streaming continues under load, and a killed node is replaced in under 5 seconds — all with zero data loss?

If the answer is yes, the project proceeds into full Phase 2 hardening.

If the answer is no, the prototype must reveal exactly which distributed assumption failed, so the architecture or implementation strategy can be corrected before further investment.

---

## 4. Relationship to KEI-ENG-200

This prototype executes the first practical stage of Phase 2 and maps directly to the work packages defined in KEI-ENG-200.

| KEI-ENG-200 Work Package | Prototype Coverage |
|---|---|
| WP-P2-A: Raft Consensus Foundation | Core prototype focus — Data Plane Raft, Metadata Raft, leader election |
| WP-P2-B: Coordinator Sharding & State Replication | Core prototype focus — consistent hashing, epoch fencing, bitmap/lease replication |
| WP-P2-C: Tier-1 Streaming & Manifests | Included — S3 uploader, manifest registration |
| WP-P2-D: Crash Recovery & Chaos Validation | Included — node recovery, kill tests, basic chaos |

The prototype intentionally compresses all four work packages into a 90-day executable proof, deferring production hardening to the full Phase 2 build.

---

## 5. Prototype Scope

### 5.1 Must Have

The prototype MUST include:

1. 3-node cluster formation via Raft.
2. Data Plane Raft group replicating WAL segment heads.
3. Metadata & State Raft group replicating coordinator assignments.
4. Leader election and basic leader transfer.
5. Consistent hashing for coordinator shard assignment.
6. Coordinator epoch fencing.
7. Bitmap snapshot replication via Metadata Raft.
8. Lease delta replication via Metadata Raft.
9. Committed watermark replication.
10. Coordinator failover with state restoration.
11. S3 multipart chunk uploader.
12. Manifest metadata registration.
13. Node failure detection.
14. Node replacement and state reconstruction.
15. Basic split-brain detection and fencing.
16. Multi-node benchmark harness.
17. Kill/restart chaos tests.
18. Evidence report.

### 5.2 Should Have

The prototype SHOULD include if schedule permits:

1. Graceful leader transfer.
2. Raft log compaction.
3. Snapshot-based state transfer for slow followers.
4. Multi-node metrics dashboard.
5. S3 backoff and jitter for throttling.
6. Elastic NVMe backlog during S3 outage.
7. 24-hour multi-node soak test.

### 5.3 Will Not Have

The prototype WILL NOT include:

1. Kafka wire protocol gateway.
2. SQS/AMQP gateway.
3. Apache Iceberg catalog committer.
4. Native Arrow Flight SDKs.
5. KMS envelope encryption.
6. Multi-region replication.
7. ABAC authorization.
8. Jepsen full certification.
9. Production deployment automation.
10. Customer-facing APIs.

---

## 6. Prototype Success Criteria

### 6.1 Functional Success Criteria

| ID | Criterion |
|---|---|
| SPIKE-P2-F-001 | 3-node cluster forms Raft quorum and elects leader. |
| SPIKE-P2-F-002 | WAL segment heads replicate synchronously across all 3 nodes. |
| SPIKE-P2-F-003 | Producer ACK is issued only after quorum commit. |
| SPIKE-P2-F-004 | Consumer groups are deterministically assigned to coordinators. |
| SPIKE-P2-F-005 | Coordinator failover restores state and resumes leasing. |
| SPIKE-P2-F-006 | Stale epoch operations are rejected. |
| SPIKE-P2-F-007 | Bitmap snapshots replicate consistently. |
| SPIKE-P2-F-008 | Lease deltas replicate consistently. |
| SPIKE-P2-F-009 | S3 chunks upload and manifest registers correctly. |
| SPIKE-P2-F-010 | Failed node is replaced and recovers state. |
| SPIKE-P2-F-011 | Phase 1 single-node mode continues to work. |

### 6.2 Performance Success Criteria

| ID | Criterion | Mandatory Target | Stretch Target |
|---|---|---:|---:|
| SPIKE-P2-P-001 | Multi-node write throughput | ≥100 MB/s (3-node) | ≥150 MB/s |
| SPIKE-P2-P-002 | Write latency with quorum | p99 ≤3 ms | p99 ≤2.5 ms |
| SPIKE-P2-P-003 | Coordinator failover time | <3.5 seconds | <2.5 seconds |
| SPIKE-P2-P-004 | Node replacement time | <5 seconds | <3 seconds |
| SPIKE-P2-P-005 | S3 upload throughput | ≥50 MB/s | ≥100 MB/s |
| SPIKE-P2-P-006 | Leader election time | <2 seconds | <1 second |

### 6.3 Reliability Success Criteria

| ID | Criterion | Mandatory Target |
|---|---|---|
| SPIKE-P2-R-001 | Data loss during node kill | Zero (JML = 0) |
| SPIKE-P2-R-002 | Double lease under partition | Zero |
| SPIKE-P2-R-003 | State invariant violations | Zero |
| SPIKE-P2-R-004 | Recovery correctness | All state restored accurately |
| SPIKE-P2-R-005 | Soak stability | 24-hour multi-node soak with no unbounded growth |

---

## 7. Prototype Architecture Slice

### 7.1 Cluster Topology

```text
┌────────────────────────────────────────────────────────────────────┐
│                    3-NODE CLUSTER PROTOTYPE                         │
│                                                                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐            │
│  │   Node 1     │  │   Node 2     │  │   Node 3     │            │
│  │              │  │              │  │              │            │
│  │ Storage Eng  │  │ Storage Eng  │  │ Storage Eng  │            │
│  │ Data Raft    │  │ Data Raft    │  │ Data Raft    │            │
│  │ Meta Raft    │  │ Meta Raft    │  │ Meta Raft    │            │
│  │ Coordinator  │  │ Coordinator  │  │ Coordinator  │            │
│  │ S3 Uploader  │  │ S3 Uploader  │  │ S3 Uploader  │            │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘            │
│         │                 │                 │                     │
│         └─────────────────┼─────────────────┘                     │
│                           │                                        │
│                    Raft Replication                                │
│                    (gRPC / TCP)                                    │
│                           │                                        │
│                           ▼                                        │
│                    ┌──────────────┐                                │
│                    │   S3 / GCS   │                                │
│                    │  (Tier-1)    │                                │
│                    └──────────────┘                                │
└────────────────────────────────────────────────────────────────────┘
```

### 7.2 Simplifications

| Full Architecture Feature | Prototype Simplification |
|---|---|
| Production Raft library | Vetted Rust Raft library (openraft / raft-rs) |
| Multi-region replication | Single datacenter / same-AZ |
| KMS encryption | Disabled |
| ABAC authorization | Disabled |
| Iceberg catalog | Local Parquet manifest only |
| Kafka/SQS/AMQP gateways | Native prototype API only |
| Full Jepsen certification | Basic chaos tests only |

---

## 8. Technical Constraints

### 8.1 Raft Implementation

| Constraint | Requirement |
|---|---|
| Raft library | Use vetted Rust library (openraft preferred) |
| Transport | gRPC (tonic) or custom TCP |
| Log storage | Disk-backed with fsync |
| Snapshot format | Custom binary with CRC32C |
| Membership | Static 3-node for prototype; dynamic join/leave deferred |

### 8.2 Architecture Constraints

The prototype MUST respect:

1. All Phase 1 Golden Invariant rules.
2. ACK issued only after quorum commit.
3. No mutation of committed WAL records.
4. Epoch fencing for coordinator operations.
5. Idempotent duplicate ACK behavior.
6. Mandatory DLQ eviction.
7. Watermark monotonicity.
8. No double lease under any condition.

### 8.3 Prototype Anti-Goals

The prototype MUST NOT become:

1. A production deployment system.
2. A multi-region replication system.
3. A gateway compatibility project.
4. A security platform.
5. A lakehouse catalog project.

---

## 9. Work Packages

### 9.1 WP-0 — Multi-Node Engineering Foundation

Objective: Prepare the codebase and infrastructure for 3-node cluster development.

Deliverables:

1. Multi-node CI pipeline.
2. Docker Compose or Kubernetes manifest for 3-node cluster.
3. Cluster configuration management.
4. Node discovery and membership bootstrap.
5. Multi-node logging and tracing.

Exit criteria:

- 3-node cluster starts and nodes discover each other.
- CI pipeline builds and tests multi-node configuration.

---

### 9.2 WP-1 — Data Plane Raft

Objective: Implement synchronous 3-node Raft replication of WAL segment heads.

Deliverables:

1. Raft library integration.
2. Raft log storage (disk-backed).
3. Leader election.
4. Log replication to followers.
5. Commit index tracking.
6. Producer ACK gating on quorum commit.
7. Leader transfer (basic).
8. Raft health metrics.

Exit criteria:

- 3-node quorum forms and elects leader.
- WAL segment heads replicate to all followers.
- Producer ACK issued only after quorum commit.
- Leader failover elects new leader.

Primary references:

- KEI-ARC-022 §5 (Two-Tier Raft Topology)
- KEI-DES-030 (WAL Binary Format)

---

### 9.3 WP-2 — Metadata & State Raft

Objective: Implement the Metadata & State Raft group for coordinator assignments, manifests, and state snapshots.

Deliverables:

1. Metadata Raft group formation.
2. Coordinator assignment replication.
3. Stream manifest replication.
4. Bitmap snapshot replication.
5. Lease delta replication.
6. Committed watermark replication.
7. Raft log compaction (basic).

Exit criteria:

- Metadata Raft replicates coordinator assignments.
- Bitmap snapshots are consistent across nodes.
- Lease deltas are applied in order.
- Committed watermarks are durable.

Primary references:

- KEI-ARC-022 §7 (Metadata & State Plane Consensus)
- KEI-DES-031 (State Plane Data Structures)

---

### 9.4 WP-3 — Coordinator Sharding & Epoch Fencing

Objective: Implement deterministic coordinator assignment and epoch-fenced failover.

Deliverables:

1. Consistent hashing ring.
2. Coordinator shard assignment.
3. Coordinator epoch generation and validation.
4. Shard ownership transfer protocol.
5. Coordinator failover state restoration.
6. Stale epoch rejection.
7. Split-brain detection (basic).

Exit criteria:

- Consumer groups deterministically assigned to coordinators.
- Coordinator failover completes in <3.5 seconds.
- Stale epoch operations rejected.
- No double lease observed during failover.

Primary references:

- KEI-ARC-021 §10 (Coordinator Sharding and Epoch Fencing)
- KEI-DES-031 §18 (Failover Reconstruction Algorithm)

---

### 9.5 WP-4 — Tier-1 S3 Streaming

Objective: Implement continuous asynchronous streaming of sealed chunks to S3/GCS.

Deliverables:

1. S3 multipart uploader.
2. Chunk sealing lifecycle.
3. Manifest metadata registration.
4. S3 key hash-prefix partitioning.
5. Upload progress metrics.
6. Basic backoff for throttling.

Exit criteria:

- Sealed chunks stream to S3 continuously.
- Manifest accurately reflects S3 contents.
- Upload throughput ≥50 MB/s.

Primary references:

- KEI-ARC-020 §5 (Two-Tier Storage Hierarchy)
- KEI-DES-034 (Iceberg Catalog Committer — simplified)

---

### 9.6 WP-5 — Crash Recovery & Node Replacement

Objective: Implement node failure detection, replacement, and state reconstruction.

Deliverables:

1. Node failure detection (heartbeat).
2. WAL delta replay from peers.
3. S3 manifest reconstruction.
4. State reconciliation from snapshots + deltas.
5. Automated node replacement.
6. Recovery time measurement.

Exit criteria:

- Failed node detected within heartbeat timeout.
- Replacement node joins cluster and catches up.
- Recovery completes in <5 seconds.
- Zero data loss after recovery.

Primary references:

- KEI-ARC-022 §9 (Failover Protocols)
- KEI-OPS-040 OPS-RB-001 (Storage Node Failure)

---

### 9.7 WP-6 — Multi-Node Benchmarks & Chaos

Objective: Produce evidence for the Phase 2 prototype gate.

Deliverables:

1. Multi-node benchmark harness.
2. P1-P2-Proto workload profile.
3. P4-P2-Proto queue churn profile.
4. Kill/restart chaos tests.
5. Network partition simulation (basic).
6. 24-hour soak test.
7. Evidence report generator.

Exit criteria:

- All mandatory performance targets measured.
- Zero data loss in all chaos tests.
- No double lease in partition scenarios.
- Evidence report complete.

Primary references:

- KEI-BENCH-101 (Performance Validation Harness)
- KEI-OPS-041 (Validation, Benchmark & Chaos Test Plan)

---

## 10. 12-Week Execution Plan

### Week 1–2 — Multi-Node Mobilization

Primary work:

- Set up 3-node cluster infrastructure.
- Configure CI for multi-node builds.
- Implement node discovery and membership bootstrap.
- Define cluster configuration schema.
- Set up multi-node logging.

Exit:

- 3-node cluster starts.
- Nodes discover each other.
- CI passes for multi-node configuration.

---

### Week 3–4 — Data Plane Raft Foundation

Primary work:

- Integrate Raft library.
- Implement Raft log storage.
- Implement leader election.
- Implement log replication.
- Gate producer ACK on quorum commit.

Exit:

- 3-node quorum forms.
- Leader elected.
- WAL heads replicate.
- ACK gated on quorum.

---

### Week 5–6 — Metadata Raft & State Replication

Primary work:

- Form Metadata Raft group.
- Replicate coordinator assignments.
- Replicate bitmap snapshots.
- Replicate lease deltas.
- Replicate committed watermarks.

Exit:

- Metadata Raft operational.
- State snapshots consistent.
- Lease deltas applied in order.

---

### Week 7–8 — Coordinator Sharding & Epoch Fencing

Primary work:

- Implement consistent hashing ring.
- Implement coordinator epoch generation.
- Implement shard ownership transfer.
- Implement coordinator failover.
- Implement stale epoch rejection.

Exit:

- Coordinator failover <3.5 seconds.
- Stale operations rejected.
- No double lease.

---

### Week 9 — Tier-1 S3 Streaming

Primary work:

- Implement S3 multipart uploader.
- Implement chunk sealing.
- Implement manifest registration.
- Implement hash-prefix partitioning.

Exit:

- Chunks stream to S3.
- Manifest accurate.
- Upload throughput measured.

---

### Week 10 — Crash Recovery & Node Replacement

Primary work:

- Implement failure detection.
- Implement WAL delta replay.
- Implement state reconstruction.
- Measure recovery time.

Exit:

- Node replacement <5 seconds.
- Zero data loss.

---

### Week 11 — Benchmarks & Chaos Tests

Primary work:

- Run multi-node benchmarks.
- Run kill/restart chaos tests.
- Run network partition simulation.
- Run 24-hour soak test.

Exit:

- All mandatory targets measured.
- Zero data loss.
- No invariant violations.

---

### Week 12 — Evidence Report & Go/No-Go Review

Primary work:

- Compile benchmark results.
- Compile chaos test results.
- Compile recovery results.
- Prepare go/no-go recommendation.
- Present to Architecture Review Board.

Exit:

- Evidence package delivered.
- Go/no-go decision made.

---

## 11. Test Plan

### 11.1 Unit Tests

Required for:

- Raft log storage.
- Consistent hashing.
- Epoch fencing logic.
- State reconciliation.
- S3 key generation.

### 11.2 Integration Tests

Required for:

- 3-node cluster formation.
- Leader election.
- Log replication.
- Coordinator failover.
- Node replacement.
- S3 upload and manifest registration.

### 11.3 Chaos Tests

Required for:

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| CHAOS-P2-001 | Kill -9 leader node | New leader elected; zero data loss |
| CHAOS-P2-002 | Kill -9 follower node | Cluster continues; node replaces |
| CHAOS-P2-003 | Kill coordinator node | Failover <3.5s; no double lease |
| CHAOS-P2-004 | Network partition (1 vs 2) | Majority continues; minority fenced |
| CHAOS-P2-005 | S3 outage during upload | Backlog; backpressure engages |
| CHAOS-P2-006 | Recovery during recovery | Idempotent; no corruption |

### 11.4 Invariant Tests

Required for:

- No terminal regression after failover.
- No double lease after failover.
- Watermark monotonicity across nodes.
- Stale epoch rejection.
- State consistency after recovery.

---

## 12. Benchmark Plan

### 12.1 Benchmark Profiles

| Profile | Purpose | Workload |
|---|---|---|
| P1-P2-Proto | Multi-node sustained throughput | 100 MB/s, 3-node quorum |
| P3-P2-Proto | High cardinality under quorum | 100K streams, 3-node |
| P4-P2-Proto | Queue churn under failover | 100K leases, coordinator kill mid-test |
| P5-P2-Proto | S3 streaming under load | Continuous export, 50 MB/s |
| P6-P2-Proto | Degraded / partition | Network partition + S3 throttle |

### 12.2 Benchmark Metrics

| Metric | Required |
|---|---|
| Multi-node write throughput (MB/s) | Yes |
| Write latency p50/p99/p999 (with quorum) | Yes |
| Leader election time | Yes |
| Coordinator failover time | Yes |
| Node replacement time | Yes |
| S3 upload throughput | Yes |
| Replication lag | Yes |
| Raft commit latency | Yes |

---

## 13. Evidence Package

The prototype evidence package MUST include:

1. Cluster formation report.
2. Raft replication report.
3. Coordinator failover report.
4. State replication consistency report.
5. S3 streaming report.
6. Node recovery report.
7. Benchmark report.
8. Chaos test report.
9. Invariant checker report.
10. Known defects list.
11. Go/no-go recommendation.

---

## 14. Prototype Go/No-Go Gate

### 14.1 Go Criteria

A GO decision requires:

1. All functional mandatory criteria pass.
2. All mandatory performance criteria pass.
3. Zero data loss in all chaos tests.
4. Zero double lease in all partition scenarios.
5. Zero unresolved invariant violations.
6. Recovery works reliably.
7. Evidence package complete.

### 14.2 Conditional Go Criteria

A CONDITIONAL GO may be granted if:

1. One or more stretch targets fail.
2. A non-critical defect remains open.
3. A remediation plan is approved.

### 14.3 Gate Outcomes

| Outcome | Meaning |
|---|---|
| GO | Continue into full Phase 2 hardening. |
| CONDITIONAL GO | Continue after specific fixes. |
| PIVOT | Core distributed assumption needs adjustment. |
| STOP | Distributed architecture fundamentally flawed. |

---

## 15. Risks and Mitigations

| Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|
| Raft library integration complexity | High | Medium | Use vetted library; start integration early |
| State replication inconsistency | Critical | Medium | Snapshot + delta replay; invariant checks |
| Coordinator failover latency | High | Medium | Optimize state restoration; pre-warm successors |
| Split-brain double lease | Critical | Low | Epoch fencing; formal verification in parallel |
| S3 throttling during tests | Medium | High | Backoff with jitter; hash-prefix partitioning |
| Multi-node test environment instability | Medium | Medium | Dedicated test cluster; containerized setup |
| Phase 1 regressions | High | Medium | Continuous Phase 1 test suite in CI |

---

## 16. Prototype Team

### 16.1 Minimum Team

| Role | Count | Responsibility |
|---|---:|---|
| Distributed Systems Lead | 1 | Raft, consensus, coordinator sharding |
| Storage Engineer | 1 | WAL replication, S3 streaming |
| State Plane Engineer | 1 | Bitmap replication, lease deltas |
| SRE / QA Engineer | 1 | Multi-node tests, chaos, benchmarks |

### 16.2 Optional Support

| Role | Responsibility |
|---|---|
| Chief Architect | Architecture compliance, conflict resolution |
| Formal Methods Advisor | TLA+ models for distributed invariants |
| Cloud Infrastructure Advisor | S3/GCS integration guidance |

---

## 17. Definition of Done

The prototype is done when:

1. 3-node cluster forms and operates correctly.
2. Raft replication works for WAL heads and metadata.
3. Coordinator failover completes within target time.
4. State replication is consistent.
5. S3 streaming operates continuously.
6. Node replacement works within target time.
7. Zero data loss in all chaos tests.
8. All mandatory success criteria measured.
9. Evidence package complete.
10. Go/no-go recommendation delivered.

---

## 18. Traceability to Architecture Documents

| Prototype Area | Governing Document |
|---|---|
| Two-tier Raft topology | KEI-ARC-022 §5 |
| Data Plane Raft | KEI-ARC-022 §6 |
| Metadata & State Raft | KEI-ARC-022 §7 |
| Coordinator sharding | KEI-ARC-021 §10 |
| Epoch fencing | KEI-ARC-021 §10, KEI-DES-031 |
| WAL binary format | KEI-DES-030 |
| State data structures | KEI-DES-031 |
| Two-tier storage | KEI-ARC-020 §5 |
| S3 streaming | KEI-ARC-020 §10 |
| Failover protocols | KEI-ARC-022 §9 |
| Node recovery | KEI-OPS-040 OPS-RB-001 |

---

## 19. Next Planning File

After this document, the next planning file is:

```text
KEI-FORMAL-201_Distributed_Consensus_Verification_Plan.md
```

It will define the TLA+ models for Raft consensus, coordinator epoch fencing, and multi-node state consistency verification.

---

## 20. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial Distributed Consensus & Coordinator Sharding Prototype Plan. Defines 3-node cluster prototype, work packages, 12-week execution plan, success criteria, chaos tests, and go/no-go gate. |