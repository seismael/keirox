# KEI-ENG-200 — Phase 2 Engineering Execution Plan
## Distributed Durability, Coordinator Sharding & Tier-1 Streaming

---

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-ENG-200 |
| Title | Phase 2 Engineering Execution Plan |
| Version | 1.0 |
| Level | Engineering Execution Plan |
| Status | Baseline — Ready for Execution |
| Phase | Phase 2 — Distributed Durability & Coordinator Sharding |
| Duration | Months 10–18 (9 months) |
| Owner | Engineering Program Lead / Chief Architect |
| Governing Architecture | KEI-ARC-020, KEI-ARC-021, KEI-ARC-022, KEI-ARC-026, KEI-DES-030, KEI-DES-031, KEI-DES-036 |
| Predecessor | KEI-ENG-100 (Phase 1), KEI-SPIKE-101 (Prototype) |
| Next Phase Plan | KEI-ENG-300 (Phase 3 — Ecosystem Gateways & Lakehouse) |

---

## 2. Executive Summary

Phase 1 proved that the Golden Invariant works on a single node. Phase 2 answers the next critical question:

> Can the single-node engine be clustered into a fault-tolerant distributed system with zero data loss, sub-3.5-second failover, and continuous cloud object storage streaming — without violating the architectural invariants?

Phase 2 transforms the single-node prototype into a **3-node production cluster** with:

1. **Tier-0 Local Multi-Raft Quorum** — synchronous replication of WAL segment heads across 3 nodes.
2. **Deterministic Coordinator Sharding** — consistent-hashing consumer group coordinator assignment with epoch-fenced failover.
3. **Tier-1 Cloud Object Storage Streaming** — continuous asynchronous multipart upload to S3/GCS with manifest registration.
4. **Crash-Recovery Protocol** — node failure recovery from S3 manifests + peer WAL delta in <5 seconds.

Phase 2 is the most architecturally dangerous phase. Distributed consensus, split-brain fencing, and cross-node state reconciliation are where most distributed systems silently corrupt data. This plan is structured to expose those failures early and prove correctness before Phase 3 ecosystem work begins.

---

## 3. Phase 2 Mission

The mission of Phase 2 is:

1. Implement synchronous 3-node Raft consensus over active WAL segment heads.
2. Implement the Metadata & State Raft group for coordinator assignments, manifests, and state snapshots.
3. Implement deterministic coordinator sharding with epoch fencing.
4. Implement continuous Tier-1 S3/GCS streaming with manifest registration.
5. Implement crash-recovery protocol (<5 second node replacement).
6. Prove zero data loss (JML=0) under automated `kill -9` failure simulations.
7. Prove failover and lease reassignment in <3.5 seconds.
8. Prove sustained S3 streaming with WAF ≤ 1.35.
9. Produce distributed benchmark and chaos evidence.
10. Prepare the codebase for Phase 3 ecosystem gateways.

---

## 4. Phase 2 Scope

### 4.1 In Scope

| Workstream | Scope |
|---|---|
| Raft Consensus | Data Plane Raft (WAL heads), Metadata & State Raft (coordinator assignments, manifests, snapshots, watermarks) |
| Coordinator Sharding | Consistent hashing, epoch fencing, shard ownership transfer, lease journal replication |
| Tier-1 Streaming | S3/GCS multipart uploader, manifest metadata registration, chunk sealing lifecycle |
| Crash Recovery | Node failure detection, WAL delta replay from peers, S3 manifest reconstruction, state reconciliation |
| State Replication | Bitmap snapshot replication, lease delta replication, committed watermark replication |
| Backpressure | Distributed backpressure coordination across nodes |
| Observability | Cluster-level metrics, Raft health, replication lag, failover telemetry |
| Testing | Multi-node integration tests, chaos tests, Jepsen-style consistency validation |
| Documentation | Runbook updates, ADR updates, RTM updates |

### 4.2 Out of Scope

| Item | Reason |
|---|---|
| Kafka wire protocol gateway | Phase 3 |
| SQS/AMQP gateway | Phase 3/4 |
| Apache Iceberg catalog committer | Phase 3 |
| Native Arrow Flight SDKs | Phase 3 |
| KMS envelope encryption | Phase 4 |
| Multi-region replication | Phase 4 |
| ABAC authorization | Phase 4 |
| Jepsen full certification | Phase 4 |
| Customer-facing APIs | Phase 3+ |

### 4.3 Phase 2 Constraints

1. All Phase 1 invariants MUST continue to hold.
2. No breaking changes to the WAL binary format (KEI-DES-030).
3. No breaking changes to the state plane data structures (KEI-DES-031).
4. Single-node mode MUST remain functional for development and testing.
5. All new distributed behavior MUST be gated behind feature flags.

---

## 5. Phase 2 Objectives

| ID | Objective | Success Metric |
|---|---|---|
| OBJ-P2-001 | Prove zero data loss under node failure | JML = 0 in automated kill -9 tests |
| OBJ-P2-002 | Prove fast failover | Coordinator failover < 3.5 seconds |
| OBJ-P2-003 | Prove fast node replacement | Node recovery < 5 seconds |
| OBJ-P2-004 | Prove sustained S3 streaming | WAF ≤ 1.35 over 72-hour soak |
| OBJ-P2-005 | Prove split-brain safety | No double-lease under network partition |
| OBJ-P2-006 | Prove epoch fencing correctness | Stale coordinator operations rejected |
| OBJ-P2-007 | Prove state replication correctness | Bitmap snapshots + lease deltas consistent |
| OBJ-P2-008 | Produce distributed benchmark evidence | Multi-node throughput/latency report |
| OBJ-P2-009 | Establish cluster operational procedures | Runbooks tested and validated |
| OBJ-P2-010 | Prepare for Phase 3 | Architecture review board approval |

---

## 6. Phase 2 Delivery Strategy

Phase 2 is divided into four major work packages executed over 9 months (36 weeks).

### 6.1 Work Package Overview

| Work Package | ID | Duration | Focus |
|---|---|---|---|
| Raft Consensus Foundation | WP-P2-A | Weeks 1–12 | Data Plane Raft, Metadata Raft, leader election, log replication |
| Coordinator Sharding & State Replication | WP-P2-B | Weeks 8–20 | Consistent hashing, epoch fencing, bitmap/lease replication |
| Tier-1 Streaming & Manifests | WP-P2-C | Weeks 12–28 | S3/GCS uploader, manifest registration, chunk lifecycle |
| Crash Recovery & Chaos Validation | WP-P2-D | Weeks 20–36 | Node recovery, failover, Jepsen-style tests, evidence package |

### 6.2 Overlap Strategy

Work packages intentionally overlap to enable parallel development:

- Weeks 8–12: WP-P2-A and WP-P2-B overlap (consensus + sharding).
- Weeks 12–20: WP-P2-B and WP-P2-C overlap (sharding + streaming).
- Weeks 20–28: WP-P2-C and WP-P2-D overlap (streaming + recovery).
- Weeks 28–36: WP-P2-D focus (chaos validation and evidence).

---

## 7. Work Package A — Raft Consensus Foundation

### 7.1 Objective

Implement the two-tier Raft topology defined in KEI-ARC-022:

- **Data Plane Raft**: Synchronous 3-node quorum replicating active WAL segment heads.
- **Metadata & State Raft**: Replicated metadata stream managing coordinator assignments, stream manifests, bitmap state snapshots, and committed watermarks.

### 7.2 Deliverables

| ID | Deliverable | Description |
|---|---|---|
| D-P2-A-001 | Raft log replication engine | Core Raft implementation (or integration of a vetted Rust Raft library) |
| D-P2-A-002 | Data Plane Raft group | 3-node synchronous quorum over WAL segment heads |
| D-P2-A-003 | Metadata & State Raft group | Replicated metadata stream |
| D-P2-A-004 | Leader election and transfer | Automated leader election with graceful transfer |
| D-P2-A-005 | Log compaction and snapshots | Raft log compaction to bound metadata growth |
| D-P2-A-006 | Cluster membership management | Node join, leave, and replacement |
| D-P2-A-007 | Raft health metrics | Leader status, replication lag, term changes |
| D-P2-A-008 | Single-node compatibility mode | Raft disabled for development/testing |

### 7.3 Technical Decisions

| Decision | Options | Recommendation | Rationale |
|---|---|---|---|
| Raft implementation | Custom vs. library | Evaluate `openraft` or `raft-rs` | Avoid re-implementing consensus; focus on integration |
| Log storage | In-memory vs. disk | Disk-backed with fsync | Durability requirement |
| Snapshot format | Custom vs. serde | Custom binary with CRC32C | Consistency with WAL format |
| Transport | gRPC vs. custom TCP | gRPC (tonic) | Ecosystem support, TLS, streaming |

### 7.4 Acceptance Criteria

- 3-node cluster forms quorum and elects leader.
- WAL segment heads replicate synchronously across all 3 nodes.
- Leader failover completes without data loss.
- Metadata Raft replicates coordinator assignments and manifests.
- Raft log compaction bounds metadata growth.
- Single-node mode continues to work for development.

---

## 8. Work Package B — Coordinator Sharding & State Replication

### 8.1 Objective

Implement deterministic coordinator sharding with epoch fencing, and replicate consumer group state (bitmap snapshots, lease deltas, committed watermarks) via the Metadata Raft group.

### 8.2 Deliverables

| ID | Deliverable | Description |
|---|---|---|
| D-P2-B-001 | Consistent hashing ring | Deterministic coordinator assignment per consumer group |
| D-P2-B-002 | Epoch fencing mechanism | Monotonic coordinator epoch with stale request rejection |
| D-P2-B-003 | Shard ownership transfer | Safe transfer of shard ownership during failover |
| D-P2-B-004 | Bitmap snapshot replication | Periodic Roaring Bitmap snapshots to Metadata Raft |
| D-P2-B-005 | Lease delta replication | Incremental lease/ACK/NACK deltas to Metadata Raft |
| D-P2-B-006 | Committed watermark replication | Durable W_base replication |
| D-P2-B-007 | Coordinator failover protocol | Successor assumes ownership, increments epoch, restores state |
| D-P2-B-008 | Split-brain detection and fencing | Network partition detection, minority partition isolation |

### 8.3 Coordinator Failover Protocol

```text
1. Detect coordinator node failure (heartbeat timeout)
2. Verify no other coordinator claims the same shard
3. Increment coordinator_epoch via Metadata Raft
4. Assign shard to successor coordinator
5. Restore shard state:
   a. Load latest bitmap snapshot from Metadata Raft
   b. Replay lease deltas after snapshot LSN
   c. Rebuild timing wheel from active leases
6. Validate state invariants
7. Reject stale-epoch requests
8. Resume lease and ACK processing
```

### 8.4 Acceptance Criteria

- Consumer groups are deterministically assigned to coordinators.
- Coordinator failover completes in <3.5 seconds.
- Stale epoch requests are rejected.
- No double-lease observed under network partition.
- Bitmap snapshots and lease deltas are consistent after replay.
- Committed watermarks are durable across node failures.

---

## 9. Work Package C — Tier-1 Streaming & Manifests

### 9.1 Objective

Implement continuous asynchronous streaming of sealed columnar chunks to cloud object storage (S3/GCS) with manifest metadata registration.

### 9.2 Deliverables

| ID | Deliverable | Description |
|---|---|---|
| D-P2-C-001 | S3/GCS multipart uploader | Async chunked upload with retry and backoff |
| D-P2-C-002 | Manifest metadata registry | Track sealed chunk ranges, S3 URIs, byte offsets |
| D-P2-C-003 | Chunk sealing lifecycle | Transition from active NVMe to sealed Tier-1 chunk |
| D-P2-C-004 | S3 key hash-prefix partitioning | Distribute PUT requests across S3 prefixes |
| D-P2-C-005 | Backoff and jitter for throttling | Exponential backoff with jitter for S3 503 responses |
| D-P2-C-006 | Elastic backlog management | Tier-0 NVMe buffer expansion during S3 outages |
| D-P2-C-007 | Upload progress metrics | Bytes uploaded, chunks pending, backlog ETA |
| D-P2-C-008 | Manifest consistency checks | Verify manifest matches actual S3 objects |

### 9.3 S3 Streaming Pipeline

```text
Sealed NVMe Segment
    │
    ▼
Chunk Sealer (64 MB target)
    │
    ▼
Multipart Uploader (async, parallel parts)
    │
    ├──► S3 PUT (with hash-prefix key)
    │
    ▼
Manifest Registration (Metadata Raft)
    │
    ▼
NVMe Truncation (after confirmed upload)
```

### 9.4 Acceptance Criteria

- Sealed chunks stream to S3/GCS continuously.
- Manifest accurately reflects S3 contents.
- S3 throttling (503) is handled with backoff and jitter.
- Elastic backlog prevents NVMe overflow during S3 outages.
- WAF ≤ 1.35 measured over 72-hour soak.
- NVMe truncation occurs only after confirmed upload.

---

## 10. Work Package D — Crash Recovery & Chaos Validation

### 10.1 Objective

Implement crash-recovery protocol and validate the entire distributed system under adversarial conditions.

### 10.2 Deliverables

| ID | Deliverable | Description |
|---|---|---|
| D-P2-D-001 | Node failure detection | Heartbeat-based failure detection with configurable timeout |
| D-P2-D-002 | WAL delta replay from peers | Reconstruct unsealed WAL segments from healthy peers |
| D-P2-D-003 | S3 manifest reconstruction | Rebuild stream registry from S3 manifests |
| D-P2-D-004 | State reconciliation | Reconcile Roaring Bitmap state from snapshots + lease deltas |
| D-P2-D-005 | Automated node replacement | Provision replacement node, join cluster, catch up |
| D-P2-D-006 | Chaos test framework | Automated kill -9, network partition, disk stall injection |
| D-P2-D-007 | Jepsen-style consistency tests | Linearizability checks under adversarial conditions |
| D-P2-D-008 | Evidence package | Benchmark reports, chaos reports, invariant checks |

### 10.3 Crash Recovery Protocol

```text
1. Detect node failure
2. Mark node as drained in cluster membership
3. Provision replacement node
4. Join replacement to cluster membership
5. Restore state:
   a. Fetch S3 manifests
   b. Reconstruct stream registry
   c. Replay active WAL delta from peers
   d. Restore coordinator shards from Metadata Raft
6. Validate checksums and invariants
7. Add replacement as follower
8. Allow catch-up replication
9. Return node to active service
```

### 10.4 Chaos Test Matrix

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| CHAOS-P2-001 | Kill -9 leader node | Raft elects new leader; zero data loss |
| CHAOS-P2-002 | Kill -9 follower node | Cluster continues; node replaces |
| CHAOS-P2-003 | Network partition (1 vs 2) | Majority continues; minority fenced |
| CHAOS-P2-004 | Network partition (coordinator) | Epoch fencing; no double lease |
| CHAOS-P2-005 | Disk stall on leader | Leader steps down or request times out |
| CHAOS-P2-006 | S3 outage during upload | Elastic backlog; backpressure engages |
| CHAOS-P2-007 | Clock skew injection | Lease expiry safe; HLC order preserved |
| CHAOS-P2-008 | Simultaneous multi-failure | System degrades safely; no corruption |
| CHAOS-P2-009 | Recovery during recovery | Idempotent recovery; no state corruption |
| CHAOS-P2-010 | Split-brain heal | Orphaned writes quarantined |

### 10.5 Acceptance Criteria

- Node replacement completes in <5 seconds.
- Zero data loss (JML=0) across all chaos tests.
- No double-lease observed under any partition scenario.
- State invariants hold after all recovery scenarios.
- Evidence package complete with benchmark and chaos reports.

---

## 11. Phase 2 Milestone Schedule

| Milestone | Target Weeks | Deliverables | Exit Criteria |
|---|---|---|---|
| M2.0 Phase 2 Mobilization | 1–2 | Team onboarding, repo updates, CI for multi-node | Multi-node CI pipeline passing |
| M2.1 Raft Foundation | 3–8 | Data Plane Raft, Metadata Raft, leader election | 3-node quorum forms; WAL heads replicate |
| M2.2 Coordinator Sharding | 6–14 | Consistent hashing, epoch fencing, state replication | Coordinator failover < 3.5s; no double lease |
| M2.3 Tier-1 Streaming | 10–20 | S3 uploader, manifests, chunk lifecycle | Sustained S3 streaming; WAF ≤ 1.35 |
| M2.4 Crash Recovery | 16–26 | Node recovery, WAL replay, state reconciliation | Node replacement < 5s; JML = 0 |
| M2.5 Chaos Validation | 22–32 | Chaos tests, Jepsen-style tests | All chaos tests pass; no invariant violations |
| M2.6 Evidence & Certification | 30–36 | Evidence package, certification review | Phase 2 certification approved |

---

## 12. Phase 2 Acceptance Criteria (Exit Gates)

### 12.1 Functional Acceptance

| ID | Requirement |
|---|---|
| ACC-P2-F-001 | 3-node cluster forms quorum and replicates WAL heads synchronously |
| ACC-P2-F-002 | Coordinator failover completes with state restoration |
| ACC-P2-F-003 | S3/GCS streaming operates continuously |
| ACC-P2-F-004 | Node replacement restores full functionality |
| ACC-P2-F-005 | Split-brain scenarios are fenced correctly |
| ACC-P2-F-006 | All Phase 1 functionality continues to work |

### 12.2 Performance Acceptance

| ID | Requirement | Target |
|---|---|---|
| ACC-P2-P-001 | Multi-node write throughput | ≥ 100 MB/s sustained (3-node cluster) |
| ACC-P2-P-002 | Write latency with quorum | p99 ≤ 3 ms (local same-AZ quorum) |
| ACC-P2-P-003 | Coordinator failover time | < 3.5 seconds |
| ACC-P2-P-004 | Node replacement time | < 5 seconds |
| ACC-P2-P-005 | S3 upload throughput | ≥ 50 MB/s sustained |
| ACC-P2-P-006 | Write Amplification Factor | ≤ 1.35 |

### 12.3 Reliability Acceptance

| ID | Requirement | Target |
|---|---|---|
| ACC-P2-R-001 | Data loss during node failure | Zero (JML = 0) |
| ACC-P2-R-002 | Double-lease under partition | Zero |
| ACC-P2-R-003 | State invariant violations | Zero |
| ACC-P2-R-004 | Chaos test pass rate | 100% |
| ACC-P2-R-005 | 72-hour soak stability | No unbounded growth, no leaks |

### 12.4 Operational Acceptance

| ID | Requirement |
|---|---|
| ACC-P2-O-001 | Cluster health metrics are observable |
| ACC-P2-O-002 | Raft leader status is observable |
| ACC-P2-O-003 | Replication lag is observable |
| ACC-P2-O-004 | Failover events are logged and audited |
| ACC-P2-O-005 | Runbooks are tested and validated |

---

## 13. Team Scaling for Phase 2

### 13.1 Phase 2 Team Additions

| Role | Count | Responsibility |
|---|---|---|
| Distributed Systems Engineer (Raft) | 1–2 | Raft implementation, consensus integration |
| Distributed Systems Engineer (State) | 1 | Coordinator sharding, state replication |
| Cloud/Storage Engineer | 1 | S3/GCS integration, manifest management |
| Chaos/QA Engineer | 1 | Chaos framework, Jepsen-style tests |

### 13.2 Phase 2 Total Team Size

| Role Category | Phase 1 | Phase 2 Addition | Phase 2 Total |
|---|---:|---:|---:|
| Architecture/Leadership | 2 | 0 | 2 |
| Storage Engine | 1–2 | 0 | 1–2 |
| State Plane | 1 | 0 | 1 |
| Distributed Systems | 0 | 2–3 | 2–3 |
| Cloud/Storage | 0 | 1 | 1 |
| Data Platform | 1 | 0 | 1 |
| SRE/QA | 1 | 1 | 2 |
| **Total** | **6–8** | **4–5** | **10–13** |

---

## 14. Dependencies and Prerequisites

### 14.1 Phase 1 Prerequisites

Phase 2 MUST NOT begin until:

1. Phase 1 certification is approved (KEI-ENG-100 M1.10).
2. Single-node engine passes all Phase 1 acceptance criteria.
3. Formal state machine validation (KEI-FORMAL-101) is complete.
4. Performance benchmark evidence package is delivered.
5. Go/No-Go gate decision is GO or CONDITIONAL GO with remediation complete.

### 14.2 External Dependencies

| Dependency | Purpose | Risk |
|---|---|---|
| Raft library (openraft / raft-rs) | Consensus implementation | Library maturity, API stability |
| Cloud provider account (AWS/GCS) | S3/GCS streaming | Cost, throttling limits |
| Multi-node test environment | Cluster testing | Hardware provisioning |
| Chaos testing tools | Failure injection | Tool reliability |

---

## 15. Risks and Mitigations

| Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|
| Raft implementation complexity | Critical | Medium | Use vetted library; extensive integration testing |
| Split-brain data corruption | Critical | Medium | Epoch fencing; Jepsen-style tests; formal verification |
| S3 throttling during bursts | High | High | Elastic backlog; backoff with jitter; hash-prefix partitioning |
| State replication inconsistency | Critical | Medium | Snapshot + delta replay; invariant checks; formal verification |
| Coordinator failover latency | High | Medium | Optimize state restoration; pre-warm successor nodes |
| Multi-node test environment cost | Medium | High | Use cloud spot instances; containerized test clusters |
| Phase 1 regressions during Phase 2 | High | Medium | Continuous Phase 1 test suite in CI; feature flags |

---

## 16. Phase 2 Evidence Package

The Phase 2 evidence package MUST include:

1. Multi-node benchmark report (throughput, latency, failover time).
2. Chaos test report (all CHAOS-P2 scenarios).
3. Jepsen-style consistency test report.
4. State replication consistency report.
5. S3 streaming report (WAF, throughput, throttling handling).
6. Node recovery report (time, data integrity).
7. 72-hour soak test report.
8. Invariant checker report (zero violations).
9. Updated runbooks (tested).
10. Updated ADRs and RTM.
11. Phase 2 certification report with go/no-go recommendation.

---

## 17. Phase 2 Exit Review

The Phase 2 exit review MUST evaluate:

1. All functional acceptance criteria met.
2. All performance acceptance criteria met.
3. All reliability acceptance criteria met.
4. All operational acceptance criteria met.
5. All chaos tests pass with zero invariant violations.
6. Evidence package complete and reviewed.
7. Architecture Review Board approval.
8. Phase 3 entry plan approved.

### 17.1 Phase 2 Outcomes

| Outcome | Meaning |
|---|---|
| PHASE 2 CERTIFIED | Proceed to Phase 3 (Ecosystem Gateways & Lakehouse) |
| CONDITIONALLY CERTIFIED | Proceed after defined remediation tasks |
| EXTENDED | Additional Phase 2 work required |
| RE-SCOPE | Major technical adjustment required |
| STOP | Core distributed assumptions failed |

---

## 18. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial Phase 2 Engineering Execution Plan. Defines mission, scope, work packages, milestones, acceptance criteria, team scaling, dependencies, risks, and evidence requirements for Distributed Durability & Coordinator Sharding phase. |