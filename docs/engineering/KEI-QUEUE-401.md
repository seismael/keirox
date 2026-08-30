# KEI-QUEUE-401 — SQS & AMQP Gateway Certification Plan

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-QUEUE-401 |
| Title | SQS & AMQP Gateway Certification Plan |
| Version | 1.0 |
| Level | Engineering Execution Plan |
| Status | Baseline — Ready for Execution |
| Phase | Phase 4 — Enterprise Hardening, Compliance & Multi-Region |
| Duration | Weeks 10–30 of Phase 4 |
| Owner | Ecosystem Engineering Lead / Gateway Lead |
| Governing Plan | KEI-ENG-400 — Phase 4 Engineering Execution Plan |
| Governing Architecture Documents | KEI-ARC-024, KEI-DES-031, KEI-DES-032, KEI-DES-035 |
| Predecessor | KEI-SPIKE-401 (Enterprise Prototype), KEI-COMPAT-301 (Kafka Certification) |
| Next Plan File | KEI-VAL-401 — Jepsen-Style Consistency Certification Plan |

---

## 2. Executive Summary

While the Kafka gateway (Phase 3) addresses high-throughput streaming and CDC migration, many enterprise workloads rely on discrete task-queue semantics provided by Amazon SQS or RabbitMQ (AMQP 0-9-1). 

This plan defines the certification program for the **SQS and AMQP Translation Gateways**. These gateways map legacy queue operations (Send, Receive, Ack, Nack) directly to the Keirox Consumption State Plane (Lease, ACK, NACK, DLQ). 

Following **ADR-070 (Compatibility by Published Subset)**, this plan strictly forbids claims of 100% protocol parity. Instead, it defines a rigorous conformance testing framework to guarantee that the *certified subset* of SQS and AMQP operations works flawlessly, while explicitly rejecting unsupported features (like SQS DelaySeconds or AMQP Topic exchanges) with protocol-native errors, preventing silent behavioral approximation.

---

## 3. Purpose and Scope

### 3.1 Purpose

The purpose of this plan is to:

1. Define the exact SQS and AMQP API surfaces that will be certified.
2. Define the semantic mapping between legacy queue concepts and Keirox state plane primitives.
3. Establish the conformance test suite for official AWS SDKs and AMQP client libraries.
4. Govern the negative testing of unsupported features.
5. Produce the Phase 4 queue gateway evidence package.

### 3.2 Scope

**In scope:**
- SQS Standard and FIFO queue translation.
- AMQP 0-9-1 Direct and Default exchange translation.
- Mapping of SQS VisibilityTimeout / AMQP delivery-tag to Keirox Lease TTL and Lease Tokens.
- Conformance testing against AWS SDKs (Java, Python, Go) and AMQP clients (RabbitMQ Java, Pika, amqp091-go).
- Negative testing for unsupported queue features.

**Out of scope:**
- SQS DelaySeconds / Message Timers (Unsupported in v1).
- AMQP Topic, Fanout, and Headers exchanges (Unsupported in v1).
- AMQP Transactions (`tx.*`) and Publisher Confirms (`confirm.*`) (Unsupported/Deferred).
- Kafka wire protocol (Owned by KEI-COMPAT-301).

---

## 4. Gateway Certification Principles

| ID | Principle | Requirement |
|---|---|---|
| Q-1 | No Silent Approximation | Unsupported features MUST return explicit protocol-native errors, NOT silently drop the feature or approximate it. |
| Q-2 | State Plane is Source of Truth | Gateways MUST NOT maintain independent queue state; all state MUST map to Keirox leases and bitmaps. |
| Q-3 | Idempotence is Explicit | Deduplication MUST rely on explicit client-provided IDs (e.g., `MessageDeduplicationId`), not implicit gateway guessing. |
| Q-4 | Ordering is Strict | FIFO and AMQP ordering guarantees MUST be strictly enforced via Keirox `entity_key` routing. |

---

## 5. SQS Certification Matrix

### 5.1 Supported SQS Operations (Certified Subset)

| SQS API | Keirox Mapping | Certification Status |
|---|---|---|
| `SendMessage` | `Append` (Queue Mode) | Certified |
| `SendMessageBatch` | `AppendBatch` | Certified |
| `ReceiveMessage` | `LeaseNext` | Certified |
| `DeleteMessage` | `Ack` (using `ReceiptHandle` as `lease_token`) | Certified |
| `DeleteMessageBatch` | Batch `Ack` | Certified |
| `ChangeMessageVisibility` | `RenewLease` or `Nack` (if timeout=0) | Certified |
| `GetQueueUrl` | Stream/Group Name Resolution | Certified |
| `GetQueueAttributes` | Stream/Group Telemetry (Approximate counts) | Certified (Limited) |
| `PurgeQueue` | Admin Purge (Requires elevated ABAC) | Certified |

### 5.2 SQS FIFO Semantics Mapping

| SQS FIFO Concept | Keirox Mapping | Validation Requirement |
|---|---|---|
| `MessageGroupId` | `entity_key` | MUST guarantee strict per-group ordering. |
| `MessageDeduplicationId` | Idempotency Key | MUST prevent duplicate appends within the dedup window. |
| Content-Based Dedup | Payload Hash | MUST hash payload to generate idempotency key if explicit ID is missing. |

### 5.3 Explicitly Unsupported SQS Features (Negative Test Targets)

| Feature | Expected Gateway Behavior |
|---|---|
| `DelaySeconds` (Message or Queue level) | Return `InvalidParameterValue` or `UnsupportedOperation`. |
| Dead-Letter Queue Redrive Policy config | Return `InvalidAttributeName`. (Keirox uses internal virtual DLQ policies). |
| Server-Side Encryption (SSE-KMS) config | Return `InvalidAttributeName`. (Keirox handles encryption at the storage layer). |
| Message Timers / Scheduled Delivery | Return `InvalidParameterValue`. |

---

## 6. AMQP Certification Matrix

### 6.1 Supported AMQP 0-9-1 Methods (Certified Subset)

| AMQP Method | Keirox Mapping | Certification Status |
|---|---|---|
| `connection.*` / `channel.*` | Session Management | Certified |
| `queue.declare` | Create/Resolve Consumer Group | Certified |
| `queue.bind` (Direct Exchange only) | Map Routing Key to `entity_key` | Certified (Limited) |
| `basic.publish` | `Append` (Queue Mode) | Certified |
| `basic.consume` / `basic.get` | Long-poll `LeaseNext` | Certified |
| `basic.ack` | `Ack` (using `delivery-tag` as `lease_token`) | Certified |
| `basic.nack` / `basic.reject` | `Nack` (requeue=true) or DLQ Eviction (requeue=false) | Certified |
| `basic.qos` | Lease quota / prefetch limit | Certified (Limited) |

### 6.2 Explicitly Unsupported AMQP Features (Negative Test Targets)

| Feature | Expected Gateway Behavior |
|---|---|
| `exchange.declare` (Topic/Fanout/Headers) | Return `540 NOT_IMPLEMENTED`. |
| `exchange.bind` / `exchange.unbind` | Return `540 NOT_IMPLEMENTED`. |
| `tx.select` / `tx.commit` / `tx.rollback` | Return `540 NOT_IMPLEMENTED` (AMQP Transactions unsupported). |
| `confirm.select` | Return `540 NOT_IMPLEMENTED` (Publisher confirms deferred). |
| `basic.recover` | Return `540 NOT_IMPLEMENTED` or map to specific lease retry logic. |
| Message `priority` | Silently ignored or rejected (Keirox does not support priority queues in v1). |
| Message `expiration` (TTL) | Silently ignored or rejected (Keirox uses retention policies, not per-message TTL). |

---

## 7. Semantic Mapping & Validation Tests

### 7.1 Lease and Visibility Mapping

The most critical certification is ensuring that legacy visibility timeouts map safely to Keirox leases without causing double-processing or lost messages.

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| SEM-T-001 | SQS `VisibilityTimeout` expires before `DeleteMessage` | Message returns to READY state; `ApproximateReceiveCount` increments. |
| SEM-T-002 | AMQP client disconnects before `basic.ack` | Lease expires; message returns to READY state. |
| SEM-T-003 | SQS `ChangeMessageVisibility(0)` | Maps to early lease release (NACK); message immediately available. |
| SEM-T-004 | AMQP `basic.nack(requeue=false)` | Maps to Keirox NACK with DLQ eviction flag (if retry limit exceeded). |
| SEM-T-005 | Stale `ReceiptHandle` / `delivery-tag` used | Gateway returns `ReceiptHandleIsInvalid` (SQS) or channel error (AMQP). |

### 7.2 FIFO and Ordering Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| ORD-T-001 | SQS FIFO: Concurrent sends to same `MessageGroupId` | Receives strictly in send order. |
| ORD-T-002 | SQS FIFO: Concurrent sends to different `MessageGroupId` | Receives concurrently (no cross-group blocking). |
| ORD-T-003 | AMQP: Concurrent publishes to same routing key (Direct) | Consumes strictly in publish order. |

---

## 8. Conformance Test Suite Architecture

### 8.1 Target Client Libraries

| Protocol | Client Library | Priority |
|---|---|---|
| SQS | AWS SDK for Java (v2) | P0 |
| SQS | AWS SDK for Python (boto3) | P0 |
| SQS | AWS SDK for Go (v2) | P1 |
| AMQP | RabbitMQ Java Client | P0 |
| AMQP | Pika (Python) | P1 |
| AMQP | amqp091-go | P1 |

### 8.2 Test Categories

1. **Happy Path:** Standard send, receive, ack/nack flows.
2. **Concurrency & Ordering:** High-throughput FIFO and routing key validation.
3. **Negative Path:** Attempting to use DelaySeconds, Topic Exchanges, or AMQP Transactions.
4. **Timeout & Churn:** High rates of lease expirations and visibility changes.
5. **Soak Test:** 72-hour continuous queue churn to detect memory leaks in gateway state mapping.

---

## 9. Certification Gates & Deliverables

### 9.1 Deliverables

| Deliverable | Description | Target Week |
|---|---|---:|
| D-Q-001 | SQS Gateway Conformance Suite | Week 14 |
| D-Q-002 | AMQP Gateway Conformance Suite | Week 16 |
| D-Q-003 | FIFO & Ordering Validation Suite | Week 18 |
| D-Q-004 | Negative Path & Unsupported Suite | Week 20 |
| D-Q-005 | 72-Hour Queue Churn Soak Test | Week 24 |
| D-Q-006 | Public SQS/AMQP Compatibility Matrices | Week 26 |
| D-Q-007 | Final Queue Gateway Evidence Package | Week 30 |

### 9.2 Certification Gates

| Gate | Requirement |
|---|---|
| **Gate Q1: Alpha Conformance** | P0 SDKs (Java/Python) pass Happy Path and basic Negative tests. |
| **Gate Q2: Ordering Certified** | FIFO and Direct Exchange ordering tests pass under high concurrency. |
| **Gate Q3: Phase 4 Exit (Gate 4C)** | All P0/P1 SDKs pass; 72-hour soak passes; Unsupported operations strictly rejected; Matrices published. |

---

## 10. Risks and Mitigations

| Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|
| **Semantic Mismatch:** Clients expect exact SQS/AMQP error codes for edge cases. | High | High | Extensive protocol trace analysis; map PEF errors to exact legacy error codes. |
| **Scope Creep:** Customers demand AMQP Topic exchanges or SQS Delays. | High | High | Strict adherence to ADR-070; publish explicit "Unsupported" documentation. |
| **State Plane Overhead:** Translating every AMQP frame to PEF leases causes CPU bottleneck. | Medium | Medium | Optimize gateway parsing; batch lease acquisitions where protocol allows. |
| **FIFO Throughput Limits:** Keirox `entity_key` hashing creates hot partitions. | Medium | Medium | Document FIFO throughput limits; advise customers on group cardinality. |
| **AMQP 0-9-1 Complexity:** Client libraries use obscure frame interleaving. | Medium | Medium | Use mature AMQP parsing libraries; restrict certified subset to basic methods. |

---

## 11. Evidence Package

The Queue Gateway evidence package MUST include:

1. SQS Supported/Unsupported API Matrix.
2. AMQP Supported/Unsupported Method Matrix.
3. AWS SDK (Java/Python/Go) Conformance Report.
4. AMQP Client (RabbitMQ/Pika/Go) Conformance Report.
5. FIFO and Ordering Validation Report.
6. Negative Test Report (proving no silent approximation).
7. 72-Hour Queue Churn Soak Report.
8. Gateway Translation Latency Report.
9. Public Compatibility Documentation Draft.

---

## 12. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial SQS & AMQP Gateway Certification Plan. Defines compatibility-by-subset matrices, semantic mapping to Keirox state plane, FIFO/ordering validation, negative testing for unsupported features, and conformance test suites for AWS and AMQP client libraries. |