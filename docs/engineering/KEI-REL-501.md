# KEI-OPS-502 — Day-2 Observability & Web Console Plan

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-OPS-502 |
| Title | Day-2 Observability & Web Console Plan |
| Version | 1.0 |
| Level | Engineering Execution Plan |
| Status | Baseline — Ready for Execution |
| Phase | Phase 5 — Productization, Distribution & Day-2 Operations |
| Duration | Weeks 12–22 of Phase 5 |
| Owner | Observability Lead / Platform Engineering Lead |
| Governing Plan | KEI-ENG-500 — Phase 5 Productization & Distribution Plan |
| Governing Architecture Documents | KEI-ARC-027 (Operability), KEI-DES-032 (API), KEI-OPS-040 (Runbooks) |
| Predecessor | KEI-REL-501 (Secure Supply Chain & Release Engineering) |

---

## 2. Executive Summary

A distributed system is only as operable as its visibility. Phases 1 through 4 built the Keirox engine with internal metrics, tracing, and logging. But enterprise operations teams do not write gRPC queries to debug stuck watermarks, inspect DLQ messages, or trigger point-in-time recovery. They use **Grafana dashboards**, **Prometheus alerts**, **OpenTelemetry traces**, and **web-based consoles**.

This plan defines the **Day-2 Observability Packaging** and **Web Operations Console** program. It transforms Keirox's internal telemetry into:

1. **Pre-built Grafana dashboards** for cluster health, streams, state plane, lakehouse, gateways, and security.
2. **Prometheus recording rules** for SLO metrics and pre-computed aggregations.
3. **Prometheus alert rules** mapped to operational runbooks.
4. **OpenTelemetry auto-instrumentation** for SDK-level trace propagation.
5. **Datadog and New Relic integrations** for enterprise observability stacks.
6. **A Web-based Operations Console** for stream inspection, DLQ management, PITR triggers, and break-glass operations.

Without this program, Keirox is a black box that requires engineering expertise to operate. With it, platform teams can run Keirox confidently using their existing toolchains.

---

## 3. Purpose and Scope

### 3.1 Purpose

The purpose of this plan is to:

1. Package Keirox internal metrics into enterprise-ready Grafana dashboards.
2. Define Prometheus recording and alert rules.
3. Implement OpenTelemetry auto-instrumentation for SDKs and gateways.
4. Build Datadog and New Relic native integrations.
5. Design and build the Web Operations Console.
6. Define SLO tracking and error budget alerting.
7. Produce the Day-2 observability certification evidence package.

### 3.2 Scope

**In scope:**

- Grafana dashboard suite (7 dashboards).
- Prometheus recording rules.
- Prometheus alert rules with runbook links.
- OpenTelemetry auto-instrumentation (Rust SDK, Go SDK, gateways).
- Datadog native integration.
- New Relic integration.
- Loki log pipeline integration.
- SLO definitions and error budget tracking.
- Web Operations Console (React/TypeScript).
- Console read-only mode and break-glass workflow.
- Console audit trail integration.

**Out of scope:**

- Core engine telemetry implementation (complete in Phases 1–4).
- Kubernetes Operator (owned by KEI-K8S-501).
- CLI tooling (owned by KEI-ENG-500 WP-P5-B).
- Migration tooling (owned by KEI-MIG-501).

### 3.3 Observability Constraints

1. All dashboards MUST be deployable via Helm chart or Grafana operator.
2. All alert rules MUST link to operational runbooks.
3. The Web Console MUST default to read-only mode.
4. All console write operations MUST require explicit confirmation and produce audit events.
5. OpenTelemetry instrumentation MUST NOT add more than 1% CPU overhead.
6. Datadog/New Relic integrations MUST NOT require code changes to the core engine.
7. SLO definitions MUST be configurable per tenant.

---

## 4. Grafana Dashboard Suite

### 4.1 Dashboard Architecture

```text
┌────────────────────────────────────────────────────────────────────────────┐
│                        KEIROX GRAFANA DASHBOARDS                           │
│                                                                            │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │                    CLUSTER OVERVIEW (Home)                            │  │
│  │  - Cluster health status (green/yellow/red)                          │  │
│  │  - Active streams count                                              │  │
│  │  - Total ingest throughput                                           │  │
│  │  - Active leases count                                               │  │
│  │  - Iceberg freshness                                                  │  │
│  │  - Open alerts                                                       │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                                                            │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐     │
│  │ Stream       │ │ State Plane  │ │ Lakehouse    │ │ Gateway      │     │
│  │ Throughput   │ │ Health       │ │ Health       │ │ Health       │     │
│  └──────────────┘ └──────────────┘ └──────────────┘ └──────────────┘     │
│                                                                            │
│  ┌──────────────┐ ┌──────────────┐                                        │
│  │ Security     │ │ Capacity     │                                        │
│  │ Events       │ │ & Resources  │                                        │
│  └──────────────┘ └──────────────┘                                        │
└────────────────────────────────────────────────────────────────────────────┘
```

### 4.2 Dashboard Specifications

#### Dashboard 1: Cluster Overview

| Panel | Data Source | Query |
|---|---|---|
| Cluster Health | Prometheus | `keirox_cluster_health_status` |
| Active Streams | Prometheus | `count(keirox_stream_active)` |
| Ingest Throughput | Prometheus | `sum(rate(keirox_ingest_messages_total[5m]))` |
| Ingest Bytes/s | Prometheus | `sum(rate(keirox_ingest_bytes_total[5m]))` |
| Active Leases | Prometheus | `sum(keirox_active_leases)` |
| Iceberg Freshness | Prometheus | `max(keirox_iceberg_snapshot_age_seconds)` |
| Raft Leader Status | Prometheus | `keirox_raft_is_leader` |
| Open Alerts | Alertmanager | `ALERTS{alertstate="firing"}` |

#### Dashboard 2: Stream Throughput

| Panel | Data Source | Query |
|---|---|---|
| Ingest Rate by Stream | Prometheus | `sum by (stream) (rate(keirox_ingest_messages_total[5m]))` |
| Read Rate by Stream | Prometheus | `sum by (stream) (rate(keirox_stream_read_total[5m]))` |
| Backlog by Stream | Prometheus | `keirox_stream_backlog_offsets` |
| Write Latency p99 | Prometheus | `histogram_quantile(0.99, rate(keirox_wal_append_latency_seconds_bucket[5m]))` |
| Read Latency p99 | Prometheus | `histogram_quantile(0.99, rate(keirox_stream_read_latency_seconds_bucket[5m]))` |
| Error Rate by Stream | Prometheus | `sum by (stream) (rate(keirox_stream_errors_total[5m]))` |

#### Dashboard 3: State Plane Health

| Panel | Data Source | Query |
|---|---|---|
| Active Leases | Prometheus | `sum(keirox_active_leases)` |
| Watermark Lag | Prometheus | `max(keirox_watermark_lag_offsets)` |
| Bitmap Memory | Prometheus | `sum(keirox_bitmap_memory_bytes)` |
| Spilled Bitmap Bytes | Prometheus | `sum(keirox_bitmap_spilled_bytes)` |
| DLQ Count | Prometheus | `sum(keirox_dlq_entries_total)` |
| Lease Acquisition Latency p99 | Prometheus | `histogram_quantile(0.99, rate(keirox_lease_acquisition_latency_seconds_bucket[5m]))` |
| ACK Latency p99 | Prometheus | `histogram_quantile(0.99, rate(keirox_ack_latency_seconds_bucket[5m]))` |
| Coordinator Shard Distribution | Prometheus | `count by (coordinator) (keirox_coordinator_shards)` |

#### Dashboard 4: Lakehouse Health

| Panel | Data Source | Query |
|---|---|---|
| Iceberg Snapshot Age | Prometheus | `max(keirox_iceberg_snapshot_age_seconds)` |
| Commit Latency p99 | Prometheus | `histogram_quantile(0.99, rate(keirox_iceberg_commit_latency_seconds_bucket[5m]))` |
| Pending Files | Prometheus | `sum(keirox_iceberg_pending_files_count)` |
| Pending Bytes | Prometheus | `sum(keirox_iceberg_pending_files_bytes)` |
| Manifest Count | Prometheus | `sum(keirox_iceberg_manifest_count)` |
| Snapshot Count | Prometheus | `sum(keirox_iceberg_snapshot_count)` |
| Small File Count | Prometheus | `sum(keirox_iceberg_small_file_count)` |
| Orphan File Count | Prometheus | `sum(keirox_iceberg_orphan_files_count)` |

#### Dashboard 5: Gateway Health

| Panel | Data Source | Query |
|---|---|---|
| Request Rate by Protocol | Prometheus | `sum by (protocol) (rate(keirox_gateway_requests_total[5m]))` |
| Error Rate by Protocol | Prometheus | `sum by (protocol) (rate(keirox_gateway_errors_total[5m]))` |
| Unsupported Requests | Prometheus | `sum(rate(keirox_gateway_unsupported_requests_total[5m]))` |
| Translation Latency p99 | Prometheus | `histogram_quantile(0.99, rate(keirox_gateway_translation_latency_seconds_bucket[5m]))` |
| Auth Failures | Prometheus | `sum(rate(keirox_gateway_auth_failures_total[5m]))` |
| Connected Clients | Prometheus | `sum(keirox_gateway_connected_clients)` |

#### Dashboard 6: Security Events

| Panel | Data Source | Query |
|---|---|---|
| Auth Failures | Prometheus | `sum(rate(keirox_auth_failures_total[5m]))` |
| ABAC Denials | Prometheus | `sum(rate(keirox_authz_denials_total[5m]))` |
| Cross-Tenant Denials | Prometheus | `sum(rate(keirox_cross_tenant_denials_total[5m]))` |
| KMS Errors | Prometheus | `sum(rate(keirox_kms_errors_total[5m]))` |
| Crypto-Shred Events | Prometheus | `sum(keirox_crypto_shred_total)` |
| Destroyed Key Access Attempts | Prometheus | `sum(rate(keirox_destroyed_key_access_attempts_total[5m]))` |

#### Dashboard 7: Capacity & Resources

| Panel | Data Source | Query |
|---|---|---|
| NVMe Usage | Prometheus | `keirox_nvme_used_bytes / keirox_nvme_total_bytes` |
| NVMe Backlog ETA | Prometheus | `keirox_nvme_backlog_eta_seconds` |
| S3 Upload Backlog | Prometheus | `keirox_s3_upload_backlog_bytes` |
| Memory Usage by Component | Prometheus | `sum by (component) (keirox_component_memory_bytes)` |
| CPU Usage by Component | Prometheus | `sum by (component) (rate(keirox_component_cpu_seconds_total[5m]))` |
| File Descriptor Count | Prometheus | `keirox_open_file_descriptors` |
| Network I/O | Prometheus | `rate(keirox_network_bytes_total[5m])` |

### 4.3 Dashboard Delivery

| Delivery Method | Description |
|---|---|
| Helm chart | Dashboards deployed as ConfigMaps via Helm subchart |
| Grafana operator | Dashboards deployed via `GrafanaDashboard` CRDs |
| JSON export | Standalone JSON files for manual import |
| Grafana.com | Published to Grafana.com dashboard registry |

### 4.4 Dashboard Certification Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| DASH-T-001 | Deploy dashboards via Helm | All 7 dashboards appear in Grafana |
| DASH-T-002 | Deploy dashboards via Grafana operator | All 7 dashboards appear in Grafana |
| DASH-T-003 | Dashboard renders with live data | All panels show data |
| DASH-T-004 | Dashboard renders with no data | Panels show "No data" gracefully |
| DASH-T-005 | Dashboard variables work | Stream/tenant filtering works |
| DASH-T-006 | Dashboard JSON is valid | JSON schema validation passes |

---

## 5. Prometheus Recording Rules

### 5.1 SLO Recording Rules

```yaml
groups:
  - name: keirox-slo-recording-rules
    interval: 30s
    rules:
      # Ingest availability SLO (99.95%)
      - record: keirox:slo_ingest_availability:ratio_rate5m
        expr: |
          1 - (
            sum(rate(keirox_ingest_errors_total{code!~"4.."}[5m]))
            /
            sum(rate(keirox_ingest_requests_total[5m]))
          )

      # Write latency SLO (p99 < 2ms)
      - record: keirox:slo_write_latency_p99:seconds
        expr: |
          histogram_quantile(0.99,
            sum(rate(keirox_wal_append_latency_seconds_bucket[5m])) by (le)
          )

      # Iceberg freshness SLO (< 60s)
      - record: keirox:slo_iceberg_freshness:seconds
        expr: |
          max(keirox_iceberg_snapshot_age_seconds)

      # Error budget remaining (30-day window)
      - record: keirox:slo_ingest_error_budget:ratio_rate30d
        expr: |
          1 - (
            sum(rate(keirox_ingest_errors_total{code!~"4.."}[30d]))
            /
            sum(rate(keirox_ingest_requests_total[30d]))
          )
```

### 5.2 Aggregation Recording Rules

```yaml
groups:
  - name: keirox-aggregation-recording-rules
    interval: 30s
    rules:
      # Per-tenant throughput
      - record: keirox:tenant_ingest_messages:rate5m
        expr: sum by (tenant) (rate(keirox_ingest_messages_total[5m]))

      # Per-stream throughput
      - record: keirox:stream_ingest_messages:rate5m
        expr: sum by (stream) (rate(keirox_ingest_messages_total[5m]))

      # Cluster-wide throughput
      - record: keirox:cluster_ingest_messages:rate5m
        expr: sum(rate(keirox_ingest_messages_total[5m]))

      # Cluster-wide active leases
      - record: keirox:cluster_active_leases:sum
        expr: sum(keirox_active_leases)

      # Raft quorum health
      - record: keirox:raft_quorum_healthy:bool
        expr: |
          count(keirox_raft_is_leader == 1) == 1
          and
          count(keirox_raft_member_healthy == 1) >= 2
```

---

## 6. Prometheus Alert Rules

### 6.1 Alert Definitions

| Alert | Condition | Severity | Runbook |
|---|---|---|---|
| `KeiroxClusterDown` | No metrics for 5 minutes | Critical | OPS-RB-001 |
| `KeiroxRaftQuorumLost` | Healthy members < 2 | Critical | OPS-RB-002 |
| `KeiroxWriteLatencyHigh` | p99 > 5ms for 10 minutes | Warning | OPS-RB-003 |
| `KeiroxWriteLatencyCritical` | p99 > 10ms for 5 minutes | Critical | OPS-RB-003 |
| `KeiroxWatermarkStuck` | Watermark lag > 100K for 15 minutes | Warning | OPS-RB-019 |
| `KeiroxWatermarkStuckCritical` | Watermark lag > 1M for 15 minutes | Critical | OPS-RB-019 |
| `KeiroxNVMeNearlyFull` | NVMe usage > 90% | Warning | OPS-RB-014 |
| `KeiroxNVMeCritical` | NVMe usage > 95% | Critical | OPS-RB-014, OPS-RB-017 |
| `KeiroxS3UploadBacklog` | S3 backlog > 10GB | Warning | OPS-RB-014 |
| `KeiroxS3UploadBacklogCritical` | S3 backlog > 50GB | Critical | OPS-RB-014 |
| `KeiroxIcebergFreshnessDegraded` | Snapshot age > 120s | Warning | OPS-RB-015 |
| `KeiroxIcebergFreshnessCritical` | Snapshot age > 300s | Critical | OPS-RB-015 |
| `KeiroxCoordinatorFailover` | Coordinator failover event | Info | OPS-RB-002 |
| `KeiroxAuthFailureSpike` | Auth failures > 100/min | Warning | SEC-RB-001 |
| `KeiroxCrossTenantDenial` | Cross-tenant denial detected | Critical | SEC-RB-002 |
| `KeiroxKMSError` | KMS errors > 10/min | Critical | SEC-RB-003 |
| `KeiroxCryptoShredEvent` | Crypto-shred executed | Info | SEC-RB-004 |
| `KeiroxErrorBudgetBurn` | Error budget burn rate > 14x | Warning | SLO-RB-001 |
| `KeiroxErrorBudgetBurnCritical` | Error budget burn rate > 30x | Critical | SLO-RB-001 |

### 6.2 Alert Rule Example

```yaml
groups:
  - name: keirox-critical-alerts
    rules:
      - alert: KeiroxNVMeCritical
        expr: |
          (keirox_nvme_used_bytes / keirox_nvme_total_bytes) > 0.95
        for: 5m
        labels:
          severity: critical
          team: platform
        annotations:
          summary: "NVMe storage critically full ({{ $value | humanizePercentage }})"
          description: "Node {{ $labels.instance }} NVMe usage is above 95%. Backpressure should be engaging. Check OPS-RB-014 and OPS-RB-017."
          runbook_url: "https://docs.keirox.io/runbooks/nvme-critical"
```

### 6.3 Alert Certification Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| ALERT-T-001 | Trigger NVMe critical condition | Alert fires within 5 minutes |
| ALERT-T-002 | Trigger watermark stuck condition | Alert fires within 15 minutes |
| ALERT-T-003 | Alert includes runbook link | Runbook URL present in annotation |
| ALERT-T-004 | Alert resolves when condition clears | Alert resolves within evaluation interval |
| ALERT-T-005 | Alertmanager routing works | Alerts route to correct team/channel |

---

## 7. OpenTelemetry Auto-Instrumentation

### 7.1 Instrumentation Scope

| Component | Instrumentation |
|---|---|
| Rust SDK | Trace context propagation, span generation for append/fetch/lease/ACK |
| Go SDK | Trace context propagation, span generation |
| Python SDK | Trace context propagation, span generation |
| Kafka Gateway | Span generation for produce/fetch operations |
| SQS Gateway | Span generation for send/receive/delete operations |
| AMQP Gateway | Span generation for publish/consume/ack operations |
| Server | Span generation for WAL append, state transitions, Iceberg commits |

### 7.2 Trace Context Propagation

```text
Producer SDK
   │ (inject trace context into message headers)
   ▼
Keirox Gateway
   │ (extract trace context, create server span)
   ▼
Keirox Server
   │ (create child spans for WAL, state plane)
   ▼
Iceberg Committer
   │ (create child span for commit)
   ▼
Consumer SDK
   │ (extract trace context from message headers)
   ▼
Consumer Application
```

### 7.3 Span Definitions

| Span Name | Parent | Attributes |
|---|---|---|
| `keirox.produce` | Producer SDK | `stream`, `tenant`, `payload_size`, `idempotency_key` |
| `keirox.gateway.translate` | Gateway | `protocol`, `api_version`, `operation` |
| `keirox.wal.append` | Server | `stream`, `offset`, `batch_size` |
| `keirox.state.lease` | Server | `stream`, `group`, `lease_token` |
| `keirox.state.ack` | Server | `stream`, `offset`, `ack_mode` |
| `keirox.iceberg.commit` | Committer | `table`, `file_count`, `record_count` |
| `keirox.consume` | Consumer SDK | `stream`, `offset`, `lease_token` |

### 7.4 OTel Configuration

| Setting | Default | Configurable |
|---|---|---|
| Exporter | OTLP gRPC | Yes |
| Sampling rate | 1% (head-based) | Yes |
| Tail-based sampling | Disabled | Yes |
| Resource attributes | `service.name`, `service.version`, `deployment.environment` | Yes |
| Batch processor | Enabled | Yes |
| CPU overhead limit | 1% | Hard limit |

### 7.5 OTel Certification Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| OTEL-T-001 | Produce with tracing enabled | Span generated with correct attributes |
| OTEL-T-002 | Consume with tracing enabled | Trace context propagated from producer to consumer |
| OTEL-T-003 | Gateway tracing | Gateway creates child span under producer span |
| OTEL-T-004 | CPU overhead measurement | Overhead < 1% with tracing enabled |
| OTEL-T-005 | Sampling configuration | Configured sampling rate respected |

---

## 8. Datadog & New Relic Integrations

### 8.1 Datadog Integration

| Component | Delivery |
|---|---|
| Datadog Agent check | Custom Python check that scrapes Keirox `/metrics` endpoint |
| Datadog dashboard | JSON dashboard template |
| Datadog monitors | Pre-built monitors mapped to alert rules |
| Datadog integration tile | Published to Datadog Marketplace (optional) |

### 8.2 New Relic Integration

| Component | Delivery |
|---|---|
| NRQL queries | Pre-built NRQL queries for key metrics |
| New Relic dashboard | JSON dashboard template |
| New Relic alerts | Pre-built alert conditions |
| OpenTelemetry export | OTLP export to New Relic OTLP endpoint |

### 8.3 Integration Certification Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| DD-T-001 | Datadog agent check collects metrics | Metrics appear in Datadog |
| DD-T-002 | Datadog dashboard renders | Dashboard displays Keirox metrics |
| DD-T-003 | Datadog monitor fires | Monitor triggers on threshold breach |
| NR-T-001 | New Relic OTLP ingestion | Metrics appear in New Relic |
| NR-T-002 | New Relic dashboard renders | Dashboard displays Keirox metrics |

---

## 9. Web Operations Console

### 9.1 Console Architecture

```text
┌────────────────────────────────────────────────────────────────────────────┐
│                        WEB OPERATIONS CONSOLE                              │
│                                                                            │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │                    FRONTEND (React/TypeScript)                        │  │
│  │                                                                      │  │
│  │  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌────────────┐ │  │
│  │  │ Cluster      │ │ Stream       │ │ DLQ          │ │ Admin      │ │  │
│  │  │ Overview     │ │ Inspector    │ │ Manager      │ │ Operations │ │  │
│  │  └──────────────┘ └──────────────┘ └──────────────┘ └────────────┘ │  │
│  │                                                                      │  │
│  │  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐                │  │
│  │  │ Lakehouse    │ │ Security     │ │ PITR /       │                │  │
│  │  │ Explorer     │ │ Events       │ │ Backup       │                │  │
│  │  └──────────────┘ └──────────────┘ └──────────────┘                │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│         │ gRPC / REST                                                      │
│         ▼                                                                  │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │                    CONSOLE BACKEND (Go/Rust)                          │  │
│  │                                                                      │  │
│  │  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐                │  │
│  │  │ Admin API    │ │ Auth         │ │ Audit        │                │  │
│  │  │ Gateway      │ │ Middleware   │ │ Logger       │                │  │
│  │  └──────────────┘ └──────────────┘ └──────────────┘                │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│         │ gRPC                                                             │
│         ▼                                                                  │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │                    KEIROX CLUSTER                                     │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────────────────┘
```

### 9.2 Console Pages

#### Page 1: Cluster Overview

| Feature | Description |
|---|---|
| Cluster health status | Green/yellow/red based on Raft quorum, node health, alert count |
| Node list | All nodes with status, role (leader/follower), uptime |
| Active streams count | Total active streams |
| Ingest throughput | Current and historical throughput |
| Active leases | Current lease count |
| Iceberg freshness | Current snapshot age |
| Open alerts | Firing alerts with severity |

#### Page 2: Stream Inspector

| Feature | Description |
|---|---|
| Stream list | Searchable list of all streams |
| Stream detail | Offsets, throughput, backlog, consumer groups |
| Message browser | Read messages by offset range (read-only) |
| Watermark visualization | Visual display of W_base and head offset |
| Consumer group view | Active consumers, offsets, lag |

#### Page 3: DLQ Manager

| Feature | Description |
|---|---|
| DLQ list | All DLQ entries with reason, retry count, timestamp |
| DLQ entry detail | Payload preview (if authorized), metadata, error history |
| Redrive action | Redrive selected entries (requires confirmation + audit) |
| Purge action | Purge selected entries (requires elevated auth + confirmation + audit) |
| Filter and search | Filter by stream, tenant, reason, date range |

#### Page 4: Lakehouse Explorer

| Feature | Description |
|---|---|
| Table list | All Iceberg tables with schema info |
| Freshness display | Current snapshot age per table |
| File explorer | Parquet file list with sizes and metadata |
| Manifest viewer | Manifest list and snapshot history |
| Query runner | Execute read-only SQL queries against Iceberg tables |

#### Page 5: Security Events

| Feature | Description |
|---|---|
| Auth failure log | Recent authentication failures |
| ABAC denial log | Recent authorization denials |
| Cross-tenant denials | Cross-tenant access attempts |
| Crypto-shred events | Erasure event history |
| KMS events | Key lifecycle events |

#### Page 6: Admin Operations (Break-Glass)

| Feature | Description |
|---|---|
| PITR trigger | Initiate point-in-time recovery (requires confirmation + audit) |
| Backup trigger | Initiate manual backup |
| Failover trigger | Initiate planned region failover (requires confirmation + audit) |
| Node drain | Drain a node for maintenance |
| Crypto-shred trigger | Initiate erasure workflow (requires legal approval + confirmation + audit) |

### 9.3 Console Security Model

| Requirement | Implementation |
|---|---|
| Authentication | OAuth2/OIDC integration; SSO support |
| Authorization | Role-based access control (Viewer, Operator, Admin) |
| Read-only default | Console starts in read-only mode; write operations require explicit role |
| Break-glass confirmation | Destructive operations require typing confirmation text |
| Audit trail | All console actions logged to Keirox audit trail |
| Session timeout | Configurable session timeout (default 30 minutes) |
| CSRF protection | CSRF tokens on all state-changing requests |

### 9.4 Console Roles

| Role | Permissions |
|---|---|
| Viewer | Read-only access to all pages; no write operations |
| Operator | Read access + DLQ redrive + stream inspection |
| Admin | Full access including break-glass operations |

### 9.5 Console Certification Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| CON-T-001 | Console deploys via Helm | Console accessible via browser |
| CON-T-002 | Read-only mode by default | Write operations disabled without Admin role |
| CON-T-003 | Stream inspection works | Messages readable by offset |
| CON-T-004 | DLQ redrive works | Selected entries redriven; audit event logged |
| CON-T-005 | Break-glass confirmation required | PITR trigger requires confirmation text |
| CON-T-006 | Unauthorized access blocked | Viewer cannot access Admin operations |
| CON-T-007 | All actions audit-logged | Console actions appear in audit trail |
| CON-T-008 | Session timeout works | Session expires after configured timeout |

---

## 10. SLO Definitions and Error Budget Tracking

### 10.1 Default SLOs

| SLO | Target | Window | Measurement |
|---|---|---|---|
| Ingest Availability | 99.95% | 30 days | Successful appends / total appends |
| Write Latency | p99 < 2ms | 5 minutes | WAL append latency histogram |
| Read Latency | p99 < 2ms | 5 minutes | Stream read latency histogram |
| Lease Acquisition | p99 < 1ms | 5 minutes | Lease next latency histogram |
| Iceberg Freshness | < 60s | 5 minutes | Max snapshot age |
| Coordinator Failover | < 3.5s | Per event | Failover duration |

### 10.2 Error Budget Burn Rate Alerts

| Burn Rate | Window | Severity | Action |
|---|---|---|---|
| 14x | 1 hour | Warning | Investigate; error budget depleting fast |
| 30x | 15 minutes | Critical | Immediate response; error budget nearly depleted |
| 1x | 30 days | Info | Normal consumption rate |

### 10.3 SLO Certification Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| SLO-T-001 | Error budget calculation correct | Budget matches manual calculation |
| SLO-T-002 | Burn rate alert fires | Alert fires when burn rate exceeds threshold |
| SLO-T-003 | SLO configurable per tenant | Tenant-specific SLOs work |
| SLO-T-004 | SLO dashboard renders | Error budget visualization correct |

---

## 11. Certification Levels

| Level | Name | Requirement |
|---|---|---|
| L1 | Dashboard Certified | All 7 Grafana dashboards deploy and render correctly |
| L2 | Alert Certified | All alert rules fire and resolve correctly with runbook links |
| L3 | OTel Certified | OpenTelemetry tracing works with < 1% CPU overhead |
| L4 | Integration Certified | Datadog and New Relic integrations validated |
| L5 | Console Certified | Web Console deploys with read-only default and break-glass workflow |
| L6 | SLO Certified | SLO tracking and error budget alerting validated |

Phase 5 exit requires **L1 through L6**.

---

## 12. Deliverables and Milestones

| Deliverable | Description | Target Week |
|---|---|---:|
| D-OPS-001 | Grafana dashboard suite (7 dashboards) | Week 14 |
| D-OPS-002 | Prometheus recording rules | Week 14 |
| D-OPS-003 | Prometheus alert rules | Week 15 |
| D-OPS-004 | OpenTelemetry auto-instrumentation (Rust SDK) | Week 16 |
| D-OPS-005 | OpenTelemetry auto-instrumentation (Go SDK, gateways) | Week 17 |
| D-OPS-006 | Datadog integration | Week 18 |
| D-OPS-007 | New Relic integration | Week 18 |
| D-OPS-008 | Web Console frontend | Week 20 |
| D-OPS-009 | Web Console backend (Admin API gateway) | Week 20 |
| D-OPS-010 | Console security model (RBAC, audit) | Week 21 |
| D-OPS-011 | SLO definitions and error budget tracking | Week 21 |
| D-OPS-012 | Observability certification test suite | Week 22 |
| D-OPS-013 | Final Day-2 observability evidence package | Week 22 |

---

## 13. Certification Gates

### 13.1 Gate OPS-A: Dashboards & Alerts Certified (Week 16)

| Criterion | Mandatory |
|---|---|
| All 7 Grafana dashboards deploy via Helm | Yes |
| All dashboards render with live data | Yes |
| All alert rules fire correctly | Yes |
| All alerts include runbook links | Yes |
| Recording rules produce correct SLO metrics | Yes |

### 13.2 Gate OPS-B: Integrations Certified (Week 19)

| Criterion | Mandatory |
|---|---|
| OpenTelemetry tracing works end-to-end | Yes |
| OTel CPU overhead < 1% | Yes |
| Datadog integration validated | Yes |
| New Relic integration validated | Yes |

### 13.3 Gate OPS-C: Console Certified (Week 22)

| Criterion | Mandatory |
|---|---|
| Web Console deploys and is accessible | Yes |
| Read-only mode is default | Yes |
| Break-glass operations require confirmation | Yes |
| All console actions are audit-logged | Yes |
| SLO tracking and error budget alerting work | Yes |
| All L1–L6 certification levels pass | Yes |
| Evidence package complete | Yes |

---

## 14. Risks and Mitigations

| Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|
| Grafana dashboard maintenance burden | Medium | Medium | Use JSON provisioning; automate dashboard testing |
| Alert fatigue from too many alerts | Medium | High | Tune thresholds; use severity levels; link runbooks |
| OTel overhead exceeds budget | Medium | Low | Head-based sampling; configurable sampling rate |
| Web Console scope creep | High | High | MVP-first; read-only default; strict feature gates |
| Datadog/New Relic API changes | Low | Medium | Abstract integration layer; pin SDK versions |
| Console security vulnerability | Critical | Low | Security review; RBAC; CSRF protection; audit trail |
| SLO definitions too aggressive | Medium | Medium | Start conservative; tune based on production data |

---

## 15. Evidence Package

The Day-2 observability evidence package MUST include:

1. Grafana dashboard JSON files.
2. Prometheus recording rule YAML files.
3. Prometheus alert rule YAML files.
4. OpenTelemetry instrumentation documentation.
5. OTel CPU overhead benchmark report.
6. Datadog integration test results.
7. New Relic integration test results.
8. Web Console deployment guide.
9. Console RBAC and security model documentation.
10. SLO definitions and error budget configuration.
11. All certification test results.
12. Customer-facing observability guide.

---

## 16. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial Day-2 Observability & Web Console Plan. Defines Grafana dashboard suite, Prometheus recording and alert rules, OpenTelemetry auto-instrumentation, Datadog/New Relic integrations, Web Operations Console architecture, SLO definitions, error budget tracking, certification levels, and evidence requirements. |