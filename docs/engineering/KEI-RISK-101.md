# KEI-RISK-101 — Risk Reduction and Go/No-Go Plan

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-RISK-101 |
| Title | Risk Reduction and Go/No-Go Plan |
| Version | 1.0 |
| Level | Engineering Execution Plan |
| Status | Baseline — Ready for Execution |
| Phase | Phase 1 Engineering Bridge + Full Phase 1 |
| Owner | Engineering Program Lead / Chief Architect |
| Governing Plan | KEI-ENG-100 — Phase 1 Engineering Execution Plan |
| Related Plans | KEI-SPIKE-101, KEI-FORMAL-101, KEI-BENCH-101 |
| Governing Architecture Documents | KEI-ARC-010..027, KEI-DES-030..036, KEI-VAL-050..052 |

---

## 2. Executive Summary

The Keirox Polymorphic Event Fabric (PEF) introduces novel paradigms—specifically the Log-Bitmap Duality, LSM-WAL multiplexing, and Internalized Columnar ELT—that deliberately break legacy distributed system dogmas. While the architecture has been mathematically bounded and audited (KEI-VAL-050), translating these paradigms into a production-grade Rust binary carries significant technical and execution risk.

This document defines the **Risk Management Framework** for Phase 1. It identifies the highest-severity technical and organizational risks, establishes concrete mitigation strategies, and defines the strict, evidence-based **Go/No-Go Gates** that must be passed before the project can proceed from the 90-day Prototype into full distributed cluster development.

---

## 3. Risk Management Methodology

### 3.1 Risk Scoring Matrix

Risks are evaluated on a 5x5 matrix based on **Likelihood** and **Impact**.

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
| **15–25** | **Critical** | Immediate mitigation required. Blocks milestone exit if unresolved. |
| **8–14** | **High** | Active mitigation plan required. Reviewed weekly. |
| **4–7** | **Medium** | Monitor and prepare contingency. Reviewed biweekly. |
| **1–3** | **Low** | Accept and monitor. |

---

## 4. Top Technical Risks & Mitigation Strategies

These are the highest-risk engineering challenges identified during the architecture phase. They must be actively de-risked during the 90-day Prototype (KEI-SPIKE-101).

### 4.1 Risk: io_uring / O_DIRECT Complexity and Kernel Dependencies
* **Description:** The hot write path relies on `io_uring` with `O_DIRECT` to achieve <2ms p99 latency. Bugs in ring buffer management, page alignment, or kernel version incompatibilities could cause data corruption or severe latency spikes.
* **Score:** 20 (Likely × Critical)
* **Mitigation:**
  1. Abstract the I/O layer behind a `WalWriter` trait.
  2. Implement a fallback `std::fs` + `O_SYNC` path for environments lacking `io_uring`.
  3. Use `KEI-DES-030` strict 4096-byte page alignment and CRC32C checks to guarantee corruption is detected, not silently written.
* **Contingency:** If `io_uring` proves too unstable in the target kernel, fall back to a dedicated thread-pool using standard blocking I/O with `O_DIRECT`, accepting a slight p99 latency penalty.

### 4.2 Risk: Roaring Bitmap Memory Fragmentation under Lease Churn
* **Description:** High-churn queue workloads (millions of leases/ACKs/NACKs per second) could cause the Roaring Bitmap containers to fragment, leading to unbounded memory growth and GC-like CPU spikes during container conversion (Array $\leftrightarrow$ Bitset).
* **Score:** 16 (Likely × Major)
* **Mitigation:**
  1. Implement the mandatory DLQ eviction and Sliding Base Watermark ($W_{base}$) purge logic early in the prototype to guarantee memory reclamation.
  2. Implement Adaptive Container Spilling to NVMe when a shard exceeds the 256 MB memory quota.
  3. Run the P4-Proto (Queue Churn) benchmark for 72 hours to measure memory drift.
* **Contingency:** If Roaring Bitmaps fail to bound memory under extreme fragmentation, pivot the state plane to a disk-backed embedded KV store (e.g., RocksDB/Sled) for the lease state, accepting a latency trade-off.

### 4.3 Risk: Compaction CPU Jitter (Internalized ELT)
* **Description:** Background threads transposing JSON rows into Arrow RecordBatches and Parquet files may steal CPU cache and memory bandwidth from the hot ingress path, causing p99 write latency to violate the <2ms SLA.
* **Score:** 15 (Possible × Critical)
* **Mitigation:**
  1. Strict CPU core pinning (`sched_setaffinity`) via `cgroups`. Hot-path threads get dedicated cores; compaction threads are isolated.
  2. Implement the progressive backpressure ladder (KEI-ARC-027) to throttle ingress if the compaction backlog exceeds 80%.
* **Contingency:** If CPU isolation is insufficient on standard cloud VMs, disable in-broker shredding by default and fall back to opaque byte storage, requiring external Flink/Spark for ELT (abandoning the "Internalized ELT" pillar for v1).

### 4.4 Risk: S3 API Throttling (503 Slow Down)
* **Description:** A massive ingress burst could cause the asynchronous Tier-1 S3 uploader to hit AWS S3 prefix throttling limits. If the Tier-0 NVMe buffer fills up before S3 accepts the data, the node will crash or drop writes.
* **Score:** 12 (Possible × Major)
* **Mitigation:**
  1. Implement S3 key hash-prefix partitioning to distribute PUT requests across multiple S3 prefixes.
  2. Size the Tier-0 NVMe buffer to hold at least 4 hours of peak ingress (Elastic Backlog).
  3. Engage TCP window clamping at the gateway edge when NVMe reaches 80% capacity.
* **Contingency:** None. The backpressure ladder MUST protect the node. If S3 is down for >4 hours, the system MUST reject new writes (Fail Secure) rather than corrupting the NVMe disk.

### 4.5 Risk: Scope Creep into Superseded Paradigms
* **Description:** Engineers or stakeholders attempt to reintroduce rejected whitepaper concepts (e.g., CXL/RDMA hardware disaggregation, in-broker SQL materialized views, 10M streams/node default) which threaten the delivery timeline.
* **Score:** 15 (Possible × Critical)
* **Mitigation:**
  1. Strict enforcement of the PR Traceability Requirement (KEI-ENG-100 §13).
  2. Any deviation from the approved L2/L3 architecture requires a formal ADR and Architecture Review Board (ARB) approval.
  3. The `INDEX.md` explicitly lists superseded claims as "Banned".

---

## 5. Go/No-Go Gate Framework

The project is governed by strict evidence gates. Progression to the next phase is **impossible** without passing the preceding gate.

### 5.1 Gate 1: Prototype Evidence Gate (End of Week 12)
* **Context:** End of the 90-day Minimum Vertical Prototype (KEI-SPIKE-101).
* **Go Criteria:**
  1. Single-node append, lease, ACK, and watermark flow works end-to-end.
  2. P1-Proto benchmark achieves ≥50 MB/s throughput with p99 ≤ 2ms.
  3. 72-hour soak test shows zero unbounded memory growth.
  4. `kill -9` recovery successfully restores state without data loss.
  5. Parquet export is queryable by DuckDB.
* **No-Go Consequence:** Project halts. Architecture assumptions are fundamentally flawed. Requires pivot or termination.

### 5.2 Gate 2: Phase 1 Certification Gate (End of Month 9)
* **Context:** End of single-node hardening and scale testing.
* **Go Criteria:**
  1. 100,000 virtual streams stable; 1,000,000 validated under controlled benchmark.
  2. 100,000 concurrent active leases stable.
  3. WAF (Write Amplification Factor) measured at ≤ 1.35.
  4. Formal TLA+ models (KEI-FORMAL-101) pass with zero unresolved counterexamples.
  5. All Phase 1 runbooks (KEI-OPS-040) tested and validated.
* **No-Go Consequence:** Phase 1 is extended. Distributed consensus (Phase 2) cannot begin until single-node invariants are mathematically and empirically proven.

### 5.3 Gate 3: Phase 2 Distributed Durability Gate (End of Month 18)
* **Context:** End of multi-node Raft clustering and S3 offload hardening.
* **Go Criteria:**
  1. Zero data loss (JML=0) during automated `kill -9` node failure simulations.
  2. Coordinator failover and lease reassignment complete in <3.5 seconds.
  3. Jepsen-style chaos tests pass under network partitions and clock skew.
* **No-Go Consequence:** Product cannot enter Beta or customer preview.

---

## 6. Pivot and Contingency Strategies

If a core architectural pillar fails during the Prototype or Phase 1, the following pivots are pre-authorized by the Architecture Review Board:

| Failing Pillar | Trigger Condition | Authorized Pivot |
|---|---|---|
| **Internalized ELT** | Compaction CPU jitter cannot be isolated; p99 latency > 5ms. | Disable in-broker shredding. Store opaque bytes. Rely on external Flink/Spark for Parquet conversion. (Loss of "Zero-ETL" value prop). |
| **Log-Bitmap Duality** | Roaring Bitmaps consume too much RAM under high-cardinality queue churn. | Restrict Queue mode to small payloads only. Force large payload queues to use standard Stream replay with application-side ACK tracking. |
| **100K+ Streams** | Stream registry memory exceeds 224 bytes/stream; OOM at 50K streams. | Abandon multi-tenant micro-stream focus. Pivot to standard Kafka-like partition limits (4,000 streams/node) and require external routing. |
| **io_uring / O_DIRECT** | Kernel panics, data corruption, or unresolvable latency spikes. | Fall back to standard OS Page Cache + `fsync`. Accept p99 latency of 5-10ms (matching legacy Kafka). |

---

## 7. Risk Register Template

All risks must be tracked in the project issue tracker (Jira/Linear) using this schema:

```text
Issue Type: Risk
Risk ID: RISK-XXX
Title: [Short Description]
Score: [Likelihood 1-5] x [Impact 1-5] = [Total]
Severity: [Critical / High / Medium / Low]
Owner: [Engineer Name]
Mitigation Strategy: [What we are doing to prevent it]
Contingency Plan: [What we do if it happens anyway]
Status: [Open / Mitigating / Realized / Closed]
```

---

## 8. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial Risk Reduction and Go/No-Go Plan. Defines risk scoring, top technical risks (io_uring, Roaring Bitmaps, Compaction, S3 throttling), Go/No-Go gates, and authorized pivot strategies. |