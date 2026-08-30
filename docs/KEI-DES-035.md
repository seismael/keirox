# KEI-DES-035 — Gateway Wire-Protocol Compatibility Matrices

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-DES-035 |
| Title | Gateway Wire-Protocol Compatibility Matrices |
| Version | 1.0 |
| Level | **L3 — Detailed Design Specification** |
| Subsystem Covered | Protocol Plane — Gateway Compatibility Governance |
| Status | Approved for Engineering |
| Classification | Internal / Engineering Confidential |
| Owner | Principal Engineer (Ecosystem & Integration) |
| Required Reviewers | Chief Architect, Principal Engineer (Distributed Systems), Security Lead, QA Lead, SDK Team Lead |
| Depends On | KEI-ARC-024 (Protocol Gateways & SDK Architecture), KEI-ARC-025 (Security), KEI-DES-032 (API & Protocol Specification), KEI-DES-031 (State Plane) |
| Consumed By | Gateway implementation teams, SDK teams, integration QA, enterprise migration teams, technical documentation |
| Keywords | MUST, MUST NOT, SHOULD, SHOULD NOT, MAY per RFC 2119 |

---

## 2. Purpose, Scope, and Audience

### 2.1 Purpose

This document defines the **normative wire-protocol compatibility matrices** for all Keirox protocol gateways. It specifies exactly which operations, versions, fields, and semantics are supported, which are unsupported, how protocol errors are mapped, and what compatibility testing is required before release.

It implements:

- ADR-070: Compatibility-by-Subset.
- ADR-071: Dual Interface strategy.
- KEI-ARC-024 gateway architecture.
- KEI-DES-032 external API contracts.

### 2.2 Scope

**In scope:**

- Kafka wire protocol compatibility matrix.
- SQS translation gateway compatibility matrix.
- AMQP translation gateway compatibility matrix.
- Protocol-to-PEF semantic mappings.
- Unsupported operation behavior.
- Protocol error mapping.
- Authentication and authorization compatibility.
- Version negotiation and capability discovery.
- Compatibility test requirements and release gates.
- Gateway observability requirements.

**Out of scope:**

- Native gRPC / Arrow Flight API contract — owned by KEI-DES-032.
- State plane internals — owned by KEI-DES-031.
- Storage engine internals — owned by KEI-DES-030.
- Customer-specific integration runbooks — owned by KEI-OPS-040.

### 2.3 Audience

- Gateway implementation engineers.
- Compatibility QA engineers.
- Migration architects.
- SDK developers.
- Security engineers.
- Technical writers publishing public compatibility documentation.

---

## 3. Compatibility Design Principles

| ID | Principle | Rationale |
|---|---|---|
| GW-1 | **Compatibility by published subset.** | Gateways guarantee only what is explicitly listed in this document. |
| GW-2 | **No silent semantic substitution.** | Unsupported features MUST return explicit errors, not approximate behavior. |
| GW-3 | **Protocol errors must be protocol-native.** | Clients should receive familiar, retriable error semantics. |
| GW-4 | **Compatibility is versioned.** | Each gateway protocol version MUST be independently tested and certified. |
| GW-5 | **Security is not downgraded for compatibility.** | Legacy plaintext or weak-auth modes MUST be disabled by default. |
| GW-6 | **Compatibility tests gate releases.** | No gateway version ships without passing the conformance suite. |
| GW-7 | **Semantic differences are documented.** | Where PEF semantics differ from the legacy system, the difference MUST be explicit. |

---

## 4. Compatibility Support Model

### 4.1 Support Tiers

| Tier | Label | Meaning |
|---:|---|---|
| S0 | Not Supported | Operation is rejected with explicit unsupported error. |
| S1 | Fully Supported | Operation is supported with validated PEF semantics. |
| S2 | Supported with Limitations | Operation is supported but with documented constraints. |
| S3 | Compatibility Shim | Operation is accepted for migration compatibility but not recommended; may be disabled by default. |

### 4.2 Compatibility Profiles

| Profile ID | Name | Description |
|---|---|---|
| `K-INGEST` | Kafka Ingest Producer Profile | Produce-path compatibility for producers and CDC connectors. |
| `K-STREAM` | Kafka Stream Consumer Profile | Fetch-path stream replay compatibility. |
| `K-GROUP` | Kafka Consumer Group Profile | Consumer group coordination compatibility. |
| `K-IDEM` | Kafka Idempotent Producer Profile | Idempotent, non-transactional producer compatibility. |
| `SQS-STD` | SQS Standard Queue Profile | Standard queue operations. |
| `SQS-FIFO` | SQS FIFO Queue Profile | FIFO queue operations with MessageGroupId ordering. |
| `AMQP-DIRECT` | AMQP Direct Exchange Profile | Direct/default exchange queueing operations. |

### 4.3 Declared Range vs. Certified Subset

For each protocol, this specification distinguishes:

- **Declared Compatibility Range:** The full protocol version range the gateway understands.
- **Certified Subset:** The exact versions and operations that have passed the compatibility test suite and are approved for production.

**Normative rule:** Public documentation MUST present the certified subset, not the declared range, unless explicitly labeled as experimental.

---

# 5. Kafka Wire Protocol Compatibility Matrix

## 5.1 Kafka Compatibility Overview

The Kafka gateway provides migration compatibility for producers, CDC connectors, log shippers, and stream consumers. It does **not** provide full Kafka broker parity.

### 5.1.1 Kafka Profiles

| Profile | Status |
|---|---|
| `K-INGEST` | Primary supported profile. |
| `K-IDEM` | Supported for non-transactional idempotent producers. |
| `K-STREAM` | Supported for stream-mode fetch consumers. |
| `K-GROUP` | Supported with limitations. |
| Kafka Transactions | Not supported. |
| Kafka Share Groups | Not supported. |
| Kafka Admin Reassignment | Not supported. |

## 5.2 Kafka API Matrix

### 5.2.1 Core Data Plane APIs

| API Key | API Name | Declared Range | Certified Subset | Tier | PEF Mapping | Limitations |
|---:|---|---|---|---|---|---|
| 0 | Produce | v0–v12 | v3–v9 | S1 | `Append` / `AppendBatch` | No transactional records. Legacy versions v0–v2 disabled by default. |
| 1 | Fetch | v0–v15 | v4–v13 | S1 | `StreamFetch` | Transaction isolation levels mapped to non-transactional semantics. |
| 2 | ListOffsets | v0–v7 | v2–v7 | S1 | Stream head/tail lookup | Timestamp-based offset lookup is S2. |
| 3 | Metadata | v0–v12 | v1–v12 | S1 | Stream discovery | Reports virtual partitions, not physical partitions. |

### 5.2.2 Consumer Group Coordination APIs

| API Key | API Name | Declared Range | Certified Subset | Tier | PEF Mapping | Limitations |
|---:|---|---|---|---|---|---|
| 10 | FindCoordinator | v0–v4 | v1–v4 | S1 | Coordinator discovery | Coordinator is gateway-managed, not Kafka controller. |
| 11 | JoinGroup | v0–v9 | v2–v9 | S2 | Group membership | Limited assignors; no dynamic partition scaling. |
| 12 | Heartbeat | v0–v4 | v1–v4 | S1 | Session liveness | Standard session timeout semantics. |
| 13 | LeaveGroup | v0–v4 | v1–v4 | S1 | Group departure | Standard behavior. |
| 14 | SyncGroup | v0–v4 | v1–v4 | S2 | Assignment distribution | Assignment is gateway-managed. |
| 8 | OffsetCommit | v0–v9 | v3–v9 | S1 | Stream offset commit | Offsets map to PEF stream offsets. |
| 9 | OffsetFetch | v0–v9 | v3–v9 | S1 | Stream offset fetch | No transactional offset state. |

### 5.2.3 Protocol Negotiation and Authentication APIs

| API Key | API Name | Declared Range | Certified Subset | Tier | PEF Mapping | Limitations |
|---:|---|---|---|---|---|---|
| 17 | SaslHandshake | v0–v1 | v1 | S1 | SASL negotiation | SCRAM only by default. |
| 18 | ApiVersions | v0–v3 | v0–v3 | S1 | Version discovery | Returns certified subset. |
| 36 | SaslAuthenticate | v0–v2 | v1–v2 | S1 | Authenticated session | SCRAM-SHA-256/512. |

### 5.2.4 Producer Idempotence APIs

| API Key | API Name | Declared Range | Certified Subset | Tier | PEF Mapping | Limitations |
|---:|---|---|---|---|---|---|
| 22 | InitProducerId | v0–v4 | v1–v4 | S2 | Producer session creation | Transactional parameters rejected. Non-transactional idempotence supported. |

### 5.2.5 Unsupported Kafka APIs

| API Key | API Name | Tier | Reason |
|---:|---|---|---|
| 24 | AddPartitionsToTxn | S0 | Kafka transactions not supported. |
| 25 | AddOffsetsToTxn | S0 | Kafka transactions not supported. |
| 26 | EndTxn | S0 | Kafka transactions not supported. |
| 27 | WriteTxnMarkers | S0 | Kafka transactions not supported. |
| 28 | TxnOffsetCommit | S0 | Kafka transactions not supported. |
| 37 | CreatePartitions | S0 | PEF streams are virtual; physical partitions do not exist. |
| 34 | AlterReplicaLogDirs | S0 | Storage topology is abstracted. |
| 35 | DescribeLogDirs | S0 | Storage topology is abstracted. |
| 29 | DescribeAcls | S0 | ACLs map to PEF ABAC, not Kafka ACLs. |
| 30 | CreateAcls | S0 | ACLs map to PEF ABAC, not Kafka ACLs. |
| 31 | DeleteAcls | S0 | ACLs map to PEF ABAC, not Kafka ACLs. |
| 32 | DescribeConfigs | S2 | Read-only compatibility may be provided for limited keys. |
| 33 | AlterConfigs | S0 | Configuration model differs. |

## 5.3 Kafka Semantic Mapping

### 5.3.1 Topic and Partition Mapping

| Kafka Concept | PEF Mapping |
|---|---|
| Topic | Stream namespace / stream name. |
| Partition | Virtual state shard or synthetic partition bucket. |
| Partition key / record key | `entity_key`. |
| Record value | PEF payload. |
| Record headers | PEF metadata attributes. |
| Consumer group | PEF consumer group in stream mode. |
| Partition offset | PEF logical offset within mapped stream shard. |

### 5.3.2 Virtual Partition Model

Because PEF does not expose physical partitions, the Kafka gateway presents a **virtual partition model**.

Default:

```text
virtual_partitions_per_topic = 64
```

Configurable range:

```text
min = 1
max = 1024
```

**Normative rules:**

- Ordering MUST be preserved within each virtual partition.
- Record keys MUST map deterministically to virtual partitions.
- Records without keys MAY be assigned round-robin or by gateway policy.
- Clients MUST NOT assume virtual partitions correspond to physical files or Raft groups.

### 5.3.3 Producer Idempotence Mapping

Kafka idempotent producer fields map to PEF producer identity:

```text
Kafka producer_id     → PEF producer_id
Kafka producer_epoch  → PEF producer_epoch
Kafka sequence number → PEF producer_seq
```

**Normative rules:**

- Duplicate sequence numbers within the dedup window MUST return the original offset.
- Transactional producer IDs MUST be rejected.
- The gateway MUST expose idempotence only within PEF dedup window limits.

### 5.3.4 Consumer Group Mapping

| Kafka Behavior | PEF Behavior |
|---|---|
| Partition assignment | Virtual partition assignment managed by gateway. |
| Rebalance | Triggered on membership change, not storage topology change. |
| Offset commit | Persisted as stream-mode offset commit. |
| Heartbeat session | Gateway session liveness. |
| Static membership | Supported with limitations. |
| Cooperative rebalance | Optional; not required in v1. |

### 5.3.5 Unsupported Kafka Semantics

The following Kafka semantics are explicitly unsupported:

- Exactly-once transactional produce/consume.
- Share groups.
- Physical partition reassignment.
- Partition-level retention overrides.
- Replica placement control.
- Kafka ACL management.
- Log directory management.
- Custom partitioner server-side behavior beyond key hash mapping.

## 5.4 Kafka Error Mapping

| PEF Condition | Kafka Error | Retryable |
|---|---|---|
| Stream not found | `UNKNOWN_TOPIC_OR_PARTITION` | No |
| Coordinator epoch mismatch | `NOT_COORDINATOR` | Yes |
| Request timed out | `REQUEST_TIMED_OUT` | Yes |
| Quota exceeded | `THROTTLING_QUOTA_EXCEEDED` | Yes |
| Message too large | `MESSAGE_TOO_LARGE` | No |
| Invalid acks value | `INVALID_REQUIRED_ACKS` | No |
| Unsupported version | `UNSUPPORTED_VERSION` | No |
| Unsupported transactional request | `TRANSACTIONAL_ID_ERROR` | No |
| Offset out of range | `OFFSET_OUT_OF_RANGE` | No |
| Invalid group ID | `INVALID_GROUP_ID` | No |
| Unknown member ID | `UNKNOWN_MEMBER_ID` | Yes |
| Rebalance in progress | `REBALANCE_IN_PROGRESS` | Yes |
| Authorization failure | `TOPIC_AUTHORIZATION_FAILED` | No |
| Authentication failure | `SASL_AUTHENTICATION_FAILED` | No |

---

# 6. SQS Translation Gateway Compatibility Matrix

## 6.1 SQS Compatibility Overview

The SQS gateway maps Amazon SQS-style queue operations to PEF queue-mode state transitions.

### 6.1.1 SQS Profiles

| Profile | Status |
|---|---|
| `SQS-STD` | Supported. |
| `SQS-FIFO` | Supported with limitations. |
| SQS delayed messages | Not supported in v1. |
| SQS DLQ configuration | Replaced by PEF virtual DLQ policy. |
| SQS server-side encryption controls | Replaced by PEF encryption policy. |

## 6.2 SQS API Matrix

### 6.2.1 Core Queue Operations

| SQS API | Tier | PEF Mapping | Limitations |
|---|---|---|---|
| `SendMessage` | S1 | `Append` in queue mode | DelaySeconds unsupported. |
| `SendMessageBatch` | S1 | `AppendBatch` in queue mode | Max batch entries per SQS spec. |
| `ReceiveMessage` | S1 | `LeaseNext` | Visibility timeout maps to lease TTL. |
| `DeleteMessage` | S1 | `Ack` | ReceiptHandle maps to lease token. |
| `DeleteMessageBatch` | S1 | Batch `Ack` | Invalid receipts reported per batch entry. |
| `ChangeMessageVisibility` | S1 | `RenewLease` | Zero visibility timeout maps to early release. |
| `ChangeMessageVisibilityBatch` | S1 | Batch `RenewLease` | Entry-level errors reported. |

### 6.2.2 Queue Discovery and Attributes

| SQS API | Tier | PEF Mapping | Limitations |
|---|---|---|---|
| `GetQueueUrl` | S1 | Stream name resolution | Queue URL is gateway-generated. |
| `GetQueueAttributes` | S2 | Stream/group telemetry | Approximate values only. |
| `ListQueues` | S2 | Stream listing | Requires tenant authorization. |
| `CreateQueue` | S2 | Stream/group creation | Limited attribute support. |
| `DeleteQueue` | S2 | Stream deletion | Requires elevated authorization. |
| `PurgeQueue` | S2 | Admin purge | Requires elevated authorization. |
| `SetQueueAttributes` | S2 | Policy update | Only supported attributes. |

### 6.2.3 Unsupported SQS Operations and Attributes

| SQS Feature | Tier | Reason |
|---|---|---|
| Per-message `DelaySeconds` | S0 | Delayed delivery not supported in v1. |
| Queue-level `DelaySeconds` | S0 | Delayed delivery not supported in v1. |
| Dead-letter queue redrive configuration | S0 | Replaced by PEF virtual DLQ. |
| Server-side encryption SSE-KMS controls | S0 | Replaced by PEF encryption policy. |
| Cross-account permissions | S0 | Replaced by PEF ABAC. |
| Message timers | S0 | Delayed delivery not supported in v1. |
| Exactly-once processing guarantee | S0 | PEF provides at-least-once with idempotent consumer requirements. |

## 6.3 SQS Attribute Mapping

### 6.3.1 Message Attributes

| SQS Field | PEF Mapping |
|---|---|
| `MessageBody` | PEF payload. |
| `MessageAttributes` | PEF metadata attributes. |
| `MessageGroupId` | `entity_key` for FIFO queues. |
| `MessageDeduplicationId` | Idempotency key. |
| `ReceiptHandle` | Lease token. |
| `ApproximateReceiveCount` | Retry count. |
| `SentTimestamp` | Ingress timestamp. |

### 6.3.2 Queue Attributes

| SQS Attribute | PEF Mapping | Support |
|---|---|---|
| `VisibilityTimeout` | Default lease TTL | S1 |
| `MaximumMessageSize` | Tenant/stream payload quota | S2 |
| `MessageRetentionPeriod` | Stream retention policy | S2 |
| `ApproximateNumberOfMessages` | Ready message estimate | S2 |
| `ApproximateNumberOfMessagesNotVisible` | Active lease estimate | S2 |
| `ApproximateNumberOfMessagesDelayed` | Delayed message count | S0 |
| `CreatedTimestamp` | Stream creation time | S1 |
| `LastModifiedTimestamp` | Stream policy update time | S2 |
| `QueueArn` | PEF stream resource identifier | S2 |
| `FifoQueue` | Queue ordering mode | S1 |
| `ContentBasedDeduplication` | Payload-hash idempotency | S2 |
| `ReceiveMessageWaitTimeSeconds` | Long-poll lease wait | S2 |

## 6.4 SQS FIFO Semantics

### 6.4.1 FIFO Mapping

```text
FifoQueue = true
MessageGroupId → entity_key
```

**Normative rules:**

- Messages with the same `MessageGroupId` MUST be processed in order.
- Different `MessageGroupId` values MAY be processed concurrently.
- Missing `MessageGroupId` in a FIFO queue MUST return `InvalidParameterValue`.
- Deduplication MUST use explicit `MessageDeduplicationId` or content-based hash if enabled.

### 6.4.2 FIFO Limitations

- FIFO throughput quotas are governed by PEF tenant quotas.
- Exactly-once deduplication is not guaranteed beyond PEF idempotence windows.
- Delayed FIFO messages are unsupported in v1.

## 6.5 SQS Error Mapping

| PEF Condition | SQS Error | Retryable |
|---|---|---|
| Queue/stream not found | `AWS.SimpleQueueService.NonExistentQueue` | No |
| Invalid receipt handle | `ReceiptHandleIsInvalid` | No |
| Lease not active | `MessageNotInflight` | No |
| Quota exceeded | `RequestThrottled` | Yes |
| Too many batch entries | `TooManyEntriesInBatchRequest` | No |
| Invalid parameter | `InvalidParameterValue` | No |
| Unsupported attribute | `InvalidAttributeName` | No |
| Authorization failure | `AccessDenied` | No |
| Authentication failure | `ExpiredToken` or `InvalidClientTokenId` | No |
| Payload too large | `MessageTooLong` | No |
| FIFO missing group ID | `InvalidParameterValue` | No |

---

# 7. AMQP Translation Gateway Compatibility Matrix

## 7.1 AMQP Compatibility Overview

The AMQP gateway supports AMQP 0-9-1 workloads that use direct or default exchanges. It does **not** support complex exchange routing topologies.

### 7.1.1 AMQP Profile

| Profile | Status |
|---|---|
| `AMQP-DIRECT` | Supported. |
| Default exchange | Supported. |
| Direct exchange | Supported. |
| Fanout exchange | Not supported. |
| Topic exchange | Not supported. |
| Headers exchange | Not supported. |
| AMQP transactions | Not supported. |
| Publisher confirms | Not supported in v1. |

## 7.2 AMQP Method Matrix

### 7.2.1 Connection and Channel Methods

| AMQP Method | Tier | Notes |
|---|---|---|
| `connection.start` / `start-ok` | S1 | Standard handshake. |
| `connection.tune` / `tune-ok` | S1 | Standard tuning. |
| `connection.open` / `open-ok` | S1 | Virtual host maps to tenant namespace. |
| `connection.close` / `close-ok` | S1 | Graceful close. |
| `channel.open` / `open-ok` | S1 | Channel session. |
| `channel.close` / `close-ok` | S1 | Channel teardown. |
| `channel.flow` | S2 | Flow control mapped to backpressure. |

### 7.2.2 Exchange Methods

| AMQP Method | Tier | Notes |
|---|---|---|
| `exchange.declare` | S2 | Direct and default exchanges only. |
| `exchange.declare-ok` | S2 | Success response. |
| `exchange.delete` | S2 | Requires authorization. |
| `exchange.delete-ok` | S2 | Success response. |
| `exchange.bind` | S0 | Exchange-to-exchange routing unsupported. |
| `exchange.unbind` | S0 | Exchange-to-exchange routing unsupported. |

### 7.2.3 Queue Methods

| AMQP Method | Tier | Notes |
|---|---|---|
| `queue.declare` | S1 | Maps to PEF stream/group. |
| `queue.declare-ok` | S1 | Returns queue metadata. |
| `queue.bind` | S2 | Direct exchange binding with routing key. |
| `queue.bind-ok` | S2 | Success response. |
| `queue.unbind` | S2 | Removes binding. |
| `queue.unbind-ok` | S2 | Success response. |
| `queue.purge` | S2 | Requires authorization. |
| `queue.purge-ok` | S2 | Success response. |
| `queue.delete` | S2 | Requires authorization. |
| `queue.delete-ok` | S2 | Success response. |

### 7.2.4 Basic Methods

| AMQP Method | Tier | PEF Mapping | Limitations |
|---|---|---|---|
| `basic.qos` | S1 | Lease quota / prefetch | Global flag ignored or limited. |
| `basic.consume` | S1 | Long-poll `LeaseNext` | Push delivery emulated. |
| `basic.consume-ok` | S1 | Consumer registration | Standard behavior. |
| `basic.cancel` | S1 | Consumer deregistration | Standard behavior. |
| `basic.cancel-ok` | S1 | Confirmation | Standard behavior. |
| `basic.publish` | S1 | `Append` in queue mode | Mandatory/immediate flags limited. |
| `basic.deliver` | S1 | Lease delivery | Delivery tag maps to lease token. |
| `basic.get` | S1 | Single-message `LeaseNext` | Pull mode. |
| `basic.get-ok` | S1 | Lease response | Standard behavior. |
| `basic.get-empty` | S1 | No lease available | Standard behavior. |
| `basic.ack` | S1 | `Ack` | Multiple flag supported with limitations. |
| `basic.nack` | S1 | `Nack` | Requeue flag mapped. |
| `basic.reject` | S1 | `Nack` | Requeue flag mapped. |
| `basic.recover` | S2 | Redeliver unacked leases | May trigger retry heap. |
| `basic.recover-ok` | S2 | Confirmation | Standard behavior. |

### 7.2.5 Unsupported AMQP Methods

| AMQP Method | Tier | Reason |
|---|---|---|
| `tx.select` | S0 | Transactions unsupported. |
| `tx.commit` | S0 | Transactions unsupported. |
| `tx.rollback` | S0 | Transactions unsupported. |
| `confirm.select` | S0 | Publisher confirms unsupported in v1. |
| `basic.return` | S2 | Limited use with mandatory publish. |

## 7.3 AMQP Semantic Mapping

### 7.3.1 Core Concept Mapping

| AMQP Concept | PEF Mapping |
|---|---|
| Virtual host | Tenant namespace. |
| Exchange | Stream namespace or routing policy. |
| Routing key | `entity_key` or stream selector. |
| Queue | PEF consumer group over a stream. |
| Binding | Stream/group routing rule. |
| Delivery tag | Lease token. |
| Message properties | PEF metadata attributes. |
| Message body | PEF payload. |

### 7.3.2 Message Property Mapping

| AMQP Property | PEF Mapping | Support |
|---|---|---|
| `message_id` | Idempotency key or metadata | S2 |
| `correlation_id` | Metadata attribute | S1 |
| `reply_to` | Metadata attribute | S1 |
| `content_type` | Metadata attribute | S1 |
| `content_encoding` | Metadata attribute | S1 |
| `delivery_mode` | Durability hint | S2 |
| `priority` | Not supported | S0 |
| `expiration` | Not supported in v1 | S0 |
| `timestamp` | Metadata timestamp | S1 |
| `type` | Metadata attribute | S1 |
| `user_id` | Authenticated principal metadata | S2 |
| `app_id` | Metadata attribute | S1 |
| `headers` | Metadata attributes | S2 |

**Normative rules:**

- `delivery_mode = 2` SHOULD map to durable PEF append.
- `delivery_mode = 1` MAY be rejected or accepted as durable depending on tenant policy. PEF v1 does not provide non-durable queue semantics by default.
- `priority` MUST be rejected or ignored with documented behavior.
- `expiration` MUST NOT silently implement delayed delivery.

### 7.3.3 Acknowledgment Mapping

| AMQP Behavior | PEF Behavior |
|---|---|
| `basic.ack(delivery_tag)` | ACK single lease. |
| `basic.ack(delivery_tag, multiple=true)` | ACK all outstanding leases with delivery tag ≤ specified tag for the same consumer, where resolvable. |
| `basic.nack(requeue=true)` | NACK and requeue unless retry limit exceeded. |
| `basic.nack(requeue=false)` | NACK without requeue; may route to DLQ. |
| `basic.reject(requeue=true)` | Same as `basic.nack(requeue=true)`. |
| `basic.reject(requeue=false)` | Same as `basic.nack(requeue=false)`. |

## 7.4 AMQP Error Mapping

| PEF Condition | AMQP Reply Code | Retryable |
|---|---|---|
| Queue not found | `404 NOT_FOUND` | No |
| Exchange unsupported | `540 NOT_IMPLEMENTED` | No |
| Authorization failure | `403 ACCESS_REFUSED` | No |
| Resource locked | `405 RESOURCE_LOCKED` | Yes |
| Precondition failed | `406 PRECONDITION_FAILED` | No |
| Not allowed | `530 NOT_ALLOWED` | No |
| Content too large | `311 CONTENT_TOO_LARGE` | No |
| No route for mandatory message | `312 NO_ROUTE` | No |
| Internal error | `541 INTERNAL_ERROR` | Yes |

---

# 8. Cross-Protocol Authentication and Authorization Compatibility

## 8.1 Authentication Mapping

| Protocol | Supported Mechanisms | PEF Mapping |
|---|---|---|
| Kafka | SASL/SCRAM-SHA-256, SASL/SCRAM-SHA-512, TLS | PEF principal via Principal Mapper. |
| SQS | AWS Signature V4-compatible signed request | PEF principal via gateway identity mapper. |
| AMQP | PLAIN over TLS | PEF principal via Principal Mapper. |
| Native gRPC | OAuth2/OIDC, mTLS | Direct PEF principal. |

## 8.2 Authorization Mapping

| Protocol Operation Class | Required PEF Permission |
|---|---|
| Produce/send/publish | `produce` |
| Fetch/receive/consume | `consume` or `lease` |
| Delete/ACK | `ack` |
| Nack/reject/requeue | `ack` |
| DLQ redrive | `dlq_redrive` |
| Queue/stream create | `admin` or `create` |
| Queue/stream delete | `delete` |
| Attribute/config read | `read_metadata` |

**Normative rule:** Gateways MUST NOT perform local authorization bypasses. All operations MUST be authorized through the PEF ABAC PDP.

## 8.3 Legacy Security Restrictions

- Plaintext Kafka connections MUST be disabled by default.
- SASL/PLAIN SHOULD be disabled by default unless explicitly enabled for migration.
- AMQP plaintext authentication MUST require TLS.
- SQS unsigned requests MUST be rejected.
- Weak TLS versions MUST be rejected.

---

# 9. Version Negotiation and Capability Discovery

## 9.1 Kafka Version Discovery

The Kafka gateway MUST support `ApiVersions` and return only certified API versions.

**Normative rules:**

- Unsupported API keys MUST be omitted or returned with error.
- Clients MUST be able to negotiate a certified version without querying undocumented endpoints.
- Deprecated versions MAY be hidden unless legacy compatibility mode is enabled.

## 9.2 SQS Capability Discovery

SQS does not have a formal version negotiation protocol. The gateway MUST expose a compatibility metadata endpoint:

```http
GET /keirox/v1/compatibility/sqs
```

Response includes:

```json
{
  "profile": "SQS-STD",
  "supported_operations": ["SendMessage", "ReceiveMessage", "DeleteMessage"],
  "unsupported_features": ["DelaySeconds", "MessageTimers"],
  "gateway_version": "1.0.0"
}
```

## 9.3 AMQP Capability Discovery

AMQP clients discover capabilities through connection negotiation and server properties.

The gateway SHOULD expose:

```text
capabilities.publisher_confirms = false
capabilities.exchange_exchange_bindings = false
capabilities.basic.nack = true
capabilities.consumer_cancel_notify = true
capabilities.connection.blocked = false
```

---

# 10. Compatibility Test Requirements

## 10.1 Test Client Matrix

### 10.1.1 Kafka Clients

| Client | Minimum Required Test Coverage |
|---|---|
| librdkafka | Produce, fetch, metadata, offset commits. |
| Apache Kafka Java client | Producer, consumer, group coordination. |
| Sarama (Go) | Produce, consume, metadata. |
| kafka-go | Produce, consume. |
| aiokafka / kafka-python | Produce, consume. |

### 10.1.2 SQS Clients

| Client | Minimum Required Test Coverage |
|---|---|
| AWS SDK for Java | Send, receive, delete, visibility change. |
| AWS SDK for Python (boto3) | Send, receive, delete, attributes. |
| AWS SDK for Go | Send, receive, delete. |
| AWS SDK for JavaScript | Send, receive, delete. |

### 10.1.3 AMQP Clients

| Client | Minimum Required Test Coverage |
|---|---|
| RabbitMQ Java client | Queue declare, publish, consume, ack. |
| Pika (Python) | Queue declare, publish, consume, ack/nack. |
| amqp091-go | Queue declare, publish, consume, ack. |
| Bunny (Ruby) or amqplib (Node.js) | Optional secondary coverage. |

## 10.2 Functional Test Classes

| Test Class | Requirement |
|---|---|
| Happy-path operations | All S1 operations MUST pass. |
| Supported limitation operations | All S2 operations MUST pass with documented constraints. |
| Unsupported operation tests | S0 operations MUST return explicit errors. |
| Version negotiation tests | Clients MUST discover certified versions. |
| Authentication tests | Valid and invalid credentials MUST behave correctly. |
| Authorization tests | Denied operations MUST return protocol-native errors. |
| Idempotence tests | Duplicate produces/ACKs MUST be safe. |
| Ordering tests | Entity-key/partition/group ordering MUST hold. |
| Retry/backpressure tests | Throttling MUST be retriable. |
| Crash recovery tests | Gateway restart MUST NOT corrupt state. |

## 10.3 Compatibility Release Gates

A gateway version MAY be released only if:

1. All S1 tests pass.
2. All S2 limitations are documented and tested.
3. All S0 operations return explicit unsupported errors.
4. No data loss or duplicate append occurs in idempotence tests.
5. No authorization bypass is detected.
6. No protocol framing corruption is detected.
7. Performance overhead is within target.
8. Long-running soak test passes for at least 72 hours.

**Normative rule:** A known P1 compatibility defect MUST block release unless an explicit exception is approved and documented.

## 10.4 Performance Compatibility Targets

| Metric | Target |
|---|---|
| Gateway translation overhead | SHOULD add ≤0.5 ms p99 under Profile P1. |
| Unsupported request handling | MUST return in ≤5 ms under normal load. |
| Version discovery latency | SHOULD be ≤10 ms. |
| Auth failure handling | MUST be rate-limited and audited. |

---

# 11. Gateway Failure and Degradation Behavior

| Failure | Required Gateway Behavior |
|---|---|
| Unsupported API version | Return protocol-native unsupported version error. |
| Unsupported operation | Return explicit unsupported error; audit if repeated. |
| State plane unavailable | Return retriable unavailable error. |
| Storage engine backpressure | Propagate throttling via protocol-native signals. |
| Authorization service unavailable | Fail closed; deny operations unless cached policy permits bounded grace. |
| Coordinator failover | Redirect or retry with protocol-native retryable errors. |
| Client protocol mismatch | Reject with clear negotiation error. |
| Malformed frame | Close connection safely; emit security telemetry. |

---

# 12. Gateway Observability Requirements

| Metric | Type | Description |
|---|---|---|
| `keirox_gateway_requests_total` | Counter | Requests by protocol, API, version, status. |
| `keirox_gateway_unsupported_requests_total` | Counter | Unsupported operations or versions. |
| `keirox_gateway_translation_latency_seconds` | Histogram | Protocol translation overhead. |
| `keirox_gateway_auth_failures_total` | Counter | Authentication failures. |
| `keirox_gateway_authz_denials_total` | Counter | Authorization denials. |
| `keirox_gateway_backpressure_total` | Counter | Throttled requests by stage. |
| `keirox_gateway_client_versions` | Gauge | Connected client version distribution. |
| `keirox_gateway_session_count` | Gauge | Active protocol sessions. |

**Normative rule:** Every unsupported operation MUST be observable. Repeated unsupported calls from a client SHOULD trigger migration guidance telemetry.

---

# 13. NFR Traceability

| Requirement Area | Source | How This Specification Satisfies It |
|---|---|---|
| Compatibility governance | ADR-070 | Explicit support tiers and certified subsets. |
| Dual interface strategy | ADR-071 | Kafka/SQS/AMQP gateways plus native API boundary. |
| Security | KEI-ARC-025 | AuthN/AuthZ mapping and legacy restrictions. |
| Operability | KEI-ARC-027 | Gateway metrics, alerts, and failure behavior. |
| Reliability | KEI-ARC-022/021 | Retryable errors during failover and backpressure. |
| Performance | KEI-ARC-011 | Gateway overhead targets and test gates. |

---

# 14. Interfaces

## 14.1 Provided Interfaces

| Interface | Consumer | Semantics |
|---|---|---|
| Kafka RPC endpoint | Kafka clients | Certified Kafka compatibility subset. |
| SQS HTTP endpoint | SQS clients | Certified SQS compatibility subset. |
| AMQP endpoint | AMQP clients | Certified AMQP direct-exchange subset. |
| Compatibility metadata endpoint | Migration tooling | Machine-readable support matrix. |

## 14.2 Consumed Interfaces

| Interface | Provider | Purpose |
|---|---|---|
| Native append/fetch/lease APIs | KEI-DES-032 | Protocol translation target. |
| State plane operations | KEI-DES-031 | Lease/ACK transitions. |
| ABAC authorization | KEI-ARC-025 | Access control. |
| Quota/admission control | KEI-ARC-027 | Throttling. |
| Audit sink | KEI-ARC-025 | Security and compatibility telemetry. |

---

# 15. Open Questions

| Item | Status | Resolution Path |
|---|---|---|
| Exact Kafka client version certification list | Open | QA conformance suite before Phase-3 exit. |
| Kafka consumer group cooperative rebalance support | Open | Evaluate client ecosystem requirements. |
| SQS legacy Query API support | Open | Evaluate migration demand. |
| AMQP publisher confirms | Open | ADR candidate for post-v1. |
| Delayed message/timer support | Open | Requires state-plane timer extension. |
| Kafka timestamp-based offset lookup precision | Open | Benchmark index cost. |
| AMQP basic.return semantics for mandatory publish | Open | Define with gateway error policy. |

---

# 16. Glossary

| Term | Definition |
|---|---|
| Certified Subset | The exact operations and versions that have passed compatibility testing. |
| Declared Range | The full protocol range the gateway can parse or recognize. |
| Support Tier | Classification of operation compatibility. |
| Virtual Partition | Gateway-visible partition abstraction mapped to PEF state shards. |
| Compatibility Shim | Limited compatibility behavior intended only for migration. |
| Compatibility Profile | Named set of supported operations for a workload class. |

---

# 17. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial gateway compatibility matrix specification. Defines Kafka, SQS, and AMQP support tiers, certified subsets, semantic mappings, error mappings, auth compatibility, capability discovery, test requirements, release gates, and observability. Implements ADR-070/071. |