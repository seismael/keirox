# KEI-DES-032 — Producer/Consumer/Lease/ACK API & Protocol Specification

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-DES-032 |
| Title | Producer/Consumer/Lease/ACK API & Protocol Specification |
| Version | 1.0 |
| Level | **L3 — Detailed Design Specification** |
| Subsystem Covered | Protocol Plane — External API Contracts |
| Status | Approved for Engineering |
| Classification | Internal / Engineering Confidential |
| Owner | Principal Engineer (Ecosystem & Integration) |
| Required Reviewers | Chief Architect, Principal Engineer (Distributed Systems), Security Lead, SDK Team Lead |
| Depends On | KEI-ARC-021 (State Plane), KEI-ARC-024 (Protocol Gateways), KEI-ARC-025 (Security), KEI-DES-031 (State Plane Data Structures) |
| Consumed By | SDK implementation teams, gateway implementation teams, integration test engineers, documentation writers |
| Keywords | MUST, MUST NOT, SHOULD, SHOULD NOT, MAY per RFC 2119 |

---

## 2. Purpose, Scope, and Audience

### 2.1 Purpose

This document specifies the **exact external API contracts, wire protocols, error codes, and compatibility rules** for all client-facing interfaces of the Polymorphic Event Fabric.

It implements:

- ADR-070: Compatibility-by-Subset (published compatibility matrices, never full parity claims).
- ADR-071: Dual Interface strategy (Kafka gateway for migration + native Arrow Flight SDK for performance).
- ADR-020: ACK_FAST and ACK_DURABLE modes exposed to clients.
- ADR-022: At-least-once default delivery semantics.

### 2.2 Scope

**In scope:**

- Native gRPC / Arrow Flight API contracts.
- Kafka wire protocol gateway operation mapping.
- SQS translation gateway operation mapping.
- AMQP translation gateway operation mapping.
- Error code taxonomy and retry semantics.
- Idempotency key contracts.
- Authentication and authorization integration at the protocol edge.
- Protocol-level backpressure and throttling signals.
- API versioning and compatibility rules.

**Out of scope:**

- Internal state plane data structures — owned by KEI-DES-031.
- WAL binary format — owned by KEI-DES-030.
- Schema registry wire format — owned by KEI-DES-033.
- Iceberg catalog commit protocol — owned by KEI-DES-034.
- Full per-version Kafka/SQS/AMQP compatibility matrices — owned by KEI-DES-035.

### 2.3 Audience

- SDK implementation engineers (Rust, Go, Python, Java, TypeScript).
- Gateway implementation engineers.
- Integration test engineers.
- Technical documentation writers.
- Enterprise integration architects evaluating compatibility.

---

## 3. Design Principles

| ID | Principle | Rationale |
|---|---|---|
| API-1 | **Explicit semantics over magic.** | Every API call MUST have documented delivery, ordering, and durability guarantees. |
| API-2 | **Compatibility by published subset.** | Gateways guarantee only what is in the compatibility matrix. |
| API-3 | **Idempotency by default.** | All mutating operations MUST support idempotency keys. |
| API-4 | **Protocol-appropriate errors.** | Errors MUST map to the client protocol's native error semantics. |
| API-5 | **Backpressure is cooperative.** | Throttling signals MUST be retriable, not fatal. |
| API-6 | **Authentication before operation.** | No operation MAY execute without successful authentication. |
| API-7 | **Version negotiation at connection.** | Clients MUST negotiate API version before issuing operations. |

---

## 4. Native gRPC / Arrow Flight API

### 4.1 Service Definition

The native API is exposed via gRPC with Apache Arrow Flight for data transfer.

```protobuf
service KeiroxService {
  // Producer operations
  rpc Append(AppendRequest) returns (AppendResponse);
  rpc AppendBatch(stream AppendBatchChunk) returns (AppendBatchResponse);
  
  // Stream consumer operations
  rpc StreamFetch(StreamFetchRequest) returns (stream StreamFetchChunk);
  
  // Queue operations
  rpc LeaseNext(LeaseNextRequest) returns (LeaseNextResponse);
  rpc Ack(AckRequest) returns (AckResponse);
  rpc Nack(NackRequest) returns (NackResponse);
  rpc RenewLease(RenewLeaseRequest) returns (RenewLeaseResponse);
  
  // DLQ operations
  rpc DlqList(DlqListRequest) returns (DlqListResponse);
  rpc DlqRedrive(DlqRedriveRequest) returns (DlqRedriveResponse);
  
  // Query operations
  rpc PushdownQuery(PushdownQueryRequest) returns (stream PushdownQueryChunk);
  
  // Admin operations
  rpc CreateStream(CreateStreamRequest) returns (CreateStreamResponse);
  rpc DeleteStream(DeleteStreamRequest) returns (DeleteStreamResponse);
  rpc CreateConsumerGroup(CreateConsumerGroupRequest) returns (CreateConsumerGroupResponse);
  
  // Health and discovery
  rpc GetStreamInfo(GetStreamInfoRequest) returns (GetStreamInfoResponse);
  rpc NegotiateVersion(NegotiateVersionRequest) returns (NegotiateVersionResponse);
}
```

### 4.2 Common Message Types

```protobuf
message StreamIdentifier {
  uint64 tenant_id = 1;
  bytes stream_id = 2;       // 128-bit UUID
  string stream_name = 3;    // Human-readable name
}

message AckMode {
  enum Mode {
    ACK_MODE_UNSPECIFIED = 0;
    ACK_MODE_FAST = 1;        // Sub-ms, bounded loss window
    ACK_MODE_DURABLE = 2;     // Raft commit before success
  }
  Mode mode = 1;
}

message OperationMetadata {
  uint64 request_id = 1;
  uint64 idempotency_key = 2;
  uint64 coordinator_epoch = 3;
  string trace_id = 4;
  string span_id = 5;
}
```

### 4.3 Producer Operations

#### AppendRequest

```protobuf
message AppendRequest {
  StreamIdentifier stream = 1;
  bytes payload = 2;
  string entity_key = 3;
  string sub_entity_key = 4;
  uint64 producer_id = 5;
  uint64 producer_epoch = 6;
  uint64 producer_seq = 7;
  uint32 schema_id = 8;
  uint64 transaction_id = 9;  // 0 = non-transactional
  OperationMetadata metadata = 10;
}

message AppendResponse {
  uint64 logical_offset = 1;
  uint64 physical_seq = 2;
  uint64 timestamp_ns = 3;
  AppendStatus status = 4;
  
  enum AppendStatus {
    APPEND_STATUS_UNSPECIFIED = 0;
    APPEND_STATUS_ACCEPTED = 1;
    APPEND_STATUS_DUPLICATE = 2;  // Idempotent dedup hit
    APPEND_STATUS_THROTTLED = 3;
  }
}
```

**Normative rules:**

- `producer_id`, `producer_epoch`, and `producer_seq` MUST be provided for idempotent produce.
- If `APPEND_STATUS_DUPLICATE` is returned, `logical_offset` MUST contain the original offset.
- `entity_key` is optional; if absent, ordering defaults to stream-level.

#### AppendBatchRequest (streaming)

```protobuf
message AppendBatchChunk {
  oneof content {
    AppendBatchHeader header = 1;
    bytes record_batch = 2;  // Arrow IPC RecordBatch
  }
}

message AppendBatchHeader {
  StreamIdentifier stream = 1;
  uint64 producer_id = 2;
  uint64 producer_epoch = 3;
  uint64 producer_seq_start = 4;
  uint32 schema_id = 5;
  uint32 record_count = 6;
  OperationMetadata metadata = 7;
}

message AppendBatchResponse {
  uint64 first_logical_offset = 1;
  uint32 accepted_count = 2;
  uint32 duplicate_count = 3;
  AppendStatus status = 4;
}
```

### 4.4 Stream Consumer Operations

#### StreamFetchRequest

```protobuf
message StreamFetchRequest {
  StreamIdentifier stream = 1;
  uint64 group_id = 2;
  uint64 start_offset = 3;
  uint32 max_records = 4;
  uint32 max_bytes = 5;
  uint32 max_wait_ms = 6;
  ReadMode read_mode = 7;
  OperationMetadata metadata = 8;
  
  enum ReadMode {
    READ_MODE_UNSPECIFIED = 0;
    READ_MODE_UNCOMMITTED = 1;
    READ_MODE_COMMITTED = 2;
  }
}

message StreamFetchChunk {
  oneof content {
    FetchResponseHeader header = 1;
    bytes record_batch = 2;  // Arrow IPC RecordBatch
  }
}

message FetchResponseHeader {
  uint64 first_offset = 1;
  uint64 last_offset = 2;
  uint32 record_count = 3;
  bool has_more = 4;
}
```

### 4.5 Queue Operations

#### LeaseNextRequest

```protobuf
message LeaseNextRequest {
  StreamIdentifier stream = 1;
  uint64 group_id = 2;
  uint64 worker_id = 3;
  uint32 max_messages = 4;
  uint32 lease_ttl_ms = 5;
  AckMode ack_mode = 6;
  OperationMetadata metadata = 7;
}

message LeaseNextResponse {
  repeated LeasedMessage messages = 1;
  uint32 granted_count = 2;
  
  enum LeaseStatus {
    LEASE_STATUS_UNSPECIFIED = 0;
    LEASE_STATUS_GRANTED = 1;
    LEASE_STATUS_EMPTY = 2;
    LEASE_STATUS_QUOTA_EXCEEDED = 3;
  }
  LeaseStatus status = 3;
}

message LeasedMessage {
  uint64 offset = 1;
  uint64 lease_token = 2;
  bytes payload = 3;
  uint64 timestamp_ns = 4;
  uint32 retry_count = 5;
  string entity_key = 6;
}
```

#### AckRequest

```protobuf
message AckRequest {
  StreamIdentifier stream = 1;
  uint64 group_id = 2;
  uint64 offset = 3;
  uint64 lease_token = 4;
  uint64 worker_id = 5;
  AckMode ack_mode = 6;
  OperationMetadata metadata = 7;
}

message AckResponse {
  AckStatus status = 1;
  
  enum AckStatus {
    ACK_STATUS_UNSPECIFIED = 0;
    ACK_STATUS_ACCEPTED = 1;
    ACK_STATUS_ALREADY_ACKED = 2;
    ACK_STATUS_STALE_LEASE = 3;
    ACK_STATUS_LEASE_NOT_ACTIVE = 4;
    ACK_STATUS_OFFSET_EVICTED = 5;
  }
}
```

#### NackRequest

```protobuf
message NackRequest {
  StreamIdentifier stream = 1;
  uint64 group_id = 2;
  uint64 offset = 3;
  uint64 lease_token = 4;
  uint64 worker_id = 5;
  string reason = 6;
  OperationMetadata metadata = 7;
}

message NackResponse {
  NackStatus status = 1;
  uint32 retry_count = 2;
  
  enum NackStatus {
    NACK_STATUS_UNSPECIFIED = 0;
    NACK_STATUS_REQUEUED = 1;
    NACK_STATUS_EVICTED_TO_DLQ = 2;
    NACK_STATUS_STALE_LEASE = 3;
  }
}
```

#### RenewLeaseRequest

```protobuf
message RenewLeaseRequest {
  StreamIdentifier stream = 1;
  uint64 group_id = 2;
  uint64 offset = 3;
  uint64 lease_token = 4;
  uint64 worker_id = 5;
  uint32 new_ttl_ms = 6;
  OperationMetadata metadata = 7;
}

message RenewLeaseResponse {
  RenewStatus status = 1;
  uint64 new_expiry_ms = 2;
  
  enum RenewStatus {
    RENEW_STATUS_UNSPECIFIED = 0;
    RENEW_STATUS_RENEWED = 1;
    RENEW_STATUS_STALE_LEASE = 2;
    RENEW_STATUS_LEASE_EXPIRED = 3;
  }
}
```

### 4.6 DLQ Operations

```protobuf
message DlqListRequest {
  StreamIdentifier stream = 1;
  uint64 group_id = 2;
  uint64 start_offset = 3;
  uint32 max_entries = 4;
  OperationMetadata metadata = 5;
}

message DlqListResponse {
  repeated DlqEntry entries = 1;
  bool has_more = 2;
}

message DlqEntry {
  uint64 offset = 1;
  uint32 retry_count = 2;
  string reason = 3;
  uint64 evicted_at_ms = 4;
  uint64 last_worker_id = 5;
}

message DlqRedriveRequest {
  StreamIdentifier stream = 1;
  uint64 group_id = 2;
  repeated uint64 offsets = 3;
  bool reset_retry_count = 4;
  OperationMetadata metadata = 5;
}

message DlqRedriveResponse {
  uint32 redriven_count = 1;
  uint32 already_terminal_count = 2;
}
```

### 4.7 Pushdown Query Operations

```protobuf
message PushdownQueryRequest {
  StreamIdentifier stream = 1;
  uint64 start_offset = 2;
  uint64 end_offset = 3;
  string predicate = 4;       // SQL-like predicate expression
  repeated string columns = 5; // Requested columns
  uint32 max_records = 6;
  OperationMetadata metadata = 7;
}

message PushdownQueryChunk {
  oneof content {
    QueryResponseHeader header = 1;
    bytes record_batch = 2;  // Arrow IPC RecordBatch (filtered)
  }
}

message QueryResponseHeader {
  uint64 scanned_records = 1;
  uint64 matched_records = 2;
  bool has_more = 3;
}
```

---

## 5. Kafka Wire Protocol Gateway Mapping

### 5.1 Supported Operations

| Kafka API | PEF Native Equivalent | Notes |
|---|---|---|
| `Produce` (v0–v12) | `Append` / `AppendBatch` | Topic → stream; partition → entity_key |
| `Fetch` (v0–v15) | `StreamFetch` | Sequential replay mode |
| `Metadata` (v0–v12) | `GetStreamInfo` | Topic discovery |
| `ListOffsets` (v0–v7) | `GetStreamInfo` | Earliest/latest offset |
| `OffsetCommit` | State plane offset commit | Stream mode only |
| `OffsetFetch` | State plane offset fetch | Stream mode only |

### 5.2 Topic-to-Stream Mapping

```text
Kafka topic name     → PEF stream_name
Kafka partition ID   → PEF entity_key (synthetic: "partition_{id}")
Kafka record key     → PEF entity_key (if present)
Kafka record value   → PEF payload
Kafka headers        → PEF metadata attributes
```

### 5.3 Consumer Group Mapping

```text
Kafka consumer group → PEF consumer group (stream mode)
Kafka partition offset → PEF stream offset commit
Kafka rebalance → Not applicable (PEF has no partitions)
```

### 5.4 Unsupported Operations

The following Kafka operations MUST return `UNSUPPORTED_FOR_MESSAGE_VERSION` or equivalent:

- `InitProducerId` (transactional producer)
- `AddPartitionsToTxn`
- `TxnOffsetCommit`
- `EndTxn`
- `ShareGroup` operations (KIP-932)
- Admin operations: `CreatePartitions`, `AlterConfigs`, `DescribeAcls`

---

## 6. SQS Translation Gateway Mapping

### 6.1 Supported Operations

| SQS API | PEF Native Equivalent | Notes |
|---|---|---|
| `SendMessage` | `Append` | Queue mode |
| `SendMessageBatch` | `AppendBatch` | Queue mode |
| `ReceiveMessage` | `LeaseNext` | Visibility timeout → lease TTL |
| `DeleteMessage` | `Ack` | Receipt handle → lease_token |
| `ChangeMessageVisibility` | `RenewLease` | Extend or shorten TTL |
| `PurgeQueue` | Admin operation | Requires authorization |
| `GetQueueAttributes` | `GetStreamInfo` | Approximate counts |

### 6.2 Attribute Mapping

```text
SQS QueueUrl           → PEF stream_name
SQS MessageBody        → PEF payload
SQS MessageGroupId     → PEF entity_key (FIFO queues)
SQS VisibilityTimeout  → PEF lease_ttl_ms
SQS ReceiptHandle      → PEF lease_token
SQS ApproximateReceiveCount → PEF retry_count
SQS MessageAttributes  → PEF metadata attributes
```

### 6.3 Unsupported Operations

- `CreateQueue` with non-standard attributes MUST return `InvalidAttributeName`.
- `SetQueueAttributes` for unsupported attributes MUST return `InvalidAttributeName`.
- Dead-letter queue configuration is handled by PEF virtual DLQ, not SQS DLQ config.

---

## 7. AMQP Translation Gateway Mapping

### 7.1 Supported Operations

| AMQP Method | PEF Native Equivalent | Notes |
|---|---|---|
| `Basic.Publish` | `Append` | Queue mode |
| `Basic.Consume` | `LeaseNext` (polling) | Push delivery via long-poll |
| `Basic.Get` | `LeaseNext` (single) | Pull delivery |
| `Basic.Ack` | `Ack` | Delivery tag → lease_token |
| `Basic.Nack` | `Nack` | With requeue flag |
| `Basic.Reject` | `Nack` | With requeue flag |
| `Queue.Declare` | `CreateStream` + `CreateConsumerGroup` | Idempotent |
| `Queue.Delete` | `DeleteStream` | Requires authorization |

### 7.2 Exchange Mapping

| AMQP Exchange Type | PEF Mapping |
|---|---|
| Direct exchange | Stream with routing_key → entity_key |
| Default exchange | Stream name = queue name |
| Fanout exchange | NOT SUPPORTED in v1 |
| Topic exchange | NOT SUPPORTED in v1 |
| Headers exchange | NOT SUPPORTED in v1 |

### 7.3 Delivery Tag Mapping

```text
AMQP delivery_tag → PEF lease_token (u64)
```

---

## 8. Error Code Taxonomy

### 8.1 Native gRPC Error Codes

| Code | Name | Retryable | Description |
|---:|---|---|---|
| 0 | `OK` | No | Operation succeeded. |
| 1 | `CANCELLED` | Yes | Operation cancelled by caller. |
| 2 | `UNKNOWN` | Yes | Unknown error. |
| 3 | `INVALID_ARGUMENT` | No | Malformed request. |
| 4 | `DEADLINE_EXCEEDED` | Yes | Operation timed out. |
| 5 | `NOT_FOUND` | No | Stream or group not found. |
| 6 | `ALREADY_EXISTS` | No | Stream or group already exists. |
| 7 | `PERMISSION_DENIED` | No | Authorization failed. |
| 8 | `RESOURCE_EXHAUSTED` | Yes | Quota exceeded. |
| 9 | `FAILED_PRECONDITION` | No | Invalid state for operation. |
| 10 | `ABORTED` | Yes | Conflict (e.g., stale epoch). |
| 11 | `OUT_OF_RANGE` | No | Offset out of range. |
| 12 | `UNIMPLEMENTED` | No | Operation not supported. |
| 13 | `INTERNAL` | Yes | Internal error. |
| 14 | `UNAVAILABLE` | Yes | Service unavailable. |
| 15 | `DATA_LOSS` | No | Unrecoverable data loss. |
| 16 | `UNAUTHENTICATED` | No | Authentication failed. |

### 8.2 PEF-Specific Error Details

```protobuf
message KeiroxErrorDetail {
  uint32 pef_error_code = 1;
  string message = 2;
  uint64 retry_after_ms = 3;
  uint64 coordinator_epoch = 4;
  
  enum PefErrorCode {
    PEF_ERROR_UNSPECIFIED = 0;
    PEF_ERROR_STALE_EPOCH = 1;
    PEF_ERROR_STALE_LEASE = 2;
    PEF_ERROR_LEASE_NOT_ACTIVE = 3;
    PEF_ERROR_OFFSET_EVICTED = 4;
    PEF_ERROR_QUOTA_EXCEEDED = 5;
    PEF_ERROR_STREAM_NOT_FOUND = 6;
    PEF_ERROR_GROUP_NOT_FOUND = 7;
    PEF_ERROR_DUPLICATE_PRODUCE = 8;
    PEF_ERROR_TRANSACTION_CONFLICT = 9;
    PEF_ERROR_BACKPRESSURE = 10;
    PEF_ERROR_ENCRYPTION_REQUIRED = 11;
    PEF_ERROR_RESIDENCY_VIOLATION = 12;
  }
}
```

### 8.3 Kafka Protocol Error Mapping

| Kafka Error | PEF Condition |
|---|---|
| `UNKNOWN_TOPIC_OR_PARTITION` | Stream not found |
| `NOT_LEADER_FOR_PARTITION` | Coordinator epoch mismatch |
| `REQUEST_TIMED_OUT` | Deadline exceeded |
| `THROTTLING_QUOTA_EXCEEDED` | Quota exceeded |
| `INVALID_PRODUCER_EPOCH` | Producer epoch mismatch |
| `DUPLICATE_SEQUENCE_NUMBER` | Idempotent dedup hit (returns success) |

### 8.4 SQS Error Mapping

| SQS Error | PEF Condition |
|---|---|
| `QueueDoesNotExist` | Stream not found |
| `ReceiptHandleIsInvalid` | Stale lease token |
| `MessageNotInflight` | Lease not active |
| `RequestThrottled` | Quota exceeded |
| `InvalidParameterValue` | Invalid argument |

---

## 9. Idempotency Contracts

### 9.1 Producer Idempotency

```text
IdempotencyKey = (producer_id, producer_epoch, producer_seq)
```

**Normative rules:**

- Duplicate produces with the same idempotency key MUST return the original offset.
- The dedup window is bounded (default: 10,000 sequences or 10 minutes).
- Outside the dedup window, duplicates MAY be appended as new records.

### 9.2 Consumer Idempotency

```text
AckIdempotencyKey = (stream_id, group_id, offset)
```

**Normative rules:**

- Duplicate ACKs for already-ACKED offsets MUST return success.
- Duplicate NACKs for already-NACKED offsets MUST return success.
- Redrive operations MUST be idempotent.

### 9.3 Administrative Idempotency

```text
AdminIdempotencyKey = (tenant_id, operation_type, resource_id, request_id)
```

**Normative rules:**

- CreateStream MUST be idempotent if the stream already exists with identical configuration.
- DeleteStream MUST be idempotent if the stream is already deleted.
- Redrive MUST be idempotent for already-redriven offsets.

---

## 10. Authentication and Authorization Integration

### 10.1 Authentication Flow

```text
1. Client connects via TLS/mTLS
2. Client presents credentials:
   - Native SDK: OAuth2 token or mTLS certificate
   - Kafka gateway: SASL/SCRAM credentials
   - SQS gateway: AWS-style access key
   - AMQP gateway: username/password
3. Protocol Plane validates credentials
4. Principal Mapper resolves to PEF principal
5. ABAC PDP authorizes each operation
```

### 10.2 Authorization Enforcement Points

| Operation | Authorization Check |
|---|---|
| `Append` | `produce` permission on stream |
| `StreamFetch` | `consume` permission on stream |
| `LeaseNext` | `lease` permission on group |
| `Ack` / `Nack` | `ack` permission on group |
| `DlqRedrive` | `dlq_redrive` permission on group |
| `CreateStream` | `admin` permission on tenant |
| `DeleteStream` | `delete` permission on stream |

### 10.3 Normative Rules

- No operation MAY execute without successful authentication.
- Authorization failures MUST return `PERMISSION_DENIED`.
- Authentication failures MUST return `UNAUTHENTICATED`.
- Credentials MUST NOT be logged or included in error messages.

---

## 11. Backpressure and Throttling Signals

### 11.1 Native gRPC Throttling

When backpressure is engaged:

```protobuf
message ThrottleSignal {
  uint32 backpressure_stage = 1;
  uint64 retry_after_ms = 2;
  string reason = 3;
}
```

- Stage 1–2: No client-visible throttling.
- Stage 3: `RESOURCE_EXHAUSTED` with `retry_after_ms`.
- Stage 4: `UNAVAILABLE` for low-priority streams.
- Stage 5: `UNAVAILABLE` for all streams.

### 11.2 Kafka Protocol Throttling

- `Produce` requests return `THROTTLING_QUOTA_EXCEEDED`.
- `throttle_time_ms` field populated in response.
- Clients SHOULD respect `throttle_time_ms` before retrying.

### 11.3 SQS Protocol Throttling

- `SendMessage` returns HTTP 429 with `Retry-After` header.
- `ReceiveMessage` returns empty response (no error).

### 11.4 AMQP Protocol Throttling

- `Basic.Publish` returns `connection.blocked` frame.
- Clients MUST wait for `connection.unblocked` before resuming.

---

## 12. API Versioning and Compatibility

### 12.1 Version Negotiation

```protobuf
message NegotiateVersionRequest {
  uint32 client_major_version = 1;
  uint32 client_minor_version = 2;
  repeated uint32 supported_features = 3;
}

message NegotiateVersionResponse {
  uint32 server_major_version = 1;
  uint32 server_minor_version = 2;
  uint32 negotiated_version = 3;
  repeated uint32 enabled_features = 4;
  bool deprecated_client = 5;
}
```

### 12.2 Version Compatibility Rules

| Rule | Requirement |
|---|---|
| Major version | Breaking changes require new major version. |
| Minor version | Backward-compatible additions allowed. |
| Feature flags | New features gated behind negotiated feature bits. |
| Deprecation | Deprecated features MUST be supported for 2 major versions. |
| N/N-1 | Server MUST support current and previous major version. |

### 12.3 Feature Flags

| Feature ID | Name | Description |
|---:|---|---|
| 1 | `FEATURE_ACK_DURABLE` | ACK_DURABLE mode support |
| 2 | `FEATURE_TRANSACTIONS` | Transactional append support |
| 3 | `FEATURE_PUSHDOWN_QUERY` | SIMD pushdown query support |
| 4 | `FEATURE_DLQ_REDRIVE` | DLQ redrive support |
| 5 | `FEATURE_MULTI_REGION` | Multi-region replication awareness |

---

## 13. Protocol-Level Ordering Guarantees

### 13.1 Native API Ordering

| Operation | Ordering Guarantee |
|---|---|
| `Append` with same `entity_key` | Strict sequential ordering |
| `Append` with different `entity_key` | No ordering guarantee |
| `StreamFetch` | Sequential by offset |
| `LeaseNext` | No ordering guarantee (out-of-order) |

### 13.2 Kafka Gateway Ordering

```text
Kafka partition order → PEF entity_key order
```

Records within a Kafka partition MUST be delivered to PEF in order.

### 13.3 SQS Gateway Ordering

```text
SQS FIFO queue → PEF entity_key order (MessageGroupId)
SQS standard queue → No ordering guarantee
```

### 13.4 AMQP Gateway Ordering

```text
AMQP per-queue order → PEF entity_key order
```

---

## 14. NFR Traceability

| NFR | Requirement | How This Specification Satisfies It |
|---|---|---|
| PERF-032 | Arrow Flight CPU efficiency | Native Arrow IPC transfer (§4). |
| SEC-003 | Authentication | Protocol-level auth flow (§10). |
| SEC-004 | ABAC authorization | Operation-level authorization (§10.2). |
| DUR-003/004 | ACK durability modes | ACK_FAST / ACK_DURABLE exposed (§4.5). |
| OPS-003 | Quota enforcement | Protocol throttling signals (§11). |
| COMPAT | Published subset | Compatibility matrices (§5, §6, §7). |

---

## 15. Interfaces

### 15.1 Provided Interfaces

| Interface | Consumer | Semantics |
|---|---|---|
| Native gRPC endpoint | Native SDKs | Full API surface (§4). |
| Kafka RPC endpoint | Kafka producers/consumers | Subset per §5. |
| SQS REST endpoint | SQS clients | Subset per §6. |
| AMQP 0-9-1 endpoint | AMQP clients | Subset per §7. |

### 15.2 Consumed Interfaces

| Interface | Provider | Purpose |
|---|---|---|
| `append(batch)` | KEI-ARC-020 | Durable persistence. |
| `lease/ack/nack/renew` | KEI-ARC-021 | State transitions. |
| `authorize(principal, op)` | KEI-ARC-025 | ABAC decisions. |
| `checkQuota(tenant, resource)` | KEI-ARC-027 | Admission control. |

---

## 16. Open Questions

| Item | Status | Resolution Path |
|---|---|---|
| Kafka Fetch version upper bound validation | Open | Test matrix in KEI-DES-035. |
| SQS FIFO deduplication ID mapping | Open | Specify in KEI-DES-035. |
| AMQP topic exchange support decision | Open | ADR candidate; currently deferred. |
| Native SDK language release order | Open | Program planning. |
| Pushdown query predicate language | Open | Evaluate SQL-like vs. Arrow compute expressions. |

---

## 17. Glossary

| Term | Definition |
|---|---|
| Native API | The gRPC/Arrow Flight API surface for PEF. |
| Compatibility Matrix | The published subset of protocol operations a gateway supports. |
| Idempotency Key | A client-supplied key enabling duplicate detection. |
| Lease Token | A unique token identifying an active lease. |
| Backpressure Stage | The current level of system-wide throttling. |
| Feature Flag | A negotiated capability bit for version compatibility. |

---

## 18. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial API and protocol specification. Defines native gRPC/Arrow Flight API, Kafka/SQS/AMQP gateway mappings, error taxonomy, idempotency contracts, auth integration, backpressure signals, and versioning rules. Implements ADR-070/071. |