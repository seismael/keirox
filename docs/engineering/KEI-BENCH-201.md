# KEI-BENCH-201 — Multi-Node Performance, Failover & Recovery Harness Plan

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-BENCH-201 |
| Title | Multi-Node Performance, Failover & Recovery Harness Plan |
| Version | 1.0 |
| Level | Engineering Execution Plan |
| Status | Baseline — Ready for Execution |
| Phase | Phase 2 Engineering Bridge (parallel track) |
| Duration | 90 days / 12 weeks |
| Owner | SRE / Performance Engineering Lead |
| Governing Plan | KEI-ENG-200 — Phase 2 Engineering Execution Plan |
| Related Plans | KEI-SPIKE-201 (Distributed Consensus Prototype), KEI-FORMAL-201 (Distributed Consensus Verification) |
| Governing Architecture Documents | KEI-ARC-011 (NFRs), KEI-ARC-020 (Storage), KEI-ARC-021 (State Plane), KEI-ARC-022 (Consensus), KEI-ARC-027 (Operability) |
| Predecessor | KEI-BENCH-101 (Phase 1 Performance Validation Harness Plan) |

---

## 2. Executive Summary

Phase 1 benchmarking (KEI-BENCH-101) proved single-node performance. Phase 2 introduces the most dangerous class of performance and reliability challenges: **distributed consensus overhead, failover latency, state replication consistency, and cloud storage streaming under cluster load**.

This plan defines the multi-node benchmark harness, failover measurement methodology, chaos test matrix, and evidence reporting framework required to prove that the 3-node cluster meets all Phase 2 acceptance criteria defined in KEI-ENG-200.

The harness must answer these questions with empirical evidence:

1. Does synchronous Raft quorum replication keep write latency within p99 ≤ 3 ms?
2. Does coordinator failover complete in < 3.5 seconds with zero double-lease?
3. Does node replacement complete in < 5 seconds with zero data loss?
4. Does S3 streaming maintain WAF ≤ 1.35 under sustained cluster load?
5. Does the system survive all chaos scenarios without invariant violations?

---

## 3. Purpose and Scope

### 3.1 Purpose

The purpose of this plan is to:

1. Define multi-node workload profiles for distributed benchmarking.
2. Specify the distributed load generation and metrics collection harness.
3. Define failover timing measurement methodology.
4. Define the chaos test matrix for distributed failure scenarios.
5. Define recovery time measurement methodology.
6. Establish S3 streaming performance validation.
7. Set mandatory pass/fail thresholds for Phase 2 evidence gates.
8. Produce the Phase 2 benchmark and chaos evidence package.

### 3.2 Scope

**In scope:**

- Multi-node load generation (3-node cluster).
- Distributed latency and throughput measurement.
- Raft replication lag measurement.
- Coordinator failover timing measurement.
- Node replacement timing measurement.
- S3 streaming throughput and WAF measurement.
- Chaos test execution and invariant checking.
- Network partition simulation.
- Clock skew injection.
- Disk stall injection.
- 72-hour multi-node soak test.
- Evidence report generation.

**Out of scope:**

- Single-node benchmarking (completed in KEI-BENCH-101).
- Multi-region replication benchmarking (Phase 4).
- Gateway protocol benchmarking (Phase 3).
- Iceberg catalog commit benchmarking (Phase 3).
- Jepsen full certification (Phase 4).

---

## 4. Relationship to KEI-BENCH-101

Phase 1 benchmarking (KEI-BENCH-101) established single-node performance baselines. This plan extends that work into the distributed domain.

| KEI-BENCH-101 Scope | KEI-BENCH-201 Extension |
|---|---|
| Single-node append throughput | Multi-node quorum write throughput |
| Single-node append latency | Multi-node quorum write latency (with replication overhead) |
| Single-node state plane performance | Distributed state replication consistency |
| Single-node recovery | Multi-node failover and node replacement |
| Local Parquet export | S3 streaming with manifest registration |
| Single-node soak | Multi-node soak with chaos injection |

**Normative rule:** All Phase 1 benchmark baselines MUST continue to pass during Phase 2 testing. Any Phase 1 regression discovered during Phase 2 MUST be treated as a critical defect.

---

## 5. Multi-Node Workload Profiles

### 5.1 Profile Definitions

| Profile ID | Name | Definition | Primary Target |
|---|---|---|---|
| **P1-P2** | Distributed Baseline Sustained | 100,000 msgs/s @ 1 KB (100 MB/s), 3-node quorum, steady state | Multi-node write latency with quorum, throughput |
| **P2-P2** | Distributed Burst | 10× P1-P2 (1,000,000 msgs/s) for 5 minutes, then drain | Backpressure under quorum, NVMe backlog |
| **P3-P2** | Distributed High Cardinality | 100,000 active streams, low per-stream throughput, 3-node | Registry scaling under quorum replication |
| **P4-P2** | Distributed Queue Churn | 100,000 concurrent active leases, high ACK/NACK churn, coordinator kill mid-test | State replication consistency, failover under load |
| **P5-P2** | Distributed Export Overhead | P1-P2 workload + continuous S3 streaming + Parquet export | S3 streaming interference on write path |
| **P6-P2** | Distributed Degraded | P1-P2 workload + network partition + S3 throttle | Split-brain safety, backpressure, recovery |

### 5.2 Profile Execution Rules

- Each profile MUST be run on a dedicated 3-node cluster.
- Each profile MUST run for a minimum of 30 minutes steady-state.
- Each profile MUST record full environmental disclosure per KEI-BENCH-101 §6.
- Profiles P4-P2 and P6-P2 MUST include chaos injection during execution.

---

## 6. Multi-Node Metrics Taxonomy

### 6.1 Distributed Latency Metrics (Histograms)

| Metric | Unit | Required Percentiles |
|---|---|---|
| `keirox_wal_append_latency_seconds` (with quorum) | seconds | p50, p90, p99, p999, max |
| `keirox_raft_commit_latency_seconds` | seconds | p50, p99 |
| `keirox_raft_replication_lag_seconds` | seconds | p50, p99 |
| `keirox_coordinator_failover_duration_seconds` | seconds | single value per failover event |
| `keirox_node_recovery_duration_seconds` | seconds | single value per recovery event |
| `keirox_lease_acquisition_latency_seconds` (distributed) | seconds | p50, p99 |
| `keirox_ack_processing_latency_seconds` (distributed) | seconds | p50, p99 |
| `keirox_s3_upload_latency_seconds` | seconds | p50, p99 |
| `keirox_s3_manifest_commit_latency_seconds` | seconds | p50, p99 |

### 6.2 Distributed Throughput Metrics (Counters/Gauges)

| Metric | Unit |
|---|---|
| `keirox_cluster_ingest_messages_per_second` | msgs/s |
| `keirox_cluster_ingest_bytes_per_second` | MB/s |
| `keirox_cluster_ack_operations_per_second` | ops/s |
| `keirox_s3_upload_bytes_per_second` | MB/s |
| `keirox_s3_upload_chunks_per_second` | chunks/s |
| `keirox_raft_log_entries_per_second` | entries/s |

### 6.3 Distributed State Metrics (Gauges)

| Metric | Unit | Purpose |
|---|---|---|
| `keirox_raft_leader_node_id` | node ID | Track leader location |
| `keirox_raft_term` | integer | Track Raft term changes |
| `keirox_raft_follower_lag_entries` | entries | Replication lag per follower |
| `keirox_coordinator_epoch` | integer | Track epoch changes |
| `keirox_coordinator_shard_count` | count | Shards per coordinator |
| `keirox_bitmap_snapshot_replication_lag_seconds` | seconds | State replication lag |
| `keirox_lease_delta_replication_lag_seconds` | seconds | Lease delta replication lag |
| `keirox_s3_backlog_bytes` | bytes | Pending S3 upload |
| `keirox_nvme_backlog_eta_seconds` | seconds | Estimated time to NVMe exhaustion |
| `keirox_write_amplification_factor` | ratio | WAF measurement |

### 6.4 Failover Event Metrics

| Metric | Unit | Purpose |
|---|---|---|
| `keirox_failover_events_total` | count | Total failover events |
| `keirox_failover_detection_latency_seconds` | seconds | Time to detect failure |
| `keirox_failover_election_latency_seconds` | seconds | Time to elect new leader |
| `keirox_failover_state_restore_latency_seconds` | seconds | Time to restore coordinator state |
| `keirox_failover_total_latency_seconds` | seconds | Total failover duration |
| `keirox_failover_data_loss_records` | count | Records lost during failover (MUST be 0) |
| `keirox_failover_double_lease_events` | count | Double lease events (MUST be 0) |

---

## 7. Benchmark Harness Architecture

### 7.1 Multi-Node Harness Topology

```text
┌────────────────────────────────────────────────────────────────────────┐
│                    BENCHMARK HARNESS (Multi-Node)                       │
│                                                                        │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                    LOAD GENERATOR CLUSTER                         │  │
│  │                                                                   │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐           │  │
│  │  │ Producer     │  │ Stream       │  │ Queue        │           │  │
│  │  │ Load Gen     │  │ Reader       │  │ Worker       │           │  │
│  │  │ (async)      │  │ Load Gen     │  │ Load Gen     │           │  │
│  │  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘           │  │
│  │         │                 │                 │                    │  │
│  │         └─────────────────┼─────────────────┘                    │  │
│  │                           │                                      │  │
│  │                           ▼                                      │  │
│  │  ┌──────────────────────────────────────────────────────────┐   │  │
│  │  │           COORDINATED CLOCK & METRICS AGGREGATOR          │   │  │
│  │  │  - NTP-synchronized timestamps across all generators      │   │  │
│  │  │  - HDR Histogram aggregation                              │   │  │
│  │  │  - Failover event correlation                             │   │  │
│  │  └──────────────────────────────────────────────────────────┘   │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                                                        │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                    CHAOS INJECTOR                                 │  │
│  │                                                                   │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐           │  │
│  │  │ Process      │  │ Network      │  │ Disk /       │           │  │
│  │  │ Killer       │  │ Partitioner  │  │ Clock Skew   │           │  │
│  │  │ (kill -9)    │  │ (iptables)   │  │ (libfaketime)│           │  │
│  │  └──────────────┘  └──────────────┘  └──────────────┘           │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                                                        │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                    EVIDENCE COLLECTOR                             │  │
│  │                                                                   │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐           │  │
│  │  │ Prometheus   │  │ Invariant    │  │ Report       │           │  │
│  │  │ Scraper      │  │ Checker      │  │ Generator    │           │  │
│  │  └──────────────┘  └──────────────┘  └──────────────┘           │  │
│  └──────────────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────────────┘
                             │
                             ▼ (Target)
              ┌──────────────────────────────────┐
              │        3-NODE CLUSTER             │
              │                                   │
              │  Node 1 ◄──── Raft ────► Node 2  │
              │     ▲                       ▲     │
              │     └────── Raft ──────────┘     │
              │              │                    │
              │              ▼                    │
              │         S3 / GCS                  │
              └──────────────────────────────────┘
```

### 7.2 Load Generator Design

- **Producer Load Gen:** Async Rust clients using `tokio`. Rate-limited to exact target MB/s or running in "max throughput" mode. Distributes writes across all 3 nodes.
- **Stream Reader Load Gen:** Sequential offset polling across multiple consumer groups.
- **Queue Worker Load Gen:** Concurrent lease acquisition, simulated processing delay (1ms–10ms), followed by ACK/NACK. Configurable failure rate for lease timeout testing.
- **Coordination:** All load generators MUST synchronize their start times using NTP and use a shared atomic clock to ensure latency measurements are accurate across nodes.

### 7.3 Chaos Injector Design

| Injector | Mechanism | Purpose |
|---|---|---|
| Process Killer | `kill -9 <pid>` | Simulate sudden node crash |
| Network Partitioner | `iptables` rules | Simulate network partition (1 vs 2, 1 vs 1 vs 1) |
| Disk Stall Injector | `dm-delay` or `tc` | Simulate slow disk I/O |
| Clock Skew Injector | `libfaketime` or NTP manipulation | Simulate clock drift |
| S3 Throttle Injector | Local S3 proxy with artificial 503 responses | Simulate S3 throttling |
| Bandwidth Limiter | `tc` (traffic control) | Simulate degraded network |

---

## 8. Failover Timing Measurement Methodology

### 8.1 Coordinator Failover Measurement

Coordinator failover timing is measured in four phases:

```text
T0: Failure injection (kill -9 coordinator node)
T1: Failure detection (heartbeat timeout detected by cluster)
T2: Successor assignment (new coordinator assigned via Metadata Raft)
T3: State restoration (bitmap snapshot + lease deltas replayed)
T4: Lease resumption (first successful lease grant by successor)

Failover Duration = T4 - T0
Detection Latency = T1 - T0
Assignment Latency = T2 - T1
Restoration Latency = T3 - T2
Resumption Latency = T4 - T3
```

**Measurement rules:**

- Timestamps MUST be captured using monotonic clocks on each node.
- Cross-node timestamps MUST be NTP-synchronized.
- Failover duration MUST be < 3.5 seconds (mandatory target).
- Each failover event MUST be recorded with full phase breakdown.
- A minimum of 10 failover events MUST be measured per benchmark run.

### 8.2 Node Replacement Measurement

Node replacement timing is measured in five phases:

```text
T0: Failure injection (kill -9 storage node)
T1: Failure detection (heartbeat timeout)
T2: Replacement node provisioning (join cluster membership)
T3: State reconstruction (S3 manifests + peer WAL delta replay)
T4: Catch-up replication complete (follower caught up to leader)
T5: Node returned to active service

Replacement Duration = T5 - T0
```

**Measurement rules:**

- Replacement duration MUST be < 5 seconds (mandatory target).
- Each replacement event MUST be recorded with full phase breakdown.
- A minimum of 5 replacement events MUST be measured per benchmark run.

### 8.3 Raft Leader Election Measurement

```text
T0: Leader failure (kill -9)
T1: Follower detects leader failure (election timeout)
T2: Candidate requests votes
T3: New leader elected (majority votes received)
T4: New leader begins accepting writes

Election Duration = T4 - T0
```

**Measurement rules:**

- Election duration MUST be < 2 seconds (mandatory target).
- Each election event MUST be recorded.
- A minimum of 10 election events MUST be measured per benchmark run.

---

## 9. Chaos Test Matrix

### 9.1 Mandatory Chaos Tests

| Test ID | Scenario | Injection Method | Expected Behavior | Duration |
|---|---|---|---|---|
| CHAOS-P2-001 | Kill -9 Raft leader | `kill -9` | New leader elected; zero data loss | 30 min |
| CHAOS-P2-002 | Kill -9 Raft follower | `kill -9` | Cluster continues; node replaces | 30 min |
| CHAOS-P2-003 | Kill -9 coordinator node | `kill -9` | Failover < 3.5s; no double lease | 30 min |
| CHAOS-P2-004 | Network partition (1 vs 2) | `iptables` | Majority continues; minority fenced | 30 min |
| CHAOS-P2-005 | Network partition (coordinator isolated) | `iptables` | Epoch fencing; no double lease | 30 min |
| CHAOS-P2-006 | Disk stall on leader | `dm-delay` | Leader steps down or request times out | 15 min |
| CHAOS-P2-007 | S3 outage during upload | S3 proxy 503 | Elastic backlog; backpressure engages | 30 min |
| CHAOS-P2-008 | Clock skew injection (±5s) | `libfaketime` | Lease expiry safe; HLC order preserved | 15 min |
| CHAOS-P2-009 | Simultaneous leader + coordinator kill | `kill -9` × 2 | System degrades safely; no corruption | 30 min |
| CHAOS-P2-010 | Recovery during recovery | `kill -9` during recovery | Idempotent recovery; no state corruption | 15 min |
| CHAOS-P2-011 | Split-brain heal after partition | Remove `iptables` | Orphaned writes quarantined | 15 min |
| CHAOS-P2-012 | S3 throttle during burst | S3 proxy 503 + burst load | Backoff with jitter; no data loss | 30 min |

### 9.2 Chaos Test Invariant Checks

During every chaos test, the following invariants MUST be continuously checked:

| Invariant | Check |
|---|---|
| No data loss | All producer-ACKed records present after recovery |
| No double lease | At most one active lease per offset across all nodes |
| No watermark regression | `W_base` never decreases across any node |
| No terminal regression | ACKED/DLQ offsets never return to READY/LEASED |
| Epoch fencing | Stale epoch operations rejected |
| Raft commit safety | Committed entries never lost |
| State consistency | Replicated state matches source after recovery |

### 9.3 Chaos Test Execution Rules

- Each chaos test MUST be run at least 3 times.
- Each chaos test MUST record full timing measurements.
- Each chaos test MUST produce an invariant checker report.
- Any invariant violation MUST halt the test and produce a defect report.
- Chaos tests MUST NOT be run in parallel (to isolate failure effects).

---

## 10. S3 Streaming Performance Validation

### 10.1 S3 Streaming Metrics

| Metric | Measurement Method |
|---|---|
| Upload throughput (MB/s) | Measure bytes uploaded / time |
| Upload latency (p50/p99) | Time from chunk seal to S3 confirmation |
| Manifest commit latency | Time from upload to manifest registration |
| Chunk sealing rate | Chunks sealed per second |
| Backlog size | Bytes pending upload |
| WAF | Total bytes written / total bytes ingested |

### 10.2 WAF Measurement Methodology

Write Amplification Factor is measured as:

```text
WAF = (WAL bytes written + S3 bytes uploaded + Manifest bytes written) / (Producer bytes ingested)
```

**Target:** WAF ≤ 1.35 over 72-hour soak test.

**Measurement rules:**

- WAF MUST be measured over a minimum 72-hour continuous soak.
- WAF MUST be measured under P1-P2 baseline workload.
- WAF MUST be measured with S3 streaming active.
- WAF MUST be reported with confidence intervals.

### 10.3 S3 Throttling Validation

| Test | Method | Expected Behavior |
|---|---|---|
| Normal operation | Standard S3 endpoint | Upload throughput ≥ 50 MB/s |
| Throttled operation | S3 proxy returning 503 for 30% of requests | Backoff with jitter; no data loss |
| Sustained outage | S3 proxy returning 503 for 100% of requests | Elastic backlog; backpressure engages |
| Recovery after outage | Remove S3 proxy | Backlog drains; normal throughput resumes |

---

## 11. 72-Hour Multi-Node Soak Test

### 11.1 Soak Test Configuration

| Parameter | Value |
|---|---|
| Duration | 72 hours continuous |
| Workload | P1-P2 baseline (100 MB/s sustained) |
| Cluster | 3-node Raft quorum |
| S3 streaming | Active |
| Chaos injection | Periodic (every 12 hours) |
| Metrics collection | Continuous |

### 11.2 Soak Test Chaos Schedule

| Time | Chaos Event |
|---|---|
| Hour 12 | Kill -9 Raft follower (CHAOS-P2-002) |
| Hour 24 | Kill -9 coordinator node (CHAOS-P2-003) |
| Hour 36 | Network partition 1 vs 2 (CHAOS-P2-004) |
| Hour 48 | Kill -9 Raft leader (CHAOS-P2-001) |
| Hour 60 | S3 throttle for 30 minutes (CHAOS-P2-012) |

### 11.3 Soak Test Pass Criteria

| Criterion | Target |
|---|---|
| No unbounded memory growth | RSS stable after warmup |
| No file descriptor leaks | FD count stable |
| No Raft log growth without compaction | Log size bounded |
| No bitmap memory growth | Bitmap bytes stable |
| No S3 backlog growth | Backlog drains within 1 hour |
| No invariant violations | Zero violations |
| No data loss | Zero records lost |
| Latency drift | p99 drift ≤ 10% over 72 hours |
| WAF | ≤ 1.35 |

---

## 12. Evidence Package

The Phase 2 benchmark evidence package MUST include:

| Artifact | Description |
|---|---|
| Multi-node benchmark report | Throughput, latency, replication lag for all profiles |
| Failover timing report | Coordinator failover, node replacement, leader election timings |
| S3 streaming report | Upload throughput, WAF, throttling behavior |
| Chaos test report | All 12 chaos scenarios with invariant checks |
| Soak test report | 72-hour soak with periodic chaos injection |
| Invariant checker report | Zero violations confirmed |
| Environmental disclosure | Hardware, OS, kernel, network topology |
| Phase 1 regression report | All Phase 1 benchmarks re-run and passing |
| Known defects list | Open defects with severity |
| Go/no-go recommendation | Evidence-based recommendation |

---

## 13. Pass/Fail Thresholds

### 13.1 Mandatory Performance Thresholds

| Metric | Mandatory Target | Stretch Target |
|---|---:|---:|
| Multi-node write throughput | ≥ 100 MB/s | ≥ 150 MB/s |
| Write latency with quorum (p99) | ≤ 3 ms | ≤ 2.5 ms |
| Coordinator failover time | < 3.5 seconds | < 2.5 seconds |
| Node replacement time | < 5 seconds | < 3 seconds |
| Raft leader election time | < 2 seconds | < 1 second |
| S3 upload throughput | ≥ 50 MB/s | ≥ 100 MB/s |
| Write Amplification Factor | ≤ 1.35 | ≤ 1.25 |
| Raft replication lag (p99) | ≤ 100 ms | ≤ 50 ms |

### 13.2 Mandatory Reliability Thresholds

| Metric | Mandatory Target |
|---|---|
| Data loss during node failure | Zero (JML = 0) |
| Double lease under partition | Zero |
| State invariant violations | Zero |
| Chaos test pass rate | 100% |
| 72-hour soak stability | No unbounded growth, no leaks |
| Phase 1 regression | All Phase 1 benchmarks pass |

### 13.3 Fail Conditions

Any of the following conditions results in an automatic FAIL:

1. Data loss > 0 records in any chaos test.
2. Double lease observed in any partition scenario.
3. State invariant violation in any test.
4. Coordinator failover > 5 seconds.
5. Node replacement > 10 seconds.
6. Phase 1 regression detected.
7. WAF > 1.5.
8. 72-hour soak shows unbounded memory growth.

---

## 14. Benchmark Execution Protocol

### 14.1 Pre-Run Checklist

Before each benchmark run:

1. Verify cluster health (all 3 nodes healthy, Raft quorum formed).
2. Verify S3 endpoint accessibility.
3. Verify NTP synchronization across all nodes.
4. Verify no background processes consuming CPU.
5. Record environmental disclosure.
6. Clear all metrics and logs from previous runs.
7. Verify Phase 1 test suite passes.

### 14.2 Execution Phases

| Phase | Duration | Purpose |
|---|---|---|
| Environment profiling | 2 min | Capture hardware, OS, network metadata |
| Warmup | 5 min | Run at 50% target rate; discard metrics |
| Steady-state | 30 min | Run at 100% target rate; primary measurement window |
| Chaos injection | Variable | Inject failures per chaos schedule |
| Recovery | Variable | Allow cluster to recover; measure recovery time |
| Cooldown | 5 min | Stop load; allow flushes and compaction |
| Extraction | 5 min | Harvest metrics; generate report |

### 14.3 Post-Run Validation

After each benchmark run:

1. Verify all metrics were collected.
2. Verify no data corruption.
3. Verify all invariants held.
4. Verify environmental disclosure is complete.
5. Archive raw metrics data.
6. Generate evidence report.

---

## 15. Deliverables

| Deliverable | Description | Due |
|---|---|---|
| D-P2-B-001 | Multi-node load generator implementation | Week 4 |
| D-P2-B-002 | Chaos injector implementation | Week 5 |
| D-P2-B-003 | Failover timing measurement tool | Week 6 |
| D-P2-B-004 | S3 streaming performance tool | Week 7 |
| D-P2-B-005 | Multi-node invariant checker | Week 8 |
| D-P2-B-006 | Evidence report generator | Week 9 |
| D-P2-B-007 | P1-P2 through P3-P2 benchmark runs | Week 9 |
| D-P2-B-008 | P4-P2 through P6-P2 benchmark runs | Week 10 |
| D-P2-B-009 | Chaos test execution (all 12 scenarios) | Week 11 |
| D-P2-B-010 | 72-hour soak test execution | Week 11–12 |
| D-P2-B-011 | Phase 2 benchmark evidence package | Week 12 |

---

## 16. Integration with Other Phase 2 Plans

| Related Plan | Integration Point |
|---|---|
| KEI-SPIKE-201 | Benchmark harness runs against the 3-node prototype |
| KEI-FORMAL-201 | Invariant checker implements oracles derived from TLA+ models |
| KEI-ENG-200 | Pass/fail thresholds map to Phase 2 acceptance criteria |
| KEI-BENCH-101 | Phase 1 benchmarks re-run as regression suite |
| KEI-OPS-041 | Chaos test scenarios align with validation plan |

---

## 17. Risks and Mitigations

| Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|
| Multi-node test environment instability | High | Medium | Dedicated hardware; containerized cluster setup |
| NTP synchronization drift | Medium | Medium | Use dedicated NTP server; verify before each run |
| Chaos injector interferes with metrics | Medium | Low | Isolate chaos injection from metrics collection path |
| S3 test costs | Medium | High | Use S3-compatible local storage (MinIO) for development; real S3 for final evidence |
| 72-hour soak test duration | Low | High | Schedule in dedicated time window; automate monitoring |
| Phase 1 regression discovered | High | Medium | Run Phase 1 suite before every Phase 2 benchmark |
| Raft library performance overhead | High | Medium | Benchmark Raft library in isolation first |

---

## 18. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial Multi-Node Performance, Failover & Recovery Harness Plan. Defines multi-node workload profiles, distributed metrics taxonomy, failover timing methodology, chaos test matrix, S3 streaming validation, 72-hour soak test, pass/fail thresholds, and evidence package requirements. |