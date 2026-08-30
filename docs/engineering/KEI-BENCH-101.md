# KEI-BENCH-001 — Performance Validation Harness Plan

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-BENCH-001 |
| Title | Performance Validation Harness Plan |
| Version | 1.0 |
| Level | Engineering Execution Plan |
| Status | Baseline — Ready for Execution |
| Phase | Phase 1 Engineering Bridge (parallel track) |
| Owner | SRE / Performance Engineering Lead |
| Governing Plan | KEI-ENG-100 — Phase 1 Engineering Execution Plan |
| Related Plans | KEI-SPIKE-001 (Prototype), KEI-FORMAL-001 (Formal Validation) |
| Governing Architecture Documents | KEI-ARC-011 (NFRs), KEI-ARC-020 (Storage), KEI-ARC-021 (State Plane), KEI-ARC-027 (Operability) |
| Next Plan File | KEI-ORG-001 — Team, Governance, and Delivery Plan |

---

## 2. Executive Summary

This document defines the plan for building the **Performance Validation Harness** for the Keirox Polymorphic Event Fabric. 

A benchmark without strict environmental controls, standardized workload profiles, and rigorous statistical reporting is merely a marketing exercise. This plan ensures that all performance, throughput, latency, and resource-utilization claims made during the Phase 1 Prototype (KEI-SPIKE-001) and subsequent phases are scientifically repeatable, empirically proven, and strictly bounded by disclosed hardware and software conditions.

The harness will serve as the single source of truth for all performance evidence gates.

---

## 3. Purpose and Scope

### 3.1 Purpose

The purpose of this plan is to:

1. Define the canonical workload profiles for the prototype.
2. Specify the architecture of the load generation and metrics collection harness.
3. Establish strict environmental disclosure requirements.
4. Define the statistical methods for latency and throughput reporting.
5. Set the mandatory pass/fail thresholds for the Phase 1 Prototype Evidence Gate.

### 3.2 Scope

**In scope:**
- Load generator design (producers, stream consumers, queue workers).
- Metrics collection and aggregation (Prometheus/OpenTelemetry).
- Hardware and OS environment profiling.
- Benchmark execution protocol (warmup, steady-state, cooldown).
- Report generation and evidence packaging.
- Pass/fail threshold definitions.

**Out of scope:**
- Correctness and invariant checking (owned by KEI-FORMAL-001 and KEI-SPIKE-001).
- Chaos engineering and failure injection (owned by KEI-OPS-041 and KEI-SPIKE-001).
- Long-term 72-hour soak testing execution (owned by KEI-SPIKE-001, though this harness provides the tooling).

---

## 4. Canonical Workload Profiles

The harness MUST support the following prototype-specific workload profiles, derived from KEI-ARC-011 but scoped for the single-node prototype.

| Profile ID | Name | Definition | Primary Target |
|---|---|---|---|
| **P1-Proto** | Baseline Sustained | 100,000 msgs/s @ 1 KB (100 MB/s), steady state, 10 streams. | Write latency (p99 ≤ 2ms), Throughput. |
| **P3-Proto** | High Cardinality | 10,000 to 100,000 active streams, low per-stream throughput (10 msgs/s each). | Registry memory, file handle stability, append latency under high fan-out. |
| **P4-Proto** | Queue Churn | 10,000 to 100,000 concurrent active leases. High ACK/NACK/Timeout churn. Out-of-order ACKs. | State plane CPU, bitmap memory, lease acquisition latency. |
| **P5-Proto** | Export Overhead | P1-Proto workload + continuous background Parquet export from sealed segments. | Compaction/export interference on hot-path write latency (must be ≤ 5% jitter). |
| **P6-Proto** | Degraded / Backpressure | P1-Proto workload + artificial S3 export lag or disk I/O throttling. | Backpressure ladder engagement, TCP clamping, graceful degradation. |

---

## 5. Metrics and Telemetry Taxonomy

The harness MUST collect and report the following metrics. Averages are strictly forbidden for latency reporting; histograms and percentiles MUST be used.

### 5.1 Latency Metrics (Histograms)

| Metric | Unit | Required Percentiles |
|---|---|---|
| `keirox_wal_append_latency_seconds` | seconds | p50, p90, p99, p999, max |
| `keirox_stream_read_latency_seconds` | seconds | p50, p99 |
| `keirox_lease_acquisition_latency_seconds` | seconds | p50, p99 |
| `keirox_ack_processing_latency_seconds` | seconds | p50, p99 |
| `keirox_parquet_export_latency_seconds` | seconds | p50, p99 |

### 5.2 Throughput Metrics (Counters/Gauges)

| Metric | Unit |
|---|---|
| `keirox_ingest_messages_per_second` | msgs/s |
| `keirox_ingest_bytes_per_second` | MB/s |
| `keirox_ack_operations_per_second` | ops/s |
| `keirox_export_bytes_per_second` | MB/s |

### 5.3 Resource & State Metrics (Gauges)

| Metric | Unit | Purpose |
|---|---|---|
| `keirox_process_resident_memory_bytes` | bytes | Total process RSS. |
| `keirox_bitmap_memory_bytes` | bytes | State plane overlay footprint. |
| `keirox_lease_table_memory_bytes` | bytes | Active lease map footprint. |
| `keirox_open_file_descriptors` | count | Verify O(1) FD behavior vs stream count. |
| `keirox_nvme_write_iops` | iops | Storage layer stress. |
| `keirox_nvme_write_bandwidth_bytes` | bytes/s | Storage layer throughput. |
| `keirox_watermark_lag_offsets` | count | Distance between head and $W_{base}$. |

---

## 6. Environmental Disclosure Requirements

**Normative Rule:** A benchmark result without full environmental disclosure is invalid and MUST NOT be included in the Phase 1 Evidence Package.

Every benchmark run MUST automatically capture and embed the following metadata into the final report:

| Category | Required Data Points |
|---|---|
| **Hardware** | CPU model, core count, NUMA nodes, RAM size/type, NVMe model & capacity. |
| **OS / Kernel** | Linux distribution, Kernel version, `io_uring` support status, filesystem type (e.g., ext4, xfs). |
| **Software** | Rust toolchain version, build profile (`--release`), allocator (e.g., jemalloc, mimalloc). |
| **Topology** | Number of nodes (1 for prototype), network interface speed, co-located processes. |
| **Configuration** | Git commit hash, segment size, batch size, compaction thread count. |

---

## 7. Harness Architecture

The benchmark harness will be implemented primarily in the `keirox-bench` crate.

### 7.1 Component Topology

```text
┌────────────────────────────────────────────────────────────┐
│                    BENCHMARK HARNESS                       │
│                                                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ Load Gen:    │  │ Load Gen:    │  │ Load Gen:    │     │
│  │ Producers    │  │ Stream Read  │  │ Queue Workers│     │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘     │
│         │                 │                 │             │
│         └────────────┬────┴────┬────────────┘             │
│                      │         │                           │
│                      ▼         ▼                           │
│  ┌──────────────────────────────────────────────────────┐ │
│  │           METRICS AGGREGATOR (Prometheus/OTel)       │ │
│  │  - Histogram buckets for latency                     │ │
│  │  - Rate calculations for throughput                  │ │
│  └──────────────────────────┬───────────────────────────┘ │
│                             │                             │
│                             ▼                             │
│  ┌──────────────────────────────────────────────────────┐ │
│  │           REPORT GENERATOR (Markdown / JSON)         │ │
│  │  - Merges load gen stats + engine metrics + env data │ │
│  └──────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────┘
                             │
                             ▼ (Target)
               ┌──────────────────────────┐
               │   KEIROX PROTOTYPE NODE  │
               └──────────────────────────┘
```

### 7.2 Load Generator Design

- **Producers:** Async Rust clients using `tokio`. Capable of rate-limiting to exact target MB/s or running in "max throughput" mode.
- **Stream Readers:** Sequential offset polling.
- **Queue Workers:** Concurrent lease acquisition, simulated processing delay (e.g., 1ms-10ms), followed by ACK/NACK.
- **Coordination:** Load generators MUST synchronize their start times and use a shared atomic clock to ensure latency measurements are accurate.

---

## 8. Benchmark Execution Protocol

Every benchmark run MUST follow this strict sequence to ensure statistical validity.

### 8.1 Execution Phases

1. **Environment Profiling (1 min):** Capture hardware, OS, and Git metadata.
2. **Warmup (5 mins):** Run workload at 50% target rate to prime OS page caches, JIT (if applicable), and allocator arenas. Metrics collected here are discarded.
3. **Steady-State (15 mins):** Run workload at 100% target rate. This is the primary measurement window.
4. **Cooldown (2 mins):** Stop load. Allow background compaction and flushes to complete.
5. **Extraction & Reporting (1 min):** Harvest metrics, calculate percentiles, generate report.

### 8.2 Statistical Rules

- **Latency:** MUST be reported using HDR Histograms or equivalent high-resolution structures to avoid coordinate compression artifacts.
- **Outliers:** p999 and max latencies MUST be reported, but pass/fail gates are evaluated on p99.
- **Throughput:** Reported as the mean rate over the Steady-State window.
- **Error Rate:** Any run with an error rate > 0.01% (excluding intentional backpressure rejections in P6-Proto) is automatically marked as FAILED.

---

## 9. Pass/Fail Thresholds (Phase 1 Prototype)

These thresholds map directly to the Mandatory Targets defined in KEI-SPIKE-001.

| Profile | Metric | Mandatory Target (PASS) | Stretch Target |
|---|---|---|---|
| **P1-Proto** | Ingest Throughput | ≥ 50 MB/s | ≥ 100 MB/s |
| **P1-Proto** | Append Latency (p99) | ≤ 2.0 ms | ≤ 1.5 ms |
| **P1-Proto** | Error Rate | 0% | 0% |
| **P3-Proto** | 100K Streams Append p99 | ≤ 3.0 ms | ≤ 2.0 ms |
| **P3-Proto** | File Descriptor Count | O(1) / Stable | O(1) / Stable |
| **P4-Proto** | 100K Leases Acq. p99 | ≤ 1.0 ms | ≤ 0.5 ms |
| **P4-Proto** | ACK Latency p99 | ≤ 1.0 ms | ≤ 0.5 ms |
| **P5-Proto** | Export Interference | p99 jitter ≤ 5% vs P1 | p99 jitter ≤ 2% |
| **P6-Proto** | Backpressure Engage | Engages before OOM/Disk Full | N/A |

**Normative Rule:** Failing a Mandatory Target blocks the GO decision at the Prototype Evidence Gate (KEI-SPIKE-001 M1.4). Failing a Stretch Target does not block the GO decision but must be documented.

---

## 10. Deliverables

| Deliverable | Description | Due |
|---|---|---|
| D-B-001 | `keirox-bench` crate skeleton and load generator CLI. | Week 4 |
| D-B-002 | Metrics aggregator and HDR Histogram integration. | Week 5 |
| D-B-003 | Environment profiler (hardware/OS/Git capture). | Week 6 |
| D-B-004 | P1-Proto and P3-Proto automated execution scripts. | Week 8 |
| D-B-005 | P4-Proto and P5-Proto automated execution scripts. | Week 10 |
| D-B-006 | Markdown/JSON Report Generator. | Week 11 |
| D-B-007 | Final Phase 1 Prototype Benchmark Evidence Package. | Week 12 |

---

## 11. Risks and Mitigations

| Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|
| **Co-location Noise:** Benchmarks run on shared CI runners or noisy VMs. | High | High | Mandate bare-metal or dedicated isolated VMs for official evidence runs. |
| **NVMe Thermal Throttling:** Sustained 100MB/s writes cause drive to throttle. | Medium | Medium | Monitor drive temperature; ensure adequate cooling or adjust burst durations. |
| **Measurement Observer Effect:** Metrics collection consumes CPU and skews latency. | Medium | Medium | Use low-overhead eBPF or highly optimized Prometheus client; isolate metrics threads. |
| **Histogram Memory Bloat:** High-resolution histograms consume too much RAM. | Low | Medium | Use logarithmic bucket boundaries; limit significant figures. |
| **OS Jitter:** Kernel background tasks cause p999 spikes. | Medium | High | Isolate CPU cores for hot threads (`taskset`/`cgroups`); document OS jitter in report. |

---

## 12. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial Performance Validation Harness Plan. Defines workload profiles, metrics taxonomy, environmental disclosure rules, harness architecture, execution protocol, and Phase 1 pass/fail thresholds. |