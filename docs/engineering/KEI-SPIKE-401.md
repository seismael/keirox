# KEI-SPIKE-401 — Enterprise Hardening & Multi-Region Prototype Plan

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-SPIKE-401 |
| Title | Enterprise Hardening & Multi-Region Prototype Plan |
| Version | 1.0 |
| Level | Engineering Execution Plan |
| Status | Baseline — Ready for Execution |
| Phase | Phase 4 Engineering Bridge |
| Duration | 90 days / 12 weeks |
| Owner | Security & Multi-Region Engineering Lead |
| Governing Plan | KEI-ENG-400 — Phase 4 Engineering Execution Plan |
| Governing Architecture Documents | KEI-ARC-025, KEI-ARC-026, KEI-DES-035, KEI-DES-036, KEI-OPS-040 |
| Predecessor | KEI-SPIKE-301 (Phase 3 Ecosystem Prototype) |
| Next Plan File | KEI-SEC-401 — Security and Compliance Certification Plan |

---

## 2. Executive Summary

Phase 3 proved that Keirox is adoptable via Kafka compatibility, native SDKs, and Iceberg lakehouse integration. Phase 4 must prove that Keirox is **safe, governable, and survivable** for regulated, multi-tenant enterprise production workloads.

This prototype validates four enterprise-critical capabilities:

1. **Envelope Encryption & Crypto-Shredding** — data is encrypted at rest, keys are managed via KMS, and destroying a key renders all associated data cryptographically unrecoverable.
2. **Default-Deny Authorization (ABAC)** — tenant isolation is enforced at the protocol edge, and cross-tenant access is blocked and audited.
3. **Multi-Region Mode A Replication** — a primary region asynchronously replicates WAL tails to a replica region, and region-epoch fencing prevents split-brain writes during failover.
4. **Advanced Queue Gateways** — basic SQS and AMQP translation gateways map legacy queue semantics to Keirox lease/ACK state machines.

The prototype is a focused 90-day executable proof. It must answer the following question:

> Can Keirox encrypt data at rest, cryptographically erase a tenant without deleting the physical log, isolate tenants via ABAC, replicate data to a secondary region, fence split-brain writes during a network partition, and accept basic SQS/AMQP traffic — all without violating the Golden Invariant or failing insecurely?

If the answer is yes, the project proceeds into full Phase 4 hardening, compliance evidence collection, and Jepsen-style adversarial testing.

---

## 3. Prototype Mission

The prototype mission is:

1. Prove fail-secure encryption at rest and in transit.
2. Prove crypto-shredding produces verifiable erasure without mutating the immutable WAL.
3. Prove default-deny ABAC blocks cross-tenant access.
4. Prove Mode A single-writer multi-region replication and region-epoch fencing.
5. Prove basic SQS and AMQP gateway mapping.
6. Produce early evidence for security, DR, and queue compatibility.
7. Reduce Phase 4 integration risk before full compliance and Jepsen certification.

---

## 4. Relationship to KEI-ENG-400

This prototype executes the first practical stage of Phase 4 and maps directly to the work packages defined in KEI-ENG-400.

| KEI-ENG-400 Work Package | Prototype Coverage |
|---|---|
| WP-P4-A: Security Foundations | Core focus — KMS adapter, envelope encryption, DEK cache, destroyed-key registry, crypto-shredding. |
| WP-P4-B: Authorization & Audit | Core focus — ABAC PDP/PEP, principal mapping, tenant isolation, basic audit trail. |
| WP-P4-C: Multi-Region & DR | Core focus — Mode A async replication, region epoch fencing, basic backup/restore spike. |
| WP-P4-D: Advanced Queue Gateways | Included — SQS and AMQP basic translation gateways. |
| WP-P4-E: Compliance & Ops | Early audit trail and break-glass hooks. |
| WP-P4-F: Certification | Early chaos/split-brain tests for region fencing. |

The prototype intentionally compresses these work packages into a 90-day executable proof, deferring full SOC2/ISO evidence collection, Jepsen certification, and production KMS integration to the full Phase 4 build.

---

## 5. Prototype Scope

### 5.1 Must Have

The prototype MUST include:

1. KMS adapter abstraction (using local HashiCorp Vault or mock KMS).
2. Envelope encryption (Root → Tenant KEK → Stream/Batch DEK).
3. Encrypted WAL batches with Authenticated Additional Data (AAD).
4. Encrypted Parquet exports.
5. DEK cache with TTL and zeroization on eviction.
6. Destroyed-key registry.
7. Crypto-shredding orchestrator (key destruction + tombstone).
8. Erasure validation (prove destroyed data cannot be read).
9. ABAC policy engine (default-deny).
10. Principal mapping for Native SDK and Kafka gateway.
11. Tenant namespace enforcement.
12. Tamper-evident audit trail for security events.
13. Mode A Multi-Region async WAL tail replication.
14. Region epoch fencing (reject writes from demoted primary).
15. Basic SQS gateway (Send, Receive, Delete).
16. Basic AMQP gateway (Publish, Consume, Ack).
17. Basic backup and restore spike.
18. Prototype evidence report.

### 5.2 Should Have

The prototype SHOULD include if schedule permits:

1. Cross-region destroyed-key propagation.
2. Planned region failover workflow.
3. SQS FIFO `MessageGroupId` ordering.
4. AMQP basic NACK/Reject mapping.
5. Point-in-Time Recovery (PITR) spike.
6. 24-hour multi-region soak test with encryption enabled.

### 5.3 Will Not Have

The prototype WILL NOT include:

1. Production AWS KMS / GCP KMS integration (use Vault/Mock).
2. Active-active multi-writer same-stream replication (Mode B).
3. Full Jepsen certification suite.
4. Full SOC2/ISO27001 audit evidence collection.
5. Complex AMQP exchange topologies (Topic/Fanout).
6. SQS DelaySeconds / Message Timers.
7. Full production break-glass UI (CLI only).

---

## 6. Prototype Success Criteria

### 6.1 Functional Success Criteria

| ID | Criterion |
|---|---|
| SPIKE-P4-F-001 | WAL batches are encrypted at rest using envelope encryption. |
| SPIKE-P4-F-002 | KMS unavailability causes new writes to fail secure (no plaintext fallback). |
| SPIKE-P4-F-003 | Crypto-shredding destroys the DEK/KEK and writes a tombstone. |
| SPIKE-P4-F-004 | Querying/reading data protected by a destroyed key fails securely. |
| SPIKE-P4-F-005 | Backup restore respects the destroyed-key registry (does not resurrect erased data). |
| SPIKE-P4-F-006 | ABAC default-deny blocks unauthenticated and unauthorized requests. |
| SPIKE-P4-F-007 | Tenant A cannot read or write to Tenant B's streams. |
| SPIKE-P4-F-008 | Mode A replication asynchronously copies WAL tails to the replica region. |
| SPIKE-P4-F-009 | Region epoch fencing rejects writes from a demoted/fenced primary region. |
| SPIKE-P4-F-010 | SQS gateway maps Send/Receive/Delete to Keirox append/lease/ACK. |
| SPIKE-P4-F-011 | AMQP gateway maps Publish/Consume/Ack to Keirox append/lease/ACK. |

### 6.2 Security & Reliability Success Criteria

| ID | Criterion | Mandatory Target |
|---|---|---|
| SPIKE-P4-S-001 | Plaintext fallback attempts | Zero |
| SPIKE-P4-S-002 | Cross-tenant access attempts allowed | Zero |
| SPIKE-P4-S-003 | Split-brain writes accepted by replica | Zero |
| SPIKE-P4-S-004 | Erased data resurrected from backup | Zero |
| SPIKE-P4-S-005 | Secrets (DEKs, KEKs) leaked in logs/metrics | Zero |
| SPIKE-P4-S-006 | Audit trail captures all security/denial events | 100% |

### 6.3 Performance Success Criteria (With Encryption Enabled)

| ID | Criterion | Mandatory Target | Stretch Target |
|---|---|---:|---:|
| SPIKE-P4-P-001 | Encrypted append throughput | ≥40 MB/s (vs 50 MB/s unencrypted) | ≥50 MB/s |
| SPIKE-P4-P-002 | Encrypted append latency p99 | ≤3.0 ms (vs 2.0 ms unencrypted) | ≤2.5 ms |
| SPIKE-P4-P-003 | Replication lag (normal network) | ≤5 seconds | ≤2 seconds |
| SPIKE-P4-P-004 | Region failover fencing time | <1 second | <500 ms |
| SPIKE-P4-P-005 | SQS/AMQP gateway overhead | p99 ≤1.5 ms | p99 ≤1.0 ms |

---

## 7. Prototype Architecture Slice

### 7.1 Prototype Topology

```text
┌────────────────────────────────────────────────────────────────────────┐
│                    ENTERPRISE PROTOTYPE TOPOLOGY                        │
│                                                                        │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                    PRIMARY REGION (Active)                        │  │
│  │                                                                   │  │
│  │  [Kafka/SQS/AMQP/SDK Clients] ──► [Protocol Edge / ABAC PEP]    │  │
│  │                                           │                       │  │
│  │  [Local KMS / Vault] ◄──► [KMS Adapter / DEK Cache]             │  │
│  │                                           │                       │  │
│  │                                           ▼                       │  │
│  │  ┌────────────────────────────────────────────────────────────┐  │  │
│  │  │              KEIROX CORE CLUSTER (Encrypted)               │  │  │
│  │  │  WAL (AES-GCM) + State Plane + Iceberg (Encrypted Parquet) │  │  │
│  │  └──────────────────────────┬─────────────────────────────────┘  │  │
│  │                             │ Async WAL Tail Replication          │  │
│  └─────────────────────────────┼────────────────────────────────────┘  │
│                                │                                        │
│                                ▼                                        │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                    REPLICA REGION (Standby)                       │  │
│  │                                                                   │  │
│  │  ┌────────────────────────────────────────────────────────────┐  │  │
│  │  │              KEIROX REPLICA CLUSTER (Read-Only)            │  │  │
│  │  │  Replicated WAL + Replicated State + Region Epoch Fencing  │  │  │
│  │  └────────────────────────────────────────────────────────────┘  │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                                                        │
│  [Audit Sink] ◄─── Tamper-Evident Security & Admin Events             │
│  [Destroyed-Key Registry] ◄─── Replicated across regions              │
└────────────────────────────────────────────────────────────────────────┘
```

### 7.2 Simplifications

| Full Architecture Feature | Prototype Simplification |
|---|---|
| Production AWS/GCP KMS | Local HashiCorp Vault or Mock KMS |
| Full ABAC policy language | Simple RBAC/ABAC JSON policies or OPA subset |
| Multi-region state plane sync | Async WAL tail replication only; state reconstructed on failover |
| Full PITR | Basic timestamp-based WAL replay spike |
| Full SQS/AMQP parity | Certified subset only (Send/Receive/Publish/Consume) |

---

## 8. Work Packages

### 8.1 WP-0 — Enterprise Engineering Foundation

Objective: Prepare the repository, security environment, and multi-region test harness.

Deliverables:
1. Local HashiCorp Vault / Mock KMS deployment.
2. Multi-region Docker Compose / Kubernetes manifest (Primary + Replica).
3. Network partition tooling (tc/iptables) for split-brain testing.
4. Audit sink deployment (local OpenSearch or file-based append-only log).

Exit criteria:
- Multi-region cluster starts.
- KMS is accessible.
- Audit sink receives events.

---

### 8.2 WP-1 — Security Foundations & Encryption

Objective: Implement envelope encryption and fail-secure behavior.

Deliverables:
1. KMS adapter interface and Vault implementation.
2. Envelope encryption engine (KEK/DEK generation and wrapping).
3. DEK cache with TTL and zeroization.
4. WAL batch encryption integration (AES-256-GCM with AAD).
5. Parquet export encryption integration.
6. Fail-secure enforcement (block writes if KMS is down and cache is empty).

Exit criteria:
- WAL and Parquet files are encrypted at rest.
- KMS outage blocks new writes (no plaintext fallback).
- DEKs are never logged or persisted in plaintext.

Primary references: KEI-DES-036.

---

### 8.3 WP-2 — Crypto-Shredding & Destroyed-Key Registry

Objective: Implement logical erasure and prove backup safety.

Deliverables:
1. Destroyed-key registry (local Metadata Raft or shared DB).
2. Crypto-shredding orchestrator (erase ticket, KMS destroy command, tombstone).
3. Erasure validation tests (attempt to read erased data).
4. Backup restore hook (check destroyed-key registry before exposing restored data).

Exit criteria:
- Erased data is cryptographically unreadable.
- Backup restore does not resurrect erased data.
- Erasure events are audited.

Primary references: KEI-DES-036, KEI-OPS-040.

---

### 8.4 WP-3 — Authorization, Audit & Tenant Isolation

Objective: Implement default-deny ABAC and tenant boundaries.

Deliverables:
1. ABAC policy engine (PDP).
2. Protocol edge enforcement points (PEP) for SDK, Kafka, SQS, AMQP.
3. Principal mapper (map gateway identities to PEF tenants).
4. Tenant namespace enforcement (stream/group/table scoping).
5. Tamper-evident audit trail for auth denials and admin actions.

Exit criteria:
- Unauthenticated requests are rejected.
- Cross-tenant requests are rejected and audited.
- Audit trail captures all security events.

Primary references: KEI-ARC-025.

---

### 8.5 WP-4 — Multi-Region Mode A & Epoch Fencing

Objective: Implement async replication and split-brain prevention.

Deliverables:
1. Region registry and role assignment (Primary/Replica).
2. Async WAL tail replicator.
3. Region epoch generation and validation.
4. Split-brain fencing (reject writes from old epoch).
5. Basic planned failover script.

Exit criteria:
- WAL tails replicate to replica region.
- Network partition + primary heal results in old primary writes being rejected (fenced).
- Replication lag is measurable and bounded.

Primary references: KEI-ARC-026.

---

### 8.6 WP-5 — Advanced Queue Gateways (SQS & AMQP)

Objective: Implement basic SQS and AMQP translation gateways.

Deliverables:
1. SQS gateway: SendMessage, ReceiveMessage, DeleteMessage, ChangeMessageVisibility.
2. AMQP gateway: Queue Declare, Basic Publish, Basic Consume, Basic Ack/Nack.
3. Principal mapping for SQS (AWS SigV4 stub) and AMQP (PLAIN auth).
4. Gateway metrics and unsupported operation rejection.

Exit criteria:
- SQS client can send, receive, and delete messages.
- AMQP client can publish, consume, and ack messages.
- Unsupported operations return explicit errors.

Primary references: KEI-DES-035.

---

### 8.7 WP-6 — Evidence, Chaos & Go/No-Go

Objective: Produce the Phase 4 prototype evidence package.

Deliverables:
1. Security chaos tests (KMS kill, DEK cache eviction).
2. Multi-region chaos tests (network partition, split-brain heal).
3. Erasure validation report.
4. Performance benchmark with encryption enabled.
5. Go/No-Go recommendation.

Exit criteria:
- All mandatory success criteria met.
- Evidence package delivered.

---

## 9. 12-Week Execution Plan

| Week | Focus Area | Key Deliverables |
|---|---|---|
| 1–2 | Enterprise Mobilization | Multi-region env, KMS/Vault, Audit sink, CI updates |
| 3–4 | Encryption & KMS | KMS adapter, Envelope encryption, WAL/Parquet encryption, Fail-secure |
| 5 | Crypto-Shredding | Destroyed-key registry, Shredding orchestrator, Backup restore hook |
| 6–7 | ABAC & Audit | PDP/PEP, Principal mapping, Tenant isolation, Audit trail |
| 8–9 | Multi-Region Mode A | Async WAL replication, Region epoch fencing, Split-brain tests |
| 10 | SQS & AMQP Gateways | SQS basic ops, AMQP basic ops, Protocol mapping |
| 11 | Chaos & Benchmarks | KMS failure, Split-brain partition, Encrypted throughput benchmark |
| 12 | Evidence & Gate | Compile reports, ARB review, Go/No-Go decision |

---

## 10. Test Plan

### 10.1 Security Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| SEC-T-001 | KMS unavailable, DEK cache empty | New writes fail secure; no plaintext fallback |
| SEC-T-002 | AAD mismatch on WAL read | Decryption fails; corruption alerted |
| SEC-T-003 | Cross-tenant stream read | ABAC denies; audit event logged |
| SEC-T-004 | Crypto-shred tenant | DEK destroyed; subsequent reads fail securely |
| SEC-T-005 | Restore backup of shredded tenant | Restored data remains unreadable (destroyed-key check) |

### 10.2 Multi-Region & Chaos Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| MR-T-001 | Normal async replication | Replica lag ≤5s |
| MR-T-002 | Network partition (Primary isolated) | Primary continues; Replica stalls |
| MR-T-003 | Split-brain heal (Old Primary reconnects) | Old Primary writes rejected (Epoch Fencing) |
| MR-T-004 | Region failover (Planned) | Replica promoted; epoch incremented; writes resume |

### 10.3 Gateway Conformance Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| GW-T-001 | SQS SendMessage | Maps to Keirox append; returns MessageId |
| GW-T-002 | SQS ReceiveMessage | Maps to Keirox lease; returns ReceiptHandle |
| GW-T-003 | SQS DeleteMessage | Maps to Keirox ACK; validates ReceiptHandle |
| GW-T-004 | AMQP Basic Publish | Maps to Keirox append |
| GW-T-005 | AMQP BasicAck | Maps to Keirox ACK; validates delivery tag |
| GW-T-006 | SQS DelaySeconds (Unsupported) | Returns explicit InvalidParameterValue error |

---

## 11. Risks and Mitigations

| Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|
| Encryption overhead breaks latency SLA | High | Medium | Profile AES-GCM hardware acceleration (AES-NI); optimize batching |
| KMS latency bottlenecks DEK unwrap | High | Medium | Tune DEK cache TTL; ensure cache hit ratio >99% |
| Region epoch fencing race condition | Critical | Low | Formalize epoch transition in TLA+ (KEI-FORMAL-201); strict fencing tests |
| SQS/AMQP semantic mismatch | Medium | High | Strict compatibility-by-subset; explicit unsupported errors |
| Audit sink overwhelms storage | Medium | Medium | Sample high-volume telemetry; retain full security/auth events |

---

## 12. Prototype Go/No-Go Gate (Gate 4A)

### 12.1 Go Criteria

A GO decision requires:

1. Encryption at rest works; fail-secure proven.
2. Crypto-shredding erases data; backup restore respects erasure.
3. ABAC default-deny enforced; cross-tenant blocked.
4. Mode A replication works; split-brain writes fenced.
5. SQS/AMQP basic operations pass conformance.
6. Zero plaintext fallbacks.
7. Zero split-brain write acceptances.
8. Evidence package complete.

### 12.2 Gate Outcomes

| Outcome | Meaning |
|---|---|
| GO | Continue into full Phase 4 hardening and compliance evidence collection. |
| CONDITIONAL GO | Continue after specific security/DR fixes (max 4 weeks). |
| PIVOT | Core enterprise assumption needs adjustment (e.g., change KMS strategy). |
| STOP | Fundamental security or DR flaw discovered. |

---

## 13. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial Enterprise Hardening & Multi-Region Prototype Plan. Defines 90-day prototype scope, work packages, security/DR/queue gateway deliverables, test plan, and Gate 4A criteria. |