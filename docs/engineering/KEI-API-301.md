# KEI-API-301 — REST API & HTTP Gateway Plan

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-API-301 |
| Title | REST API & HTTP Gateway Plan |
| Version | 1.0 |
| Level | Engineering Execution Plan |
| Status | Baseline — Ready for Execution |
| Phase | Spans Phase 2–5 (incremental delivery) |
| Owner | API / Platform Engineering Lead |
| Governing Plan | KEI-ENG-500 — Phase 5 Productization & Distribution Plan |
| Governing Architecture Documents | KEI-ARC-024 (Protocol Gateways), KEI-DES-032 (API & Protocol), KEI-ARC-025 (Security), KEI-ARC-027 (Operability) |
| Related Plans | KEI-ENG-200, KEI-ENG-300, KEI-ENG-500, KEI-K8S-501, KEI-OPS-502 |
| Next Action | Update KEI-ENG-200, KEI-ENG-300, KEI-ENG-500 scope |

---

## 2. Executive Summary

The Keirox native API is gRPC/Arrow Flight (KEI-DES-032). This is the correct choice for high-performance data plane operations. However, the enterprise ecosystem operates overwhelmingly on HTTP/REST:

- **Kubernetes** uses HTTP for liveness, readiness, and startup probes.
- **Terraform** providers communicate via REST APIs.
- **Monitoring stacks** (Prometheus, Datadog, New Relic) scrape HTTP endpoints.
- **Enterprise API gateways** (Kong, Apigee, AWS API Gateway) route REST traffic.
- **Developers** debug with `curl`, Postman, and browser-based tools.
- **CI/CD pipelines** integrate via HTTP webhooks.
- **Web Consoles** consume REST APIs from frontend JavaScript.

Without an HTTP/REST layer, Keirox cannot integrate with any of these systems. This plan defines the REST API surface, the HTTP Gateway architecture, health endpoints, webhook notifications, OpenAPI specification, and the phased delivery strategy.

---

## 3. Purpose and Scope

### 3.1 Purpose

The purpose of this plan is to:

1. Define the complete REST API surface for Keirox.
2. Define the HTTP Gateway architecture (REST-to-gRPC transcoding).
3. Define health, readiness, and startup probe endpoints.
4. Define the Admin REST API for cluster, stream, group, DLQ, and schema management.
5. Define webhook notification endpoints.
6. Define OpenAPI specification generation.
7. Define REST API authentication, authorization, rate limiting, and error handling.
8. Define the phased delivery strategy across Phases 2–5.
9. Produce the REST API certification evidence package.

### 3.2 Scope

**In scope:**

- REST API surface design (all endpoints).
- HTTP Gateway / REST-to-gRPC transcoder.
- Health, readiness, and startup endpoints.
- Admin REST API (CRUD operations).
- Webhook notifications.
- OpenAPI 3.1 specification generation.
- REST API authentication (Bearer tokens, OAuth2, mTLS).
- REST API authorization (ABAC integration).
- REST API rate limiting.
- REST API error response format.
- REST API pagination.
- REST API versioning strategy.
- REST API certification tests.

**Out of scope:**

- Native gRPC/Arrow Flight data plane API (owned by KEI-DES-032).
- Kafka/SQS/AMQP wire protocol gateways (owned by KEI-COMPAT-301, KEI-QUEUE-401).
- Web Console UI (owned by KEI-OPS-502).
- CLI tooling (owned by KEI-ENG-500 WP-P5-B).

### 3.3 API Constraints

1. The REST API is a **control-plane and management-plane** interface. It is NOT the high-throughput data plane.
2. High-throughput data operations (append, fetch, lease, ACK) MUST use gRPC/Arrow Flight or the protocol gateways.
3. The REST API MUST NOT be used for streaming data.
4. All REST endpoints MUST be authenticated and authorized via ABAC.
5. All REST endpoints MUST produce audit events.
6. The REST API MUST support OpenAPI 3.1 specification generation.
7. The REST API MUST be versioned via URL path (`/api/v1/...`).

---

## 4. REST API Surface Design

### 4.1 API Surface Overview

```text
/api/v1/
├── health/                          # Health & Probes (Phase 2)
│   ├── GET  /healthz                # Liveness probe
│   ├── GET  /readyz                 # Readiness probe
│   └── GET  /startupz               # Startup probe
│
├── metrics/                         # Observability (Phase 2)
│   └── GET  /metrics                # Prometheus scrape endpoint
│
├── cluster/                         # Cluster Management (Phase 3)
│   ├── GET  /cluster                # Cluster status and health
│   ├── GET  /cluster/nodes          # Node list and roles
│   ├── POST /cluster/drain          # Drain a node
│   └── POST /cluster/replace-node   # Replace a failed node
│
├── streams/                         # Stream Management (Phase 3)
│   ├── GET  /streams                # List streams
│   ├── POST /streams                # Create stream
│   ├── GET  /streams/{stream_id}    # Stream detail
│   ├── DELETE /streams/{stream_id}  # Delete stream
│   ├── GET  /streams/{stream_id}/offsets  # Offset info
│   └── GET  /streams/{stream_id}/consumers  # Active consumers
│
├── groups/                          # Consumer Group Management (Phase 3)
│   ├── GET  /groups                 # List consumer groups
│   ├── POST /groups                 # Create consumer group
│   ├── GET  /groups/{group_id}      # Group detail
│   ├── DELETE /groups/{group_id}    # Delete consumer group
│   ├── GET  /groups/{group_id}/offsets  # Committed offsets
│   └── POST /groups/{group_id}/offsets/commit  # Commit offsets
│
├── dlq/                             # Dead Letter Queue (Phase 3)
│   ├── GET  /dlq                    # List DLQ entries
│   ├── GET  /dlq/{entry_id}         # DLQ entry detail
│   ├── POST /dlq/{entry_id}/redrive # Redrive entry
│   ├── POST /dlq/redrive-batch      # Redrive batch
│   └── DELETE /dlq/{entry_id}       # Purge entry (admin only)
│
├── schemas/                         # Schema Registry (Phase 3)
│   ├── GET  /schemas                # List schemas
│   ├── POST /schemas                # Register schema
│   ├── GET  /schemas/{schema_id}    # Schema detail
│   ├── GET  /schemas/{schema_id}/versions  # Version history
│   └── POST /schemas/{schema_id}/evolve  # Evolve schema
│
├── lakehouse/                       # Lakehouse Management (Phase 4)
│   ├── GET  /lakehouse/tables       # List Iceberg tables
│   ├── GET  /lakehouse/tables/{table_id}  # Table detail
│   ├── GET  /lakehouse/tables/{table_id}/snapshots  # Snapshot history
│   ├── GET  /lakehouse/tables/{table_id}/freshness  # Freshness status
│   └── POST /lakehouse/tables/{table_id}/expire-snapshots  # Expire snapshots
│
├── security/                        # Security & Compliance (Phase 4)
│   ├── GET  /security/events        # Security event log
│   ├── GET  /security/audit         # Audit trail
│   ├── POST /security/erasure       # Initiate crypto-shredding
│   ├── GET  /security/erasure/{ticket_id}  # Erasure status
│   ├── GET  /security/legal-holds   # List legal holds
│   └── POST /security/legal-holds   # Create legal hold
│
├── admin/                           # Admin Operations (Phase 4)
│   ├── POST /admin/backup           # Trigger backup
│   ├── POST /admin/restore          # Trigger restore
│   ├── POST /admin/pitr             # Point-in-time recovery
│   ├── POST /admin/failover         # Region failover
│   ├── GET  /admin/runbooks         # List available runbooks
│   └── POST /admin/break-glass      # Break-glass access
│
├── webhooks/                        # Webhook Management (Phase 4)
│   ├── GET  /webhooks               # List webhooks
│   ├── POST /webhooks               # Create webhook
│   ├── GET  /webhooks/{webhook_id}  # Webhook detail
│   ├── PUT  /webhooks/{webhook_id}  # Update webhook
│   └── DELETE /webhooks/{webhook_id}  # Delete webhook
│
└── system/                          # System & Diagnostics (Phase 5)
    ├── GET  /system/version         # Version info
    ├── GET  /system/config          # Sanitized configuration
    ├── GET  /system/flags           # Feature flags
    ├── POST /system/flags/{flag}    # Toggle feature flag
    └── GET  /system/pprof           # Performance profiling (admin only)
```

### 4.2 REST API vs gRPC Responsibility Matrix

| Operation Category | REST API | gRPC/Arrow Flight | Protocol Gateways |
|---|---|---|---|
| Health & probes | ✅ Primary | ❌ | ❌ |
| Metrics scrape | ✅ Primary | ❌ | ❌ |
| Cluster management | ✅ Primary | ✅ Admin API | ❌ |
| Stream CRUD | ✅ Primary | ✅ Admin API | ❌ |
| Consumer group CRUD | ✅ Primary | ✅ Admin API | ❌ |
| DLQ management | ✅ Primary | ✅ Admin API | ❌ |
| Schema registry CRUD | ✅ Primary | ✅ Admin API | ❌ |
| Lakehouse management | ✅ Primary | ✅ Admin API | ❌ |
| Security & erasure | ✅ Primary | ✅ Admin API | ❌ |
| Backup/restore/PITR | ✅ Primary | ✅ Admin API | ❌ |
| Webhook management | ✅ Primary | ❌ | ❌ |
| **High-throughput append** | ❌ | ✅ Primary | ✅ Kafka/SQS/AMQP |
| **Stream fetch** | ❌ | ✅ Primary | ✅ Kafka Fetch |
| **Lease/ACK/NACK** | ❌ | ✅ Primary | ✅ SQS/AMQP |
| **Arrow Flight query** | ❌ | ✅ Primary | ❌ |

**Normative rule:** The REST API MUST NOT be used for data-plane operations exceeding 1,000 requests/second. High-throughput workloads MUST use gRPC, Arrow Flight, or protocol gateways.

---

## 5. HTTP Gateway Architecture

### 5.1 Gateway Topology

```text
┌────────────────────────────────────────────────────────────────────────────┐
│                        HTTP / REST GATEWAY                                 │
│                                                                            │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │                    INGRESS LAYER                                     │  │
│  │                                                                      │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │  │
│  │  │ TLS          │  │ HTTP/1.1 +   │  │ Rate Limiter │              │  │
│  │  │ Termination  │  │ HTTP/2       │  │ (per tenant) │              │  │
│  │  │ (mTLS opt.)  │  │ Handler      │  │              │              │  │
│  │  └──────────────┘  └──────────────┘  └──────────────┘              │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│         │                                                                  │
│         ▼                                                                  │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │                    AUTH LAYER                                        │  │
│  │                                                                      │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │  │
│  │  │ Bearer Token │  │ OAuth2 /     │  │ ABAC PEP     │              │  │
│  │  │ Validation   │  │ OIDC         │  │ (Policy      │              │  │
│  │  │              │  │              │  │  Enforcement)│              │  │
│  │  └──────────────┘  └──────────────┘  └──────────────┘              │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│         │                                                                  │
│         ▼                                                                  │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │                    TRANSCODING LAYER                                  │  │
│  │                                                                      │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │  │
│  │  │ REST Router  │  │ JSON ↔       │  │ HTTP Status  │              │  │
│  │  │ (path +      │  │ Protobuf     │  │ ↔ gRPC       │              │  │
│  │  │  method)     │  │ Transcoder   │  │ Status Map   │              │  │
│  │  └──────────────┘  └──────────────┘  └──────────────┘              │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│         │                                                                  │
│         ▼                                                                  │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │                    AUDIT & RESPONSE LAYER                             │  │
│  │                                                                      │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │  │
│  │  │ Audit Event  │  │ Response     │  │ OpenAPI      │              │  │
│  │  │ Emitter      │  │ Serializer   │  │ Introspection│              │  │
│  │  └──────────────┘  └──────────────┘  └──────────────┘              │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│         │                                                                  │
│         ▼                                                                  │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │                    INTERNAL gRPC SERVICES                             │  │
│  │  Admin API · State Plane · Storage · Schema Registry · Security     │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────────────────┘
```

### 5.2 Implementation Strategy

| Component | Technology | Rationale |
|---|---|---|
| HTTP server | Rust `axum` or Go `net/http` | High performance; native TLS support |
| REST-to-gRPC transcoding | `tonic` (Rust) or `grpc-gateway` (Go) | Mature protobuf-to-JSON mapping |
| OpenAPI generation | `utoipa` (Rust) or `protoc-gen-openapi` | Auto-generated from protobuf definitions |
| Rate limiting | Token bucket per tenant | Prevents REST API abuse |
| Authentication | Bearer token + OAuth2/OIDC | Enterprise SSO integration |

### 5.3 Deployment Model

The REST Gateway can be deployed in three modes:

| Mode | Description | Use Case |
|---|---|---|
| **Sidecar** | REST gateway runs as a sidecar container alongside each Keirox server | Low latency; per-node REST access |
| **Standalone** | REST gateway runs as a separate deployment with load balancer | Centralized management; shared access |
| **Embedded** | REST endpoints embedded directly in the Keirox server process | Development; minimal deployment |

**Default for production:** Standalone mode with Kubernetes Service and load balancer.
**Default for development:** Embedded mode.

---

## 6. Health, Readiness & Startup Probes

### 6.1 Probe Endpoints

| Endpoint | Purpose | Response |
|---|---|---|
| `GET /api/v1/health/healthz` | Liveness probe — is the process running? | `200 OK` if process is alive |
| `GET /api/v1/health/readyz` | Readiness probe — can the node serve traffic? | `200 OK` if Raft quorum is healthy, WAL is writable, S3 is reachable |
| `GET /api/v1/health/startupz` | Startup probe — has initialization completed? | `200 OK` after initial state restoration |

### 6.2 Readiness Response Schema

```json
{
  "status": "ready",
  "checks": {
    "raft_quorum": {
      "status": "healthy",
      "leader_id": "node-1",
      "term": 42,
      "members_healthy": 3
    },
    "wal_writable": {
      "status": "healthy",
      "nvme_usage_percent": 34.2
    },
    "s3_reachable": {
      "status": "healthy",
      "upload_backlog_bytes": 0
    },
    "state_plane": {
      "status": "healthy",
      "active_leases": 1523,
      "watermark_lag": 0
    }
  },
  "version": "1.0.0",
  "node_id": "node-2",
  "uptime_seconds": 86400
}
```

### 6.3 Kubernetes Probe Configuration

```yaml
livenessProbe:
  httpGet:
    path: /api/v1/health/healthz
    port: 8080
  initialDelaySeconds: 10
  periodSeconds: 15
  failureThreshold: 3

readinessProbe:
  httpGet:
    path: /api/v1/health/readyz
    port: 8080
  initialDelaySeconds: 30
  periodSeconds: 10
  failureThreshold: 3

startupProbe:
  httpGet:
    path: /api/v1/health/startupz
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 5
  failureThreshold: 30
```

---

## 7. Authentication & Authorization for REST API

### 7.1 Authentication Methods

| Method | Header | Use Case |
|---|---|---|
| Bearer Token | `Authorization: Bearer <token>` | API keys, service tokens |
| OAuth2 / OIDC | `Authorization: Bearer <jwt>` | Enterprise SSO (Okta, Azure AD, Auth0) |
| mTLS | Client certificate | High-assurance internal communication |
| API Key | `X-API-Key: <key>` | Simple integrations (development only) |

### 7.2 Authorization Model

All REST endpoints MUST be authorized via the ABAC policy engine (KEI-ARC-025).

| REST Endpoint | Required Permission |
|---|---|
| `GET /api/v1/streams` | `stream:read` |
| `POST /api/v1/streams` | `stream:create` |
| `DELETE /api/v1/streams/{id}` | `stream:delete` |
| `POST /api/v1/dlq/{id}/redrive` | `dlq:redrive` |
| `POST /api/v1/admin/backup` | `admin:backup` |
| `POST /api/v1/admin/failover` | `admin:failover` |
| `POST /api/v1/security/erasure` | `security:erasure` + `compliance:approve` |
| `POST /api/v1/admin/break-glass` | `admin:break-glass` + two-person approval |

### 7.3 Rate Limiting

| Tier | Limit | Scope |
|---|---|---|
| Default | 100 requests/minute | Per tenant |
| Elevated | 1,000 requests/minute | Per tenant (with API key) |
| Admin | 10 requests/minute | Per admin operation |
| Break-glass | 1 request/hour | Per break-glass operation |

Rate limit responses MUST include:

```json
{
  "error": {
    "code": "RATE_LIMITED",
    "message": "Rate limit exceeded. Retry after 30 seconds.",
    "retry_after_seconds": 30
  }
}
```

HTTP headers:

```text
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1693500000
Retry-After: 30
```

---

## 8. Error Response Format

### 8.1 Error Schema

All REST API errors MUST follow this format:

```json
{
  "error": {
    "code": "STREAM_NOT_FOUND",
    "message": "Stream 'orders' not found in tenant 'acme'.",
    "request_id": "req-abc-123",
    "timestamp": "2026-08-30T12:00:00Z",
    "details": {
      "stream_id": "orders",
      "tenant_id": "acme"
    },
    "doc_url": "https://docs.keirox.io/errors/STREAM_NOT_FOUND"
  }
}
```

### 8.2 HTTP Status Code Mapping

| gRPC Status | HTTP Status | Keirox Error Code |
|---|---|---|
| `OK` | `200 OK` | — |
| `NOT_FOUND` | `404 Not Found` | `STREAM_NOT_FOUND`, `GROUP_NOT_FOUND` |
| `ALREADY_EXISTS` | `409 Conflict` | `STREAM_ALREADY_EXISTS` |
| `INVALID_ARGUMENT` | `400 Bad Request` | `INVALID_REQUEST` |
| `PERMISSION_DENIED` | `403 Forbidden` | `PERMISSION_DENIED` |
| `UNAUTHENTICATED` | `401 Unauthorized` | `UNAUTHENTICATED` |
| `RESOURCE_EXHAUSTED` | `429 Too Many Requests` | `RATE_LIMITED`, `QUOTA_EXCEEDED` |
| `FAILED_PRECONDITION` | `412 Precondition Failed` | `LEGAL_HOLD_ACTIVE` |
| `UNAVAILABLE` | `503 Service Unavailable` | `SERVICE_UNAVAILABLE` |
| `INTERNAL` | `500 Internal Server Error` | `INTERNAL_ERROR` |
| `DEADLINE_EXCEEDED` | `504 Gateway Timeout` | `TIMEOUT` |
| `UNIMPLEMENTED` | `501 Not Implemented` | `UNSUPPORTED_OPERATION` |

---

## 9. Pagination

### 9.1 Pagination Schema

All list endpoints MUST support cursor-based pagination:

```text
GET /api/v1/streams?page_size=50&page_token=eyJvZmZzZXQiOjEwMH0
```

Response:

```json
{
  "items": [...],
  "page_size": 50,
  "next_page_token": "eyJvZmZzZXQiOjE1MH0",
  "total_count": 234
}
```

### 9.2 Pagination Rules

| Rule | Requirement |
|---|---|
| Default page size | 50 |
| Maximum page size | 500 |
| Pagination style | Cursor-based (opaque token) |
| Total count | Included in response |
| Empty result | Return `200 OK` with empty `items` array |

---

## 10. Webhook Notifications

### 10.1 Webhook Events

| Event | Trigger | Payload |
|---|---|---|
| `stream.created` | Stream created | Stream metadata |
| `stream.deleted` | Stream deleted | Stream ID, tenant ID |
| `dlq.entry_created` | Message evicted to DLQ | DLQ entry metadata |
| `dlq.entry_redriven` | DLQ entry redriven | DLQ entry metadata |
| `iceberg.commit.completed` | Iceberg snapshot committed | Table ID, snapshot ID, file count |
| `iceberg.freshness.breached` | Freshness target exceeded | Table ID, current age, target |
| `security.erasure.completed` | Crypto-shredding completed | Erasure ticket ID, key IDs |
| `security.auth_failure` | Authentication failure | Principal, source IP, reason |
| `cluster.node_failure` | Node failure detected | Node ID, failure reason |
| `cluster.failover.completed` | Region failover completed | Old primary, new primary, epoch |
| `backup.completed` | Backup completed | Backup ID, size, duration |
| `backup.failed` | Backup failed | Backup ID, error |

### 10.2 Webhook Delivery

| Property | Specification |
|---|---|
| Delivery method | HTTP POST to configured URL |
| Content type | `application/json` |
| Authentication | HMAC-SHA256 signature in `X-Keirox-Signature` header |
| Retry policy | 3 retries with exponential backoff (1s, 5s, 30s) |
| Timeout | 10 seconds per delivery attempt |
| Dead letter | Failed webhooks after 3 retries are logged to internal DLQ |
| Ordering | Best-effort; no ordering guarantee |

### 10.3 Webhook Signature Verification

```text
X-Keirox-Signature: sha256=<hex(hmac_sha256(secret, payload))>
X-Keirox-Timestamp: 1693500000
X-Keirox-Event: dlq.entry_created
X-Keirox-Delivery-ID: del-abc-123
```

Customers MUST verify the HMAC signature before processing webhook payloads.

---

## 11. OpenAPI Specification

### 11.1 Generation Strategy

The OpenAPI 3.1 specification MUST be auto-generated from protobuf definitions:

```text
1. Define gRPC services and messages in .proto files
2. Run protoc-gen-openapi to generate OpenAPI 3.1 YAML
3. Validate generated spec with spectral linter
4. Publish spec to /api/v1/openapi.yaml
5. Generate interactive docs (Swagger UI / Redoc)
6. Include spec in release artifacts
```

### 11.2 OpenAPI Deliverables

| Deliverable | Format | Purpose |
|---|---|---|
| `openapi.yaml` | OpenAPI 3.1 YAML | Machine-readable API specification |
| Swagger UI | Interactive HTML | API exploration and testing |
| Redoc | Static HTML | API documentation |
| Postman collection | JSON | Importable into Postman |
| curl examples | Markdown | Quick reference for developers |

### 11.3 OpenAPI Publication

| Location | Access |
|---|---|
| `GET /api/v1/openapi.yaml` | Public (no auth required) |
| `https://docs.keirox.io/api/v1/` | Public documentation site |
| Release artifacts | Included in every release bundle |
| Terraform provider | Used for resource schema generation |

---

## 12. API Versioning Strategy

### 12.1 Versioning Rules

| Rule | Requirement |
|---|---|
| URL path versioning | `/api/v1/...`, `/api/v2/...` |
| Breaking changes | Require new major version |
| Additive changes | Allowed within existing version |
| Deprecation notice | 6 months before removal |
| Sunset header | `Sunset: <date>` on deprecated endpoints |

### 12.2 Version Compatibility

| Version | Status | Support Window |
|---|---|---|
| `/api/v1/` | Active | Full support |
| `/api/v2/` | Future | When breaking changes are needed |

---

## 13. Phased Delivery Strategy

### 13.1 Phase 2: Health & Metrics (Minimal)

| Deliverable | Description |
|---|---|
| Health endpoints | `/healthz`, `/readyz`, `/startupz` |
| Metrics endpoint | `/metrics` (Prometheus scrape) |
| Basic CLI | `keirox cluster status`, `keirox stream list` |

**Rationale:** Kubernetes probes and Prometheus scraping are required before any K8s deployment. The basic CLI enables engineers to debug the Phase 2 distributed consensus work.

### 13.2 Phase 3: Admin REST API & Gateway

| Deliverable | Description |
|---|---|
| REST Gateway deployment | Standalone or embedded mode |
| Stream CRUD API | Create, read, delete streams |
| Consumer group CRUD API | Create, read, delete groups |
| DLQ management API | List, inspect, redrive DLQ entries |
| Schema registry API | Register, resolve, evolve schemas |
| Authentication & authorization | Bearer token, OAuth2, ABAC integration |
| Rate limiting | Per-tenant rate limits |
| Error response format | Standardized error schema |
| Pagination | Cursor-based pagination |
| OpenAPI generation | Auto-generated from protobuf |

**Rationale:** The Terraform provider, Web Console, and enterprise tooling depend on a stable Admin REST API.

### 13.3 Phase 4: Security, Lakehouse & Webhooks

| Deliverable | Description |
|---|---|
| Security API | Erasure, legal holds, audit events |
| Lakehouse API | Table management, freshness, snapshot operations |
| Admin operations API | Backup, restore, PITR, failover |
| Webhook notifications | Event-driven HTTP callbacks |
| Webhook signature verification | HMAC-SHA256 signing |

**Rationale:** Security and compliance operations require REST endpoints for integration with enterprise security tools (SIEM, SOAR, compliance platforms).

### 13.4 Phase 5: Full API, CLI & Console Integration

| Deliverable | Description |
|---|---|
| Full CLI | All commands: cluster, stream, group, dlq, schema, migration, admin |
| Web Console integration | Console consumes REST API exclusively |
| OpenAPI publication | Public API documentation site |
| Postman collection | Published for developer onboarding |
| API SDK generation | Auto-generated client SDKs from OpenAPI spec |
| System diagnostics API | Version, config, flags, profiling |

**Rationale:** The full developer experience (CLI + Console + API docs) is the final adoption layer.

---

## 14. REST API Certification Tests

### 14.1 Functional Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| REST-T-001 | `GET /healthz` on healthy node | Returns `200 OK` |
| REST-T-002 | `GET /readyz` on unhealthy node | Returns `503 Service Unavailable` |
| REST-T-003 | `POST /streams` with valid payload | Stream created; `201 Created` |
| REST-T-004 | `POST /streams` with duplicate name | `409 Conflict` |
| REST-T-005 | `GET /streams/{id}` for nonexistent stream | `404 Not Found` |
| REST-T-006 | `DELETE /streams/{id}` without permission | `403 Forbidden` |
| REST-T-007 | `POST /dlq/{id}/redrive` | DLQ entry redriven; audit event logged |
| REST-T-008 | `POST /admin/erasure` without approval | `403 Forbidden` |
| REST-T-009 | `GET /streams` with pagination | Correct page returned; next_page_token valid |
| REST-T-010 | Request without auth token | `401 Unauthorized` |
| REST-T-011 | Request with expired token | `401 Unauthorized` |
| REST-T-012 | Request exceeding rate limit | `429 Too Many Requests` with `Retry-After` |

### 14.2 Webhook Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| WEB-T-001 | Create webhook | Webhook registered; confirmation sent |
| WEB-T-002 | Event triggers webhook | HTTP POST delivered with correct payload |
| WEB-T-003 | Webhook signature verification | HMAC signature valid |
| WEB-T-004 | Webhook delivery failure | Retry with backoff; dead-lettered after 3 failures |
| WEB-T-005 | Delete webhook | Webhook removed; no further deliveries |

### 14.3 OpenAPI Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| OAPI-T-001 | OpenAPI spec generated | Valid OpenAPI 3.1 YAML |
| OAPI-T-002 | Spec passes spectral lint | No errors |
| OAPI-T-003 | Swagger UI renders | All endpoints visible |
| OAPI-T-004 | Postman collection import | All endpoints importable |
| OAPI-T-005 | Spec matches actual API | No drift between spec and implementation |

---

## 15. Certification Levels

| Level | Name | Requirement |
|---|---|---|
| L1 | Health Certified | Health, readiness, and startup probes work correctly |
| L2 | Metrics Certified | Prometheus scrape endpoint works |
| L3 | Admin API Certified | Stream, group, DLQ, schema CRUD operations pass |
| L4 | Security Certified | Auth, ABAC, rate limiting, error format validated |
| L5 | Webhook Certified | Webhook delivery, signing, and retry validated |
| L6 | OpenAPI Certified | Spec generated, linted, published, and matches implementation |
| L7 | CLI Certified | Full CLI passes all integration tests |
| L8 | Console Certified | Web Console consumes REST API exclusively |

Phase 5 exit requires **L1 through L8**.

---

## 16. Deliverables and Milestones

| Deliverable | Description | Target Phase | Target Week |
|---|---|---|---:|
| D-API-001 | Health & readiness endpoints | Phase 2 | Week 4 |
| D-API-002 | Prometheus metrics endpoint | Phase 2 | Week 4 |
| D-API-003 | Basic CLI (cluster status, stream list) | Phase 2 | Week 6 |
| D-API-004 | REST Gateway deployment | Phase 3 | Week 10 |
| D-API-005 | Stream CRUD API | Phase 3 | Week 12 |
| D-API-006 | Consumer group CRUD API | Phase 3 | Week 14 |
| D-API-007 | DLQ management API | Phase 3 | Week 16 |
| D-API-008 | Schema registry API | Phase 3 | Week 18 |
| D-API-009 | Auth & rate limiting | Phase 3 | Week 18 |
| D-API-010 | OpenAPI generation pipeline | Phase 3 | Week 20 |
| D-API-011 | Security & erasure API | Phase 4 | Week 24 |
| D-API-012 | Lakehouse management API | Phase 4 | Week 26 |
| D-API-013 | Admin operations API | Phase 4 | Week 28 |
| D-API-014 | Webhook notifications | Phase 4 | Week 30 |
| D-API-015 | Full CLI | Phase 5 | Week 14 |
| D-API-016 | Web Console REST integration | Phase 5 | Week 18 |
| D-API-017 | OpenAPI publication & docs | Phase 5 | Week 20 |
| D-API-018 | Postman collection | Phase 5 | Week 20 |
| D-API-019 | REST API certification test suite | Phase 5 | Week 22 |
| D-API-020 | Final REST API evidence package | Phase 5 | Week 22 |

---

## 17. Certification Gates

### 17.1 Gate API-A: Health & Probes Certified (Phase 2, Week 6)

| Criterion | Mandatory |
|---|---|
| Health, readiness, startup probes work | Yes |
| Prometheus metrics endpoint works | Yes |
| Basic CLI operational | Yes |
| Kubernetes probe configuration validated | Yes |

### 17.2 Gate API-B: Admin API Certified (Phase 3, Week 20)

| Criterion | Mandatory |
|---|---|
| Stream, group, DLQ, schema CRUD pass | Yes |
| Authentication and ABAC enforced | Yes |
| Rate limiting works | Yes |
| Error response format correct | Yes |
| Pagination works | Yes |
| OpenAPI spec generated and valid | Yes |

### 17.3 Gate API-C: Full API Certified (Phase 5, Week 22)

| Criterion | Mandatory |
|---|---|
| All L1–L8 certification levels pass | Yes |
| Webhook delivery and signing validated | Yes |
| Full CLI passes all tests | Yes |
| Web Console consumes REST API exclusively | Yes |
| OpenAPI published and matches implementation | Yes |
| Evidence package complete | Yes |

---

## 18. Risks and Mitigations

| Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|
| REST API becomes data-plane bottleneck | High | Low | Enforce 1,000 req/s limit; document that data plane uses gRPC |
| OpenAPI spec drifts from implementation | Medium | Medium | Auto-generate spec in CI; fail build on drift |
| Webhook delivery failures cause event loss | Medium | Medium | Dead-letter queue for failed webhooks; retry with backoff |
| REST API surface scope creep | High | Medium | Strict API review board; versioning discipline |
| Rate limiting too aggressive for enterprise tools | Medium | Medium | Configurable per-tenant limits; elevated tier with API key |
| REST gateway single point of failure | Medium | Low | Deploy as multi-replica with load balancer |

---

## 19. Evidence Package

The REST API evidence package MUST include:

1. REST API surface documentation.
2. HTTP Gateway architecture documentation.
3. Health probe test results.
4. Admin API conformance test results.
5. Authentication and authorization test results.
6. Rate limiting test results.
7. Error response format validation.
8. Pagination test results.
9. Webhook delivery and signing test results.
10. OpenAPI specification and validation report.
11. CLI integration test results.
12. Web Console REST integration test results.
13. Customer-facing API documentation.

---

## 20. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial REST API & HTTP Gateway Plan. Defines complete REST API surface, HTTP Gateway architecture, health/readiness probes, Admin REST API, webhook notifications, OpenAPI specification, authentication/authorization, rate limiting, error format, pagination, versioning, phased delivery strategy (Phase 2–5), certification levels, and evidence requirements. |

---

## Phase Updates Required

This document requires the following phase plan updates:

### Update 1: KEI-ENG-200 (Phase 2)

Add to Phase 2 scope:

| Addition | Description |
|---|---|
| Health & readiness endpoints | `/healthz`, `/readyz`, `/startupz` |
| Prometheus metrics endpoint | `/metrics` for K8s probes and monitoring |
| Basic CLI | `keirox cluster status`, `keirox stream list`, `keirox dlq list` |

### Update 2: KEI-ENG-300 (Phase 3)

Add to Phase 3 scope:

| Addition | Description |
|---|---|
| REST Gateway deployment | Standalone or embedded HTTP gateway |
| Admin REST API | Stream, group, DLQ, schema CRUD |
| OpenAPI generation | Auto-generated from protobuf |
| REST API authentication | Bearer token, OAuth2, ABAC integration |

### Update 3: KEI-ENG-500 (Phase 5)

Add to Phase 5 scope:

| Addition | Description |
|---|---|
| Full CLI | All command groups |
| Webhook notifications | Event-driven HTTP callbacks |
| OpenAPI publication | Public API documentation |
| Postman collection | Developer onboarding |
| REST API certification | L1–L8 certification levels |