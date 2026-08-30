# KEI-COMPAT-301 — Protocol Compatibility Certification Plan

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-COMPAT-301 |
| Title | Protocol Compatibility Certification Plan |
| Version | 1.0 |
| Level | Engineering Execution Plan |
| Status | Baseline — Ready for Execution |
| Phase | Phase 3 — Ecosystem Compatibility Gateways & Lakehouse |
| Duration | Months 19–27 (Continuous throughout Phase 3) |
| Owner | Ecosystem Engineering Lead / QA Lead |
| Governing Plan | KEI-ENG-300 — Phase 3 Engineering Execution Plan |
| Governing Architecture Documents | KEI-ARC-024 (Protocol Gateways), KEI-DES-032 (API & Protocol), KEI-DES-035 (Compatibility Matrices) |
| Predecessor | KEI-SPIKE-301 (Ecosystem Gateway Prototype Plan) |
| Next Plan File | KEI-LAKE-301 — Lakehouse Iceberg Certification Plan |

---

## 2. Executive Summary

The commercial viability of the Keirox Polymorphic Event Fabric relies on the "Trojan Horse" adoption strategy: enabling enterprises to migrate existing Kafka producers and CDC pipelines without code changes. However, attempting to achieve 100% wire-protocol parity with Apache Kafka, Amazon SQS, or RabbitMQ is an architectural trap that leads to infinite compatibility debt and silent behavioral divergence.

This document defines the **Protocol Compatibility Certification Plan** for Phase 3. It operationalizes **ADR-070 (Compatibility by Published Subset)** by establishing a rigorous, repeatable conformance testing framework. It ensures that the Keirox gateways guarantee *exactly* what is published in the compatibility matrices, explicitly reject what is unsupported, and never silently approximate legacy behavior.

---

## 3. Purpose and Scope

### 3.1 Purpose

The purpose of this plan is to:

1. Define the exact client libraries and versions that will be certified against the Keirox Kafka Gateway.
2. Establish the conformance test suite architecture (happy path, negative path, and soak tests).
3. Define the certification gates required before a gateway version can be released.
4. Govern the public publication of compatibility matrices.
5. Establish the framework for future SQS and AMQP certification (Phase 4).

### 3.2 Scope

**In scope:**

- Kafka wire protocol conformance testing (Produce, Fetch, Metadata, Offsets, Idempotence).
- Kafka client library matrix (librdkafka, Java, Go, Python).
- Negative testing for unsupported Kafka features (Transactions, Share Groups, Admin APIs).
- Gateway soak and stability testing.
- Compatibility matrix versioning and publication governance.
- SQS/AMQP conformance framework design (for Phase 4 execution).

**Out of scope:**

- Kafka transaction certification (Explicitly excluded by ADR-070).
- Full SQS/AMQP production certification (Deferred to Phase 4).
- Native Arrow Flight SDK testing (Owned by KEI-SDK-301).
- Iceberg/Lakehouse query engine compatibility (Owned by KEI-LAKE-301).

---

## 4. Compatibility Philosophy: The "Subset" Rule

### 4.1 The Golden Rule of Compatibility

> **Keirox MUST NEVER claim 100% protocol parity. Keirox guarantees ONLY the published compatibility subset.**

### 4.2 Behavioral Mandates

1. **No Silent Approximations:** If a client requests an unsupported feature (e.g., Kafka Transactions), the gateway MUST return a protocol-native error (e.g., `TRANSACTIONAL_ID_ERROR` or `UNSUPPORTED_VERSION`), NOT silently downgrade the operation to non-transactional.
2. **No Hidden State:** Virtual partitions MUST NOT leak into client-facing metadata unless explicitly mapped.
3. **Explicit Documentation:** Every supported API version, every supported configuration parameter, and every unsupported feature MUST be documented in the public Compatibility Matrix.
4. **Version Negotiation:** The gateway MUST correctly implement `ApiVersions` so clients can automatically negotiate the highest mutually supported API version.

---

## 5. Kafka Compatibility Certification Matrix

### 5.1 Target Client Libraries

The Phase 3 certification suite MUST validate against the following ecosystem clients:

| Language | Client Library | Priority | Rationale |
|---|---|---|---|
| C/C++ | `librdkafka` | P0 | Foundation for Python, Go, and .NET wrappers; heavily used in high-throughput C++ apps. |
| Java | `kafka-clients` (Official) | P0 | Dominant enterprise ecosystem; Debezium and Kafka Connect rely on it. |
| Go | `segmentio/kafka-go` | P1 | Popular in modern cloud-native microservices. |
| Go | `IBM/sarama` | P1 | Legacy but heavily used Go client. |
| Python | `confluent-kafka-python` | P1 | Wraps librdkafka; dominant in data engineering. |
| Python | `aiokafka` / `kafka-python` | P2 | Pure Python async clients. |

### 5.2 Certified API Surface (Phase 3)

| API Key | API Name | Certified Versions | Notes |
|---:|---|---|---|
| 0 | Produce | v3–v9 | Idempotent produce certified. Batching certified. |
| 1 | Fetch | v4–v13 | Stream-mode fetch. |
| 2 | ListOffsets | v2–v7 | Earliest/Latest certified. Timestamp lookup S2 (limited). |
| 3 | Metadata | v1–v12 | Virtual partition topology. |
| 8 | OffsetCommit | v3–v9 | Stream-mode offset commits. |
| 9 | OffsetFetch | v3–v9 | Stream-mode offset fetch. |
| 10 | FindCoordinator | v1–v4 | Gateway-managed coordinator. |
| 11 | JoinGroup | v2–v9 | Stream consumer groups. |
| 12 | Heartbeat | v1–v4 | Session liveness. |
| 13 | LeaveGroup | v1–v4 | Group departure. |
| 14 | SyncGroup | v1–v4 | Assignment distribution. |
| 18 | ApiVersions | v0–v3 | Version discovery. |
| 22 | InitProducerId | v1–v4 | Idempotence only. |

### 5.3 Explicitly Unsupported APIs (Negative Test Targets)

| API Key | API Name | Expected Gateway Behavior |
|---:|---|---|
| 24 | AddPartitionsToTxn | Return `TRANSACTIONAL_ID_ERROR` or `UNSUPPORTED_VERSION`. |
| 25 | AddOffsetsToTxn | Return `TRANSACTIONAL_ID_ERROR` or `UNSUPPORTED_VERSION`. |
| 26 | EndTxn | Return `TRANSACTIONAL_ID_ERROR` or `UNSUPPORTED_VERSION`. |
| 37 | CreatePartitions | Return `INVALID_REQUEST` (Virtual streams do not support physical partition creation). |
| 33 | AlterConfigs | Return `INVALID_REQUEST`. |

---

## 6. Conformance Test Suite Architecture

### 6.1 Test Categories

| Category | Purpose | Pass Criteria |
|---|---|---|
| **Happy Path** | Validate certified APIs work as expected. | 100% of certified operations succeed. |
| **Idempotence** | Validate duplicate produce deduplication. | Duplicate sequence numbers return original offset. |
| **Consumer Groups** | Validate stream-mode rebalance and offset commits. | Rebalances complete; offsets persist across restarts. |
| **Negative Path** | Validate unsupported APIs fail safely. | 100% of unsupported APIs return explicit errors; zero state corruption. |
| **Version Negotiation** | Validate `ApiVersions` and fallback behavior. | Clients successfully negotiate and connect. |
| **Soak / Stability** | Validate gateway under sustained load. | 72-hour soak with zero memory leaks or OOMs. |

### 6.2 Test Harness Design

The conformance harness (`keirox-compat-tests`) will be a standalone Rust/Python test suite that:

1. Spins up a local 3-node Keirox cluster via Docker Compose.
2. Spawns target client containers (Java, Go, Python, C++).
3. Executes the test matrix against the Keirox Kafka Gateway.
4. Captures client-side logs, gateway metrics, and network traces.
5. Generates a machine-readable Conformance Report.

### 6.3 Debezium and Kafka Connect Certification

Because CDC (Change Data Capture) is a primary migration vector, the suite MUST include specific integration tests for:

- **Debezium MySQL/Postgres Connectors:** Validate that Debezium can produce schema-registry-compatible Avro/JSON payloads to Keirox.
- **Kafka Connect Standalone/Distributed:** Validate that standard sink connectors can fetch from Keirox.

---

## 7. SQS & AMQP Certification Framework (Phase 4 Prep)

While full SQS and AMQP production gateways are Phase 4 deliverables, Phase 3 MUST establish the certification framework.

### 7.1 SQS Certification Targets

| AWS SDK | Priority | Certified Operations (Phase 4 Target) |
|---|---|---|
| Java (v2) | P0 | Send, Receive, Delete, ChangeVisibility |
| Python (boto3) | P0 | Send, Receive, Delete, ChangeVisibility |
| Go (aws-sdk-go-v2) | P1 | Send, Receive, Delete, ChangeVisibility |

### 7.2 AMQP Certification Targets

| Client | Priority | Certified Operations (Phase 4 Target) |
|---|---|---|
| RabbitMQ Java Client | P0 | Declare, Publish, Consume, Ack/Nack (Direct Exchange) |
| Pika (Python) | P1 | Declare, Publish, Consume, Ack/Nack (Direct Exchange) |
| amqp091-go | P1 | Declare, Publish, Consume, Ack/Nack (Direct Exchange) |

---

## 8. Certification Gates & Release Rules

### 8.1 Gate C1: Internal Matrix Sign-Off

Before a gateway feature branch can be merged to `develop`:

1. All Happy Path tests for the affected API MUST pass.
2. All Negative Path tests MUST pass.
3. No new unsupported behavior may be silently approximated.
4. The Compatibility Matrix YAML/JSON MUST be updated in the PR.

### 8.2 Gate C2: Public Documentation Review

Before a release candidate is cut:

1. The Compatibility Matrix MUST be reviewed by Product and Developer Experience teams.
2. Public documentation MUST explicitly list unsupported features.
3. Migration guides MUST be updated with known client-specific quirks.

### 8.3 Gate C3: Phase 3 Exit Certification

To pass the Phase 3 Exit Gate (KEI-ENG-300 Gate 3C):

1. 100% of P0 and P1 Kafka clients MUST pass the Happy Path suite.
2. 100% of Negative Path tests MUST pass.
3. Debezium CDC integration MUST pass.
4. 72-hour Gateway Soak Test MUST pass with zero memory leaks.
5. The official Keirox Compatibility Matrix MUST be published.

---

## 9. Deliverables & Milestones

| Deliverable | Description | Target Week |
|---|---|---|
| D-C-001 | Compatibility Test Harness (`keirox-compat-tests`) | Week 16 |
| D-C-002 | Kafka Happy Path & Idempotence Suite | Week 18 |
| D-C-003 | Kafka Negative Path & Unsupported Suite | Week 20 |
| D-C-004 | Debezium / Kafka Connect Integration Suite | Week 22 |
| D-C-005 | 72-Hour Gateway Soak Test Execution | Week 26 |
| D-C-006 | SQS/AMQP Framework Design Document | Week 24 |
| D-C-007 | Public Compatibility Matrix Publication | Week 30 |
| D-C-008 | Phase 3 Certification Evidence Report | Week 32 |

---

## 10. Risks and Mitigations

| Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|
| **Client Library Updates** break existing compatibility | High | High | Pin client versions in CI; run nightly compatibility tests against client `main` branches. |
| **Scope Creep** into full Kafka parity | High | High | Strict adherence to ADR-070; ARB approval required for any matrix expansion. |
| **Silent Behavioral Drift** (gateway approximates unsupported feature) | Critical | Medium | Mandatory negative testing; code review checklist for gateway handlers. |
| **Debezium Schema Registry** incompatibility | High | Medium | Implement basic schema registry stub or integrate with Apicurio/Confluent SR in test harness. |
| **Virtual Partition Leakage** confuses clients | Medium | Medium | Strict mapping layer; validate client partition assignment logic in tests. |

---

## 11. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial Protocol Compatibility Certification Plan. Defines compatibility-by-subset philosophy, Kafka client matrix, conformance test suite architecture, Debezium integration, certification gates, and SQS/AMQP framework prep. |