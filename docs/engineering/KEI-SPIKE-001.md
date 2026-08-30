# KEI-SPIKE-001 — Minimum Vertical Prototype Plan

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-SPIKE-001 |
| Title | Minimum Vertical Prototype Plan |
| Version | 1.0 |
| Level | Engineering Execution Plan |
| Status | Baseline — Ready for Execution |
| Phase | Phase 1 Engineering Bridge |
| Duration | 90 days / 12 weeks |
| Owner | Prototype Engineering Lead |
| Governing Plan | KEI-ENG-100 — Phase 1 Engineering Execution Plan |
| Governing Architecture Documents | KEI-ARC-010, KEI-ARC-011, KEI-ARC-020, KEI-ARC-021, KEI-ARC-023, KEI-ARC-027, KEI-DES-030, KEI-DES-031, KEI-OPS-041 |
| Next Plan File | KEI-FORMAL-001 — Formal State Machine Validation Plan |

---

## 2. Executive Summary

This document defines the plan for building the **Minimum Vertical Prototype** for the Keirox Polymorphic Event Fabric.

The prototype is not a product release. It is a narrow, executable proof of the core architecture.

Its purpose is to prove that the following end-to-end flow works correctly on a single node:

```text
Producer append
   ↓
Immutable WAL batch commit
   ↓
Stream read by offset
   ↓
Queue lease
   ↓
Out-of-order ACK
   ↓
Watermark advancement
   ↓
DLQ eviction
   ↓
Parquet export
   ↓
Recovery after process restart
```

The prototype must produce evidence that the Golden Invariant is implementable and that the most dangerous engineering risks are understood before full Phase 1 hardening.

---

## 3. Prototype Mission

The prototype must answer the following question:

> Can a single-node Keirox engine durably append events to an immutable multiplexed WAL, project the same data as a stream and a leased task queue, advance watermarks safely, evict poison-pill messages to a virtual DLQ, export Parquet data, and recover cleanly after a crash?

If the answer is yes, the project can proceed into full Phase 1 hardening.

If the answer is no, the prototype must reveal exactly why, so the architecture or implementation strategy can be corrected early.

---

## 4. Relationship to KEI-ENG-100

This prototype is the first executable stage inside KEI-ENG-100.

It corresponds primarily to:

| KEI-ENG-100 Milestone | Prototype Responsibility |
|---|---|
| M1.0 Engineering Mobilization | Repository, workspace, CI, test foundation. |
| M1.1 WAL Foundation | Batch framing, CRC, segments, replay. |
| M1.2 State Plane Foundation | Bitmaps, leases, ACKs, watermark, DLQ. |
| M1.3 Recovery and Invariants | Restart recovery, invariant checker, stale operation rejection. |
| M1.4 Prototype Evidence Gate | Final prototype report and go/no-go recommendation. |

The prototype is intentionally narrower than full Phase 1. It is designed to produce evidence quickly, not to deliver production readiness.

---

## 5. Prototype Scope

### 5.1 Must Have

The prototype MUST include:

1. Rust workspace and prototype binary.
2. Single-process, single-node engine.
3. Multiplexed virtual stream registry.
4. Batch-oriented WAL writer.
5. CRC32C integrity validation.
6. Segment creation and sealing.
7. Stream read by logical offset.
8. State shard with Roaring Bitmap overlays.
9. Lease grant.
10. Lease renewal.
11. ACK_FAST acknowledgment.
12. NACK and requeue.
13. Lease timeout.
14. Retry count tracking.
15. Mandatory DLQ eviction.
16. Virtual DLQ listing.
17. Watermark advancement.
18. State invariant checker.
19. Local state journal.
20. Restart recovery.
21. Basic Parquet export.
22. Basic benchmark harness.
23. Basic chaos kill test.
24. Prototype evidence report.

### 5.2 Should Have

The prototype SHOULD include if schedule permits:

1. Simple JSON schema inference.
2. Arrow RecordBatch builder for common fields.
3. Basic metrics endpoint.
4. Basic health endpoint.
5. Simple admin CLI.
6. 100,000-stream experimental scaling test.
7. 100,000-lease experimental scaling test.
8. 24-hour soak test.

### 5.3 Will Not Have

The prototype WILL NOT include:

1. Multi-node Raft.
2. Multi-region replication.
3. Kafka gateway.
4. SQS gateway.
5. AMQP gateway.
6. Full Iceberg committer.
7. KMS integration.
8. Encryption at rest.
9. Authentication or authorization.
10. Production multi-tenancy isolation.
11. Rolling upgrades.
12. Customer-facing APIs.
13. Deployment automation.
14. High-availability failover.

Any addition to the prototype scope requires approval from the Prototype Engineering Lead and Chief Architect.

---

## 6. Prototype Success Criteria

### 6.1 Functional Success Criteria

| ID | Criterion |
|---|---|
| SPIKE-F-001 | Producer appends are written to the WAL and survive restart. |
| SPIKE-F-002 | Stream reads return records by logical offset. |
| SPIKE-F-003 | Queue leases grant offsets to workers. |
| SPIKE-F-004 | ACK marks individual offsets terminal out of order. |
| SPIKE-F-005 | NACK returns offsets to ready state. |
| SPIKE-F-006 | Lease timeout returns offsets to ready state. |
| SPIKE-F-007 | Retry count increments after NACK or timeout. |
| SPIKE-F-008 | Off exceeding retry limit transitions to virtual DLQ. |
| SPIKE-F-009 | Watermark advances past terminal offsets. |
| SPIKE-F-010 | Watermark advances past poison-pill offsets after DLQ eviction. |
| SPIKE-F-011 | Restart recovery reconstructs committed state. |
| SPIKE-F-012 | Parquet export is queryable by DuckDB or Polars. |

### 6.2 Performance Success Criteria

| ID | Criterion | Mandatory Target | Stretch Target |
|---|---|---:|---:|
| SPIKE-P-001 | Sustained ingress throughput | ≥50 MB/s | ≥100 MB/s |
| SPIKE-P-002 | Local durable append latency | p99 ≤2 ms | p99 ≤2 ms at 100 MB/s |
| SPIKE-P-003 | Stream read latency | p99 ≤2 ms for active data | p99 ≤1.5 ms |
| SPIKE-P-004 | Lease acquisition latency | p99 ≤1 ms | p99 ≤0.5 ms |
| SPIKE-P-005 | ACK_FAST latency | p99 ≤1 ms | p99 ≤0.5 ms |

The mandatory targets MUST be met for a GO decision. Stretch targets are useful but not blocking.

### 6.3 Scale Success Criteria

| ID | Criterion | Mandatory Target | Stretch Target |
|---|---|---:|---:|
| SPIKE-S-001 | Stable virtual streams | 10,000 | 100,000 |
| SPIKE-S-002 | Experimental virtual streams | 100,000 | 1,000,000 |
| SPIKE-S-003 | Stable active leases | 10,000 | 100,000 |
| SPIKE-S-004 | Experimental active leases | 100,000 | 1,000,000 |
| SPIKE-S-005 | File handle behavior | O(1) with stream count | O(1) under 1M streams |

### 6.4 Reliability Success Criteria

| ID | Criterion | Mandatory Target |
|---|---|---|
| SPIKE-R-001 | Process restart recovery | State restored correctly |
| SPIKE-R-002 | Recovery time | <5 seconds for prototype dataset |
| SPIKE-R-003 | Corruption detection | CRC detects injected corruption |
| SPIKE-R-004 | Invariant violations | Zero unresolved violations |
| SPIKE-R-005 | Soak stability | 24-hour soak with no unbounded memory growth |

---

## 7. Prototype Architecture Slice

The prototype implements a simplified vertical slice of the full architecture.

### 7.1 Prototype Topology

```text
Single Node / Single Process
┌────────────────────────────────────────────────────────────┐
│                      KEIROX PROTOTYPE                      │
│                                                            │
│  Producer API                                              │
│      │                                                     │
│      ▼                                                     │
│  Ingress Arena                                             │
│      │                                                     │
│      ▼                                                     │
│  WAL Batch Writer                                          │
│      │                                                     │
│      ├──► Stream Reader                                    │
│      │                                                     │
│      ├──► State Plane                                      │
│      │       │                                             │
│      │       ├── Leases                                    │
│      │       ├── ACKs                                      │
│      │       ├── NACKs                                     │
│      │       ├── Timing Wheel                              │
│      │       ├── Watermark                                 │
│      │       └── Virtual DLQ                               │
│      │                                                     │
│      └──► Columnar Exporter                                │
│              │                                             │
│              ▼                                             │
│          Parquet Files                                     │
│                                                            │
│  Recovery Manager                                          │
│      │                                                     │
│      ▼                                                     │
│  WAL Replay + State Journal Replay                         │
└────────────────────────────────────────────────────────────┘
```

### 7.2 Simplifications

| Full Architecture Feature | Prototype Simplification |
|---|---|
| Multi-node Raft | Local single-node commit only. |
| Metadata Raft | Local file-based state journal. |
| Distributed coordinator | Single in-process coordinator. |
| ACK_DURABLE consensus commit | Local journal fsync simulation. |
| KMS encryption | Disabled. |
| ABAC authorization | Disabled. |
| Multi-tenant quotas | Simplified in-memory quotas only. |
| Iceberg catalog | Local Parquet manifest only. |
| Kafka/SQS/AMQP gateways | Native prototype API only. |

These simplifications are allowed because the prototype validates core behavior, not distributed durability or enterprise integration.

---

## 8. Technical Constraints

### 8.1 Language and Runtime

| Constraint | Requirement |
|---|---|
| Language | Rust stable. |
| Async I/O | io_uring preferred for WAL path; blocking fallback acceptable for early weeks. |
| Allocator | jemalloc or mimalloc recommended. |
| Bitmaps | Roaring bitmap implementation compatible with 64-bit offset partitioning. |
| Columnar format | Apache Arrow and Parquet. |
| OS | Linux first; local development on macOS acceptable if io_uring is abstracted. |

### 8.2 Architecture Constraints

The prototype MUST respect:

1. Immutable WAL append behavior.
2. Batch-oriented WAL framing.
3. CRC32C integrity boundaries.
4. State overlays separate from physical log.
5. Mandatory DLQ eviction.
6. Watermark monotonicity.
7. Idempotent duplicate ACK behavior.
8. Stale lease rejection.
9. No mutation of committed WAL records.
10. No hidden destructive queue semantics.

### 8.3 Prototype Anti-Goals

The prototype MUST NOT become:

1. A distributed cluster.
2. A gateway compatibility project.
3. A lakehouse catalog project.
4. A security platform.
5. A production deployment system.
6. A general-purpose query engine.

---

## 9. Work Packages

### 9.1 WP-0 — Engineering Foundation

Objective:

Create a clean engineering base.

Deliverables:

1. Repository.
2. Rust workspace.
3. CI pipeline.
4. Formatter and linter configuration.
5. Test harness skeleton.
6. PR traceability template.
7. Engineering decision log.
8. Local run instructions.

Exit criteria:

- Workspace compiles.
- CI passes.
- Test harness runs.
- PR template enforced.

---

### 9.2 WP-1 — Core Types

Objective:

Define shared primitive types.

Deliverables:

1. Tenant ID.
2. Stream ID.
3. Group ID.
4. State shard key.
5. Logical offset.
6. Physical sequence.
7. Producer ID.
8. Producer sequence.
9. Timestamp types.
10. Error taxonomy.

Exit criteria:

- Types are tested.
- Serialization is stable.
- Error categories are machine-readable.

---

### 9.3 WP-2 — WAL Engine

Objective:

Implement the immutable batch log.

Deliverables:

1. WAL batch header.
2. Record entry structure.
3. CRC32C calculation and validation.
4. Page alignment helpers.
5. Segment creation.
6. Segment append.
7. Segment sealing.
8. Batch replay.
9. Corruption detection.
10. Golden file tests.

Exit criteria:

- Batch append works.
- Replay reconstructs records.
- CRC detects corruption.
- Segment sealing works.
- Golden tests pass.

Primary references:

- KEI-DES-030
- KEI-ARC-020

---

### 9.4 WP-3 — Stream Registry and Reader

Objective:

Enable stream-oriented reads over the multiplexed WAL.

Deliverables:

1. Stream registry.
2. Head offset tracking.
3. Logical offset assignment.
4. Stream read cursor.
5. Active segment read path.
6. Basic sparse index stub.
7. Stream info API.

Exit criteria:

- Multiple streams can append concurrently.
- Reads return correct records per stream.
- Offsets are monotonic per stream.
- Registry memory is measured.

Primary references:

- KEI-ARC-020
- KEI-DES-030

---

### 9.5 WP-4 — State Plane Core

Objective:

Implement the polymorphic consumption state overlay.

Deliverables:

1. State shard key mapping.
2. Roaring Bitmap wrappers.
3. ACK bitmap.
4. DLQ bitmap.
5. Leased bitmap.
6. Lease table.
7. Lease token generation.
8. Timing wheel.
9. ACK_FAST path.
10. NACK path.
11. Lease timeout path.
12. Retry count tracking.
13. Watermark advancement.
14. Mandatory DLQ eviction.
15. Sparse exception table.

Exit criteria:

- Out-of-order ACKs work.
- Lease timeout works.
- NACK requeue works.
- Retry count increments.
- DLQ eviction works.
- Watermark advances.
- Duplicate ACKs are idempotent.
- Stale lease operations are rejected.

Primary references:

- KEI-ARC-021
- KEI-DES-031

---

### 9.6 WP-5 — Recovery and Invariants

Objective:

Prove restart recovery and state correctness.

Deliverables:

1. Local state journal.
2. State snapshot stub.
3. WAL replay integration.
4. State journal replay.
5. Invariant checker.
6. Kill/restart test.
7. Stale lease rejection after restart.
8. Recovery metrics.

Exit criteria:

- Restart recovers committed state.
- Recovery passes invariant checks.
- Stale leases are rejected.
- Recovery time is measured.
- Kill/restart test passes.

Primary references:

- KEI-DES-031
- KEI-OPS-041

---

### 9.7 WP-6 — Columnar Export

Objective:

Prove basic lakehouse projection.

Deliverables:

1. Sealed segment reader.
2. Simple schema detection.
3. Arrow RecordBatch builder.
4. Parquet writer.
5. Export manifest.
6. Query validation script.
7. Export metrics.

Exit criteria:

- Sealed data exports to Parquet.
- Exported data is queryable.
- Export does not corrupt WAL state.
- Export does not block append path for extended periods.

Primary references:

- KEI-ARC-023
- KEI-DES-033

---

### 9.8 WP-7 — Benchmark and Evidence

Objective:

Produce repeatable evidence for the prototype gate.

Deliverables:

1. Benchmark CLI.
2. Workload generator.
3. Latency histogram reporter.
4. Memory usage reporter.
5. Stream scale test.
6. Lease churn test.
7. Soak test.
8. Recovery test.
9. Corruption test.
10. Evidence report template.

Exit criteria:

- Benchmarks are repeatable.
- Hardware profile is recorded.
- Mandatory targets are measured.
- Evidence report is generated.

Primary references:

- KEI-OPS-041
- KEI-ENG-100

---

## 10. 12-Week Execution Plan

### Week 1 — Engineering Mobilization

Primary work:

- Create repository.
- Create Rust workspace.
- Configure CI.
- Configure rustfmt and clippy.
- Create test skeleton.
- Define PR traceability template.
- Define crate boundaries.

Exit:

- Workspace builds.
- CI passes.
- Team agrees on engineering conventions.

---

### Week 2 — Core Types and Test Foundation

Primary work:

- Implement core IDs.
- Implement error taxonomy.
- Implement timestamp utilities.
- Implement serialization helpers.
- Add unit tests.
- Add golden file framework.

Exit:

- Core types are stable.
- Tests pass.
- Golden file framework works.

---

### Week 3 — WAL Batch Writer

Primary work:

- Implement WAL batch header.
- Implement record entry.
- Implement CRC32C utilities.
- Implement page alignment.
- Implement segment creation.
- Implement batch append.

Exit:

- Batch append works.
- CRC validation works.
- Page alignment works.

---

### Week 4 — WAL Replay and Segment Lifecycle

Primary work:

- Implement segment sealing.
- Implement batch replay.
- Implement corruption detection.
- Implement WAL reader.
- Add golden tests.
- Add truncation and malformed batch tests.

Exit:

- Replay reconstructs appended records.
- Corruption is detected.
- Segment lifecycle works.

---

### Week 5 — Stream Registry and Reads

Primary work:

- Implement stream registry.
- Implement logical offset assignment.
- Implement stream read cursor.
- Implement active segment reads.
- Add concurrent multi-stream tests.

Exit:

- Multiple streams append concurrently.
- Stream reads are correct.
- Offsets are monotonic per stream.

---

### Week 6 — State Plane Foundations

Primary work:

- Implement state shard key.
- Implement Roaring Bitmap wrappers.
- Implement ACK bitmap.
- Implement DLQ bitmap.
- Implement leased bitmap.
- Implement lease table.
- Implement ACK_FAST.

Exit:

- Out-of-order ACKs work.
- ACK state is tracked correctly.
- Duplicate ACKs are idempotent.

---

### Week 7 — Leases and Timing Wheel

Primary work:

- Implement lease grant.
- Implement lease renewal.
- Implement lease token validation.
- Implement timing wheel.
- Implement lease timeout.
- Implement NACK and requeue.

Exit:

- Leases expire correctly.
- Renewals work.
- NACK requeue works.
- Stale lease operations are rejected.

---

### Week 8 — Watermark and DLQ

Primary work:

- Implement watermark advancement.
- Implement retry count tracking.
- implement mandatory DLQ eviction.
- Implement sparse exception table.
- Implement virtual DLQ listing.
- Add poison-pill tests.

Exit:

- Watermark advances past terminal offsets.
- Poison-pill messages are evicted.
- Stuck offsets do not leak memory.

---

### Week 9 — Recovery and Invariant Checker

Primary work:

- Implement local state journal.
- Implement snapshot stub.
- Implement restart recovery.
- Implement invariant checker.
- Add kill/restart tests.
- Add recovery timing metrics.

Exit:

- Restart recovery works.
- Invariant checker detects violations.
- Recovery time is measured.

---

### Week 10 — Columnar Export

Primary work:

- Implement sealed segment reader.
- Implement Arrow RecordBatch builder.
- Implement Parquet writer.
- Implement export manifest.
- Add DuckDB or Polars query validation.

Exit:

- Parquet files are generated.
- Exported files are queryable.
- Export metadata is readable.

---

### Week 11 — Benchmarks, Soak, and Chaos

Primary work:

- Run throughput benchmark.
- Run latency benchmark.
- Run stream scale test.
- Run lease churn test.
- Run 24-hour soak test.
- Run kill/restart chaos test.
- Run corruption injection test.

Exit:

- Mandatory targets measured.
- Stretch targets measured.
- Evidence collected.

---

### Week 12 — Evidence Report and Go/No-Go Review

Primary work:

- Compile benchmark results.
- Compile test results.
- Compile invariant results.
- Compile recovery results.
- Compile export validation results.
- Prepare go/no-go recommendation.
- Present to architecture review board.

Exit:

- Prototype evidence package delivered.
- Go/no-go decision made.

---

## 11. Test Plan

### 11.1 Unit Tests

Required for:

- Core types.
- WAL batch serialization.
- CRC validation.
- Bitmap operations.
- Lease token validation.
- Retry count logic.
- Watermark advancement.
- DLQ eviction.

### 11.2 Integration Tests

Required for:

- Append then read.
- Append then lease.
- Lease then ACK.
- Lease then NACK.
- Lease timeout.
- Retry exhaustion.
- DLQ listing.
- Restart recovery.
- Export after sealing.

### 11.3 Property Tests

Required for:

- State transition legality.
- Watermark monotonicity.
- Duplicate ACK idempotence.
- Lease uniqueness.
- Replay determinism.
- Bitmap consistency after random ACK/NACK sequences.

### 11.4 Golden Tests

Required for:

- WAL batch header encoding.
- Record entry encoding.
- CRC behavior.
- Segment footer behavior.
- State journal entry encoding.
- Parquet export metadata.

### 11.5 Failure Tests

Required for:

- Malformed batch header.
- Truncated batch.
- CRC mismatch.
- Invalid magic bytes.
- Stale lease token.
- Stale epoch or session ID.
- Process kill during append.
- Process kill during ACK.
- Process kill during export.

---

## 12. Benchmark Plan

### 12.1 Benchmark Profiles

| Profile | Purpose | Workload |
|---|---|---|
| P1-Proto | Append throughput and latency | 1 KB messages, sustained write load |
| P3-Proto | Stream registry scaling | 10K and 100K virtual streams |
| P4-Proto | Lease churn | 10K and 100K active leases with ACK/NACK churn |
| P5-Proto | Export overhead | Sealed segment export to Parquet |
| P6-Proto | Degraded behavior | Export lag or disk pressure simulation |

### 12.2 Benchmark Metrics

| Metric | Required |
|---|---|
| Messages/sec | Yes |
| MB/sec | Yes |
| Append p50/p99/p999 | Yes |
| Read p50/p99 | Yes |
| Lease acquisition p99 | Yes |
| ACK p99 | Yes |
| Bitmap memory bytes | Yes |
| Lease table memory bytes | Yes |
| Watermark lag | Yes |
| DLQ eviction count | Yes |
| Recovery time | Yes |
| Export time | Yes |
| Error count | Yes |

### 12.3 Benchmark Environment

Every benchmark run MUST record:

1. CPU model.
2. Core count.
3. RAM.
4. NVMe model.
5. Filesystem.
6. Kernel version.
7. Rust toolchain version.
8. Allocator.
9. Build profile.
10. Workload configuration.
11. Start and end timestamps.
12. Git commit hash.

---

## 13. Evidence Package

The prototype evidence package MUST include:

1. Prototype summary report.
2. Functional test report.
3. Property test report.
4. Golden test report.
5. Integration test report.
6. Benchmark report.
7. Memory profile report.
8. Recovery report.
9. Corruption test report.
10. Soak test report.
11. Export validation report.
12. Invariant checker report.
13. Known defects list.
14. Unresolved risks list.
15. Go/no-go recommendation.

---

## 14. Prototype Go/No-Go Gate

### 14.1 Gate Inputs

The gate review considers:

1. Mandatory success criteria results.
2. Stretch criteria results.
3. Defect severity.
4. Invariant violations.
5. Memory behavior.
6. Recovery behavior.
7. Engineering velocity.
8. Risk exposure.
9. Architecture compliance.
10. Evidence quality.

### 14.2 Gate Outcomes

| Outcome | Meaning |
|---|---|
| GO | Mandatory criteria pass; continue into Phase 1 hardening. |
| CONDITIONAL GO | Mandatory criteria mostly pass; specific fixes required before hardening. |
| PIVOT | Core approach needs technical adjustment before continuation. |
| STOP | Core assumptions failed; project should pause or re-scope. |

### 14.3 Go Criteria

A GO decision requires:

1. All functional mandatory criteria pass.
2. All mandatory performance criteria pass.
3. All mandatory reliability criteria pass.
4. Zero unresolved invariant violations.
5. Recovery works reliably.
6. Evidence package is complete.
7. No critical defect remains open.

### 14.4 Conditional Go Criteria

A CONDITIONAL GO may be granted if:

1. One or more stretch targets fail.
2. A non-critical defect remains open.
3. Benchmark evidence is promising but incomplete.
4. A remediation plan is approved.

A CONDITIONAL GO MUST include:

- Specific remediation tasks.
- Owners.
- Deadlines.
- Re-review criteria.

---

## 15. Risks and Mitigations

| Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|
| io_uring implementation complexity | High | Medium | Begin with simple direct I/O path; abstract io_uring behind WAL writer interface. |
| Roaring bitmap memory grows unexpectedly | High | Medium | Measure early; implement watermark purge and spill stub. |
| Timing wheel correctness issues | Medium | Medium | Add property tests; validate lease expiry order. |
| Recovery complexity underestimated | High | High | Start recovery work by Week 7; journal all state mutations. |
| Arrow export consumes too much time | Medium | High | Keep export minimal; defer schema inference complexity. |
| Prototype scope creep | High | High | Strict exclusion list; architecture review approval for additions. |
| Benchmark environment inconsistency | Medium | Medium | Record full environment metadata for every run. |
| Team unfamiliar with Rust systems programming | Medium | Medium | Assign senior systems engineer; keep design simple. |

---

## 16. Prototype Team

### 16.1 Minimum Team

| Role | Count | Responsibility |
|---|---:|---|
| Prototype Engineering Lead | 1 | Overall execution, scope control, gate reporting. |
| Storage Engineer | 1 | WAL, segments, CRC, recovery. |
| State Plane Engineer | 1 | Bitmaps, leases, watermark, DLQ. |
| Data/QA Engineer | 1 | Arrow export, benchmarks, tests, evidence. |

### 16.2 Optional Support

| Role | Responsibility |
|---|---|
| Chief Architect | Architecture compliance and conflict resolution. |
| SRE Advisor | Benchmark environment and observability guidance. |
| Security Advisor | Ensure no unsafe security shortcuts become permanent. |

---

## 17. Definition of Done

The prototype is done when:

1. The end-to-end prototype flow works.
2. Mandatory success criteria are measured.
3. Tests pass.
4. Invariant checker passes.
5. Recovery test passes.
6. Parquet export is queryable.
7. Benchmark evidence is collected.
8. Known defects are documented.
9. Evidence package is complete.
10. Go/no-go recommendation is delivered.

---

## 18. Traceability to Architecture Documents

| Prototype Area | Governing Document |
|---|---|
| Golden Invariant | KEI-ARC-010 |
| NFR targets | KEI-ARC-011 |
| Storage engine behavior | KEI-ARC-020 |
| State plane behavior | KEI-ARC-021 |
| Columnar ELT behavior | KEI-ARC-023 |
| Operability behavior | KEI-ARC-027 |
| WAL binary format | KEI-DES-030 |
| State data structures | KEI-DES-031 |
| API semantics | KEI-DES-032 |
| Validation method | KEI-OPS-041 |
| Engineering governance | KEI-ENG-100 |

---

## 19. Next Planning File

After this document, the next planning file is:

```text
KEI-FORMAL-001_State_Machine_Validation_Plan.md
```

It will define how the state machine, watermark advancement, lease lifecycle, and recovery behavior are formally validated before distributed implementation.

---

## 20. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial Minimum Vertical Prototype Plan. Defines prototype mission, scope, success criteria, work packages, 12-week execution plan, test plan, benchmark plan, evidence package, and go/no-go gate. |