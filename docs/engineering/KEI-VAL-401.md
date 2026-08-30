# KEI-VAL-401 — Jepsen-Style Consistency Certification Plan

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-VAL-401 |
| Title | Jepsen-Style Consistency Certification Plan |
| Version | 1.0 |
| Level | Engineering Execution Plan |
| Status | Baseline — Ready for Execution |
| Phase | Phase 4 — Enterprise Hardening, Compliance & Multi-Region |
| Duration | Weeks 20–34 of Phase 4 |
| Owner | Reliability Engineering Lead / Chaos Engineering Lead |
| Governing Plan | KEI-ENG-400 — Phase 4 Engineering Execution Plan |
| Governing Architecture Documents | KEI-ARC-022, KEI-ARC-026, KEI-DES-031, KEI-OPS-041 |
| Predecessor | KEI-QUEUE-401 (SQS/AMQP Certification) |

---

## 2. Executive Summary

Phases 1 through 3 proved that Keirox works correctly under normal conditions. Phase 4 must prove that Keirox remains **correct under adversarial conditions** — network partitions, clock skew, process crashes, disk stalls, and split-brain scenarios.

This plan defines the **Jepsen-style adversarial consistency certification program**. It is inspired by the methodology of Jepsen.io but tailored specifically to the Keirox architecture. The goal is to empirically prove that:

1. **No committed data is ever lost** (JML = 0).
2. **No double lease is ever granted** under any partition scenario.
3. **Watermarks never regress** across node failures.
4. **State replication remains consistent** after recovery.
5. **Split-brain writes are fenced** and never accepted by the surviving topology.

This certification is the final technical proof that Keirox is safe for enterprise production workloads.

---

## 3. Purpose and Scope

### 3.1 Purpose

The purpose of this plan is to:

1. Define the adversarial fault injection methodology.
2. Define the invariant checker that validates correctness after every fault scenario.
3. Define the test matrix covering partitions, kills, stalls, skew, and split-brain.
4. Define linearizability verification for committed writes.
5. Define the evidence requirements for Phase 4 certification.

### 3.2 Scope

**In scope:**

- Network partition injection (1 vs 2, 1 vs 1 vs 1, asymmetric).
- Process kill injection (`kill -9`, `SIGSTOP`, OOM).
- Disk stall injection (NVMe latency injection).
- Clock skew injection (±5s, ±30s, monotonic drift).
- Split-brain fencing validation.
- State replication consistency verification.
- Watermark monotonicity verification.
- Lease uniqueness verification.
- Recovery determinism verification.
- Multi-node Raft consensus stress testing.

**Out of scope:**

- Performance benchmarking (owned by KEI-BENCH-101).
- Gateway protocol conformance (owned by KEI-COMPAT-301 and KEI-QUEUE-401).
- Security penetration testing (owned by KEI-SEC-401).
- Multi-region DR drills (owned by KEI-MR-401).

---

## 4. Adversarial Testing Methodology

### 4.1 Philosophy

Jepsen-style testing follows a simple loop:

```text
1. Start cluster in known state
2. Run concurrent workload (producers, consumers, workers)
3. Inject fault (partition, kill, stall, skew)
4. Allow system to respond and recover
5. Heal fault (restore network, restart process)
6. Run invariant checker
7. Assert: zero violations
8. Repeat with different fault combinations
```

### 4.2 Key Principles

| ID | Principle | Requirement |
|---|---|---|
| JEP-1 | Determinism over luck | Tests MUST be reproducible. Random seeds MUST be logged. |
| JEP-2 | Invariants are non-negotiable | Any invariant violation is a Critical defect. |
| JEP-3 | Faults are combined | Single faults are necessary but insufficient. Multi-fault scenarios MUST be tested. |
| JEP-4 | Recovery is part of the test | Healing the fault and verifying recovery is mandatory. |
| JEP-5 | Evidence is archived | Every test run MUST produce a machine-readable report. |

---

## 5. Invariant Checker

The invariant checker is the core of the certification. It runs after every fault scenario and validates the following invariants.

### 5.1 Durability Invariants

| ID | Invariant | Formal Statement |
|---|---|---|
| INV-DUR-001 | No committed data loss | Every record ACKed by a producer MUST be readable after recovery. |
| INV-DUR-002 | No uncommitted data visible | Records not yet ACKed MUST NOT be visible to consumers. |
| INV-DUR-003 | WAL integrity | All WAL segments MUST pass CRC32C validation after recovery. |

### 5.2 State Plane Invariants

| ID | Invariant | Formal Statement |
|---|---|---|
| INV-STATE-001 | No double lease | At most one active lease per offset across all nodes. |
| INV-STATE-002 | No terminal regression | ACKED or DLQ offsets MUST NEVER return to READY or LEASED. |
| INV-STATE-003 | Watermark monotonicity | `W_base` MUST NEVER decrease. |
| INV-STATE-004 | Terminal below watermark | All offsets below `W_base` MUST be in terminal state. |
| INV-STATE-005 | Lease token uniqueness | No two active leases share the same lease token. |

### 5.3 Consensus Invariants

| ID | Invariant | Formal Statement |
|---|---|---|
| INV-CONS-001 | Leader uniqueness | At most one Raft leader per term. |
| INV-CONS-002 | Log matching | If two logs contain an entry at the same index and term, all preceding entries match. |
| INV-CONS-003 | Commit safety | A committed entry is never lost after leader change. |
| INV-CONS-004 | Epoch fencing | Operations with stale epochs MUST be rejected. |

### 5.4 Split-Brain Invariants

| ID | Invariant | Formal Statement |
|---|---|---|
| INV-SB-001 | Majority authority | Only the majority partition can issue leases. |
| INV-SB-002 | Minority fencing | Minority partition operations MUST be rejected after healing. |
| INV-SB-003 | No conflicting writes | No two nodes accept conflicting writes for the same offset. |
| INV-SB-004 | Healing safety | After partition heals, state converges to a single consistent view. |

---

## 6. Fault Injection Matrix

### 6.1 Network Partitions

| Test ID | Scenario | Injection Method | Expected Behavior |
|---|---|---|---|
| NET-001 | Single node isolated (1 vs 2) | `iptables` DROP | Majority continues; minority fenced |
| NET-002 | Symmetric partition (1 vs 1 vs 1) | `iptables` DROP all | No quorum; writes pause; no corruption |
| NET-003 | Asymmetric partition (A sees B, B cannot see A) | `iptables` one-way DROP | Split-brain detection; minority fenced |
| NET-004 | Intermittent packet loss (10%) | `tc netem loss 10%` | Retries succeed; no data loss |
| NET-005 | High latency injection (500ms) | `tc netem delay 500ms` | Timeouts handled; no corruption |
| NET-006 | Network heal after partition | Remove `iptables` rules | State converges; orphaned writes quarantined |

### 6.2 Process Kills

| Test ID | Scenario | Injection Method | Expected Behavior |
|---|---|---|---|
| KILL-001 | Kill Raft leader | `kill -9 <leader_pid>` | New leader elected; zero data loss |
| KILL-002 | Kill Raft follower | `kill -9 <follower_pid>` | Cluster continues; node replaces |
| KILL-003 | Kill coordinator node | `kill -9 <coordinator_pid>` | Failover <3.5s; no double lease |
| KILL-004 | Kill all followers simultaneously | `kill -9` both followers | Leader continues; recovery on restart |
| KILL-005 | Kill leader during write | `kill -9` mid-append | Committed writes survive; uncommitted rejected |
| KILL-006 | Kill during recovery | `kill -9` during WAL replay | Idempotent recovery; no corruption |

### 6.3 Disk Stalls

| Test ID | Scenario | Injection Method | Expected Behavior |
|---|---|---|---|
| DISK-001 | NVMe stall on leader (5s) | `dm-delay` or `cgroup io` | Leader steps down or request times out |
| DISK-002 | NVMe stall on follower | `dm-delay` | Follower catches up after stall |
| DISK-003 | Disk full simulation | `fallocate` to fill disk | Backpressure engages; no corruption |

### 6.4 Clock Skew

| Test ID | Scenario | Injection Method | Expected Behavior |
|---|---|---|---|
| SKEW-001 | +5s clock skew on leader | `libfaketime` or NTP manipulation | Lease expiry safe; HLC order preserved |
| SKEW-002 | -5s clock skew on follower | `libfaketime` | Replication continues; no corruption |
| SKEW-003 | ±30s skew across cluster | `libfaketime` | Watermark safe; no double lease |
| SKEW-004 | Monotonic clock drift | `libfaketime` with drift | Timers safe; no premature expiry |

### 6.5 Combined Fault Scenarios

| Test ID | Scenario | Injection Method | Expected Behavior |
|---|---|---|---|
| COMBO-001 | Kill leader + network partition | `kill -9` + `iptables` | Majority elects new leader; minority fenced |
| COMBO-002 | Kill coordinator + disk stall | `kill -9` + `dm-delay` | Failover succeeds; no double lease |
| COMBO-003 | Network partition + clock skew | `iptables` + `libfaketime` | Split-brain fenced; watermark safe |
| COMBO-004 | Kill leader during partition heal | `kill -9` during heal | State converges; no corruption |
| COMBO-005 | Triple fault: kill + partition + skew | All three | System degrades safely; invariants hold |

---

## 7. Workload Generator

### 7.1 Concurrent Workload

During every fault injection test, the following workload MUST be running:

| Component | Operation | Rate |
|---|---|---|
| Producer A | Append records to Stream 1 | 1,000 msgs/s |
| Producer B | Append records to Stream 2 | 1,000 msgs/s |
| Stream Consumer | Fetch from Stream 1 | Continuous |
| Queue Worker 1 | Lease/ACK from Stream 2 | Continuous |
| Queue Worker 2 | Lease/NACK from Stream 2 | Continuous |
| DLQ Operator | List DLQ entries | Periodic |

### 7.2 Workload Validation

After each fault scenario:

1. Count total records produced (from producer logs).
2. Count total records readable (from stream reads).
3. Verify: `readable >= committed` (no loss).
4. Verify: `no duplicate leases` (from lease table).
5. Verify: `watermark monotonic` (from watermark history).

---

## 8. Test Execution Framework

### 8.1 Framework Architecture

```text
┌────────────────────────────────────────────────────────────┐
│                  JEPSEN-STYLE TEST RUNNER                   │
│                                                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │ Fault        │  │ Workload     │  │ Invariant    │    │
│  │ Injector     │  │ Generator    │  │ Checker      │    │
│  └──────────────┘  └──────────────┘  └──────────────┘    │
│                                                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │ Cluster      │  │ Evidence     │  │ Report       │    │
│  │ Orchestrator │  │ Collector    │  │ Generator    │    │
│  └──────────────┘  └──────────────┘  └──────────────┘    │
└────────────────────────────────────────────────────────────┘
```

### 8.2 Execution Rules

1. Each test scenario MUST be run at least **3 times** with different random seeds.
2. Each test scenario MUST produce a machine-readable JSON report.
3. Any invariant violation MUST halt the test suite and produce a defect report.
4. Test environment MUST be isolated from production.
5. Test logs MUST be retained for audit.

---

## 9. Certification Levels

| Level | Name | Requirement |
|---|---|---|
| L1 | Partition Certified | All NET scenarios pass with zero violations |
| L2 | Kill Certified | All KILL scenarios pass with zero violations |
| L3 | Stall Certified | All DISK scenarios pass with zero violations |
| L4 | Skew Certified | All SKEW scenarios pass with zero violations |
| L5 | Combined Certified | All COMBO scenarios pass with zero violations |
| L6 | Full Jepsen Certified | All scenarios pass across 3 runs with different seeds |

Phase 4 exit requires **L1 through L6**.

---

## 10. Deliverables and Milestones

| Deliverable | Description | Target Week |
|---|---|---:|
| D-VAL-001 | Invariant checker implementation | Week 22 |
| D-VAL-002 | Fault injection framework | Week 24 |
| D-VAL-003 | Workload generator | Week 24 |
| D-VAL-004 | Network partition test suite | Week 26 |
| D-VAL-005 | Process kill test suite | Week 27 |
| D-VAL-006 | Disk stall test suite | Week 28 |
| D-VAL-007 | Clock skew test suite | Week 29 |
| D-VAL-008 | Combined fault test suite | Week 30 |
| D-VAL-009 | Full Jepsen certification run (3 seeds) | Week 32 |
| D-VAL-010 | Final consistency certification report | Week 34 |

---

## 11. Certification Gates

### 11.1 Gate VAL-A — Framework Ready (Week 24)

| Criterion | Mandatory |
|---|---|
| Invariant checker operational | Yes |
| Fault injector operational | Yes |
| Workload generator operational | Yes |
| Test runner produces JSON reports | Yes |

### 11.2 Gate VAL-B — Single Fault Certified (Week 30)

| Criterion | Mandatory |
|---|---|
| All NET scenarios pass | Yes |
| All KILL scenarios pass | Yes |
| All DISK scenarios pass | Yes |
| All SKEW scenarios pass | Yes |
| Zero invariant violations | Yes |

### 11.3 Gate VAL-C — Full Jepsen Certified (Week 34)

| Criterion | Mandatory |
|---|---|
| All COMBO scenarios pass | Yes |
| All scenarios pass across 3 seeds | Yes |
| Zero invariant violations across all runs | Yes |
| Evidence package complete | Yes |
| Certification report approved by ARB | Yes |

---

## 12. Risks and Mitigations

| Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|
| Invariant checker misses a subtle bug | High | Medium | Multiple invariant layers; formal model cross-check (KEI-FORMAL-101/201) |
| Fault injection tooling unreliable | Medium | Medium | Use proven tools (iptables, tc, dm-delay, libfaketime); validate injection before each test |
| Combined fault scenarios expose deep bug | High | Medium | Reserve remediation buffer; escalate to ARB immediately |
| Test environment differs from production | Medium | High | Use identical hardware/kernel config; document environment |
| Test execution takes too long | Medium | Medium | Parallelize independent scenarios; use containerized clusters |
| Flaky tests produce false positives | Medium | Medium | Require 3 runs per scenario; investigate all failures |

---

## 13. Evidence Package

The Jepsen-style certification evidence package MUST include:

1. Invariant checker specification and implementation.
2. Fault injection framework documentation.
3. Workload generator specification.
4. All test scenario definitions.
5. Machine-readable JSON reports for every test run.
6. Invariant violation log (expected: empty).
7. Seed values and environment configuration for reproducibility.
8. Final consistency certification report.
9. ARB approval record.

---

## 14. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial Jepsen-Style Consistency Certification Plan. Defines adversarial fault injection methodology, invariant checker, test matrix (partitions, kills, stalls, skew, combined), workload generator, execution framework, certification levels, and evidence requirements. |