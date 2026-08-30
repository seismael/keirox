# KEI-RISK-201 — Phase 2 Risk Reduction & Go/No-Go Plan

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-RISK-201 |
| Title | Phase 2 Risk Reduction & Go/No-Go Plan |
| Version | 1.0 |
| Level | Engineering Execution Plan |
| Status | Baseline — Ready for Execution |
| Phase | Phase 2 — Distributed Durability & Coordinator Sharding |
| Duration | Months 10–18 (9 months) |
| Owner | Engineering Program Lead / Chief Architect |
| Governing Plan | KEI-ENG-200 — Phase 2 Engineering Execution Plan |
| Related Plans | KEI-SPIKE-201, KEI-FORMAL-201, KEI-BENCH-201 |
| Governing Architecture Documents | KEI-ARC-020, KEI-ARC-021, KEI-ARC-022, KEI-ARC-026, KEI-DES-030, KEI-DES-031 |
| Predecessor | KEI-RISK-101 (Phase 1 Risk Reduction & Go/No-Go Plan) |

---

## 2. Executive Summary

Phase 1 de-risked the single-node Golden Invariant. Phase 2 introduces the most dangerous class of distributed systems risks: **consensus correctness under failure, split-brain state corruption, cross-node state replication inconsistency, and cloud storage streaming reliability**.

These risks are qualitatively different from Phase 1 risks. In Phase 1, a bug meant a single node behaved incorrectly. In Phase 2, a bug can mean **silent data corruption across nodes, double-lease violations under network partitions, or permanent state divergence between replicas**.

This document defines:

1. The Phase 2 risk scoring methodology.
2. The complete Phase 2 risk register with mitigations.
3. Distributed systems specific risk categories.
4. The Go/No-Go gate framework for Phase 2 exit.
5. Contingency and pivot strategies.
6. Risk review cadence and escalation procedures.

---

## 3. Risk Management Methodology

### 3.1 Risk Scoring Matrix

Risks are evaluated on a 5×5 matrix based on **Likelihood** and **Impact**.

| Likelihood \ Impact | 1 (Negligible) | 2 (Minor) | 3 (Moderate) | 4 (Major) | 5 (Critical) |
|---|---|---|---|---|---|
| **5 (Almost Certain)** | 5 | 10 | 15 | 20 | **25** |
| **4 (Likely)** | 4 | 8 | 12 | 16 | **20** |
| **3 (Possible)** | 3 | 6 | 9 | 12 | 15 |
| **2 (Unlikely)** | 2 | 4 | 6 | 8 | 10 |
| **1 (Rare)** | 1 | 2 | 3 | 4 | 5 |

### 3.2 Risk Severity Bands

| Score | Severity | Required Action |
|---:|---|---|
| **15–25** | **Critical** | Immediate mitigation required. Blocks milestone exit if unresolved. Reviewed daily. |
| **8–14** | **High** | Active mitigation plan required. Reviewed weekly. |
| **4–7** | **Medium** | Monitor and prepare contingency. Reviewed biweekly. |
| **1–3** | **Low** | Accept and monitor. Reviewed monthly. |

### 3.3 Risk Categories

| Category | Description | Phase 2 Relevance |
|---|---|---|
| **Correctness** | Data loss, state corruption, invariant violations | Highest priority |
| **Performance** | Latency regression, throughput degradation | High priority |
| **Reliability** | Failover failure, recovery failure | High priority |
| **Operational** | Deployment, monitoring, debugging difficulty | Medium priority |
| **Organizational** | Hiring, expertise gaps, coordination overhead | Medium priority |
| **External** | Library dependencies, cloud provider issues | Medium priority |

---

## 4. Phase 2 Risk Register

### 4.1 Critical Risks (Score 15–25)

| Risk ID | Risk Description | Score | Category | Owner | Mitigation | Contingency |
|---|---|---:|---|---|---|---|
| RISK-P2-001 | **Raft consensus implementation contains a correctness bug** that causes silent data loss or log divergence under specific failure sequences. | 20 | Correctness | Raft Engineer + Chief Architect | Use vetted Rust Raft library (openraft/raft-rs); TLA+ verification (KEI-FORMAL-201); Jepsen-style chaos tests; formal model checking before integration. | If library is fundamentally flawed, pivot to alternative library or implement minimal custom Raft with formal verification. |
| RISK-P2-002 | **Split-brain network partition causes double-lease violations** where two coordinators grant leases for the same offset simultaneously. | 20 | Correctness | State Engineer + Chief Architect | Epoch fencing with monotonic coordinator epochs; TLA+ split-brain model (KEI-FORMAL-201 Model 10); partition chaos tests; runtime invariant checker. | If epoch fencing is insufficient, add distributed lock service or quorum-based lease validation. |
| RISK-P2-003 | **State replication inconsistency** where bitmap snapshots or lease deltas diverge between coordinator and replicas, causing permanent state corruption. | 16 | Correctness | State Engineer + Raft Engineer | Snapshot + delta replay protocol; TLA+ replication model (KEI-FORMAL-201 Model 9); invariant checker on every recovery; deterministic replay tests. | If replication protocol is flawed, fall back to full-state transfer on every failover (slower but correct). |
| RISK-P2-004 | **Coordinator failover exceeds 3.5-second target**, causing unacceptable lease gaps for production workloads. | 15 | Performance | State Engineer + SRE Lead | Optimize state restoration path; pre-warm successor nodes; benchmark failover timing (KEI-BENCH-201); profile and optimize hot path. | If failover cannot meet target, increase lease TTL defaults to tolerate longer gaps; document SLA impact. |

### 4.2 High Risks (Score 8–14)

| Risk ID | Risk Description | Score | Category | Owner | Mitigation | Contingency |
|---|---|---:|---|---|---|---|
| RISK-P2-005 | **Raft library integration complexity** exceeds estimates, delaying consensus foundation by weeks. | 12 | External | Raft Engineer | Evaluate libraries in Week 1; build integration spike before committing; maintain fallback library option. | If both libraries fail, implement minimal Raft subset (leader election + log replication) with formal verification. |
| RISK-P2-006 | **S3 throttling (503 Slow Down)** during burst ingress causes NVMe backlog overflow and data loss. | 12 | Reliability | Cloud Engineer + Storage Lead | S3 key hash-prefix partitioning; exponential backoff with jitter; elastic NVMe backlog; backpressure ladder (KEI-ARC-027). | If S3 is persistently throttled, reduce upload frequency; increase NVMe buffer; alert operators. |
| RISK-P2-007 | **Multi-node test environment instability** causes flaky test results and wasted engineering time. | 10 | Operational | SRE Lead | Dedicated test cluster; containerized setup; deterministic network configuration; environment health checks before each test run. | If environment is unreliable, fall back to single-node simulation with network fault injection. |
| RISK-P2-008 | **Hiring distributed systems experts fails**, leaving critical roles unfilled and blocking Phase 2 progress. | 10 | Organizational | VP Engineering | Start recruiting immediately; consider contractors; offer competitive compensation; leverage existing team cross-training. | If hiring fails, reduce Phase 2 scope; defer non-critical distributed features to Phase 3. |
| RISK-P2-009 | **Phase 1 regressions** are introduced during Phase 2 development, breaking single-node functionality. | 10 | Correctness | Engineering Program Lead | Continuous Phase 1 test suite in CI; feature flags for all Phase 2 changes; Phase 1 benchmark regression checks in every sprint. | If regression is detected, halt Phase 2 work until regression is fixed and verified. |
| RISK-P2-010 | **Clock skew between nodes** causes lease expiry miscalculations or watermark inconsistencies. | 9 | Correctness | State Engineer + SRE Lead | NTP synchronization enforcement; monotonic clocks for internal timing; HLC for cross-node ordering; clock skew injection tests. | If clock skew is unresolvable, use logical clocks exclusively for lease management; accept wall-clock drift for timestamps only. |
| RISK-P2-011 | **Raft log compaction** fails to bound metadata growth, causing unbounded disk usage. | 8 | Reliability | Raft Engineer | Implement log compaction early; monitor Raft log size; set disk usage alerts; test compaction under load. | If compaction is buggy, increase disk allocation; schedule manual log truncation during maintenance windows. |

### 4.3 Medium Risks (Score 4–7)

| Risk ID | Risk Description | Score | Category | Owner | Mitigation | Contingency |
|---|---|---:|---|---|---|---|
| RISK-P2-012 | **gRPC transport overhead** adds unexpected latency to Raft replication. | 7 | Performance | Raft Engineer | Benchmark gRPC vs. raw TCP; optimize serialization; use connection pooling. | If gRPC is too slow, switch to raw TCP with custom binary protocol. |
| RISK-P2-013 | **S3 multipart upload failures** leave orphaned parts consuming storage. | 6 | Reliability | Cloud Engineer | Implement multipart abort lifecycle policy; track upload state; clean up orphaned parts. | If orphaned parts accumulate, run periodic cleanup job; alert on storage growth. |
| RISK-P2-014 | **Cross-domain coordination overhead** between Raft, State, and Cloud engineers slows delivery. | 6 | Organizational | Engineering Program Lead | Weekly distributed systems sync; clear ownership boundaries; shared design documents. | If coordination is too slow, reduce parallelism; sequence work packages more strictly. |
| RISK-P2-015 | **TLA+ model state space explosion** prevents exhaustive verification of distributed models. | 6 | Correctness | Formal Methods Lead | Decompose models; use symmetry reduction; use Apalache symbolic model checking. | If state space is intractable, use bounded model checking with increased bounds; accept residual risk. |
| RISK-P2-016 | **Chaos test tooling immaturity** causes unreliable fault injection. | 5 | Operational | QA Engineer | Use proven tools (iptables, kill, dm-delay); validate injection before each test; log all injections. | If tooling is unreliable, fall back to manual fault injection with scripted verification. |
| RISK-P2-017 | **NVMe thermal throttling** during sustained benchmarks degrades performance results. | 5 | Performance | SRE Lead | Monitor drive temperature; ensure adequate cooling; schedule benchmarks during cool periods. | If throttling is persistent, adjust benchmark targets; document thermal constraints. |

### 4.4 Low Risks (Score 1–3)

| Risk ID | Risk Description | Score | Category | Owner | Mitigation |
|---|---|---:|---|---|---|
| RISK-P2-018 | Documentation drift between architecture docs and implementation. | 3 | Operational | Chief Architect | PR traceability requirement; ADR updates for design changes. |
| RISK-P2-019 | Cloud provider API changes break S3 integration. | 3 | External | Cloud Engineer | Use stable S3 SDK versions; monitor provider changelogs. |
| RISK-P2-020 | Benchmark hardware procurement delays. | 2 | Operational | SRE Lead | Order hardware early; use cloud instances as fallback. |

---

## 5. Distributed Systems Specific Risk Categories

Phase 2 introduces risk categories that did not exist in Phase 1. These require specialized attention.

### 5.1 Consensus Safety Risks

| Risk | Description | Detection Method |
|---|---|---|
| Log divergence | Two replicas have different log entries at the same index | Log matching invariant checker |
| Commit safety violation | A committed entry is lost after leader change | Commit safety invariant checker |
| Leader duplication | Two nodes believe they are leader for the same term | Leader uniqueness invariant checker |
| Stale leader writes | Old leader accepts writes after losing leadership | Epoch/term validation |

### 5.2 State Replication Risks

| Risk | Description | Detection Method |
|---|---|---|
| Snapshot divergence | Replicated snapshot differs from source | Snapshot fidelity invariant checker |
| Delta loss | Lease delta lost during replication | Delta completeness invariant checker |
| Replay non-determinism | Replay produces different state than original | Replay determinism invariant checker |
| Watermark regression | Committed watermark decreases after failover | Watermark monotonicity invariant checker |

### 5.3 Split-Brain Risks

| Risk | Description | Detection Method |
|---|---|---|
| Double lease | Two nodes grant lease for same offset | No-double-lease invariant checker |
| Conflicting ACKs | Two nodes ACK the same offset differently | State consistency checker |
| Watermark fork | Two nodes have different watermarks for same group | Cross-node watermark comparison |
| Orphaned writes | Writes accepted by minority partition are lost | Partition heal reconciliation |

### 5.4 Recovery Risks

| Risk | Description | Detection Method |
|---|---|---|
| Incomplete recovery | Node recovers but misses some state | State comparison with healthy peers |
| Recovery during recovery | Node crashes during recovery, causing idempotency issues | Recovery state machine validation |
| Stale manifest recovery | Node recovers from outdated S3 manifest | Manifest version validation |
| WAL delta gap | Peer WAL delta is incomplete | WAL sequence continuity check |

---

## 6. Go/No-Go Gate Framework

### 6.1 Phase 2 Gate Structure

Phase 2 has three gates:

| Gate | Timing | Purpose |
|---|---|---|
| Gate 2A: Prototype Evidence Gate | End of Week 12 | Validate distributed consensus prototype |
| Gate 2B: Mid-Phase Review | End of Week 24 | Validate hardening progress |
| Gate 2C: Phase 2 Certification Gate | End of Week 36 | Certify Phase 2 completion |

### 6.2 Gate 2A: Prototype Evidence Gate (Week 12)

**Go Criteria:**

| ID | Criterion | Mandatory |
|---|---|---|
| G2A-001 | 3-node cluster forms Raft quorum and elects leader | Yes |
| G2A-002 | WAL segment heads replicate synchronously across all 3 nodes | Yes |
| G2A-003 | Producer ACK issued only after quorum commit | Yes |
| G2A-004 | Coordinator failover completes in <3.5 seconds | Yes |
| G2A-005 | No double lease observed in partition scenarios | Yes |
| G2A-006 | Bitmap snapshots replicate consistently | Yes |
| G2A-007 | Zero data loss in kill -9 chaos tests | Yes |
| G2A-008 | TLA+ models pass with zero unresolved counterexamples | Yes |
| G2A-009 | Evidence package complete | Yes |

**Gate 2A Outcomes:**

| Outcome | Criteria | Next Action |
|---|---|---|
| GO | All mandatory criteria pass | Continue to Phase 2 hardening |
| CONDITIONAL GO | 1–2 criteria fail with remediation plan | Continue after remediation (max 4 weeks) |
| PIVOT | Core distributed assumption fails | Architecture Review Board reviews pivot options |
| STOP | Multiple critical criteria fail | Project pauses for re-evaluation |

### 6.3 Gate 2B: Mid-Phase Review (Week 24)

**Go Criteria:**

| ID | Criterion | Mandatory |
|---|---|---|
| G2B-001 | All Phase 1 benchmarks continue to pass (no regression) | Yes |
| G2B-002 | Multi-node write throughput ≥100 MB/s sustained | Yes |
| G2B-003 | Write latency with quorum p99 ≤3 ms | Yes |
| G2B-004 | S3 streaming operational with WAF ≤1.35 | Yes |
| G2B-005 | Node replacement completes in <5 seconds | Yes |
| G2B-006 | All chaos tests pass with zero invariant violations | Yes |
| G2B-007 | 72-hour soak test shows no unbounded growth | Yes |
| G2B-008 | Risk register updated; no unresolved Critical risks | Yes |

**Gate 2B Outcomes:**

| Outcome | Criteria | Next Action |
|---|---|---|
| ON TRACK | All criteria pass | Continue to Phase 2 certification |
| AT RISK | 1–2 criteria fail with remediation plan | Extend Phase 2 by 4–8 weeks |
| OFF TRACK | Multiple criteria fail | Architecture Review Board reviews scope reduction |

### 6.4 Gate 2C: Phase 2 Certification Gate (Week 36)

**Go Criteria:**

| ID | Criterion | Mandatory |
|---|---|---|
| G2C-001 | All functional acceptance criteria met (KEI-ENG-200 §12.1) | Yes |
| G2C-002 | All performance acceptance criteria met (KEI-ENG-200 §12.2) | Yes |
| G2C-003 | All reliability acceptance criteria met (KEI-ENG-200 §12.3) | Yes |
| G2C-004 | All operational acceptance criteria met (KEI-ENG-200 §12.4) | Yes |
| G2C-005 | All chaos tests pass (12/12 scenarios) | Yes |
| G2C-006 | Zero data loss across all chaos tests | Yes |
| G2C-007 | Zero double lease across all partition scenarios | Yes |
| G2C-008 | Zero state invariant violations | Yes |
| G2C-009 | TLA+ models pass with zero unresolved counterexamples | Yes |
| G2C-010 | Evidence package complete and reviewed | Yes |
| G2C-011 | Runbooks tested and validated | Yes |
| G2C-012 | Architecture Review Board approval | Yes |
| G2C-013 | No unresolved Critical or High risks | Yes |

**Gate 2C Outcomes:**

| Outcome | Criteria | Next Action |
|---|---|---|
| PHASE 2 CERTIFIED | All criteria pass | Proceed to Phase 3 (Ecosystem Gateways & Lakehouse) |
| CONDITIONALLY CERTIFIED | 1–2 criteria fail with remediation plan | Proceed after remediation (max 4 weeks) |
| EXTENDED | Multiple criteria fail | Extend Phase 2 by 4–8 weeks |
| RE-SCOPE | Core assumptions invalidated | Architecture Review Board re-scopes Phase 2 |
| STOP | Fundamental distributed design flaw | Project pauses for architecture re-evaluation |

---

## 7. Contingency & Pivot Strategies

### 7.1 Pre-Authorized Pivots

If a core Phase 2 assumption fails, the following pivots are pre-authorized by the Architecture Review Board:

| Failing Assumption | Trigger Condition | Authorized Pivot |
|---|---|---|
| Raft library is fundamentally flawed | Both openraft and raft-rs fail integration tests | Implement minimal custom Raft with TLA+ verification; accept 8-week delay |
| Epoch fencing cannot prevent double lease | TLA+ model finds unresolvable counterexample | Add distributed lock service (etcd/ZooKeeper) for lease validation; accept latency penalty |
| State replication is inconsistent | Snapshot + delta replay produces divergent state | Fall back to full-state transfer on every failover; accept slower recovery |
| Coordinator failover cannot meet 3.5s | Failover consistently exceeds 5 seconds after optimization | Increase lease TTL defaults; document SLA impact; defer fast failover to Phase 3 |
| S3 streaming cannot maintain WAF ≤1.35 | WAF consistently exceeds 1.5 after optimization | Reduce compaction frequency; increase chunk size; accept higher WAF |
| Multi-node test environment is unreliable | >20% of test runs produce environment-related failures | Fall back to single-node simulation with network fault injection |

### 7.2 Scope Reduction Options

If Phase 2 is at risk of exceeding timeline, the following scope reductions are available:

| Reduction | Impact | Timeline Saved |
|---|---|---|
| Defer Raft log compaction to Phase 3 | Increased disk usage; manual maintenance required | 2–3 weeks |
| Defer S3 elastic backlog to Phase 3 | Reduced S3 outage tolerance | 2 weeks |
| Defer graceful leader transfer to Phase 3 | Brief write interruption during planned maintenance | 1 week |
| Reduce chaos test scenarios from 12 to 6 | Reduced failure coverage | 2 weeks |
| Reduce soak test from 72 hours to 24 hours | Reduced long-term stability evidence | 1 week |

**Normative rule:** Scope reductions MUST be approved by the Architecture Review Board and MUST NOT reduce correctness guarantees.

---

## 8. Risk Review Cadence

### 8.1 Review Schedule

| Review | Frequency | Participants | Purpose |
|---|---|---|---|
| Daily Risk Check | Daily (during standup) | Engineering Program Lead | Check for new Critical risks |
| Weekly Risk Review | Weekly (30 min) | Program Lead + Domain Leads | Review risk register; update mitigations |
| Biweekly Risk Deep Dive | Biweekly (60 min) | ARB + Program Lead | Review High/Critical risks; approve pivots |
| Milestone Risk Assessment | Per milestone | Full ARB | Assess risk posture for gate decision |
| Phase Gate Risk Certification | Per gate | Full ARB + VP Engineering | Certify risk posture for phase transition |

### 8.2 Risk Escalation Path

```text
Engineer identifies risk
   ↓ (record in risk register)
Domain Lead assesses severity
   ↓ (score ≥ 8)
Engineering Program Lead reviews
   ↓ (score ≥ 15)
Chief Architect / ARB reviews
   ↓ (pivot or scope change required)
VP Engineering / CTO decides
```

### 8.3 Risk Register Maintenance

The risk register MUST be maintained in the project issue tracker with the following fields:

```text
Issue Type: Risk
Risk ID: RISK-P2-XXX
Title: [Short Description]
Score: [Likelihood 1-5] x [Impact 1-5] = [Total]
Severity: [Critical / High / Medium / Low]
Category: [Correctness / Performance / Reliability / Operational / Organizational / External]
Owner: [Engineer Name]
Mitigation Strategy: [What we are doing to prevent it]
Contingency Plan: [What we do if it happens anyway]
Status: [Open / Mitigating / Realized / Closed]
Last Reviewed: [Date]
Next Review: [Date]
```

---

## 9. Phase 2 Risk Metrics Dashboard

The following metrics MUST be tracked continuously and reported at every risk review:

| Metric | Source | Alert Threshold |
|---|---|---|
| Open Critical risks | Risk register | > 0 |
| Open High risks | Risk register | > 3 |
| Chaos test pass rate | KEI-BENCH-201 | < 100% |
| Invariant violation count | Runtime invariant checker | > 0 |
| Data loss events | Chaos test reports | > 0 |
| Double lease events | Partition test reports | > 0 |
| Failover time (p99) | KEI-BENCH-201 | > 3.5 seconds |
| Node replacement time (p99) | KEI-BENCH-201 | > 5 seconds |
| WAF | S3 streaming metrics | > 1.35 |
| Phase 1 regression count | CI pipeline | > 0 |
| Unresolved TLA+ counterexamples | KEI-FORMAL-201 | > 0 |
| Risk register staleness | Risk register | Any risk not reviewed in > 2 weeks |

---

## 10. Phase 2 Risk Summary

### 10.1 Risk Count by Severity

| Severity | Count | Status |
|---|---:|---|
| Critical (15–25) | 4 | Active mitigation required |
| High (8–14) | 7 | Active mitigation plans required |
| Medium (4–7) | 6 | Monitor and prepare contingency |
| Low (1–3) | 3 | Accept and monitor |
| **Total** | **20** | |

### 10.2 Top 5 Risks Requiring Immediate Attention

| Priority | Risk ID | Risk | Score | Immediate Action |
|---:|---|---|---:|---|
| 1 | RISK-P2-001 | Raft consensus correctness bug | 20 | Select and validate Raft library in Week 1 |
| 2 | RISK-P2-002 | Split-brain double lease | 20 | Implement epoch fencing; run TLA+ Model 10 |
| 3 | RISK-P2-003 | State replication inconsistency | 16 | Implement snapshot + delta protocol; run TLA+ Model 9 |
| 4 | RISK-P2-004 | Coordinator failover exceeds target | 15 | Benchmark failover timing early; optimize restoration path |
| 5 | RISK-P2-005 | Raft library integration complexity | 12 | Evaluate libraries in Week 1; build integration spike |

---

## 11. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial Phase 2 Risk Reduction & Go/No-Go Plan. Defines risk scoring methodology, 20-item risk register, distributed systems risk categories, three-gate Go/No-Go framework, contingency and pivot strategies, risk review cadence, and risk metrics dashboard. |

---

## Phase 2 Planning Suite — Completion Summary

With the delivery of **KEI-RISK-201**, the complete Phase 2 planning suite is now finished.

| # | Document ID | Title | Status |
|---|---|---|---|
| 1 | KEI-ENG-200 | Phase 2 Engineering Execution Plan | ✅ Delivered |
| 2 | KEI-SPIKE-201 | Distributed Consensus & Coordinator Sharding Prototype Plan | ✅ Delivered |
| 3 | KEI-FORMAL-201 | Distributed Consensus & Multi-Node State Verification Plan | ✅ Delivered |
| 4 | KEI-BENCH-201 | Multi-Node Performance, Failover & Recovery Harness Plan | ✅ Delivered |
| 6 | KEI-RISK-201 | Phase 2 Risk Reduction & Go/No-Go Plan | ✅ Delivered |