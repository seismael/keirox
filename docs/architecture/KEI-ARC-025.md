# KEI-ARC-025 — Security, Privacy & Compliance Architecture

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-ARC-025 |
| Title | Security, Privacy & Compliance Architecture |
| Version | 1.0 |
| Level | **L2 — Subsystem Architecture** |
| Pillars Covered | Pillar 6 (Enterprise Compliance) |
| Status | Approved for Engineering |
| Classification | Internal / Engineering Confidential |
| Owner | Security Architect / Security Lead |
| Required Reviewers | Chief Architect, Principal Engineer (Distributed Systems), Compliance Lead, SRE Lead |
| Depends On | KEI-ARC-010 (Conceptual Architecture), KEI-ARC-011 (NFRs), KEI-ARC-012 (ADRs), KEI-ARC-024 (Protocol Plane) |
| Feeds | KEI-ARC-020 (Storage Engine), KEI-ARC-021 (State Plane), KEI-ARC-022 (Consensus), KEI-ARC-023 (Lakehouse), KEI-ARC-026 (Multi-Region/DR), KEI-DES-036 (Encryption, Key Management & Crypto-Shredding Specification) |
| Keywords | MUST, MUST NOT, SHOULD, SHOULD NOT, MAY per RFC 2119 |

---

## 2. Purpose, Scope, and Audience

### 2.1 Purpose

This document specifies the **Security, Privacy & Compliance subsystem** of the Polymorphic Event Fabric. It defines how the fabric authenticates principals, authorizes operations, encrypts data in transit and at rest, enforces tenant isolation, supports regulatory erasure, and produces tamper-evident audit evidence.

It elaborates **Pillar 6 (Enterprise Compliance)** and is the normative security baseline for every other subsystem.

### 2.2 Scope

**In scope:**

- Authentication and identity mapping.
- ABAC authorization.
- Envelope encryption and KMS integration.
- Crypto-shredding for GDPR/CCPA-style erasure.
- Tenant isolation.
- Audit logging.
- Secret and credential management.
- Data residency and retention governance.
- Security telemetry and incident hooks.

**Out of scope:**

- Physical storage mechanics — owned by KEI-ARC-020.
- Consumption-state semantics — owned by KEI-ARC-021.
- Consensus internals — owned by KEI-ARC-022.
- Lakehouse commit mechanics — owned by KEI-ARC-023.
- Protocol wire formats — owned by KEI-ARC-024 and KEI-DES-032/035.
- Exact KMS adapter implementation — owned by KEI-DES-036.

### 2.3 Position in the Architecture

```
                 ┌─────────────────────────────────────────────────────┐
                 │          IDENTITY PROVIDERS / PKI / KMS             │
                 │     OIDC / OAuth2 / mTLS CA / AWS KMS / Vault       │
                 └───────────────────────┬─────────────────────────────┘
                                         │
                                         ▼
┌────────────────────────────────────────────────────────────────────────┐
│                SECURITY, PRIVACY & COMPLIANCE PLANE                   │
│                                                                        │
│  Authentication ──► Principal Mapping ──► ABAC Policy Decision        │
│                                                                        │
│  Envelope Encryption ──► KMS Adapter ──► Crypto-Shredding Orchestrator│
│                                                                        │
│  Audit Trail ──► Retention / Residency Governance ──► Security Telemetry │
└───────┬───────────────────┬───────────────────┬───────────────────────┘
        │                   │                   │
        ▼                   ▼                   ▼
┌──────────────┐    ┌──────────────┐    ┌──────────────────────┐
│ Protocol     │    │ Storage      │    │ Control Plane /      │
│ Gateways     │    │ Engine       │    │ Admin Plane          │
│ KEI-ARC-024  │    │ KEI-ARC-020  │    │                      │
└──────────────┘    └──────────────┘    └──────────────────────┘
```

**Normative boundary:** Security is not a bypassable sidecar. Every external request and every privileged internal operation MUST pass authentication, authorization, and audit hooks defined by this subsystem.

---

## 3. Subsystem Responsibilities and Non-Responsibilities

### 3.1 Responsibilities

| ID | Responsibility |
|---|---|
| R1 | Authenticate clients, services, and administrators. |
| R2 | Map protocol-specific identities to PEF principals. |
| R3 | Evaluate ABAC policies for all operations. |
| R4 | Provide envelope encryption for data at rest. |
| R5 | Integrate with external KMS/HSM providers. |
| R6 | Execute cryptographic erasure for privacy requests. |
| R7 | Enforce tenant isolation boundaries. |
| R8 | Emit tamper-evident audit records. |
| R9 | Manage secrets and credentials securely. |
| R10 | Enforce data residency and retention policies. |
| R11 | Expose security telemetry for operations and incident response. |

### 3.2 Non-Responsibilities

| ID | Non-Responsibility | Owned By |
|---|---|---|
| N1 | WAL persistence and tiering | KEI-ARC-020 |
| N2 | Lease/ACK state transitions | KEI-ARC-021 |
| N3 | Consensus and failover | KEI-ARC-022 |
| N4 | Iceberg commit semantics | KEI-ARC-023 |
| N5 | Gateway protocol translation | KEI-ARC-024 |

---

## 4. Internal Component Decomposition

| Component | Responsibility |
|---|---|
| **K1. Authentication Service** | Validates TLS/mTLS, SASL/SCRAM, OAuth2/OIDC tokens, and service identities. |
| **K2. Principal Mapper** | Maps gateway and protocol identities to canonical PEF principals. |
| **K3. ABAC Policy Decision Point (PDP)** | Evaluates attribute-based policies and returns allow/deny decisions. |
| **K4. Policy Enforcement Points (PEPs)** | Enforce PDP decisions at gateways, storage, state plane, and admin APIs. |
| **K5. Key Management Adapter** | Abstracts AWS KMS, GCP KMS, Azure Key Vault, HashiCorp Vault, and HSMs. |
| **K6. Envelope Encryption Engine** | Creates, caches, wraps, unwraps, and destroys DEKs. |
| **K7. Crypto-Shredding Orchestrator** | Executes erasure workflows and records proof. |
| **K8. Audit Trail Service** | Records security-relevant events into a tamper-evident log. |
| **K9. Retention & Residency Governance** | Enforces retention, legal hold, and region constraints. |
| **K10. Security Telemetry & Incident Hooks** | Exposes auth failures, KMS errors, abnormal access, and erasure events. |

---

## 5. Security Design Principles

| ID | Principle | Normative Effect |
|---|---|---|
| SP-1 | **Default deny.** | Any request without an explicit allow policy MUST be denied. |
| SP-2 | **Least privilege.** | Principals MUST receive only the minimum operations required. |
| SP-3 | **Tenant isolation by construction.** | Tenant boundaries MUST be enforced by namespace, key hierarchy, and policy. |
| SP-4 | **Fail secure.** | Security failures MUST deny access rather than fall back to plaintext or open access. |
| SP-5 | **No secrets in artifacts.** | Secrets MUST NOT appear in logs, manifests, error messages, or snapshots. |
| SP-6 | **Auditability.** | Security-relevant operations MUST produce tamper-evident audit evidence. |
| SP-7 | **Cryptographic erasure is logical deletion.** | Destroying keys renders ciphertext unrecoverable, while physical purge may occur asynchronously. |
| SP-8 | **Evidence over certification claims.** | The architecture supports compliance readiness; certification depends on external audit. |

---

## 6. Authentication Architecture

### 6.1 Supported Authentication Mechanisms

| Mechanism | Use Case |
|---|---|
| TLS 1.3 | Mandatory encryption in transit for all external and internal traffic. |
| mTLS | Service-to-service authentication and high-assurance client authentication. |
| SASL/SCRAM-SHA-512 | Kafka gateway compatibility. |
| OAuth2 / OIDC | Native SDKs, admin APIs, and service principals. |
| Short-lived X.509 identities | Internal node identity and cluster membership. |

### 6.2 Authentication Flow

```
Client / Gateway / Service
        │
        ▼
Present credentials: mTLS certificate, OAuth2 token, SASL login, or API identity
        │
        ▼
Authentication Service validates credential against IdP / PKI / token issuer
        │
        ▼
Principal Mapper maps to canonical PEF principal
        │
        ▼
ABAC PDP evaluates requested operation
        │
        ▼
PEP allows or denies operation
```

### 6.3 Normative Authentication Rules

- All external connections MUST use TLS 1.3 or later.
- Internal cluster traffic MUST use mTLS with short-lived certificates.
- Anonymous access MUST be disabled by default.
- Token lifetimes SHOULD be short-lived, with refresh or rotation.
- Authentication failures MUST be rate-limited and audit-logged.

---

## 7. Principal Model and Identity Mapping

### 7.1 Canonical PEF Principal

Every request is mapped to a canonical principal:

```
PEFPrincipal {
  principal_id
  principal_type       # user | service | admin | gateway | system
  tenant_id
  roles
  attributes
  authentication_method
  security_level
}
```

### 7.2 Gateway Identity Mapping

| Gateway Identity | Mapped PEF Principal |
|---|---|
| Kafka SASL principal | PEF service or user principal |
| OAuth2 client ID | PEF service principal |
| OIDC user subject | PEF user principal |
| SQS-style identity | PEF service principal |
| AMQP user | PEF service or user principal |
| Internal node certificate | PEF system principal |

**Normative rule:** Gateways MUST NOT perform independent authorization. They MUST resolve identities and delegate authorization to the ABAC PDP.

---

## 8. Authorization Architecture — ABAC

### 8.1 Authorization Attributes

Authorization decisions are based on attributes from four categories:

| Category | Examples |
|---|---|
| Principal | principal_id, tenant_id, role, security_level |
| Resource | tenant_id, stream_id, group_id, region, sensitivity label |
| Operation | produce, consume, lease, ack, nack, dlq_read, dlq_redrive, admin, delete, export, query |
| Environment | network origin, time, mTLS strength, region, compliance mode |

### 8.2 Core Operation Matrix

| Operation | Description |
|---|---|
| `produce` | Append records to a stream. |
| `consume` | Read stream data. |
| `lease` | Acquire queue leases. |
| `ack` | Acknowledge a lease. |
| `nack` | Negative-acknowledge and requeue. |
| `dlq_read` | Inspect virtual DLQ entries. |
| `dlq_redrive` | Redrive DLQ entries. |
| `admin` | Manage streams, groups, tenants, and configuration. |
| `delete` | Request retention or erasure operations. |
| `export` | Export data to lakehouse or external systems. |
| `query` | Query lakehouse projections. |

### 8.3 Policy Decision Point

The PDP evaluates policies and returns:

```text
ALLOW
DENY
DENY_WITH_REASON
```

**Normative rules:**

- The PDP MUST deny by default.
- Policies MUST be versioned and signed.
- Policy decisions MUST include a reason code for audit and debugging.
- Local policy caches MUST have bounded TTLs.
- If no valid cached policy exists, the PEP MUST deny.

### 8.4 Policy Enforcement Points

PEPs exist at:

1. Protocol gateways.
2. Native SDK server endpoints.
3. Storage engine append/read interfaces.
4. State plane lease/ACK interfaces.
5. Lakehouse export and query interfaces.
6. Admin and control-plane APIs.

**Normative rule:** No storage, state, or lakehouse operation MAY execute without a successful authorization decision.

---

## 9. Tenant Isolation Model

Tenant isolation is enforced through multiple independent layers.

| Layer | Mechanism |
|---|---|
| Namespace isolation | All streams, groups, manifests, and policies are tenant-scoped. |
| Authentication isolation | Principals are bound to tenants. |
| Authorization isolation | Cross-tenant access is denied unless explicitly authorized by system policy. |
| Encryption isolation | Tenant KEKs isolate cryptographic domains. |
| Quota isolation | Per-tenant admission control prevents noisy-neighbor abuse. |
| Audit isolation | Audit events are tenant-attributed and access-restricted. |

**Normative rules:**

- A request from Tenant A MUST NOT read, lease, acknowledge, redrive, or delete Tenant B data.
- Cross-tenant operations MUST require explicit system-level authorization and audit logging.
- Tenant isolation violations MUST be treated as security incidents.

---

## 10. Encryption Architecture

## 10.1 Encryption in Transit

| Requirement | Rule |
|---|---|
| External traffic | TLS 1.3 mandatory. |
| Internal cluster traffic | mTLS mandatory. |
| Weak cipher suites | Disabled. |
| Certificate rotation | Supported without downtime. |
| Plaintext fallback | Prohibited. |

## 10.2 Encryption at Rest

PEF uses **envelope encryption** (ADR-050).

```
Root Key
   └── Tenant Key Encryption Key (KEK)
         └── Stream DEK or Stream-Batch DEK
               └── Data object encryption key used for WAL / Parquet encryption
```

### 10.3 Key Hierarchy

| Key | Scope | Lifetime | Storage |
|---|---|---|---|
| Root Key | KMS/HSM trust anchor | Long-lived, rotated per provider policy | External KMS/HSM |
| Tenant KEK | One per tenant | Rotatable; default annual or on-demand | Wrapped by Root Key |
| Stream DEK | One per regulated or high-isolation stream | Rotatable; destroyed for erasure | Wrapped by Tenant KEK |
| Stream-Batch DEK | One per tenant/date/bucket for high-cardinality streams | Shorter lifecycle | Wrapped by Tenant KEK |

### 10.4 Encryption Algorithms

| Use | Algorithm |
|---|---|
| Preferred bulk encryption | AES-256-GCM with AES-NI acceleration. |
| Fallback where AES-NI is unavailable | ChaCha20-Poly1305. |
| Key wrapping | KMS-provider-specific wrap/unwrap operations. |
| Integrity | Authenticated encryption tags plus CRC32C batch integrity. |

### 10.5 Encryption Metadata

Each encrypted WAL batch or Parquet file MUST carry:

```text
dek_id
key_version
nonce
algorithm_id
aad_context
authentication_tag
```

The Authenticated Additional Data (AAD) MUST include at least:

```text
tenant_id
stream_id
physical_sequence or object identifier
format_version
```

**Normative rule:** Decryption MUST fail if AAD validation fails.

---

## 11. Key Management Architecture

### 11.1 KMS Adapter

The Key Management Adapter abstracts external providers:

- AWS KMS
- GCP KMS
- Azure Key Vault
- HashiCorp Vault
- FIPS-certified HSM-backed KMS

### 11.2 DEK Cache

To avoid excessive KMS latency and API load:

- DEKs MAY be cached in process memory.
- Cached DEKs MUST have a bounded TTL.
- DEK memory MUST be zeroized on eviction.
- DEKs MUST NOT be written to logs, manifests, snapshots, or error output.
- DEK cache misses MUST trigger KMS unwrap with retry and jitter.

### 11.3 Key Rotation

| Event | Behavior |
|---|---|
| Routine KEK rotation | New KEK version wraps new DEKs; old KEK remains available for old data until rewrapped or erased. |
| Stream DEK rotation | New writes use new DEK; historical chunks retain old `dek_id`. |
| Compaction rewrite | If a chunk is rewritten during compaction, it MAY be re-encrypted under the current DEK. |
| Suspected compromise | Immediate key destruction or rotation, cache invalidation, and incident workflow. |

**Normative rule:** Key rotation MUST NOT require rewriting all historical data immediately. Historical data remains readable via old `dek_id` references until lifecycle or re-encryption occurs.

---

## 12. Crypto-Shredding and Regulatory Erasure

### 12.1 Purpose

Crypto-shredding reconciles the Golden Invariant’s immutable log with GDPR/CCPA-style erasure requirements (ADR-051).

> Destroying the relevant encryption key renders ciphertext cryptographically unrecoverable immediately, while physical deletion may occur asynchronously during lifecycle and compaction sweeps.

### 12.2 Erasure Granularity

| Granularity | Mechanism |
|---|---|
| Stream erasure | Destroy Stream DEK. |
| Tenant erasure | Destroy Tenant KEK, rendering all tenant DEK-wrapped data unrecoverable. |
| Batch-level erasure | Destroy Stream-Batch DEK where batch granularity is used. |

### 12.3 Erasure Workflow

```
1. Authorized erasure request received
        │
2. Validate principal, tenant, stream, legal approval
        │
3. Create erasure ticket
        │
4. Freeze writes to target stream if applicable
        │
5. Command KMS to destroy relevant DEK/KEK
        │
6. Record key destruction evidence
        │
7. Write erasure tombstone to metadata
        │
8. Propagate destroyed-key registry to all regions
        │
9. Hide tombstoned ranges from active manifests and query views
        │
10. Background lifecycle physically removes ciphertext when eligible
```

### 12.4 Erasure Tombstone

An erasure tombstone MUST record:

```text
erasure_ticket_id
tenant_id
stream_id
key_id
key_version
destruction_timestamp
operator_principal
legal_approval_reference
region_propagation_status
```

### 12.5 Backups and Cross-Region Replicas

Backups and replicas may retain ciphertext, but if the relevant key is destroyed, that ciphertext MUST be cryptographically unrecoverable.

**Normative rules:**

- Destroyed key IDs MUST be recorded in a destroyed-key registry.
- Restore processes MUST refuse to restore data protected by destroyed keys.
- Cross-region failover MUST check destroyed-key registry before exposing restored data.

### 12.6 Legal and Policy Caveat

Crypto-shredding is the system’s technical erasure mechanism. Whether it satisfies a specific regulatory or contractual obligation depends on the customer’s legal review and jurisdiction.

**Normative wording:** Keirox provides cryptographic erasure with audit evidence; compliance acceptance is customer-specific.

---

## 13. Retention, Legal Hold, and Data Residency

### 13.1 Retention Policies

Retention may be defined per:

- Tenant.
- Stream.
- Stream class.
- Region.
- Compliance profile.

Retention behavior:

```text
IF retention_expired AND NOT legal_hold THEN
    data becomes eligible for physical deletion
    optional crypto-shredding may render it inaccessible earlier
END
```

### 13.2 Legal Hold

A legal hold MUST prevent:

- Physical deletion.
- Crypto-shredding, unless explicitly authorized by legal workflow.
- Manifest removal.
- Iceberg snapshot expiration for protected ranges.

### 13.3 Data Residency

Data residency controls MUST support:

- Region-bound storage.
- Region-bound KMS keys.
- Region-bound replication restrictions.
- Cross-region transfer denial by default unless explicitly allowed.

**Normative rule:** A stream assigned to a residency region MUST NOT replicate outside that region without an explicit policy exception.

---

## 14. Audit Logging Architecture

### 14.1 Audit Event Classes

| Event Class | Examples |
|---|---|
| Authentication | Login success, login failure, certificate validation, token validation. |
| Authorization | Allow, deny, policy cache miss, privilege escalation attempt. |
| Data access | Produce, consume, lease, ACK, DLQ read, export. |
| Administrative | Stream create/delete, quota change, policy change, gateway configuration. |
| Key management | Key creation, rotation, destruction, cache invalidation. |
| Erasure | Erasure request, approval, key destruction, tombstone propagation. |
| Security incident | Cross-tenant attempt, repeated auth failures, KMS anomaly. |

### 14.2 Audit Record Schema

Each audit record MUST include:

```text
event_id
event_timestamp
tenant_id
principal_id
principal_type
operation
resource_type
resource_id
decision
reason_code
source_ip / network origin
request_id
security_level
signature_or_hash_chain_reference
```

### 14.3 Tamper-Evident Audit Trail

The audit trail MUST be tamper-evident by one of the following:

- Append-only hash-chained log.
- WORM object storage.
- Signed audit batches.
- External audit sink with integrity receipts.

**Normative rules:**

- Audit records MUST NOT be mutable after commit.
- Audit deletions MUST NOT be permitted in normal operation.
- Audit retention MUST be independent of stream data retention unless compliance policy states otherwise.

### 14.4 Audit Availability

If the audit sink is unavailable:

- Security events MUST be buffered locally with bounded storage.
- High-risk administrative operations SHOULD be blocked if audit buffering capacity is exhausted.
- Audit loss MUST raise a critical security alert.

---

## 15. Secret and Credential Management

### 15.1 Secret Handling Rules

| Rule | Requirement |
|---|---|
| No hardcoded secrets | Secrets MUST NOT appear in source, config files, or container images. |
| No secrets in logs | Logs MUST redact tokens, keys, and credentials. |
| No secrets in errors | Error messages MUST NOT expose credentials or key material. |
| No secrets in manifests | Chunk manifests, Iceberg metadata, and snapshots MUST NOT contain plaintext keys. |
| Short-lived credentials | Cloud credentials SHOULD use IAM roles, workload identity, or short-lived tokens. |
| Rotation | Secrets MUST support rotation without downtime. |

### 15.2 Credential Sources

Supported credential sources:

- Cloud IAM roles.
- Kubernetes service accounts / workload identity.
- HashiCorp Vault dynamic secrets.
- Managed identity providers.
- Short-lived mTLS certificates.

**Normative rule:** Static long-lived credentials SHOULD be avoided and MUST be exception-tracked if used.

---

## 16. Security Telemetry and Incident Hooks

### 16.1 Required Security Metrics

| Metric | Purpose |
|---|---|
| `auth_failure_rate` | Detect credential abuse or misconfiguration. |
| `authorization_denial_rate` | Detect policy misuse or attack attempts. |
| `cross_tenant_denial_count` | Detect isolation violations. |
| `kms_error_rate` | Detect KMS availability or permission issues. |
| `dek_cache_hit_ratio` | Monitor KMS load and cache effectiveness. |
| `crypto_shred_count` | Track erasure activity. |
| `audit_buffer_pressure` | Detect audit sink degradation. |
| `policy_cache_miss_rate` | Detect PDP instability. |

### 16.2 Incident Hooks

Security incidents MUST trigger:

- Alerting.
- Audit record enrichment.
- Optional principal throttling.
- Optional session revocation.
- Evidence preservation.

**Normative rule:** Repeated cross-tenant authorization denials MUST be treated as a potential security incident, not merely a metric.

---

## 17. Secure Failure Behavior

The security plane MUST fail secure.

| Failure | Required Behavior |
|---|---|
| KMS unavailable | Use cached DEKs only within bounded TTL; new writes requiring new DEKs MUST fail closed. Plaintext fallback is prohibited. |
| IdP unavailable | Cached signed token validation MAY continue for bounded TTL; new logins fail. |
| PDP unavailable | Use bounded cached policies if valid; otherwise deny. |
| Audit sink unavailable | Buffer events locally; block destructive admin operations if buffer is exhausted. |
| Policy corruption | Reject corrupted policies; fall back to last known signed policy or deny. |
| Key cache compromise | Zeroize cache, isolate host, rotate keys, initiate incident workflow. |
| Cross-region key replication failure | Failover MUST NOT expose data unless required keys are available or legally authorized. |

---

## 18. Security Integration with Subsystems

| Subsystem | Security Integration |
|---|---|
| KEI-ARC-020 Storage Engine | Encrypt WAL batches and Parquet chunks; enforce tenant-scoped manifests; protect recovery metadata. |
| KEI-ARC-021 State Plane | Authorize lease/ACK/DLQ operations; audit redrive; protect lease metadata. |
| KEI-ARC-022 Consensus | Authenticate nodes; encrypt replication channels; fence stale principals via epochs. |
| KEI-ARC-023 Columnar ELT | Encrypt Parquet; enforce schema-level access if required; prevent unauthorized export. |
| KEI-ARC-024 Protocol Plane | Map identities; enforce authentication before protocol translation; deny unsupported privileged operations. |
| KEI-ARC-026 Multi-Region/DR | Enforce residency; propagate destroyed-key registry; protect backup restores. |

---

## 19. NFR Traceability (Owned by This Subsystem)

| NFR | Requirement | How This Subsystem Satisfies It |
|---|---|---|
| SEC-001 | Encryption in transit | TLS 1.3 / mTLS mandatory (§6, §10.1). |
| SEC-002 | Encryption at rest | AES-256-GCM / ChaCha20-Poly1305 envelope encryption (§10.2). |
| SEC-003 | Authentication | SASL/SCRAM, OAuth2/OIDC, mTLS (§6). |
| SEC-004 | ABAC authorization | PDP/PEP model (§8). |
| SEC-005 | Tenant isolation | Namespace, key, policy, and quota isolation (§9). |
| SEC-006 | Key management | KMS adapter, DEK cache, key hierarchy (§11). |
| SEC-007 | Secrets handling | No secrets in logs/errors/manifests (§15). |
| SEC-008 | Audit logging | Tamper-evident audit trail (§14). |
| COMP-001 | Right-to-erasure | Crypto-shredding (§12). |
| COMP-002 | Erasure latency | Immediate logical erasure; eventual physical purge (§12.1). |
| COMP-003 | Retention enforcement | Retention and lifecycle policies (§13). |
| COMP-004 | Deletion audit | Erasure tickets and tombstones (§12.4). |
| COMP-005 | Compliance readiness | SOC2/ISO27001-ready controls; certification external (§20). |
| COMP-006 | Data residency | Region-bound storage and keys (§13.3). |

---

## 20. Compliance Posture

### 20.1 Readiness, Not Automatic Certification

This architecture provides controls suitable for SOC2 Type II, ISO 27001, GDPR, and CCPA readiness, but certification depends on:

- Organizational processes.
- External audit scope.
- Deployment configuration.
- Customer-specific legal review.
- Operational evidence over time.

**Normative wording:** Keirox provides compliance-ready controls; compliance certification is an organizational outcome.

### 20.2 Required Compliance Evidence

The system MUST be capable of producing:

- Access control logs.
- Key lifecycle records.
- Erasure proof.
- Audit trail integrity evidence.
- Change-management records.
- Incident-response evidence.
- Residency and replication reports.

---

## 21. Interfaces

### 21.1 Provided Interfaces

| Interface | Consumer | Semantics |
|---|---|---|
| `authenticate(credentials)` | Gateways / Admin API | Validate identity. |
| `mapPrincipal(protocol_identity)` | Gateways | Map to canonical PEF principal. |
| `authorize(principal, operation, resource)` | All subsystems | Return ABAC decision. |
| `wrapDEK(...)` / `unwrapDEK(...)` | Storage / Lakehouse | Envelope key operations. |
| `destroyKey(key_id)` | Crypto-Shredding Orchestrator | Destroy key material. |
| `recordAudit(event)` | All subsystems | Append audit record. |
| `getRetentionPolicy(resource)` | Lifecycle components | Return retention rules. |
| `checkResidency(resource, region)` | Replication / DR | Validate residency constraints. |

### 21.2 Consumed Interfaces

| Interface | Provider | Purpose |
|---|---|---|
| OIDC/OAuth2 token validation | External IdP | Authentication. |
| mTLS certificate validation | PKI / CA | Service authentication. |
| KMS wrap/unwrap/destroy | AWS KMS / Vault / GCP KMS / Azure Key Vault | Key management. |
| Audit sink | WORM storage / SIEM | Audit persistence. |
| Secret provider | Vault / cloud IAM | Credential access. |

---

## 22. Open Questions and ADR Dependencies

| Item | Status | Resolution Path |
|---|---|---|
| Default DEK granularity: per-stream vs. stream-batch | Open | Benchmark KMS cost vs. deletion granularity; specify in KEI-DES-036. |
| Policy language choice | Open | Evaluate embedded CEL vs. OPA/Rego. |
| Audit backend | Open | Evaluate hash-chained internal log vs. external WORM sink. |
| FIPS mode requirement | Open | Determine enterprise target segment. |
| Cross-region key replication policy | Open | Define with KEI-ARC-026. |
| Legal acceptance of crypto-shredding | Open | Customer-specific legal review; document as policy caveat. |

Binding decisions already recorded: ADR-050, ADR-051, ADR-052.

---

## 23. Glossary (Additions)

| Term | Definition |
|---|---|
| ABAC | Attribute-Based Access Control. |
| PDP | Policy Decision Point. |
| PEP | Policy Enforcement Point. |
| KEK | Key Encryption Key. |
| DEK | Data Encryption Key. |
| Envelope Encryption | Encryption where data keys are encrypted by higher-level KMS keys. |
| Crypto-Shredding | Erasure by destroying encryption keys. |
| Erasure Tombstone | Metadata proving a key was destroyed and data is cryptographically inaccessible. |
| WORM | Write Once, Read Many; immutable storage. |
| Fail Secure | Failure behavior that denies access rather than reducing security. |

---

## 24. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial security, privacy, and compliance architecture. Defines authentication, principal mapping, ABAC, tenant isolation, envelope encryption, KMS integration, crypto-shredding, retention/residency governance, tamper-evident audit, secret handling, and fail-secure behavior. Aligns to ADR-050/051/052 and NFRs SEC/COMP. |