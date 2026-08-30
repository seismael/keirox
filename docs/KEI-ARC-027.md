# KEI-ARC-027 — Operability, Observability & Capacity Architecture

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-ARC-027 |
| Title | Operability, Observability & Capacity Architecture |
| Version | 1.0 |
| Level | **L2 — Subsystem Architecture** |
| Pillars Covered | Cross-cutting (SRE, Observability, FinOps, Lifecycle) |
| Status | Approved for Engineering |
| Classification | Internal / Engineering Confidential |
| Owner | SRE Lead / Principal Engineer (Platform) |
| Required Reviewers | Chief Architect, Principal Engineer (Storage), Security Lead, FinOps Lead |
| Depends On | KEI-ARC-010 (Conceptual Architecture), KEI-ARC-011 (NFRs), KEI-ARC-012 (ADRs), KEI-ARC-020..026 (All Subsystems) |
| Feeds | KEI-OPS-040 (Operations Runbooks), KEI-OPS-041 (Validation & Test Plan) |
| Keywords | MUST, MUST NOT, SHOULD, SHOULD NOT, MAY per RFC 2119 |

---

## 2. Purpose, Scope, and Audience

### 2.1 Purpose

This document specifies the **Operability, Observability, and Capacity Management subsystem** of the Polymorphic Event Fabric.

A distributed system is only as reliable as its operational visibility and failure-mode boundaries. This subsystem ensures that the internal mechanics of the storage engine, state plane, consensus layer, lakehouse pipeline, protocol gateways, security plane, and multi-region replication are transparent to operators, and that the system degrades gracefully under overload rather than failing catastrophically.

It elaborates:

- The unified metrics, logging, and distributed tracing architecture.
- The multi-layer backpressure and admission control ladder.
- Tenant quota enforcement mechanics.
- Rolling upgrade and feature-flag governance.
- Capacity planning and FinOps telemetry models.
- Operability-specific failure handling.

### 2.2 Scope

**In scope:**

- OpenTelemetry/Prometheus metrics catalog.
- Distributed tracing context propagation.
- Token-bucket quotas and admission control.
- Progressive backpressure and priority shedding.
- Rolling upgrades and mixed-version compatibility governance.
- Capacity forecasting and FinOps telemetry.
- Operability failure modes and safe degradation behavior.

**Out of scope:**

- The internal implementation of the subsystems being observed.
- Security audit logging, which is owned by KEI-ARC-025.
- Step-by-step incident response runbooks, which are owned by KEI-OPS-040.
- Chaos and benchmark test definitions, which are owned by KEI-OPS-041.

### 2.3 Audience

This document is intended for:

- SRE and platform engineering teams.
- Subsystem owners exposing metrics and health endpoints.
- FinOps and capacity planning stakeholders.
- Engineering leads responsible for rolling upgrades and feature flags.
- Test engineers validating operational behavior under failure.

---

## 3. Position in the Architecture

```
                         ┌─────────────────────────────────────┐
                         │      EXTERNAL OBSERVABILITY STACK   │
                         │  Prometheus / OTel / Logs / SLOs    │
                         └──────────────────▲──────────────────┘
                                            │ OTLP / Prometheus scrape
                                            │
┌───────────────────────────────────────────┴──────────────────────────────────────┐
│                     OPERABILITY, OBSERVABILITY & CAPACITY PLANE                  │
│                                                                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐    │
│  │ O1. Telemetry │  │ O2. Tracing  │  │ O3. Quota &  │  │ O4. Backpressure │    │
│  │    Engine     │  │   Propagator │  │   Admission  │  │    & Shedding    │    │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────────┘    │
│                                                                                  │
│  ┌──────────────┐  ┌──────────────┐                                            │
│  │ O5. Capacity  │  │ O6. Lifecycle │                                           │
│  │   & FinOps    │  │   & Upgrade   │                                           │
│  └──────────────┘  └──────────────┘                                            │
└──────▲──────────────────────▲──────────────────────▲──────────────────────▲─────┘
       │                      │                      │                      │
       │                      │                      │                      │
┌──────┴──────┐        ┌──────┴──────┐        ┌──────┴──────┐        ┌──────┴──────┐
│ Storage     │        │ State Plane │        │ Consensus   │        │ ELT /       │
│ Engine      │        │             │        │ & HA        │        │ Gateways /  │
│ KEI-ARC-020 │        │ KEI-ARC-021 │        │ KEI-ARC-022 │        │ Security /  │
└─────────────┘        └─────────────┘        └─────────────┘        │ DR Planes   │
                                                                      └─────────────┘
```

### 3.1 Normative Boundary

This subsystem does **not** own the internal behavior of other subsystems. It provides the cross-cutting operational layer that:

1. Observes their behavior.
2. Bounds their resource usage.
3. Coordinates safe degradation.
4. Governs lifecycle changes.
5. Provides capacity and cost telemetry.

**Normative rule:** Observability and operability hooks MUST NOT become a blocking dependency on the hot write path. Telemetry, quota evaluation, and tracing MUST be designed to fail without stalling durable ingress.

---

## 4. Subsystem Responsibilities and Non-Responsibilities

### 4.1 Responsibilities

| ID | Responsibility |
|---|---|
| R1 | Collect and expose system metrics from all subsystems. |
| R2 | Propagate distributed tracing context across ingress, WAL, state plane, compaction, and egress. |
| R3 | Enforce tenant quotas and admission control before resource allocation. |
| R4 | Coordinate progressive backpressure under resource pressure. |
| R5 | Execute priority shedding when the system approaches unsafe capacity thresholds. |
| R6 | Govern rolling upgrades, node draining, and mixed-version compatibility. |
| R7 | Manage feature flags for staged capability enablement. |
| R8 | Provide capacity forecasts and FinOps telemetry. |
| R9 | Expose operational health for SLO monitoring and alerting. |
| R10 | Ensure operability mechanisms degrade safely under failure. |

### 4.2 Non-Responsibilities

| ID | Non-Responsibility | Owned By |
|---|---|---|
| N1 | WAL persistence and segment lifecycle | KEI-ARC-020 |
| N2 | Consumption state machine semantics | KEI-ARC-021 |
| N3 | Consensus protocol behavior | KEI-ARC-022 |
| N4 | Lakehouse commit semantics | KEI-ARC-023 |
| N5 | Protocol translation behavior | KEI-ARC-024 |
| N6 | Security audit logs and key lifecycle | KEI-ARC-025 |
| N7 | Multi-region failover decisions | KEI-ARC-026 |
| N8 | Human incident-response runbooks | KEI-OPS-040 |
| N9 | Chaos and benchmark test definitions | KEI-OPS-041 |

---

## 5. Internal Component Decomposition

| Component | Responsibility |
|---|---|
| **O1. Telemetry & Metrics Engine** | Aggregates internal counters, gauges, and histograms; exposes them via OTLP/Prometheus. |
| **O2. Distributed Tracing Propagator** | Injects and propagates W3C Trace Context across subsystem boundaries. |
| **O3. Quota & Admission Controller** | Evaluates per-tenant token buckets and rejects or throttles over-quota requests. |
| **O4. Backpressure & Shedding Controller** | Monitors node and cluster resources and triggers the backpressure ladder. |
| **O5. Capacity & FinOps Predictor** | Projects NVMe, S3, CPU, and network usage; produces scaling and cost telemetry. |
| **O6. Lifecycle & Upgrade Manager** | Coordinates node draining, rolling upgrades, feature flags, and mixed-version routing. |

---

## 6. Observability Architecture

### 6.1 Design Principles

| ID | Principle | Normative Effect |
|---|---|---|
| OB-1 | Observability is a product feature. | Internal state MUST be exposed as first-class metrics. |
| OB-2 | Telemetry MUST NOT block the hot path. | Metrics and tracing MUST be asynchronous and drop-safe. |
| OB-3 | Every bounded resource MUST be observable. | Quotas, bitmaps, NVMe, leases, WAL, and S3 backlog MUST have metrics. |
| OB-4 | Every failure mode MUST have a signal. | Each red-team scenario MUST expose at least one leading indicator. |
| OB-5 | Cardinality MUST be controlled. | Metrics labels MUST be bounded to prevent telemetry explosion. |

### 6.2 Metrics Catalog

#### Ingress and Write Path

| Metric Name | Type | Description |
|---|---|---|
| `keirox_ingest_messages_total` | Counter | Total messages ingested, by tenant/stream/status. |
| `keirox_ingest_bytes_total` | Counter | Total bytes ingested. |
| `keirox_wal_write_latency_seconds` | Histogram | p50/p99/p999 Tier-0 quorum commit latency. |
| `keirox_producer_dedup_hits_total` | Counter | Idempotent producer deduplication hits. |
| `keirox_ingress_quota_rejections_total` | Counter | Requests rejected by quota enforcement. |

#### State Plane and Queue Path

| Metric Name | Type | Description |
|---|---|---|
| `keirox_active_leases` | Gauge | Active leases per coordinator shard. |
| `keirox_lease_age_seconds` | Histogram | Time tasks spend leased before terminal state. |
| `keirox_lease_timeouts_total` | Counter | Lease expirations caused by worker failure or delay. |
| `keirox_watermark_lag_offsets` | Gauge | Delta between head offset and `W_base`. |
| `keirox_dlq_evictions_total` | Counter | Mandatory DLQ evictions. |
| `keirox_bitmap_memory_bytes` | Gauge | In-memory Roaring Bitmap footprint. |
| `keirox_bitmap_spilled_bytes` | Gauge | Bitmap state spilled to NVMe SSTables. |
| `keirox_ack_replication_lag_seconds` | Gauge | Delay between fast-path ACK and metadata Raft commit. |

#### Storage and Tiering

| Metric Name | Type | Description |
|---|---|---|
| `keirox_nvme_used_bytes` | Gauge | Tier-0 NVMe utilization. |
| `keirox_nvme_backlog_eta_seconds` | Gauge | Estimated time to NVMe exhaustion at current ingress. |
| `keirox_s3_upload_backlog_bytes` | Gauge | Bytes pending Tier-1 upload. |
| `keirox_s3_upload_errors_total` | Counter | Failed S3 uploads, by error class. |
| `keirox_segment_seal_total` | Counter | Number of sealed WAL segments. |
| `keirox_recovery_duration_seconds` | Histogram | Time to restore node state. |

#### Columnar ELT and Lakehouse

| Metric Name | Type | Description |
|---|---|---|
| `keirox_compaction_lag_seconds` | Gauge | Time between segment seal and Parquet upload completion. |
| `keirox_arrow_transpose_cpu_ratio` | Gauge | CPU consumed by Arrow transposition. |
| `keirox_iceberg_snapshot_age_seconds` | Gauge | Freshness of latest Iceberg commit. |
| `keirox_iceberg_commit_errors_total` | Counter | Failed Iceberg catalog commits. |
| `keirox_small_file_count` | Gauge | Number of files below target lakehouse size threshold. |

#### Security and Compliance Telemetry

This subsystem exposes operational telemetry only. Security audit events remain owned by KEI-ARC-025.

| Metric Name | Type | Description |
|---|---|---|
| `keirox_security_telemetry_export_errors_total` | Counter | Failures exporting security operational metrics. |
| `keirox_quota_policy_cache_miss_total` | Counter | Quota policy cache misses requiring refresh. |

### 6.3 Distributed Tracing

Every external request MUST be traceable using W3C Trace Context.

Required spans:

```text
ingress_admission
quota_check
wal_quorum_commit
state_lease_acquire
state_ack
arrow_shredding
parquet_encode
s3_multipart_upload
iceberg_commit
gateway_protocol_translation
```

**Normative rules:**

- Default sampling SHOULD be 1%.
- Tail-based sampling MAY be enabled for error paths.
- Trace propagation MUST NOT add more than 1% CPU overhead under Profile P1.
- Trace context MUST be propagated across internal RPC boundaries.

### 6.4 Logging

Operational logs MUST include:

- Node ID.
- Cluster ID.
- Tenant ID, where safe and authorized.
- Component name.
- Severity.
- Trace ID, where available.
- Structured key-value context.

**Normative rules:**

- Logs MUST NOT contain secrets, tokens, DEKs, or customer payload data.
- Log levels MUST be dynamically adjustable without restart.
- Error logs MUST include machine-readable reason codes.

---

## 7. Quotas and Admission Control

### 7.1 Purpose

Quota enforcement protects the fabric from noisy-neighbor starvation and ensures that bounded resources are not exhausted by a single tenant.

### 7.2 Quota Dimensions

| Dimension | Scope | Enforcement Point |
|---|---|---|
| Ingress messages/sec | Per tenant | Gateway socket admission. |
| Ingress bytes/sec | Per tenant | Gateway socket admission. |
| Maximum streams | Per tenant | Control-plane stream creation. |
| Maximum consumer groups | Per tenant or stream | Control-plane group creation. |
| Maximum active leases | Per group | State-plane lease API. |
| Maximum bitmap memory | Per state shard | State-plane spill controller. |
| Maximum retained data | Per stream/tenant | Lifecycle governance. |
| Maximum query scan bytes | Per tenant | Lakehouse/query gateway. |

### 7.3 Token-Bucket Model

Each quota is modeled as a token bucket:

```text
tokens_available = min(bucket_capacity,
                         tokens_available + refill_rate × elapsed_time)
```

A request is admitted only if sufficient tokens exist.

### 7.4 Admission Behavior

**Normative rules:**

- When a quota is exceeded, the system MUST return a protocol-appropriate retriable error.
- The system MUST NOT silently drop data without an error unless operating in explicit emergency shedding mode.
- Quota enforcement MUST occur before expensive resource allocation.
- Local quota caches MUST have bounded TTLs.
- If quota configuration is unavailable, the node MUST use last-known-good policy or a conservative global throttle. It MUST NOT fail open with unlimited ingress.

---

## 8. Progressive Backpressure and Shedding Ladder

### 8.1 Purpose

Backpressure protects Tier-0 NVMe, memory arenas, and state-plane resources from exhaustion during overload, S3 degradation, or compaction lag.

### 8.2 Backpressure Ladder

| Stage | Trigger Condition | Action |
|---|---|---|
| 1. Alert | NVMe > 60% OR arena > 60% OR compaction lag > threshold | Emit alerts; increase compaction priority. |
| 2. TCP Clamp | NVMe > 80% OR arena > 80% | Reduce TCP receive window to slow producers at network layer. |
| 3. Protocol Throttle | NVMe > 90% OR upload backlog sustained | Inject response latency or explicit throttling errors. |
| 4. Priority Shed | NVMe > 95% OR memory pressure critical | Shed streams marked `priority=low` before `priority=critical`. |
| 5. Hard Reject | NVMe > 98% OR corruption risk | Reject all new ingress with retriable unavailable errors. |

### 8.3 Priority Classes

| Priority | Example Workloads | Shed Order |
|---|---|---|
| `critical` | Payments, orders, security events | Shed last. |
| `standard` | Application events, workflows | Shed after low priority is exhausted. |
| `low` | Debug logs, metrics, non-critical telemetry | Shed first. |

### 8.4 Normative Backpressure Rules

- Backpressure MUST engage progressively.
- Backpressure MUST be observable at every stage.
- Emergency shedding MUST be auditable and metric-exposed.
- The system MUST NOT reach NVMe exhaustion in a way that causes kernel I/O failure or WAL corruption.
- Shedding MUST NOT delete immutable committed records. Shedding applies to new ingress acceptance, not historical truth.

---

## 9. Rolling Upgrades and Lifecycle Management

### 9.1 Versioned Surfaces

The system maintains explicit version numbers for:

```text
WAL binary format version
Raft metadata version
State snapshot version
Gateway protocol version
SDK/gRPC schema version
Iceberg committer version
Feature-flag schema version
```

### 9.2 Mixed-Version Compatibility

**Normative rule:** The cluster MUST support N and N-1 mixed-version operation.

- Nodes running version N MUST read data written by N-1.
- Nodes running N-1 are not required to understand N-only features.
- New features MUST be gated by feature flags until all nodes support them.
- Breaking on-disk format changes MUST go through a migration ADR.

### 9.3 Rolling Upgrade Procedure

```text
1. Validate cluster health and SLO budget.
2. Select target node.
3. Enter drain mode.
4. Stop new stream/lease assignments.
5. Allow active leases to expire or transfer.
6. Flush state snapshots and WAL deltas.
7. Stop node process.
8. Deploy new binary.
9. Rejoin cluster.
10. Catch up via Raft and manifests.
11. Validate health.
12. Exit drain mode.
```

### 9.4 Feature Flags

Feature flags MUST support:

- Global enable/disable.
- Per-tenant enablement.
- Per-stream enablement.
- Percentage rollout.
- Emergency kill-switch.
- Audit logging of flag changes.

**Normative rule:** Feature flag state changes MUST be auditable and MUST NOT require a restart for non-kernel-level features.

---

## 10. Capacity Planning and FinOps Telemetry

### 10.1 Capacity Model

The Capacity Predictor estimates required nodes as:

```text
Required_Nodes = max(
    Total_Ingress_MBps / Target_Node_Ingress_MBps,
    Total_Active_Streams / Max_Streams_Per_Node,
    Total_Active_Leases / Max_Leases_Per_Node,
    Total_Bitmap_Memory / Node_State_Memory_Budget,
    Total_Compaction_CPU / Node_Compaction_CPU_Budget
) + Quorum_Overhead
```

### 10.2 FinOps Metrics

| Metric | Purpose |
|---|---|
| `keirox_s3_api_calls_total` | Track S3 PUT/GET/LIST API cost exposure. |
| `keirox_s3_storage_bytes` | Track cold storage usage. |
| `keirox_cross_az_egress_bytes` | Track inter-AZ network cost. |
| `keirox_compute_core_hours` | Track CPU allocation and cost. |
| `keirox_nvme_utilization_ratio` | Track ephemeral storage pressure. |
| `keirox_tenant_usage_cost_estimate` | Provide per-tenant cost attribution. |

### 10.3 Scaling Recommendations

The Capacity Predictor SHOULD emit recommendations for:

- Node scale-up.
- Node scale-down.
- Compaction core rebalancing.
- S3 lifecycle tuning.
- Tenant quota adjustment.
- Stream density rebalancing.

**Normative rule:** Capacity recommendations MUST be advisory unless an external autoscaler is explicitly authorized. The operability plane MUST NOT autonomously shrink a cluster without policy approval.

---

## 11. Operability-Specific Failure Handling

| Scenario | Required Behavior |
|---|---|
| Metrics sink unavailable | Buffer locally with bounded memory; drop lowest-priority telemetry if buffer full; never block ingress. |
| Tracing collector unavailable | Continue request processing; disable trace export temporarily. |
| Quota service unavailable | Use last-known-good local quota cache; if absent, apply conservative global throttle. |
| Backpressure controller fault | Node-local safety valve MUST still protect NVMe at hard threshold. |
| Upgrade manager fault | Block upgrade progression; maintain current stable version. |
| Feature flag store unavailable | Use last-known-good flags; deny unknown experimental features. |
| Capacity predictor failure | Continue serving; suppress recommendations; alert operators. |
| Metric cardinality explosion | Enforce label caps; aggregate high-cardinality labels; drop unsafe series. |
| Log pipeline failure | Buffer critical logs locally; avoid log storms; never stall hot path. |
| Emergency shedding triggered | Emit high-severity event; preserve committed data; shed only new low-priority ingress. |

**Normative rule:** Operability components MUST fail in a way that preserves data durability and hot-path availability wherever possible. Observability failure MUST NOT become a data-loss vector.

---

## 12. NFR Traceability

| NFR | Requirement | How This Subsystem Satisfies It |
|---|---|---|
| OPS-001 | Metric coverage | Metrics catalog in §6.2. |
| OPS-002 | Distributed tracing | W3C Trace Context propagation in §6.3. |
| OPS-003 | Quota enforcement | Token-bucket admission control in §7. |
| OPS-004 | Backpressure behavior | Progressive ladder in §8. |
| OPS-005 | Rolling upgrade safety | N/N-1 mixed-version policy and drain protocol in §9. |
| OPS-006 | DLQ operability | DLQ metrics exposed; redrive operation remains owned by KEI-ARC-021. |
| OPS-007 | Capacity forecasting | NVMe ETA, S3 backlog, and capacity model in §10. |
| PERF-003 | Compaction interference observability | Compaction lag and CPU metrics in §6.2. |
| MEM-003/005/006 | Bounded state observability | Bitmap memory/spill and lease metrics in §6.2. |
| AVAIL-002/003 | Recovery observability | Recovery duration and failover health metrics in §6.2. |

---

## 13. Interfaces

### 13.1 Provided Interfaces

| Interface | Consumer | Semantics |
|---|---|---|
| `exportMetrics()` | Prometheus / OTel collector | Expose metrics via OTLP or Prometheus scrape. |
| `injectTraceContext(request)` | Gateways / SDK server | Inject W3C trace context into request metadata. |
| `checkQuota(tenant, resource, amount)` | Gateways / control plane | Return admit/deny decision. |
| `getBackpressureState(scope)` | Admin / SRE tooling | Return current backpressure stage and triggers. |
| `triggerBackpressureStage(stage)` | Internal safety controller | Manually or automatically escalate backpressure. |
| `getCapacityForecast(cluster)` | FinOps / control plane | Return projected capacity and cost telemetry. |
| `drainNode(node_id)` | Upgrade manager / admin | Remove node from active assignment safely. |
| `beginUpgrade(node_id)` | Lifecycle manager | Start controlled rolling upgrade. |
| `setFeatureFlag(flag, scope, state)` | Admin / control plane | Change feature enablement state. |
| `getHealthReport()` | Load balancers / control plane | Return liveness, readiness, and degradation state. |

### 13.2 Consumed Interfaces

| Interface | Provider | Purpose |
|---|---|---|
| Subsystem telemetry hooks | KEI-ARC-020..026 | Collect metrics, traces, and logs. |
| Cluster membership state | KEI-ARC-022 | Understand node roles and health. |
| Tenant quota configuration | Control plane | Enforce admission rules. |
| Authorization decision | KEI-ARC-025 | Protect admin and observability APIs. |
| External observability sink | Prometheus / OTel / SIEM | Persist telemetry externally. |
| Object storage metrics | KEI-ARC-020 | Track S3 backlog and upload health. |
| State-plane metrics | KEI-ARC-021 | Track leases, watermarks, bitmaps, DLQ. |

---

## 14. Open Questions and ADR Dependencies

| Item | Status | Resolution Path |
|---|---|---|
| Default trace sampling strategy | Open | Evaluate head-based vs. tail-based sampling under P1/P4. |
| Metric label cardinality cap | Open | Define tenant/stream label aggregation policy. |
| Feature flag backend | Open | Evaluate embedded flag store vs. external flag service. |
| Autoscaling authority | Open | Decide whether recommendations are advisory only or integrated with cloud autoscaler. |
| Observability retention period | Open | Define cost-aligned retention for metrics/traces. |
| Backpressure stage thresholds | Open | Benchmark under P2/P6 before Phase-2 exit. |
| Emergency shedding policy defaults | Open | Define default priority classes and tenant overrides. |

### 14.1 Governing Principles

This document is governed by:

- P3: Bounded everything.
- P4: Explicit semantics over magic.
- P8: Observability is a product feature.
- P9: Evidence gates phases.

No new binding ADR is introduced in this baseline. Items above are ADR candidates.

---

## 15. Glossary

| Term | Definition |
|---|---|
| OTLP | OpenTelemetry Protocol. |
| Token Bucket | Rate-limiting model based on refillable tokens. |
| TCP Clamping | Reducing TCP window size to slow producers. |
| Priority Shedding | Rejecting lower-priority ingress to preserve critical workloads. |
| Feature Flag | Runtime mechanism to enable or disable capabilities. |
| N/N-1 Compatibility | Support for current and immediately previous software versions. |
| FinOps Telemetry | Cost and usage metrics used for cloud financial governance. |
| Drain Mode | Operational state where a node stops accepting new work before upgrade or removal. |
| Backpressure Ladder | Progressive sequence of throttling actions under resource pressure. |
| SLO Budget | Remaining allowable error or latency violation for a service objective. |

---

## 16. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial operability, observability, and capacity architecture. Defines unified metrics catalog, distributed tracing, token-bucket quotas and admission control, progressive backpressure and shedding ladder, rolling upgrades and mixed-version compatibility, capacity planning and FinOps telemetry, and operability failure handling. Aligns to NFRs OPS/PERF/MEM/AVAIL. |