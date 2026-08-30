# Benchmark Plans & Results

This directory contains benchmark execution methodologies, raw telemetry data, and reproducible performance reports for the Keirox Polymorphic Event Fabric.

---

## ⚡ Fast Reference

- **Methodology & Workload Profiles (P1–P6)**: [`docs/architecture/KEI-OPS-041.md`](../architecture/KEI-OPS-041.md)
- **Harness Architecture & Telemetry Taxonomy**: [`docs/engineering/KEI-BENCH-001.md`](../engineering/KEI-BENCH-001.md)
- **Implementation Crate**: [`crates/keirox-bench/`](../../crates/keirox-bench/)

---

## 📊 Canonical Workload Profiles

| Profile ID | Workload Type | Key Target Metrics | Governing Spec |
|---|---|---|---|
| **P1** | Extreme Low Latency (1KB, Tier-0 NVMe) | Append p99 $\le 2.0\text{ms}$, Ingest $\ge 100\text{ MB/s}$ | [`KEI-ARC-011`](../architecture/KEI-ARC-011.md) |
| **P2** | High Throughput Streaming | Ingest $\ge 1\text{ GB/s}$, sustained batching | [`KEI-ARC-011`](../architecture/KEI-ARC-011.md) |
| **P3** | Massive Micro-Stream Fanout | $100\text{K}–1\text{M}$ active virtual streams, O(1) FDs | [`KEI-ARC-020`](../architecture/KEI-ARC-020.md) |
| **P4** | Mixed Stream & Queue Churn | $100\text{K}$ concurrent active leases, ACK/NACK churn | [`KEI-ARC-021`](../architecture/KEI-ARC-021.md) |
| **P5** | Columnar ELT Export | Background Parquet encoding jitter $\le 5\%$ vs P1 | [`KEI-ARC-023`](../architecture/KEI-ARC-023.md) |
| **P6** | Multi-Region WAN Replication | HLC causal consistency, cross-region lag | [`KEI-ARC-026`](../architecture/KEI-ARC-026.md) |
