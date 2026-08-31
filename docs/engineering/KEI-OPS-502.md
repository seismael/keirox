---
id: KEI-OPS-502
title: Keirox Observability, Telemetry & Admin Console
version: 1.0
phase: Phase 5
status: Approved
authority: Chief Architect
last_updated: 2026-08-31
---

# KEI-OPS-502 — Keirox Observability, Telemetry & Admin Console

## 1. Observability Principles
Keirox follows a strict "white-box" observability model. All node state, network boundaries, and storage operations MUST be transparently exposed via Prometheus metrics.

### 1.1 Metrics Port
- The Keirox daemon exposes a dedicated HTTP listener on port `9090` (default) for observability.
- The `/metrics` endpoint serves Prometheus-formatted telemetry.
- The `/health` endpoint serves a JSON-formatted `HealthProbeReport`.

### 1.2 Telemetry Subsystems
The `TelemetryRegistry` tracks:
1. **Network Ingress**: Bytes/sec, messages/sec, active connections.
2. **Storage Latency**: WAL append `p99`, `fsync` durations.
3. **Memory Backpressure**: Arena allocations, active leases, RoaringBitmap sizes.
4. **Replication/Consensus**: Raft term, commit index, HLC divergence.

## 2. Admin Console & CLI (D-L1 Resolution)
The CLI tool `keirox-server` operates in two modes:
1. **Daemon Mode**: (`start`) Bootstraps the fabric.
2. **Admin Mode**: (`status`, `metrics`, `inspect-*`) Functions as a stateless client to the running daemon.

### 2.1 CLI-to-Daemon Protocol
Admin commands MUST interact with the running daemon rather than instantiating detached in-memory objects. They should query the `http://127.0.0.1:9090/` or similar admin endpoints to extract live runtime statistics.

## 3. Grafana Dashboards
A canonical set of Grafana JSON dashboards is maintained for operator use, covering:
- **Cluster Health**: Overview of quorum status and node up-time.
- **Throughput & Saturation**: End-to-end pipeline speed (producer -> WAL -> consumer -> ELT).
- **Errors & Drops**: Dead-letter queue eviction rates and API errors.
