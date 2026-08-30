# KEI-SPIKE-301 — Ecosystem Gateway & Lakehouse Prototype Plan

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-SPIKE-301 |
| Title | Ecosystem Gateway & Lakehouse Prototype Plan |
| Version | 1.0 |
| Level | Engineering Execution Plan |
| Status | Baseline — Ready for Execution |
| Phase | Phase 3 Engineering Bridge |
| Duration | 90 days / 12 weeks |
| Owner | Ecosystem Engineering Lead |
| Governing Plan | KEI-ENG-300 — Phase 3 Engineering Execution Plan |
| Governing Architecture Documents | KEI-ARC-023, KEI-ARC-024, KEI-DES-032, KEI-DES-033, KEI-DES-034, KEI-DES-035 |
| Predecessor | KEI-SPIKE-201 (Distributed Consensus Prototype) |
| Next Plan File | KEI-COMPAT-301 — Protocol Compatibility Certification Plan |

---

## 2. Executive Summary

Phase 1 proved the Golden Invariant on a single node. Phase 2 proved distributed durability and coordinator sharding. Phase 3 must prove that Keirox is adoptable by real engineering teams.

This prototype validates three adoption-critical capabilities:

1. **Kafka Wire Protocol Ingest Gateway** — existing Kafka producers can write to Keirox without code changes, within a certified compatibility subset.
2. **Native Arrow Flight / gRPC SDK** — new applications can use the high-performance native path for streaming, leasing, ACK/NACK, and vectorized reads.
3. **Apache Iceberg Catalog Committer** — sealed Keirox data becomes directly queryable as governed Iceberg tables.

The prototype is not a production release. It is a focused 90-day executable proof that the ecosystem bridge works end-to-end.

The prototype must answer the following question:

> Can an unmodified Kafka producer write into Keirox, can a native SDK consumer process the same data as a stream or leased task queue, and can that same data be queried as an Iceberg table with governed freshness — all without violating the Golden Invariant or compatibility governance?

If the answer is yes, the project proceeds into full Phase 3 hardening and compatibility certification.

---

## 3. Prototype Mission

The prototype mission is:

1. Prove Kafka producer migration is possible through a bounded compatibility subset.
2. Prove the native SDK provides a cleaner and faster developer path.
3. Prove Iceberg commits are idempotent, recoverable, and queryable.
4. Prove schema governance works during live ingestion.
5. Produce early evidence for compatibility, freshness, and gateway overhead.
6. Reduce Phase 3 integration risk before full certification.

---

## 4. Relationship to KEI-ENG-300

This prototype executes the first practical stage of Phase 3 and maps directly to the work packages defined in KEI-ENG-300.

| KEI-ENG-300 Work Package | Prototype Coverage |
|---|---|
| WP-P3-A: Kafka Gateway Foundation | Core prototype focus — Kafka produce, metadata, fetch, idempotence, unsupported behavior |
| WP-P3-B: Native SDK Foundation | Core prototype focus — Rust/Go alpha SDK, Arrow Flight path, lease/ACK APIs |
| WP-P3-C: Iceberg Committer Productionization | Included — commit ledger, snapshots, basic manifest hygiene |
| WP-P3-D: Schema Governance & Shredding | Included — schema registry, schema fingerprint, unstructured payload fallback |
| WP-P3-E: Compatibility Certification | Early compatibility matrix and negative tests |
| WP-P3-F: Lakehouse Query Readiness | Early DuckDB/Polars query validation and freshness measurement |

The prototype intentionally compresses these work packages into a 90-day executable proof, deferring production hardening to the full Phase 3 build.

---

## 5. Prototype Scope

### 5.1 Must Have

The prototype MUST include:

1. Kafka gateway endpoint accepting certified producer requests.
2. Kafka topic-to-stream mapping.
3. Kafka virtual partition mapping.
4. Kafka idempotent produce mapping.
5. Kafka metadata endpoint.
6. Kafka unsupported operation rejection.
7. Native gRPC/Arrow Flight API endpoint.
8. Native SDK append operation.
9. Native SDK stream fetch operation.
10. Native SDK lease operation.
11. Native SDK ACK/NACK operation.
12. Native SDK renew lease operation.
13. Iceberg commit ledger.
14. Iceberg snapshot creation.
15. Iceberg table query validation using DuckDB.
16. Iceberg table query validation using Polars.
17. Schema registry registration and resolution.
18. Schema fingerprinting.
19. Basic schema evolution behavior.
20. `_unstructured_payload` fallback behavior.
21. Gateway metrics.
22. SDK client metrics.
23. Iceberg commit metrics.
24. End-to-end demonstration scenarios.
25. Prototype evidence report.

### 5.2 Should Have

The prototype SHOULD include if schedule permits:

1. Kafka fetch path for stream consumers.
2. Kafka offset commit/fetch mapping.
3. Kafka consumer group coordination subset.
4. Go SDK alpha.
5. Python SDK prototype.
6. Spark Iceberg query validation.
7. Fast-mode freshness demonstration.
8. 24-hour gateway soak test.
9. Basic SQS translation design spike.
10. Basic AMQP translation design spike.

### 5.3 Will Not Have

The prototype WILL NOT include:

1. Full Kafka broker parity.
2. Kafka transactions.
3. Kafka admin reassignment or partition management.
4. Production SQS gateway.
5. Production AMQP gateway.
6. Full Jepsen certification.
7. Multi-region replication.
8. KMS production encryption.
9. Full ABAC authorization production rollout.
10. Customer-facing documentation.
11. In-broker SQL or materialized views.
12. CXL/RDMA hardware paths.

---

## 6. Prototype Success Criteria

### 6.1 Functional Success Criteria

| ID | Criterion |
|---|---|
| SPIKE-P3-F-001 | Unmodified Kafka producer writes to Keirox through the gateway. |
| SPIKE-P3-F-002 | Kafka idempotent produce duplicates are safely deduplicated. |
| SPIKE-P3-F-003 | Unsupported Kafka operations return explicit protocol-native errors. |
| SPIKE-P3-F-004 | Native SDK appends records to Keirox. |
| SPIKE-P3-F-005 | Native SDK fetches records by stream offset. |
| SPIKE-P3-F-006 | Native SDK leases records from a consumer group. |
| SPIKE-P3-F-007 | Native SDK ACKs leased records out of order. |
| SPIKE-P3-F-008 | Native SDK NACKs leased records and triggers requeue. |
| SPIKE-P3-F-009 | Native SDK renews active leases. |
| SPIKE-P3-F-010 | Sealed data is committed to an Iceberg table. |
| SPIKE-P3-F-011 | Iceberg table is queryable by DuckDB. |
| SPIKE-P3-F-012 | Iceberg table is queryable by Polars. |
| SPIKE-P3-F-013 | Schema registry registers and resolves schemas. |
| SPIKE-P3-F-014 | Unknown or polymorphic fields route to `_unstructured_payload`. |
| SPIKE-P3-F-015 | Gateway restart does not lose committed data. |
| SPIKE-P3-F-016 | Iceberg committer restart does not duplicate committed snapshots. |

### 6.2 Performance Success Criteria

| ID | Criterion | Mandatory Target | Stretch Target |
|---|---|---:|---:|
| SPIKE-P3-P-001 | Gateway append throughput | ≥50 MB/s | ≥100 MB/s |
| SPIKE-P3-P-002 | Gateway translation overhead | p99 ≤1.0 ms | p99 ≤0.5 ms |
| SPIKE-P3-P-003 | Native SDK append throughput | ≥50 MB/s | ≥100 MB/s |
| SPIKE-P3-P-004 | Native SDK fetch latency | p99 ≤2.0 ms | p99 ≤1.5 ms |
| SPIKE-P3-P-005 | Native SDK lease latency | p99 ≤1.0 ms | p99 ≤0.5 ms |
| SPIKE-P3-P-006 | Native SDK ACK latency | p99 ≤1.0 ms | p99 ≤0.5 ms |
| SPIKE-P3-P-007 | Default Iceberg freshness | ≤60 seconds | ≤30 seconds |
| SPIKE-P3-P-008 | Fast-mode Iceberg freshness | Not mandatory | ≤5 seconds |

### 6.3 Reliability Success Criteria

| ID | Criterion | Mandatory Target |
|---|---|---|
| SPIKE-P3-R-001 | No loss of committed data during gateway restart | Zero |
| SPIKE-P3-R-002 | No duplicate Iceberg snapshots after committer restart | Zero |
| SPIKE-P3-R-003 | No invariant violations during end-to-end tests | Zero |
| SPIKE-P3-R-004 | Unsupported operations do not corrupt state | Zero |
| SPIKE-P3-R-005 | Schema evolution preserves historical readability | Pass |
| SPIKE-P3-R-006 | 24-hour soak stable | No unbounded growth |

---

## 7. Prototype Architecture Slice

### 7.1 Prototype Topology

```text
┌────────────────────────────────────────────────────────────────────────┐
│                    ECOSYSTEM PROTOTYPE TOPOLOGY                        │
│                                                                        │
│  Kafka Producer                                                        │
│  (unmodified, certified subset)                                        │
│        │                                                               │
│        ▼                                                               │
│  ┌──────────────────────┐                                              │
│  │ Kafka Gateway        │                                              │
│  │ - produce            │                                              │
│  │ - metadata           │                                              │
│  │ - idempotence        │                                              │
│  │ - unsupported errors │                                              │
│  └──────────┬───────────┘                                              │
│             │                                                          │
│             ▼                                                          │
│  ┌──────────────────────────────────────────────────────────────┐     │
│  │                 KEIROX CORE CLUSTER                          │     │
│  │  Storage Engine + State Plane + Coordinator + S3 Streaming   │     │
│  └───────────────┬──────────────────────────┬───────────────────┘     │
│                  │                          │                          │
│                  ▼                          ▼                          │
│  ┌──────────────────────┐     ┌──────────────────────────┐            │
│  │ Native SDK Client    │     │ Iceberg Committer        │            │
│  │ - Append             │     │ - Commit ledger          │            │
│  │ - StreamFetch        │     │ - Snapshot creation      │            │
│  │ - LeaseNext          │     │ - Manifest management    │            │
│  │ - Ack/Nack           │     │ - Freshness controller   │            │
│  │ - RenewLease         │     └────────────┬─────────────┘            │
│  └──────────────────────┘                  │                           │
│                                            ▼                           │
│                              ┌──────────────────────────┐              │
│                              │ Object Storage + Iceberg │              │
│                              │ Catalog                  │              │
│                              └────────────┬─────────────┘              │
│                                           │                            │
│                                           ▼                            │
│                              ┌──────────────────────────┐              │
│                              │ DuckDB / Polars / Spark  │              │
│                              └──────────────────────────┘              │
└────────────────────────────────────────────────────────────────────────┘
```

### 7.2 Simplifications

| Full Architecture Feature | Prototype Simplification |
|---|---|
| Full Kafka parity | Certified subset only |
| Kafka transactions | Explicitly unsupported |
| Full SQS/AMQP gateways | Excluded; design notes only |
| Production KMS encryption | Disabled or local key stub |
| Full ABAC authorization | Disabled or simplified tenant check |
| Multi-region replication | Excluded |
| Full Iceberg catalog concurrency | Single-writer committer with basic conflict handling |
| Full schema registry governance | Basic registry with versioning and fingerprints |
| Java/TypeScript SDKs | Design only |

---

## 8. Technical Constraints

### 8.1 Gateway Constraints

| Constraint | Requirement |
|---|---|
| Protocol boundary | Implement only certified Kafka APIs |
| Unsupported APIs | Return explicit protocol-native error |
| Transactions | Reject with unsupported error |
| Virtual partitions | Map to Keirox state shard buckets |
| Idempotence | Map Kafka producer ID/sequence to Keirox producer identity |
| Metrics | Request count by API/version/status |

### 8.2 SDK Constraints

| Constraint | Requirement |
|---|---|
| API contract | MUST match KEI-DES-032 |
| Transport | gRPC + Arrow Flight |
| Languages | Rust first; Go optional |
| Error handling | Typed errors with retry hints |
| Telemetry | Client-side latency histograms |
| Idempotence | Expose producer ID/sequence controls |

### 8.3 Iceberg Constraints

| Constraint | Requirement |
|---|---|
| Commit ledger | Durable and idempotent |
| Snapshot operation | Append-only snapshots |
| Manifest hygiene | Basic manifest tracking |
| Orphan cleanup | Manual or limited automatic cleanup |
| Catalog adapter | Pluggable; default REST/JDBC/MinIO-compatible |
| Legal hold | Not fully implemented; design hook only |

### 8.4 Schema Constraints

| Constraint | Requirement |
|---|---|
| Schema registry | Register/resolve by schema ID and fingerprint |
| Field cap | 64 shredded primitive keys |
| Fallback | `_unstructured_payload` for unknown/polymorphic fields |
| Evolution | Add nullable columns only |
| Unsafe changes | Require new schema version |

---

## 9. Work Packages

### 9.1 WP-0 — Prototype Engineering Foundation

Objective:

Prepare the repository, environment, and integration test harness for ecosystem prototyping.

Deliverables:

1. Prototype branch/workspace.
2. CI pipeline for gateway, SDK, and Iceberg committer.
3. Local object storage environment (MinIO or S3-compatible).
4. Local Iceberg catalog environment.
5. End-to-end test harness.
6. Demo scenario scripts.
7. Metrics collection pipeline.

Exit criteria:

- Prototype components build.
- CI passes.
- Local object storage and catalog accessible.
- End-to-end test harness runs.

---

### 9.2 WP-1 — Kafka Gateway Spike

Objective:

Prove that certified Kafka producers can write to Keirox without modification.

Deliverables:

1. Kafka request decoder.
2. Kafka response encoder.
3. ApiVersions handler.
4. Metadata handler.
5. Produce handler.
6. Idempotent produce mapping.
7. Unsupported operation handler.
8. Topic-to-stream mapper.
9. Virtual partition mapper.
10. Gateway metrics.

Exit criteria:

- Kafka producer can append records.
- Duplicate idempotent produces are deduplicated.
- Unsupported operations return explicit errors.
- Gateway metrics are observable.
- Gateway restart does not lose committed data.

Primary references:

- KEI-ARC-024
- KEI-DES-032
- KEI-DES-035

---

### 9.3 WP-2 — Native SDK Spike

Objective:

Prove the high-performance native path for append, fetch, lease, ACK/NACK, and renew.

Deliverables:

1. Native API client core.
2. Append operation.
3. StreamFetch operation.
4. LeaseNext operation.
5. Ack operation.
6. Nack operation.
7. RenewLease operation.
8. Retry/backoff policy.
9. Client telemetry.
10. Rust SDK examples.
11. Optional Go SDK examples.

Exit criteria:

- Native SDK can append records.
- Native SDK can fetch by offset.
- Native SDK can lease, ACK, NACK, and renew.
- Out-of-order ACK works.
- Client telemetry is observable.
- SDK examples pass integration tests.

Primary references:

- KEI-DES-032
- KEI-ARC-024

---

### 9.4 WP-3 — Iceberg Committer Spike

Objective:

Prove that sealed data can be committed to Iceberg tables idempotently and queried externally.

Deliverables:

1. Commit ledger.
2. Commit batcher.
3. Snapshot creator.
4. Manifest tracker.
5. Catalog adapter.
6. Restart recovery.
7. Duplicate commit prevention.
8. Commit metrics.
9. Freshness controller stub.
10. DuckDB query validation script.
11. Polars query validation script.

Exit criteria:

- Committed Iceberg table is queryable by DuckDB.
- Committed Iceberg table is queryable by Polars.
- Commit ledger survives restart.
- Duplicate snapshots are prevented.
- Commit metrics are observable.
- Default freshness target measured.

Primary references:

- KEI-DES-034
- KEI-ARC-023

---

### 9.5 WP-4 — Schema Governance Spike

Objective:

Prove that schema registration, fingerprinting, evolution, and unstructured fallback work during live ingestion.

Deliverables:

1. Schema registry storage.
2. Schema registration API.
3. Schema resolution API.
4. Schema fingerprint generation.
5. Schema version tracking.
6. Field promotion stub.
7. 64-field cap enforcement.
8. `_unstructured_payload` routing.
9. Schema evolution test scenarios.

Exit criteria:

- Schema registry registers and resolves schemas.
- Schema fingerprint is stable.
- New nullable fields can be added.
- Unknown fields route to `_unstructured_payload`.
- Historical data remains readable.
- Unsafe type changes require new schema version.

Primary references:

- KEI-DES-033
- KEI-ARC-023

---

### 9.6 WP-5 — Compatibility Evidence Spike

Objective:

Produce early compatibility evidence and unsupported-behavior validation.

Deliverables:

1. Certified Kafka subset matrix.
2. Supported operation test suite.
3. Unsupported operation test suite.
4. Idempotence test suite.
5. Gateway error mapping report.
6. Client compatibility report.

Exit criteria:

- Certified subset matrix generated.
- Supported operations pass.
- Unsupported operations return explicit errors.
- No silent approximation behavior detected.
- Compatibility report generated.

Primary references:

- KEI-DES-035
- KEI-ARC-024

---

### 9.7 WP-6 — Lakehouse Evidence Spike

Objective:

Prove that committed lakehouse tables are queryable, fresh, and operationally understandable.

Deliverables:

1. DuckDB validation suite.
2. Polars validation suite.
3. Freshness measurement tool.
4. File size report.
5. Snapshot count report.
6. Basic manifest hygiene report.
7. Lakehouse evidence summary.

Exit criteria:

- DuckDB queries pass.
- Polars queries pass.
- Default freshness measured.
- Fast mode measured if implemented.
- No small-file explosion observed in prototype scope.
- Evidence report generated.

Primary references:

- KEI-DES-034
- KEI-ARC-023

---

## 10. 12-Week Execution Plan

### Week 1–2 — Prototype Mobilization

Primary work:

- Set up prototype workspace.
- Configure CI.
- Deploy local object storage.
- Deploy local Iceberg catalog.
- Create end-to-end test harness.
- Define demo scenarios.

Exit:

- Prototype environment operational.
- CI passes.
- Test harness runs.

---

### Week 3–4 — Kafka Gateway Foundation

Primary work:

- Implement Kafka request decoder.
- Implement metadata and produce handlers.
- Implement topic-to-stream mapping.
- Implement idempotent produce mapping.
- Implement unsupported operation handler.

Exit:

- Kafka producer writes to Keirox.
- Duplicate produces deduplicated.
- Unsupported operations rejected explicitly.

---

### Week 5–6 — Native SDK Foundation

Primary work:

- Implement native API client core.
- Implement append and stream fetch.
- Implement lease and ACK/NACK.
- Implement renew lease.
- Add client telemetry.

Exit:

- Native SDK append/fetch works.
- Native SDK lease/ACK/NACK works.
- Out-of-order ACK demonstrated.

---

### Week 7 — Iceberg Committer Foundation

Primary work:

- Implement commit ledger.
- Implement commit batcher.
- Implement snapshot creator.
- Implement catalog adapter.
- Add restart recovery.

Exit:

- Iceberg table created.
- Commit ledger survives restart.
- Duplicate commits prevented.

---

### Week 8 — Schema Governance Foundation

Primary work:

- Implement schema registry.
- Implement schema fingerprinting.
- Implement unstructured payload routing.
- Implement 64-field cap.
- Add schema evolution tests.

Exit:

- Schema registry resolves schemas.
- Unknown fields route to `_unstructured_payload`.
- Schema evolution preserves historical readability.

---

### Week 9 — End-to-End Integration

Primary work:

- Connect gateway to core cluster.
- Connect native SDK to core cluster.
- Connect Iceberg committer to sealed data path.
- Connect schema registry to shredding pipeline.
- Run end-to-end demo scenarios.

Exit:

- All demo scenarios pass.
- Metrics are observable.
- No invariant violations detected.

---

### Week 10 — Compatibility and Negative Testing

Primary work:

- Run supported Kafka operation tests.
- Run unsupported Kafka operation tests.
- Run idempotence tests.
- Run SDK error handling tests.
- Generate compatibility matrix.

Exit:

- Compatibility matrix generated.
- Unsupported behavior explicit.
- No silent approximation detected.

---

### Week 11 — Lakehouse and Freshness Validation

Primary work:

- Run DuckDB validation.
- Run Polars validation.
- Measure default freshness.
- Measure fast mode if available.
- Generate file and snapshot hygiene report.

Exit:

- Lakehouse tables queryable.
- Freshness evidence collected.
- File hygiene evidence collected.

---

### Week 12 — Evidence Report and Go/No-Go Review

Primary work:

- Compile functional test results.
- Compile performance results.
- Compile compatibility results.
- Compile lakehouse results.
- Prepare go/no-go recommendation.
- Present to Architecture Review Board.

Exit:

- Prototype evidence package delivered.
- Go/no-go decision made.

---

## 11. End-to-End Demonstration Scenarios

The prototype MUST demonstrate the following scenarios.

### Scenario 1 — Kafka Producer Migration

```text
Kafka producer
   ↓ Kafka protocol
Kafka gateway
   ↓ Keirox append
Immutable WAL
   ↓ Stream fetch
Native SDK consumer
```

Expected result:

- Existing Kafka producer writes without modification.
- Data is durable in Keirox.
- Native SDK can read the same data as a stream.

### Scenario 2 — Queue Worker Processing

```text
Kafka producer
   ↓ Kafka protocol
Kafka gateway
   ↓ Keirox append
Immutable WAL
   ↓ LeaseNext
Native SDK worker
   ↓ Out-of-order ACK
State plane
```

Expected result:

- Worker leases tasks.
- Worker ACKs tasks out of order.
- State plane updates correctly.
- Watermark advances.

### Scenario 3 — Lakehouse Query

```text
Keirox sealed segments
   ↓ Arrow/Parquet export
Iceberg committer
   ↓ Iceberg snapshot
DuckDB / Polars query
```

Expected result:

- Committed data is queryable.
- Query result matches source records.
- Freshness target measured.

### Scenario 4 — Schema Evolution

```text
Producer v1 schema
   ↓ ingest
Producer v2 schema with new field
   ↓ ingest
Schema registry evolution
   ↓
Historical data readable
```

Expected result:

- New field is added safely.
- Historical data remains readable.
- Unknown/polymorphic fields route to `_unstructured_payload`.

### Scenario 5 — Unsupported Operation Rejection

```text
Kafka client requests transactional operation
   ↓
Kafka gateway
   ↓
Explicit unsupported error
```

Expected result:

- Operation rejected.
- No partial state change.
- Error is protocol-native and documented.

---

## 12. Test Plan

### 12.1 Unit Tests

Required for:

- Kafka request/response encoding.
- Topic-to-stream mapping.
- Virtual partition mapping.
- Idempotence key mapping.
- SDK request builders.
- SDK retry policy.
- Commit ledger logic.
- Schema fingerprint generation.
- Schema evolution rules.

### 12.2 Integration Tests

Required for:

- Kafka producer to Keirox append.
- Native SDK append/fetch.
- Native SDK lease/ACK/NACK.
- Iceberg commit and query.
- Schema registry registration/resolution.
- Gateway restart recovery.
- Committer restart recovery.

### 12.3 Negative Tests

Required for:

- Unsupported Kafka API.
- Unsupported Kafka version.
- Transactional Kafka request.
- Invalid schema evolution.
- Duplicate commit replay.
- Malformed SDK request.
- Malformed Kafka frame.
- Corrupted commit ledger entry.

### 12.4 End-to-End Tests

Required for:

- Kafka produce → native stream fetch.
- Kafka produce → native lease/ACK.
- Kafka produce → Iceberg query.
- Schema evolution → Iceberg query.
- Unsupported operation → explicit error.

---

## 13. Benchmark Plan

### 13.1 Prototype Benchmark Profiles

| Profile | Purpose | Workload |
|---|---|---|
| P1-P3-Proto | Gateway append throughput and latency | 1 KB messages via Kafka gateway |
| P2-P3-Proto | Native SDK throughput and latency | 1 KB messages via native SDK |
| P3-P3-Proto | Queue churn via native SDK | Lease/ACK/NACK churn |
| P4-P3-Proto | Iceberg freshness | Continuous ingest with commit cadence |
| P5-P3-Proto | Gateway + export interference | Gateway load while Iceberg commits |

### 13.2 Benchmark Metrics

| Metric | Required |
|---|---|
| Gateway append throughput | Yes |
| Gateway append latency p50/p99 | Yes |
| Gateway translation overhead | Yes |
| Native SDK append throughput | Yes |
| Native SDK fetch latency | Yes |
| Native SDK lease latency | Yes |
| Native SDK ACK latency | Yes |
| Iceberg commit latency | Yes |
| Iceberg freshness | Yes |
| Commit ledger replay time | Yes |
| Error rate | Yes |

---

## 14. Evidence Package

The prototype evidence package MUST include:

1. Functional test report.
2. Integration test report.
3. Negative test report.
4. Compatibility matrix.
5. Gateway performance report.
6. Native SDK performance report.
7. Iceberg commit report.
8. Freshness measurement report.
9. Schema evolution report.
10. Demonstration scenario report.
11. Known defects list.
12. Unresolved risks list.
13. Go/no-go recommendation.

---

## 15. Prototype Go/No-Go Gate

### 15.1 Go Criteria

A GO decision requires:

1. All functional mandatory criteria pass.
2. All mandatory performance criteria pass.
3. All reliability mandatory criteria pass.
4. Compatibility matrix generated and approved.
5. Unsupported operations explicitly rejected.
6. Iceberg tables queryable by DuckDB and Polars.
7. Schema evolution tests pass.
8. Evidence package complete.
9. No unresolved invariant violations.

### 15.2 Conditional Go Criteria

A CONDITIONAL GO may be granted if:

1. One or more stretch targets fail.
2. A non-critical defect remains open.
3. Performance is close to target with clear remediation.
4. A remediation plan is approved.

### 15.3 Gate Outcomes

| Outcome | Meaning |
|---|---|
| GO | Continue into full Phase 3 hardening. |
| CONDITIONAL GO | Continue after specific fixes. |
| PIVOT | Core ecosystem assumption needs adjustment. |
| STOP | Core adoption path is invalid. |

---

## 16. Risks and Mitigations

| Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|
| Kafka client behavior differs from specification | High | High | Test against multiple real clients; define certified subset tightly. |
| Gateway overhead exceeds target | High | Medium | Profile early; minimize serialization copies; isolate gateway threads. |
| Arrow Flight SDK complexity delays delivery | Medium | Medium | Rust SDK first; defer additional languages. |
| Iceberg catalog concurrency issues | Medium | Medium | Single-writer committer in prototype; add conflict handling later. |
| Schema evolution edge cases explode scope | High | Medium | Enforce simple nullable additions only; unsafe changes require new version. |
| Local Iceberg environment differs from production catalog | Medium | Medium | Use pluggable catalog adapter; document environment assumptions. |
| Unsupported operation behavior is ambiguous | High | Medium | Define explicit error mapping; negative tests mandatory. |
| Prototype scope creep into full gateway parity | High | High | Strict scope exclusions; ARB approval required for additions. |

---

## 17. Prototype Team

### 17.1 Minimum Team

| Role | Count | Responsibility |
|---|---:|---|
| Ecosystem Engineering Lead | 1 | Overall prototype execution and gate reporting. |
| Gateway Engineer | 1 | Kafka protocol gateway and compatibility behavior. |
| SDK Engineer | 1 | Native gRPC/Arrow Flight SDK. |
| Lakehouse Engineer | 1 | Iceberg committer and catalog integration. |
| Schema/Data Engineer | 1 | Schema registry and adaptive shredding governance. |
| QA Engineer | 1 | Compatibility tests, integration tests, evidence package. |

### 17.2 Optional Support

| Role | Responsibility |
|---|---|
| Chief Architect | Architecture compliance and conflict resolution. |
| SRE Advisor | Metrics, observability, environment stability. |
| Security Advisor | Ensure gateway does not introduce unsafe defaults. |
| Developer Experience Advisor | SDK usability and example quality. |

---

## 18. Definition of Done

The prototype is done when:

1. Kafka gateway writes certified producer traffic.
2. Native SDK supports append, fetch, lease, ACK, NACK, and renew.
3. Iceberg committer produces queryable tables.
4. Schema registry supports registration, resolution, and evolution.
5. Unsupported operations are explicit and safe.
6. Demonstration scenarios pass.
7. Mandatory performance targets measured.
8. Evidence package complete.
9. Known defects documented.
10. Go/no-go recommendation delivered.

---

## 19. Traceability to Architecture Documents

| Prototype Area | Governing Document |
|---|---|
| Protocol gateways architecture | KEI-ARC-024 |
| Native API contract | KEI-DES-032 |
| Compatibility matrices | KEI-DES-035 |
| Schema registry and shredding | KEI-DES-033 |
| Iceberg committer | KEI-DES-034 |
| Columnar ELT architecture | KEI-ARC-023 |
| Operability and metrics | KEI-ARC-027 |
| Validation requirements | KEI-OPS-041 |
| Phase 3 execution plan | KEI-ENG-300 |

---

## 20. Next Planning File

After this document, the next planning file is:

```text
KEI-COMPAT-301_Protocol_Compatibility_Certification_Plan.md
```

It will define the formal compatibility certification process, client conformance matrix, negative test governance, certification levels, and release publication rules.

---

## 21. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial Ecosystem Gateway & Lakehouse Prototype Plan. Defines 90-day prototype scope, work packages, demonstration scenarios, test plan, benchmark plan, evidence package, go/no-go gate, and traceability to Phase 3 architecture documents. |