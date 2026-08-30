# KEI-ENG-100 — Phase 1 Engineering Execution Plan

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-ENG-100 |
| Title | Phase 1 Engineering Execution Plan |
| Version | 1.0 |
| Level | Engineering Execution Plan |
| Status | Baseline — Ready for Execution |
| Phase | Phase 1 — Single-Node Core Engine |
| Owner | Engineering Program Lead / Chief Architect |
| Required Reviewers | Chief Architect, Storage Lead, State Plane Lead, Data Platform Lead, SRE/QA Lead |
| Governing Architecture Documents | KEI-ARC-001, KEI-ARC-010, KEI-ARC-011, KEI-ARC-012, KEI-ARC-020, KEI-ARC-021, KEI-ARC-023, KEI-ARC-027, KEI-DES-030, KEI-DES-031, KEI-DES-032, KEI-OPS-041 |
| Next Plan File | KEI-SPIKE-001 — Minimum Vertical Prototype Plan |

---

## 2. Executive Summary

This document defines the engineering execution plan for **Phase 1** of the Keirox Polymorphic Event Fabric.

Phase 1 converts the approved architecture into a working, testable, single-node core engine that proves the central architectural claim:

> One immutable physical log can serve as the durable source of truth for stream replay, task leasing, out-of-order acknowledgment, virtual dead-lettering, and columnar lakehouse export through mutable consumption-state overlays.

Phase 1 does **not** attempt to build the full distributed production system. Its purpose is to prove the core engine under realistic engineering constraints before committing to Phase 2 distributed durability.

---

## 3. Phase 1 Mission

The mission of Phase 1 is:

1. Build the single-node storage engine.
2. Build the consumption state plane.
3. Build the minimum columnar export pipeline.
4. Prove crash recovery and state invariants.
5. Produce benchmark and soak evidence.
6. Establish engineering governance for all later phases.

Phase 1 must answer this question:

> Can the Golden Invariant be implemented correctly, efficiently, and safely on a single node?

---

## 4. Phase 1 Scope

### 4.1 In Scope

Phase 1 includes:

1. Rust workspace and engineering foundation.
2. Single-node multiplexed WAL engine.
3. Batch-oriented WAL framing per KEI-DES-030.
4. Segment lifecycle and recovery replay.
5. Stream registry and sparse indexing foundation.
6. Consumption state plane per KEI-DES-031.
7. Roaring Bitmap state overlays.
8. Lease table and hierarchical timing wheel.
9. Out-of-order ACK support.
10. ACK_FAST mode.
11. Minimal ACK_DURABLE journal behavior for single-node recovery.
12. Mandatory DLQ eviction.
13. Watermark advancement.
14. Virtual DLQ and sparse exception table.
15. Basic schema handling for structured payloads.
16. Arrow RecordBatch generation.
17. Parquet export.
18. Basic metrics and health endpoints.
19. Benchmark harness.
20. Chaos and recovery tests.
21. Engineering traceability process.

### 4.2 Out of Scope

Phase 1 explicitly excludes:

1. Multi-node Raft consensus.
2. Multi-region replication.
3. Kafka wire protocol gateway.
4. SQS gateway.
5. AMQP gateway.
6. Full Apache Iceberg catalog committer.
7. Production KMS integration.
8. Full ABAC authorization.
9. Multi-tenant enterprise isolation.
10. In-broker SQL or materialized views.
11. CXL/RDMA hardware paths.
12. Customer-facing deployment tooling.

Any work not listed in Phase 1 scope requires a formal change request and ADR update.

---

## 5. Phase 1 Objectives

| Objective | Description |
|---|---|
| OBJ-1 | Prove immutable log append and replay correctness. |
| OBJ-2 | Prove multiplexed virtual stream handling. |
| OBJ-3 | Prove Roaring Bitmap state overlays for stream and queue consumption. |
| OBJ-4 | Prove out-of-order ACKs without corrupting watermark state. |
| OBJ-5 | Prove mandatory DLQ eviction prevents stuck watermarks. |
| OBJ-6 | Prove recovery after process crash. |
| OBJ-7 | Prove basic columnar export to Parquet. |
| OBJ-8 | Produce benchmark evidence for throughput, latency, and memory. |
| OBJ-9 | Establish repeatable test and evidence processes. |
| OBJ-10 | Prepare the codebase for Phase 2 distributed durability. |

---

## 6. Phase 1 Delivery Strategy

Phase 1 is divided into two major parts:

### 6.1 Part A — Engineering Bridge

Duration: Weeks 1–12.

Purpose:

- Bootstrap the repository.
- Build a minimum vertical prototype.
- Prove the core architecture end-to-end.
- Produce early evidence.
- Decide whether to continue into full Phase 1 hardening.

Exit gate:

```text
Engineering Bridge Review / Go-No-Go Gate
```

### 6.2 Part B — Phase 1 Hardening

Duration: Weeks 13–36.

Purpose:

- Harden the WAL engine.
- Harden the state plane.
- Improve recovery.
- Improve columnar export.
- Run scale and soak tests.
- Produce the Phase 1 certification evidence package.

Exit gate:

```text
Phase 1 Certification Review
```

---

## 7. Workstreams

| Workstream | ID | Owner Role | Responsibility |
|---|---|---|---|
| Engineering Foundation | WS-0 | Engineering Lead | Repository, CI, tooling, standards, traceability. |
| Storage Engine | WS-1 | Storage Lead | WAL, segments, CRC, recovery, indexing. |
| State Plane | WS-2 | Distributed Systems Lead | Bitmaps, leases, timers, watermark, DLQ. |
| Columnar ELT | WS-3 | Data Platform Lead | Arrow, Parquet, schema handling, export. |
| Quality and Evidence | WS-4 | SRE/QA Lead | Tests, benchmarks, soak, chaos, evidence reports. |
| Architecture Governance | WS-5 | Chief Architect | ADRs, RTM updates, conflict resolution, scope control. |

---

## 8. Milestone Plan

### 8.1 Milestone Overview

| Milestone | Name | Target Weeks | Purpose |
|---|---|---:|---|
| M1.0 | Engineering Mobilization | 1–2 | Repository, CI, workspace, standards. |
| M1.1 | WAL Foundation | 3–6 | Batch framing, CRC, segments, replay. |
| M1.2 | State Plane Foundation | 5–8 | Bitmaps, leases, ACKs, watermark, DLQ. |
| M1.3 | Recovery and Invariants | 7–10 | Crash recovery, invariant checker, stale operation rejection. |
| M1.4 | Prototype Evidence Gate | 11–12 | End-to-end prototype review and go/no-go decision. |
| M1.5 | Storage Hardening | 13–18 | Performance, corruption handling, index improvements. |
| M1.6 | State Plane Hardening | 15–22 | Lease scale, spill behavior, timer robustness. |
| M1.7 | Columnar Export | 17–26 | Arrow/Parquet export and query validation. |
| M1.8 | Scale and Soak | 23–30 | Stream scaling, lease scaling, 72-hour soak. |
| M1.9 | Operational Readiness | 27–32 | Metrics, health, runbook inputs, debug tooling. |
| M1.10 | Phase 1 Certification | 33–36 | Final evidence package and certification review. |

---

## 9. Detailed Milestone Definitions

### 9.1 M1.0 — Engineering Mobilization

Target: Weeks 1–2.

Deliverables:

1. Git repository.
2. Rust workspace.
3. CI pipeline.
4. Formatting and linting rules.
5. Test harness skeleton.
6. PR template with architecture traceability.
7. Initial crate boundaries.
8. Engineering decision log.
9. Local development environment instructions.
10. Benchmark folder structure.

Exit criteria:

- Workspace compiles.
- CI passes.
- Test skeleton runs.
- PR template enforced.
- Engineering conventions documented.

---

### 9.2 M1.1 — WAL Foundation

Target: Weeks 3–6.

Deliverables:

1. Core types:
   - Tenant ID.
   - Stream ID.
   - Logical offset.
   - Physical sequence.
   - Producer ID.
   - Producer sequence.
   - Schema ID.
   - Timestamp.
2. WAL batch header implementation.
3. Record entry implementation.
4. CRC32C validation.
5. Page alignment helpers.
6. Segment preallocation.
7. Segment sealing.
8. Batch append path.
9. Batch replay path.
10. Golden file tests.
11. Corruption detection tests.

Exit criteria:

- Batch frames serialize and deserialize correctly.
- CRC validation detects corruption.
- Segment sealing works.
- Replay reconstructs appended records.
- Unit and golden tests pass.

Primary references:

- KEI-ARC-020
- KEI-DES-030

---

### 9.3 M1.2 — State Plane Foundation

Target: Weeks 5–8.

Deliverables:

1. State shard key model.
2. Roaring Bitmap wrappers.
3. ACK bitmap.
4. DLQ bitmap.
5. Leased bitmap.
6. Lease table.
7. Timing wheel.
8. ACK_FAST path.
9. NACK path.
10. Lease timeout handling.
11. Retry count tracking.
12. Watermark advancement.
13. Mandatory DLQ eviction.
14. Sparse exception table.
15. State invariant checker.

Exit criteria:

- Out-of-order ACKs work.
- Lease expiration works.
- NACK requeue works.
- Retry count increments correctly.
- Poison-pill eviction works.
- Watermark advances despite stuck offsets.
- State invariant checker detects violations.

Primary references:

- KEI-ARC-021
- KEI-DES-031

---

### 9.4 M1.3 — Recovery and Invariants

Target: Weeks 7–10.

Deliverables:

1. State journal stub.
2. State snapshot stub.
3. WAL replay integration.
4. State reconstruction after restart.
5. Stale lease rejection.
6. Duplicate ACK idempotence.
7. Recovery test suite.
8. Kill/restart test.
9. Invariant validation after recovery.

Exit criteria:

- Process restart recovers committed state.
- Duplicate ACKs are idempotent.
- Stale lease operations are rejected.
- Recovery passes invariant checks.
- Kill/restart test passes.

Primary references:

- KEI-DES-031
- KEI-OPS-041

---

### 9.5 M1.4 — Prototype Evidence Gate

Target: Weeks 11–12.

Purpose:

Produce a working vertical prototype and decide whether to continue into full Phase 1 hardening.

Prototype flow:

```text
Producer append
   ↓
WAL batch commit
   ↓
Stream read
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
Recovery after restart
```

Exit criteria:

- End-to-end prototype works.
- Benchmark report produced.
- Memory report produced.
- Recovery report produced.
- Invariant report produced.
- Go/no-go recommendation delivered.

Possible gate outcomes:

| Outcome | Meaning |
|---|---|
| GO | Continue into Phase 1 hardening. |
| CONDITIONAL GO | Continue after specific fixes. |
| PIVOT | Adjust technical approach before continuing. |
| STOP | Major architectural assumption failed. |

---

### 9.6 M1.5 — Storage Hardening

Target: Weeks 13–18.

Deliverables:

1. Improved batch writer performance.
2. Segment rotation hardening.
3. Sparse index improvements.
4. Stream registry scaling.
5. Read path optimization.
6. Corruption recovery improvements.
7. Storage metrics.
8. Benchmark automation.

Exit criteria:

- Sustained ingress target achieved on reference hardware.
- Read path stable under concurrent load.
- Registry memory measured and bounded.
- Corruption tests pass.
- Storage benchmark report produced.

---

### 9.7 M1.6 — State Plane Hardening

Target: Weeks 15–22.

Deliverables:

1. Bitmap spill behavior.
2. Lease table scaling.
3. Timing wheel robustness.
4. Retry heap behavior.
5. Watermark optimization.
6. State snapshot hardening.
7. Journal replay hardening.
8. State metrics.
9. Lease churn tests.

Exit criteria:

- Bitmap memory bounded under churn.
- Spill behavior works without corrupting state.
- Lease timers remain accurate under load.
- Watermark advances continuously.
- State plane passes extended soak test.

---

### 9.8 M1.7 — Columnar Export

Target: Weeks 17–26.

Deliverables:

1. Sealed segment reader.
2. Basic schema inference or schema registry stub.
3. Arrow RecordBatch builder.
4. Parquet writer.
5. Export file naming.
6. Export metadata manifest.
7. Query validation using DuckDB or Polars.
8. Export metrics.

Exit criteria:

- Sealed data exports to Parquet.
- Exported files are queryable.
- Export does not block hot append path.
- Export metadata is readable.
- Export test report produced.

Primary references:

- KEI-ARC-023
- KEI-DES-033
- KEI-DES-034

Note: Full Iceberg commit governance is not required in Phase 1.

---

### 9.9 M1.8 — Scale and Soak

Target: Weeks 23–30.

Deliverables:

1. High-cardinality stream test.
2. High lease churn test.
3. 72-hour soak test.
4. Memory profiling report.
5. Latency drift report.
6. File handle report.
7. Watermark behavior report.
8. Recovery under larger state test.

Exit criteria:

- 100,000 streams stable.
- 1,000,000 streams validated under controlled conditions.
- 72-hour soak shows no unbounded memory growth.
- Latency drift remains within approved threshold.
- No invariant violations occur.

---

### 9.10 M1.9 — Operational Readiness

Target: Weeks 27–32.

Deliverables:

1. Metrics endpoint.
2. Health endpoint.
3. Debug state endpoint.
4. Log structure.
5. Error taxonomy.
6. Basic admin operations:
   - Stream list.
   - Stream info.
   - Consumer group info.
   - DLQ list.
   - Watermark status.
7. Operational runbook input for Phase 2.

Exit criteria:

- Metrics are machine-readable.
- Errors are classified and observable.
- Debug endpoints do not expose secrets or payload data.
- Operational signals are sufficient for failure diagnosis.

Primary references:

- KEI-ARC-027
- KEI-OPS-040

---

### 9.11 M1.10 — Phase 1 Certification

Target: Weeks 33–36.

Deliverables:

1. Phase 1 evidence package.
2. Benchmark summary.
3. Test summary.
4. Invariant validation summary.
5. Recovery summary.
6. Memory and soak summary.
7. Columnar export summary.
8. Updated RTM.
9. Updated ADRs.
10. Phase 1 certification report.

Exit criteria:

- All mandatory Phase 1 acceptance criteria pass.
- All blocking defects resolved or formally accepted.
- Architecture review board approves Phase 1 completion.
- Phase 2 entry recommendation produced.

---

## 10. Phase 1 Acceptance Criteria

### 10.1 Functional Acceptance

| ID | Requirement |
|---|---|
| ACC-F-001 | Producer appends are durably written to the local WAL. |
| ACC-F-002 | Stream reads return appended records by offset. |
| ACC-F-003 | Queue leases grant messages to workers. |
| ACC-F-004 | Out-of-order ACKs mark individual offsets terminal. |
| ACC-F-005 | NACK returns messages to ready state. |
| ACC-F-006 | Lease expiration returns messages to ready state. |
| ACC-F-007 | Retry counts increment correctly. |
| ACC-F-008 | Poison-pill messages are evicted to virtual DLQ. |
| ACC-F-009 | Watermark advances despite stuck offsets. |
| ACC-F-010 | Restart recovers committed WAL and state. |
| ACC-F-011 | Parquet export is queryable externally. |
| ACC-F-012 | Metrics and health endpoints expose engine state. |

### 10.2 Performance Acceptance

| ID | Requirement | Target |
|---|---|---:|
| ACC-P-001 | Sustained ingress throughput | ≥100 MB/s with 1 KB messages |
| ACC-P-002 | Append latency | p99 ≤2 ms local durable append under reference profile |
| ACC-P-003 | Stream read latency | p99 ≤2 ms for active Tier-0 reads |
| ACC-P-004 | Lease acquisition latency | p99 ≤1 ms fast path |
| ACC-P-005 | ACK_FAST latency | p99 ≤1 ms |
| ACC-P-006 | Compaction/export interference | ≤5% p99 jitter versus export-off baseline |

These are Phase 1 local-node targets. Distributed quorum durability targets are validated in Phase 2.

### 10.3 Scale Acceptance

| ID | Requirement | Target |
|---|---|---:|
| ACC-S-001 | Stable virtual streams | 100,000 |
| ACC-S-002 | Validated virtual streams | 1,000,000 under controlled benchmark |
| ACC-S-003 | Active leases | 100,000 stable |
| ACC-S-004 | Validated active leases | 1,000,000 under controlled benchmark |
| ACC-S-005 | File handle behavior | O(1) with respect to stream count |
| ACC-S-006 | Registry memory | Measured and consistent with approved model |

### 10.4 Reliability Acceptance

| ID | Requirement | Target |
|---|---|---|
| ACC-R-001 | Recovery after process kill | Restores valid state |
| ACC-R-002 | Recovery time | <5 seconds for defined prototype dataset |
| ACC-R-003 | Corruption detection | CRC detects injected corruption |
| ACC-R-004 | Invariant violations | Zero unresolved violations |
| ACC-R-005 | Soak test | 72 hours with no unbounded growth |

### 10.5 Quality Acceptance

| ID | Requirement |
|---|---|
| ACC-Q-001 | Unit tests pass. |
| ACC-Q-002 | Integration tests pass. |
| ACC-Q-003 | Golden file tests pass. |
| ACC-Q-004 | Property tests pass for state transitions. |
| ACC-Q-005 | Benchmark harness produces repeatable reports. |
| ACC-Q-006 | Chaos kill test passes. |
| ACC-Q-007 | All critical code paths have traceability annotations. |

---

## 11. Repository Structure

Recommended repository layout:

```text
keirox/
  Cargo.toml
  README.md
  docs/
    architecture/
    engineering/
    benchmarks/
    reports/
    archive/
  crates/
    keirox-core/
    keirox-wal/
    keirox-index/
    keirox-state/
    keirox-timer/
    keirox-arena/
    keirox-arrow-elt/
    keirox-api/
    keirox-server/
    keirox-bench/
    keirox-chaos/
    keirox-testkit/
  scripts/
  deploy/
  tests/
    integration/
    golden/
    chaos/
    soak/
```

### 11.1 Crate Responsibilities

| Crate | Responsibility |
|---|---|
| keirox-core | Common IDs, errors, timestamps, config, tracing utilities. |
| keirox-wal | WAL batch framing, segment lifecycle, writer, reader, replay. |
| keirox-index | Stream registry, sparse index, bloom filter structures. |
| keirox-state | Roaring bitmap state shards, leases, watermark, DLQ. |
| keirox-timer | Hierarchical timing wheel. |
| keirox-arena | Lock-free row ingress arena. |
| keirox-arrow-elt | Arrow RecordBatch generation and Parquet export. |
| keirox-api | Local admin and diagnostic API. |
| keirox-server | Single-node process composition. |
| keirox-bench | Benchmark workloads and reporting. |
| keirox-chaos | Failure injection and recovery tests. |
| keirox-testkit | Shared test utilities and fixtures. |

---

## 12. Engineering Standards

### 12.1 Language and Tooling

| Standard | Requirement |
|---|---|
| Language | Rust is the implementation language for Phase 1. |
| Zig | Not used in Phase 1. |
| Formatter | rustfmt with project configuration. |
| Linter | clippy with warnings treated as errors in CI. |
| Testing | cargo test, property tests, golden tests, integration tests. |
| Benchmarking | criterion or equivalent reproducible harness. |
| CI | Build, test, lint, format check, benchmark smoke. |
| Dependency audit | cargo-audit or equivalent in CI. |

### 12.2 Coding Rules

1. No unsafe code without explicit review and justification.
2. No blocking I/O on hot append path.
3. No secrets in logs, errors, or test fixtures.
4. No panics in normal operational paths.
5. All public crate APIs require documentation.
6. All binary structures require serialization tests.
7. All state transitions require invariant checks in debug builds.
8. All performance-sensitive paths require benchmark coverage.

---

## 13. Pull Request Traceability Requirement

Every pull request that affects architecture, behavior, performance, or data structures MUST include:

```text
Architecture-Documents:
  - KEI-ARC-xxx §section
  - KEI-DES-xxx §section

Requirement-IDs:
  - REQ-xxx-nnn

ADRs:
  - ADR-xxx

Tests:
  - test name or test file
  - KEI-OPS-041 mapping if applicable

Invariants Checked:
  - Golden Invariant
  - Bounded State
  - Watermark Advancement
  - Fail Secure
  - Compatibility by Subset

Superseded Claims Avoided:
  - Zero-ETL
  - 100% protocol parity
  - Universal exactly-once
  - CXL/RDMA zero-broker
  - Universal sub-2ms SLA
```

Pull requests without traceability SHOULD be blocked.

---

## 14. Testing and Evidence Requirements

### 14.1 Test Categories

| Category | Purpose |
|---|---|
| Unit tests | Validate isolated functions and structs. |
| Golden tests | Validate binary format stability. |
| Property tests | Validate state machine behavior. |
| Integration tests | Validate subsystem interaction. |
| Recovery tests | Validate restart and replay. |
| Chaos tests | Validate failure behavior. |
| Soak tests | Validate long-running stability. |
| Benchmark tests | Validate performance evidence. |
| Invariant tests | Validate architectural invariants. |

### 14.2 Mandatory Phase 1 Tests

| Test Area | Mandatory Tests |
|---|---|
| WAL | Batch serialization, CRC validation, segment sealing, replay. |
| State | Lease grant, ACK, NACK, timeout, DLQ eviction, watermark. |
| Recovery | Kill/restart, journal replay, snapshot recovery. |
| Corruption | Bit flip detection, invalid magic, truncated batch. |
| Scale | 10K, 100K, and 1M stream registry tests. |
| Lease churn | 100K active lease test. |
| Soak | 72-hour continuous workload. |
| Export | Parquet file generation and query validation. |
| Metrics | Metrics endpoint validation. |

### 14.3 Evidence Package

Phase 1 certification requires an evidence package containing:

1. CI test report.
2. Unit and integration test summary.
3. Golden file test summary.
4. Benchmark report.
5. Memory profile report.
6. Soak test report.
7. Recovery test report.
8. Chaos test report.
9. Invariant checker report.
10. Columnar export validation report.
11. Updated ADR log.
12. Updated RTM mapping.

---

## 15. Dependencies

### 15.1 Architecture Dependencies

| Dependency | Document |
|---|---|
| Golden Invariant | KEI-ARC-010 |
| NFR targets | KEI-ARC-011 |
| Binding ADRs | KEI-ARC-012 |
| Storage architecture | KEI-ARC-020 |
| State plane architecture | KEI-ARC-021 |
| Columnar ELT architecture | KEI-ARC-023 |
| Operability architecture | KEI-ARC-027 |
| WAL binary format | KEI-DES-030 |
| State plane data structures | KEI-DES-031 |
| API semantics | KEI-DES-032 |
| Validation plan | KEI-OPS-041 |

### 15.2 External Technical Dependencies

| Dependency | Purpose |
|---|---|
| Rust stable toolchain | Implementation language. |
| io_uring | Async direct I/O on Linux. |
| Roaring bitmap library | State overlay compression. |
| Arrow implementation | Columnar RecordBatch generation. |
| Parquet implementation | Lakehouse file export. |
| Prometheus/OpenTelemetry libraries | Metrics and tracing. |
| Criterion or equivalent | Benchmarking. |
| proptest or equivalent | Property testing. |

---

## 16. Risk Management

| Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|
| io_uring performance does not meet target | High | Medium | Benchmark early; tune batching; validate reference hardware. |
| Roaring bitmap memory grows under fragmentation | High | Medium | Implement spill thresholds; run lease churn tests. |
| State machine correctness issues | Critical | Medium | Property tests, invariant checker, formal modeling in later plan. |
| Recovery complexity underestimated | High | Medium | Start recovery work early; journal all state changes. |
| Arrow export consumes too much CPU | Medium | High | Isolate export threads; limit export batch size. |
| Scope creep into gateways or distributed features | High | High | Strict Phase 1 exclusions and PR governance. |
| Benchmark environment inconsistency | Medium | Medium | Record hardware/kernel/config in every report. |
| Team unfamiliar with io_uring or Rust performance patterns | Medium | Medium | Assign senior systems engineers; create spike tasks. |

---

## 17. Governance and Cadence

### 17.1 Meetings

| Meeting | Frequency | Purpose |
|---|---|---|
| Engineering standup | Daily | Execution coordination. |
| Architecture review | Weekly | ADRs, conflicts, invariant issues. |
| Benchmark review | Weekly | Performance evidence and regressions. |
| Risk review | Biweekly | Risk register updates. |
| Milestone review | Per milestone | Formal acceptance. |

### 17.2 Decision Rules

1. Architecture conflicts are resolved via KEI-ARC-012 ADRs.
2. Performance claims require benchmark evidence.
3. New requirements require RTM update.
4. Scope additions require change approval.
5. Blocking defects block milestone exit.

---

## 18. Definition of Done for Phase 1 Tasks

A Phase 1 task is complete only when:

1. Code is implemented.
2. Tests are added and passing.
3. Documentation references are included.
4. PR traceability is complete.
5. No invariant violations are introduced.
6. No benchmark regressions are introduced without approval.
7. No banned claims are reintroduced.
8. Review approval is obtained.
9. CI passes.
10. Evidence is archived where applicable.

---

## 19. Phase 1 Exit Review

The Phase 1 exit review MUST evaluate:

1. Functional correctness.
2. Performance evidence.
3. Scale evidence.
4. Recovery evidence.
5. Memory stability.
6. Test coverage.
7. Architectural compliance.
8. Engineering quality.
9. Outstanding defects.
10. Phase 2 readiness.

### 19.1 Possible Phase 1 Outcomes

| Outcome | Meaning |
|---|---|
| PHASE 1 CERTIFIED | Proceed to Phase 2 distributed durability. |
| CONDITIONALLY CERTIFIED | Proceed after defined remediation tasks. |
| EXTENDED | Additional Phase 1 work required before Phase 2. |
| RE-SCOPE | Major technical adjustment required. |
| STOP | Core assumptions failed. |

---

## 20. Traceability Summary

| Area | Governing Documents | Phase 1 Evidence |
|---|---|---|
| Immutable log | KEI-ARC-010, KEI-ARC-020, KEI-DES-030 | WAL tests, replay tests, CRC tests |
| Stream multiplexing | KEI-ARC-020, KEI-DES-030 | Registry scale tests |
| State overlays | KEI-ARC-021, KEI-DES-031 | State machine tests, invariant checker |
| Out-of-order ACK | KEI-ARC-021, KEI-DES-031 | Lease/ACK tests |
| Watermark | KEI-ARC-021, KEI-DES-031 | Watermark tests, DLQ eviction tests |
| Recovery | KEI-ARC-020, KEI-DES-031 | Kill/restart tests |
| Columnar export | KEI-ARC-023, KEI-DES-033, KEI-DES-034 | Parquet export tests |
| Observability | KEI-ARC-027 | Metrics and health endpoint tests |
| Validation | KEI-OPS-041 | Benchmark, soak, chaos reports |

---

## 21. Next Planning Files

After this document, the planning suite continues with:

| Next File | Purpose |
|---|---|
| KEI-SPIKE-001 | Minimum Vertical Prototype Plan. |
| KEI-FORMAL-001 | Formal State Machine Validation Plan. |
| KEI-BENCH-001 | Benchmark and Evidence Harness Plan. |
| KEI-ORG-001 | Team, Governance, and Delivery Plan. |
| KEI-RISK-001 | Risk Reduction and Go/No-Go Plan. |

---

## 22. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial Phase 1 Engineering Execution Plan. Defines Phase 1 mission, scope, workstreams, milestones, acceptance criteria, repository structure, testing requirements, governance, and exit gates. |