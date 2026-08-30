# KEI-SDK-301 — Native SDK & Developer Experience Plan

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-SDK-301 |
| Title | Native SDK & Developer Experience Plan |
| Version | 1.0 |
| Level | Engineering Execution Plan |
| Status | Baseline — Ready for Execution |
| Phase | Phase 3 — Ecosystem Compatibility Gateways & Lakehouse |
| Duration | Weeks 6–30 of Phase 3 |
| Owner | SDK / Developer Experience Lead |
| Governing Plan | KEI-ENG-300 — Phase 3 Engineering Execution Plan |
| Governing Architecture Documents | KEI-ARC-024, KEI-DES-032, KEI-DES-033, KEI-ARC-027 |
| Predecessor | KEI-SPIKE-301 — Ecosystem Gateway & Lakehouse Prototype Plan |

---

## 2. Executive Summary

The Kafka gateway provides migration. The lakehouse committer provides analytics. The **native SDK provides the future**.

This plan defines how Keirox will deliver a high-performance native developer experience through **Arrow Flight / gRPC SDKs**. The native SDK is the recommended path for new applications that want direct access to Keirox’s polymorphic primitives:

1. Durable append.
2. Stream fetch.
3. Lease-based task consumption.
4. Out-of-order ACK/NACK.
5. Lease renewal.
6. Virtual DLQ operations.
7. Vectorized Arrow Flight reads.
8. Schema-aware payload handling.
9. Client telemetry.
10. Idempotent producer behavior.

The SDK plan is deliberately staged. Rust is the core implementation. Go is the second priority. Python receives a prototype. Java and TypeScript receive design-only treatment in Phase 3, with implementation planned for later phases based on adoption demand.

---

## 3. Purpose and Scope

### 3.1 Purpose

The purpose of this plan is to:

1. Define the native SDK language roadmap.
2. Define the SDK API surface and behavioral contract.
3. Define client architecture and shared core components.
4. Define developer documentation and examples.
5. Define SDK testing, benchmarking, and certification gates.
6. Define retry, error handling, idempotence, and telemetry behavior.
7. Prepare SDKs for Phase 3 certification and future production use.

### 3.2 Scope

**In scope:**

- Rust SDK alpha/beta.
- Go SDK alpha.
- Python SDK prototype.
- Java SDK design specification.
- TypeScript SDK design specification.
- Native Arrow Flight/gRPC client behavior.
- Producer, stream consumer, task worker, and operator APIs.
- Client retry/backoff policies.
- Client telemetry.
- SDK examples and quickstarts.
- SDK benchmarking.
- SDK conformance tests.
- Developer documentation.

**Out of scope:**

- Kafka wire protocol gateway — owned by KEI-COMPAT-301.
- SQS/AMQP gateway — Phase 4.
- Lakehouse query engine internals — owned by KEI-LAKE-301.
- Full schema registry productization — owned by KEI-ENG-300 WP-P3-D.
- Production multi-tenant authentication rollout — Phase 4.
- Managed cloud SDK distribution — future phase.

---

## 4. SDK Strategy Principles

| ID | Principle | Requirement |
|---|---|---|
| SDK-1 | Native path is explicit | SDK MUST expose Keirox semantics directly, not hide them behind generic queue/stream abstractions. |
| SDK-2 | API contract stability | SDK APIs MUST match KEI-DES-032. |
| SDK-3 | Safety by default | Default settings MUST favor durability, idempotence, and retry safety. |
| SDK-4 | Performance is evidence | SDK performance claims MUST be benchmarked, not assumed. |
| SDK-5 | Language parity is gradual | Rust first, Go second, Python prototype, Java/TypeScript design-only in Phase 3. |
| SDK-6 | Errors are typed | Clients MUST expose retryable/non-retryable error categories. |
| SDK-7 | Telemetry is built-in | SDKs MUST expose metrics and traces. |
| SDK-8 | Documentation is part of the product | No SDK release without quickstart, API reference, and examples. |

---

## 5. Language Roadmap

### 5.1 Phase 3 Language Targets

| Language | Phase 3 Target | Priority | Rationale |
|---|---|---:|---|
| Rust | Alpha → Beta | P0 | Core implementation language; highest performance. |
| Go | Alpha | P0/P1 | Common backend language; strong operational ecosystem. |
| Python | Prototype | P1 | Data/AI workload demand; useful for lakehouse integration. |
| Java | Design only | P2 | Kafka migration market; requires larger runtime investment. |
| TypeScript | Design only | P2 | Application developer demand; browser/Node use cases. |

### 5.2 Language Maturity Definitions

| Maturity Level | Meaning |
|---|---|
| Design | API surface and architecture documented; no production code. |
| Prototype | Functional but unsupported; limited tests. |
| Alpha | Feature-complete for core operations; limited production readiness. |
| Beta | Stable API, passing conformance and benchmark gates; approved for controlled production trials. |
| GA | Fully supported, documented, benchmarked, and operationally validated. |

### 5.3 Phase 3 Exit Targets

| Language | Required Exit Maturity |
|---|---|
| Rust | Beta |
| Go | Alpha |
| Python | Prototype |
| Java | Design |
| TypeScript | Design |

---

## 6. SDK API Surface

The SDK API surface MUST align with KEI-DES-032.

### 6.1 Producer Operations

| Operation | Description |
|---|---|
| `append` | Append a single record. |
| `append_batch` | Append a batch of records or Arrow RecordBatch. |
| `flush` | Flush buffered records. |
| `begin_producer_session` | Establish idempotent producer session. |
| `close` | Gracefully close producer. |

### 6.2 Stream Consumer Operations

| Operation | Description |
|---|---|
| `stream_fetch` | Fetch records by stream and offset. |
| `fetch_latest` | Fetch from current head. |
| `fetch_from_offset` | Fetch from explicit offset. |
| `commit_stream_offset` | Commit stream-mode consumer offset. |
| `fetch_committed_offset` | Retrieve committed offset. |

### 6.3 Queue Worker Operations

| Operation | Description |
|---|---|
| `lease_next` | Acquire one or more task leases. |
| `ack` | Acknowledge a leased offset. |
| `ack_batch` | Acknowledge multiple offsets. |
| `nack` | Negative-acknowledge and requeue. |
| `renew_lease` | Extend lease TTL. |
| `worker_run` | Optional high-level worker loop. |

### 6.4 DLQ Operator Operations

| Operation | Description |
|---|---|
| `dlq_list` | List virtual DLQ entries. |
| `dlq_fetch` | Fetch DLQ payload metadata or payload. |
| `dlq_redrive` | Redrive selected DLQ offsets. |
| `dlq_purge` | Purge DLQ entries with authorization. |

### 6.5 Query / Data Access Operations

| Operation | Description |
|---|---|
| `pushdown_query` | Execute predicate-filtered Arrow Flight read. |
| `arrow_fetch` | Fetch Arrow RecordBatches directly. |
| `schema_resolve` | Resolve schema by ID/fingerprint. |

### 6.6 Admin / Discovery Operations

| Operation | Description |
|---|---|
| `get_stream_info` | Retrieve stream metadata. |
| `create_stream` | Create stream when authorized. |
| `delete_stream` | Request stream deletion when authorized. |
| `create_consumer_group` | Create consumer group. |
| `negotiate_version` | Negotiate client/server API compatibility. |

---

## 7. SDK Behavioral Contracts

### 7.1 Producer Idempotence

SDK producers MUST support:

```text
producer_id
producer_epoch
producer_seq
```

Behavior:

1. If server returns duplicate sequence response, SDK MUST treat it as success and return original offset.
2. If producer session expires, SDK MUST surface a retriable session error.
3. If idempotence is disabled, SDK MUST warn that duplicates are possible on retry.

### 7.2 ACK Modes

SDKs MUST expose ACK durability modes:

| Mode | Client Behavior |
|---|---|
| `ACK_FAST` | Default; low-latency acknowledgment. |
| `ACK_DURABLE` | Stronger durability; higher latency. |

**Normative rule:** SDKs MUST NOT hide the difference between ACK modes. Documentation MUST explain redelivery implications.

### 7.3 Retry and Backoff

SDKs MUST implement retry policies for transient failures.

Retryable errors include:

- Server unavailable.
- Request timeout.
- Quota throttling.
- Coordinator failover.
- Temporary backpressure.
- Network reset.

Non-retryable errors include:

- Invalid argument.
- Unauthorized.
- Stream not found.
- Schema violation in strict mode.
- Unsupported operation.
- Lease token mismatch.
- Stale lease.
- DLQ purge unauthorized.

Default retry policy:

```text
initial_backoff = 100 ms
max_backoff = 5,000 ms
backoff_multiplier = 2
jitter = full jitter
max_retries = configurable
```

### 7.4 Lease Handling

SDK task workers MUST:

1. Track lease tokens.
2. Renew leases before expiry when processing is long-running.
3. Surface stale lease errors.
4. Support graceful shutdown with lease release or timeout.
5. Avoid ACKing after lease expiry unless server reports already ACKed.

### 7.5 Timeout Behavior

SDK operations MUST expose timeout configuration.

Recommended defaults:

| Operation | Default Timeout |
|---|---:|
| Append | 5 seconds |
| Batch append | 10 seconds |
| Stream fetch | 30 seconds or long-poll configurable |
| Lease next | 5 seconds |
| ACK/NACK | 5 seconds |
| Renew lease | 5 seconds |
| DLQ operations | 10 seconds |
| Admin operations | 10 seconds |

---

## 8. SDK Architecture

### 8.1 Shared Core Components

All language SDKs SHOULD share behavioral design, even if implemented natively.

```text
┌────────────────────────────────────────────────────────────┐
│                    KEIROX SDK CORE                         │
│                                                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │ Connection   │  │ Auth         │  │ Protocol     │    │
│  │ Manager      │  │ Manager      │  │ Codec        │    │
│  └──────────────┘  └──────────────┘  └──────────────┘    │
│                                                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │ Retry        │  │ Idempotence  │  │ Telemetry    │    │
│  │ Engine       │  │ Manager      │  │ Exporter     │    │
│  └──────────────┘  └──────────────┘  └──────────────┘    │
│                                                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │ Producer     │  │ Stream       │  │ Task Worker  │    │
│  │ Client       │  │ Consumer     │  │ Client       │    │
│  └──────────────┘  └──────────────┘  └──────────────┘    │
│                                                            │
│  ┌──────────────┐  ┌──────────────┐                      │
│  │ DLQ Operator │  │ Arrow Flight │                      │
│  │ Client       │  │ Data Reader  │                      │
│  └──────────────┘  └──────────────┘                      │
└────────────────────────────────────────────────────────────┘
```

### 8.2 Transport Layer

| Transport | Use |
|---|---|
| gRPC | Control-plane operations, leases, ACKs, admin. |
| Arrow Flight | High-throughput data fetch and vectorized query. |
| TLS | Required for all external SDK traffic. |
| mTLS | Optional high-assurance authentication. |
| OAuth2/OIDC | Token-based authentication. |

### 8.3 Payload Support

SDKs MUST support:

| Payload Type | Requirement |
|---|---|
| Raw bytes | Mandatory. |
| UTF-8 string | Mandatory. |
| JSON | Recommended helper. |
| Protobuf | Recommended helper. |
| Arrow RecordBatch | Mandatory for high-throughput paths. |
| Avro | Optional later. |

### 8.4 Schema Integration

SDKs SHOULD support:

1. Schema ID attachment.
2. Schema fingerprint validation.
3. Optional schema registry lookup.
4. Strict/permissive mode configuration.
5. Structured payload serialization helpers.

---

## 9. Developer Experience Deliverables

### 9.1 Documentation

| Document | Purpose |
|---|---|
| Native SDK Quickstart | First append/fetch/lease in under 15 minutes. |
| Producer Guide | Idempotence, batching, retries, durability. |
| Stream Consumer Guide | Offset fetch, commit, replay. |
| Task Worker Guide | Lease, ACK, NACK, renewal, DLQ. |
| DLQ Operator Guide | List, inspect, redrive, purge. |
| Arrow Flight Query Guide | Vectorized reads and predicate pushdown. |
| Error Catalog | Typed errors, retry guidance, diagnostics. |
| Migration Guide | Moving from Kafka/SQS patterns to Keirox native APIs. |

### 9.2 Examples

Each supported language MUST include examples for:

1. Simple producer.
2. Batch producer.
3. Idempotent producer.
4. Stream consumer.
5. Task worker pool.
6. Lease renewal worker.
7. DLQ redrive operator.
8. Arrow Flight query client.
9. Telemetry-enabled client.
10. Graceful shutdown pattern.

### 9.3 Developer Tooling

| Tool | Purpose |
|---|---|
| CLI examples | Demonstrate local development flow. |
| Docker Compose dev environment | Run local Keirox node plus SDK examples. |
| SDK conformance runner | Validate language SDK against API contract. |
| Benchmark CLI | Measure append/fetch/lease throughput and latency. |
| Debug logging mode | Safe diagnostics without leaking secrets/payloads. |

---

## 10. SDK Performance Plan

### 10.1 Performance Targets

| Metric | Mandatory Target | Stretch Target |
|---|---:|---:|
| Rust SDK append throughput | ≥50 MB/s | ≥100 MB/s |
| Go SDK append throughput | ≥50 MB/s | ≥100 MB/s |
| Python prototype append throughput | ≥10 MB/s | ≥25 MB/s |
| Stream fetch latency p99 | ≤2 ms active data | ≤1.5 ms |
| Lease next latency p99 | ≤1 ms | ≤0.5 ms |
| ACK latency p99 | ≤1 ms | ≤0.5 ms |
| Arrow Flight query CPU efficiency | Measured | ≤1/3 CPU vs JVM Kafka consumer under vectorized workload |

**Normative rule:** The Arrow Flight CPU claim is conditional and MUST be validated under a defined benchmark profile before being used externally.

### 10.2 Benchmark Profiles

| Profile | Purpose |
|---|---|
| SDK-P1 | Append throughput and latency |
| SDK-P2 | Batch append efficiency |
| SDK-P3 | Stream fetch throughput |
| SDK-P4 | Queue lease/ACK churn |
| SDK-P5 | Arrow Flight vectorized query |
| SDK-P6 | Mixed producer/worker load |
| SDK-P7 | Long-running stability |

### 10.3 Benchmark Evidence Requirements

Each SDK benchmark run MUST record:

1. SDK language and version.
2. Server version and cluster configuration.
3. Network environment.
4. Payload size and batch size.
5. Concurrency level.
6. Latency percentiles.
7. Throughput.
8. Error rate.
9. CPU and memory usage.
10. Git commit hash.

---

## 11. SDK Conformance Plan

### 11.1 Conformance Test Classes

| Class | Purpose |
|---|---|
| API contract tests | Validate operation names, request fields, response fields. |
| Behavioral tests | Validate retry, idempotence, lease lifecycle. |
| Error tests | Validate typed errors and retry hints. |
| Compatibility tests | Validate version negotiation. |
| Security tests | Validate auth failure behavior. |
| Telemetry tests | Validate metrics and traces. |
| Stability tests | Validate graceful shutdown and reconnect behavior. |

### 11.2 Mandatory Conformance Scenarios

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| SDK-T-001 | Append single record | Offset returned. |
| SDK-T-002 | Append duplicate producer sequence | Original offset returned. |
| SDK-T-003 | Append batch | Batch offsets returned. |
| SDK-T-004 | Server unavailable during append | Retryable error surfaced. |
| SDK-T-005 | Stream fetch from offset | Correct records returned. |
| SDK-T-006 | Stream fetch with committed offset | Committed offset respected. |
| SDK-T-007 | Lease next | Lease token returned. |
| SDK-T-008 | ACK with correct token | Success. |
| SDK-T-009 | ACK with stale token | Stale lease error. |
| SDK-T-010 | Duplicate ACK | Idempotent success. |
| SDK-T-011 | NACK with requeue | Record becomes ready. |
| SDK-T-012 | Lease expiry | Record re-leased or DLQ-evicted by policy. |
| SDK-T-013 | Renew lease | Expiry extended. |
| SDK-T-014 | DLQ list | Evicted entries visible. |
| SDK-T-015 | DLQ redrive | Entries requeued. |
| SDK-T-016 | Unauthorized operation | Permission error surfaced. |
| SDK-T-017 | Unsupported operation | Explicit unsupported error surfaced. |
| SDK-T-018 | Version mismatch | Negotiation fallback or explicit error. |

---

## 12. SDK Observability Requirements

### 12.1 Client Metrics

| Metric | Type | Description |
|---|---|---|
| `keirox_sdk_append_total` | Counter | Append operations by status. |
| `keirox_sdk_append_bytes_total` | Counter | Bytes appended. |
| `keirox_sdk_append_latency_seconds` | Histogram | Append latency. |
| `keirox_sdk_fetch_total` | Counter | Fetch operations. |
| `keirox_sdk_lease_total` | Counter | Lease operations. |
| `keirox_sdk_ack_total` | Counter | ACK operations by mode/status. |
| `keirox_sdk_nack_total` | Counter | NACK operations. |
| `keirox_sdk_retry_total` | Counter | Retries by error class. |
| `keirox_sdk_active_leases` | Gauge | Active leases tracked by worker. |
| `keirox_sdk_connection_state` | Gauge | Connected/reconnecting/failed. |

### 12.2 Client Traces

SDKs SHOULD emit trace spans for:

- Append.
- Batch append.
- Fetch.
- Lease.
- ACK/NACK.
- Renew.
- DLQ redrive.
- Reconnect.
- Authentication.

### 12.3 Safe Logging Rules

SDK logs MUST NOT include:

- Secrets.
- Tokens.
- Private keys.
- Full payloads unless explicitly enabled in debug mode.
- Tenant data in shared logs without redaction.

---

## 13. SDK Security Requirements

| Requirement | Description |
|---|---|
| TLS required | SDK MUST NOT default to plaintext transport. |
| Credential safety | Credentials MUST be loaded from environment, config, or secret provider. |
| Token rotation | OAuth token refresh MUST be supported. |
| mTLS support | High-assurance deployments MUST be supported. |
| Authorization errors | Permission errors MUST be typed and visible. |
| No secret leakage | Errors and logs MUST NOT expose credentials. |
| Config validation | SDK MUST reject insecure default configurations in production mode. |

---

## 14. Milestones and Deliverables

| Milestone | Target Weeks | Deliverables | Exit Criteria |
|---|---|---|---|
| M3.SDK-1 | Weeks 6–10 | Core Rust client | Append/fetch/lease/ACK/NACK working |
| M3.SDK-2 | Weeks 8–14 | Go client alpha | Core operations pass conformance |
| M3.SDK-3 | Weeks 10–16 | Python prototype | Basic append/fetch/lease demo |
| M3.SDK-4 | Weeks 12–18 | Retry/idempotence hardening | Behavioral tests pass |
| M3.SDK-5 | Weeks 14–20 | Telemetry integration | Metrics/traces validated |
| M3.SDK-6 | Weeks 16–22 | Arrow Flight query path | Vectorized fetch/query demo |
| M3.SDK-7 | Weeks 18–24 | Documentation and examples | Quickstart and guides published |
| M3.SDK-8 | Weeks 22–28 | Benchmark suite | SDK evidence package produced |
| M3.SDK-9 | Weeks 26–30 | Java/TypeScript design specs | API design reviewed and approved |

---

## 15. SDK Release Gates

### 15.1 Rust Beta Gate

| Criterion | Mandatory |
|---|---|
| Core operations pass conformance | Yes |
| Retry and idempotence tests pass | Yes |
| Telemetry validated | Yes |
| Benchmark evidence produced | Yes |
| Documentation complete | Yes |
| Security review passed | Yes |
| No critical defects open | Yes |

### 15.2 Go Alpha Gate

| Criterion | Mandatory |
|---|---|
| Core operations pass conformance | Yes |
| Retry behavior validated | Yes |
| Examples pass | Yes |
| Basic benchmark evidence produced | Yes |
| Known limitations documented | Yes |

### 15.3 Python Prototype Gate

| Criterion | Mandatory |
|---|---|
| Basic append/fetch/lease demo works | Yes |
| Examples pass | Yes |
| Limitations documented | Yes |
| No production support claim | Yes |

---

## 16. Evidence Package

The SDK evidence package MUST include:

1. API conformance report.
2. Behavioral test report.
3. Error handling report.
4. Retry/idempotence report.
5. Benchmark report by language.
6. Telemetry validation report.
7. Documentation inventory.
8. Example execution report.
9. Known limitations list.
10. Security review summary.
11. SDK maturity recommendation.

---

## 17. Risks and Mitigations

| Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|
| SDK scope expands to too many languages | High | High | Strict Phase 3 language targets; Java/TS design-only. |
| API instability breaks early users | High | Medium | Freeze API contract at Beta; semver discipline. |
| Performance claims overused | Medium | High | Benchmark evidence required before external claims. |
| Retry logic causes duplicate side effects | High | Medium | Document idempotence and ACK mode implications. |
| Lease renewal complexity confuses developers | Medium | Medium | Provide worker helper and examples. |
| Telemetry overhead harms performance | Medium | Medium | Sampling and low-overhead metrics design. |
| Language bindings drift from core contract | High | Medium | Shared conformance suite for all SDKs. |
| Documentation lags implementation | Medium | High | Docs included in release gate. |

---

## 18. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial Native SDK & Developer Experience Plan. Defines language roadmap, API surface, behavioral contracts, client architecture, documentation, benchmarks, conformance tests, observability, security requirements, release gates, and SDK evidence package. |