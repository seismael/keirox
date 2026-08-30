# KEI-ARC-024 — Protocol Gateways & SDK Architecture

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-ARC-024 |
| Title | Protocol Gateways & SDK Architecture |
| Version | 1.0 |
| Level | **L2 — Subsystem Architecture** |
| Pillars Covered | Cross-cutting Protocol Plane |
| Status | Approved for Engineering |
| Classification | Internal / Engineering Confidential |
| Owner | Principal Engineer (Ecosystem & Integration) |
| Required Reviewers | Chief Architect, Principal Engineer (Distributed Systems), Security Lead |
| Depends On | KEI-ARC-010 (Conceptual Architecture), KEI-ARC-011 (NFRs), KEI-ARC-012 (ADRs), KEI-ARC-020 (Storage Engine), KEI-ARC-021 (State Plane) |
| Feeds | KEI-DES-032 (Lease/ACK Protocol), KEI-DES-035 (Gateway Compatibility Matrices) |
| Keywords | MUST, MUST NOT, SHOULD, SHOULD NOT, MAY per RFC 2119 |

---

## 2. Purpose, Scope, and Audience

### 2.1 Purpose

This document specifies the **Protocol Plane** — the set of gateways and SDKs that expose the Polymorphic Event Fabric to external producers, consumers, and existing ecosystem tooling.

It elaborates the **Dual Interface strategy** (ADR-071):

- A **Kafka wire-protocol ingest gateway** for zero-code-change migration of existing producers and CDC connectors.
- A **native Arrow Flight / gRPC SDK** for high-performance streaming and out-of-order task leasing.
- **SQS and AMQP translation gateways** for work-queue workloads.

It enforces the **Compatibility-by-Subset principle** (ADR-070): each gateway publishes a validated compatibility matrix rather than claiming full protocol parity.

### 2.2 Scope

**In scope:** gateway and SDK component architecture, protocol translation semantics, compatibility-subset definition, identity mapping to ABAC, and gateway-specific performance and failure behavior.

**Out of scope:**
- Physical storage and durability — owned by KEI-ARC-020.
- Consumption state machine internals — owned by KEI-ARC-021.
- Exact wire-level byte encoding of lease/ACK RPCs — owned by KEI-DES-032.
- Full per-API-version compatibility matrices — owned by KEI-DES-035.

### 2.3 Position in the Architecture

```
   Kafka producers / Debezium / FluentBit        Arrow Flight / gRPC apps
        │                                              │
        ▼                                              ▼
┌───────────────────────────┐              ┌──────────────────────────────┐
│ KAFKA WIRE-PROTOCOL       │              │ NATIVE ARROW FLIGHT / gRPC   │
│ INGEST GATEWAY            │              │ SDK (Rust/Go/Python/Java/TS) │
└───────────┬───────────────┘              └──────────────┬───────────────┘
            │                                             │
   SQS clients / AMQP clients                             │
        │                                                 │
        ▼                                                 │
┌───────────────────────────┐                             │
│ SQS / AMQP TRANSLATION    │                             │
│ GATEWAY                   │                             │
└───────────┬───────────────┘                             │
            │                                             │
            └──────────────────────┬──────────────────────┘
                                   │ mapped ABAC principal
                                   ▼
                    ┌──────────────────────────────────┐
                    │  PROTOCOL PLANE (this doc)       │
                    │  Identity Mapping + Translation  │
                    └───────────────┬──────────────────┘
                                    │ append / read / lease / ack
                                    ▼
                    ┌──────────────────────────────────┐
                    │ STORAGE ENGINE (KEI-ARC-020)     │
                    │ STATE PLANE    (KEI-ARC-021)     │
                    └──────────────────────────────────┘
                                    ▲
                                    │ ABAC decision
                    ┌───────────────┴──────────────────┐
                    │ SECURITY (KEI-ARC-025)           │
                    └──────────────────────────────────┘
```

**Normative boundary:** Gateways are protocol translators. They MUST NOT implement consumption semantics themselves; all state transitions delegate to the State Plane, and all durability delegates to the Storage Engine.

---

## 3. Subsystem Responsibilities and Non-Responsibilities

### 3.1 Responsibilities

| ID | Responsibility |
|---|---|
| R1 | Translate Kafka wire protocol produce/fetch/metadata into PEF operations. |
| R2 | Provide native Arrow Flight / gRPC streaming and task-leasing APIs. |
| R3 | Translate SQS send/receive/delete/visibility into PEF lease/ACK. |
| R4 | Translate AMQP publish/consume/ack into PEF lease/ACK. |
| R5 | Map gateway identities to PEF ABAC principals. |
| R6 | Publish and enforce compatibility-subset matrices. |
| R7 | Apply protocol-level backpressure and admission control. |

### 3.2 Non-Responsibilities

| ID | Non-Responsibility | Owned By |
|---|---|---|
| N1 | Record durability | KEI-ARC-020 |
| N2 | Consumption state transitions | KEI-ARC-021 |
| N3 | Consensus and replication | KEI-ARC-022 |
| N4 | Authorization policy evaluation | KEI-ARC-025 |
| N5 | Lakehouse query serving | KEI-ARC-023 |

---

## 4. Internal Component Decomposition

| Component | Responsibility |
|---|---|
| **G1. Kafka Ingest Gateway** | Parses Kafka RPC; translates Produce/Fetch/Metadata/ListOffsets/OffsetCommit. |
| **G2. Arrow Flight / gRPC Server** | Serves native streaming and task-leasing RPCs. |
| **G3. SQS Translation Gateway** | Maps SQS REST commands to lease/ACK. |
| **G4. AMQP Translation Gateway** | Maps AMQP 0-9-1 framing to lease/ACK. |
| **G5. Identity Mapper** | Maps gateway principals to PEF ABAC principals. |
| **G6. Compatibility Matrix Registry** | Stores and enforces supported API subsets. |
| **G7. Protocol Admission Controller** | Applies tenant quotas and backpressure at the protocol edge. |
| **G8. SDK Library Set** | Native client libraries for Arrow Flight and task leasing. |

---

## 5. Compatibility-by-Subset Principle (ADR-070)

### 5.1 The Rule

> Each gateway publishes a **Compatibility Matrix** of validated operations. Unsupported operations are explicitly listed. The gateway guarantees the published subset, never full parity.

**Normative rules:**
- No gateway documentation MAY claim "100% protocol parity."
- Every gateway MUST publish a compatibility matrix listing supported operations, versions, and known limitations.
- Unsupported operations MUST return a clear, documented error rather than silently misbehaving.

### 5.2 Compatibility Matrix Registry (G6)

Each gateway registers its matrix in a central registry so that:

- Clients can discover supported operations at connection time.
- The control plane can enforce version gates.
- The compatibility test suite (KEI-OPS-041) validates the published subset.

**Delegation:** The exact per-API-version matrices are specified in KEI-DES-035.

---

## 6. Kafka Wire-Protocol Ingest Gateway

### 6.1 Purpose

The Kafka Ingest Gateway enables zero-code-change migration for existing Kafka producers, CDC connectors (Debezium), and log shippers (FluentBit) by speaking the Kafka binary RPC protocol.

### 6.2 Supported Operations (Subset)

| Kafka API | Supported Versions | Purpose |
|---|---|---|
| Produce | v0–v12 | Append records to a PEF virtual stream. |
| Fetch | v0–v15 | Sequential stream replay. |
| Metadata | v0–v12 | Topic/stream discovery. |
| ListOffsets | v0–v7 | Earliest/latest offset resolution. |
| OffsetCommit | subset | Commit stream consumer offsets. |
| OffsetFetch | subset | Fetch committed offsets. |

**Delegation:** Exact per-version behavior and limitations are in KEI-DES-035.

### 6.3 Topic-to-Stream Mapping

A Kafka `topic` maps to a PEF virtual stream. A Kafka `partition` is a compatibility abstraction:

```
Kafka (topic, partition) ──► PEF stream_id
Kafka record key        ──► PEF entity_key
```

**Normative rules:**
- The gateway MUST map a Kafka partition to a PEF stream such that ordering per partition is preserved.
- The gateway MUST NOT require the client to size partitions; PEF streams are virtual.
- Kafka consumer group offsets map to PEF stream-mode offset commits.

### 6.4 CDC Connector Support

Debezium and Kafka Connect source connectors MUST be able to write through the Produce path without modification, subject to the published compatibility matrix.

---

## 7. Native Arrow Flight & gRPC SDKs

### 7.1 Purpose

The native SDKs provide the high-performance path for streaming and out-of-order task leasing, bypassing Kafka protocol overhead.

### 7.2 SDK Operations

| Operation | Semantics |
|---|---|
| `Append(batch)` | Append an Arrow RecordBatch to a stream. |
| `StreamFetch(stream, offset, max)` | Sequential replay. |
| `LeaseNext(group, stream, max, τ)` | Acquire task leases. |
| `Ack(group, stream, offset, ack_mode)` | Acknowledge a lease. |
| `Nack(group, stream, offset)` | Negative-acknowledge; requeue. |
| `RenewLease(group, stream, offset, τ)` | Extend a lease. |
| `PushdownQuery(predicate, range)` | SIMD-filtered Arrow read. |

### 7.3 SDK Languages

Native client libraries: **Rust, Go, Python, Java, TypeScript.**

### 7.4 Performance Target

The Arrow Flight client SHOULD achieve ≤1/3 the CPU consumption of an equivalent JVM Kafka consumer for vectorized workloads (PERF-032, Class B). This derives from zero-copy Arrow transfer and SIMD predicate pushdown eliminating consumer SerDe.

**Normative rule:** The SDK MUST support both `ACK_FAST` and `ACK_DURABLE` acknowledgment modes, passing the selection through to the State Plane.

---

## 8. SQS Translation Gateway

### 8.1 Purpose

The SQS Translation Gateway exposes PEF task queues through the Amazon SQS REST command surface, enabling migration of SQS-based workloads.

### 8.2 Operation Mapping

| SQS Command | PEF Operation |
|---|---|
| `SendMessage` | Append record to stream (queue mode). |
| `ReceiveMessage` | `LeaseNext` with visibility timeout τ. |
| `DeleteMessage` | `Ack` (lease terminal). |
| `ChangeMessageVisibility` | `RenewLease` or requeue. |
| `PurgeQueue` | Stream-level operation (subject to authorization). |
| Redrive | DLQ redrive via State Plane. |

### 8.3 FIFO Support

SQS `MessageGroupId` maps to PEF `entity_key`, preserving FIFO ordering per group.

**Normative rule:** The SQS gateway MUST map visibility timeout semantics to PEF lease TTL, and MUST return SQS-compatible error codes for unsupported operations.

---

## 9. AMQP Translation Gateway

### 9.1 Purpose

The AMQP Translation Gateway exposes PEF task queues through an AMQP 0-9-1-compatible surface for workloads migrating from RabbitMQ.

### 9.2 Supported Subset

| AMQP Feature | Support |
|---|---|
| Direct / default exchange | Supported. |
| Queue declare | Supported (maps to PEF stream + group). |
| Basic publish | Supported. |
| Basic consume | Supported. |
| Basic ack / nack / reject | Supported (maps to PEF ACK/NACK). |
| Dead-letter routing | Supported (maps to virtual DLQ). |

### 9.3 Deferred Features

The following are explicitly out of scope for the initial subset and MUST be listed as unsupported:

- Complex multi-hop exchange topologies (topic/fanout/headers with routing graphs).
- AMQP transactions.
- Priority queues beyond basic policy.

**Normative rule:** Complex AMQP exchange routing is a non-goal for v1 (see KEI-ARC-001 non-goals). The gateway MUST reject unsupported routing with a clear error.

---

## 10. Identity Mapping (G5)

Gateway identities are mapped to PEF ABAC principals so that authorization is consistent across all protocols.

```
Kafka principal (SASL/SCRAM)   ──┐
OAuth2 / OIDC token            ──┼──►  PEF ABAC Principal ──► KEI-ARC-025 policy evaluation
SQS identity (IAM-style)       ──┤
AMQP user                      ──┘
```

**Normative rules:**
- Every gateway request MUST resolve to a PEF ABAC principal before any storage or state operation.
- The Identity Mapper MUST be authoritative; gateways MUST NOT perform their own authorization.
- Gateway credentials MUST be transported over TLS/mTLS.

**Delegation:** The ABAC policy model and enforcement are specified in KEI-ARC-025.

---

## 11. Protocol Translation Semantics

### 11.1 Stream vs. Queue Mode Selection

A gateway consumer selects its consumption mode, which determines how the State Plane overlay is interpreted (KEI-ARC-021 §12):

| Gateway | Default Mode | Rationale |
|---|---|---|
| Kafka Ingest Gateway | Stream (sequential replay) | Matches Kafka semantics. |
| SQS Translation Gateway | Queue (leases + ACK) | Matches SQS semantics. |
| AMQP Translation Gateway | Queue (leases + ACK) | Matches AMQP semantics. |
| Arrow Flight SDK | Caller-selected | Native callers choose explicitly. |

### 11.2 Ordering Preservation

**Normative rule:** A gateway MUST preserve the ordering guarantees of the protocol it emulates. Kafka partition order, SQS MessageGroupId order, and AMQP per-queue order MUST all map to PEF entity-key ordering without reordering.

### 11.3 Delivery Guarantee Propagation

Gateways propagate the PEF delivery model:

- Default is at-least-once (ADR-022).
- Idempotent produce is available where the source protocol supports it (e.g., Kafka idempotent producer).
- Exactly-once end-to-end requires consumer cooperation; gateways MUST NOT claim otherwise.

---

## 12. Protocol Admission Control and Backpressure (G7)

The Protocol Plane is the enforcement point for tenant quotas before work reaches the storage engine.

| Control | Mechanism |
|---|---|
| Ingress rate limiting | Per-tenant token bucket at the gateway socket. |
| Connection limiting | Max concurrent connections per tenant. |
| Request size limiting | Max batch/message size per protocol. |
| Backpressure propagation | TCP clamping coordinated with KEI-ARC-020 Backpressure Controller. |

**Normative rule:** Protocol admission MUST reject or backpressure before allocating storage-engine resources, to protect the fabric from noisy neighbors.

---

## 13. Gateway-Specific Failure Handling

| Scenario | Defense (this subsystem) |
|---|---|
| Unsupported protocol operation | Clear documented error; never silent misbehavior. |
| Identity resolution failure | Reject request; audit-log the attempt. |
| Gateway crash | Stateless gateways restart without state loss (state lives in State Plane). |
| Downstream storage backpressure | Propagate via TCP clamping; return retriable errors. |
| Version mismatch | Compatibility Matrix gate rejects unsupported versions. |
| CDC connector schema drift | Produce path accepts raw payloads; schema governance in KEI-ARC-023. |

---

## 14. NFR Traceability (Owned by This Subsystem)

| NFR | Requirement | How This Subsystem Satisfies It |
|---|---|---|
| PERF-032 | Arrow Flight CPU ≤1/3 JVM Kafka | Zero-copy Arrow + SIMD pushdown (§7.4). |
| SCALE (gateway) | High connection cardinality | Stateless gateways + admission control (§12). |
| SEC (gateway) | Authenticated, authorized access | Identity Mapping to ABAC (§10). |
| COMPAT | Published subset only | Compatibility Matrix Registry (§5.2). |
| AVAIL (gateway) | Stateless restartability | Gateways hold no durable state (§13). |

---

## 15. Interfaces

### 15.1 Provided Interfaces

| Interface | Consumer | Semantics |
|---|---|---|
| Kafka RPC endpoint | Kafka producers / CDC | Produce/Fetch/Metadata/ListOffsets/Offset*. |
| Arrow Flight / gRPC endpoint | Native SDKs | Append/StreamFetch/LeaseNext/Ack/Nack/RenewLease/PushdownQuery. |
| SQS REST endpoint | SQS clients | SendMessage/ReceiveMessage/DeleteMessage/ChangeMessageVisibility. |
| AMQP 0-9-1 endpoint | AMQP clients | publish/consume/ack/nack/reject. |

### 15.2 Consumed Interfaces

| Interface | Provider | Purpose |
|---|---|---|
| `append(batch)` | KEI-ARC-020 | Durable record persistence. |
| `read(stream, range)` | KEI-ARC-020 | Stream replay. |
| `lease / ack / nack / renew` | KEI-ARC-021 | Consumption state transitions. |
| ABAC policy evaluation | KEI-ARC-025 | Authorization decisions. |
| Tenant quotas | Control Plane | Admission control. |

---

## 16. Open Questions and ADR Dependencies

| Item | Status | Resolution Path |
|---|---|---|
| Kafka Fetch version upper bound (v15) validation | Open | Test matrix in KEI-DES-035 before Phase-3 exit. |
| SQS FIFO deduplication semantics mapping | Open | Specify in KEI-DES-035. |
| AMQP topic-exchange support decision | Open | ADR candidate; currently deferred. |
| SDK language release ordering | Open | Program planning (Rust first per ADR-080). |
| Kafka transactions gateway support | Open | Deferred; record as explicit non-goal unless superseded. |

Binding decisions already recorded: ADR-070, ADR-071, ADR-080.

---

## 17. Glossary (Additions)

| Term | Definition |
|---|---|
| Compatibility Matrix | The published, validated subset of protocol operations a gateway supports. |
| Compatibility-by-Subset | The principle that gateways guarantee a published subset, never full parity. |
| Dual Interface | The strategy of a Kafka gateway for migration plus a native Arrow Flight SDK for performance. |
| Identity Mapper | The component mapping gateway principals to PEF ABAC principals. |
| Protocol Admission | Tenant quota and backpressure enforcement at the protocol edge. |

---

## 18. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial protocol plane architecture. Defines Kafka ingest gateway subset, Arrow Flight/gRPC SDK operations, SQS/AMQP translation gateways, identity mapping to ABAC, protocol translation semantics, and admission control. Enforces Compatibility-by-Subset (ADR-070) and Dual Interface (ADR-071). Retires "100% parity" language. |