# KEI-SEC-401 — Security & Compliance Certification Plan

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-SEC-401 |
| Title | Security & Compliance Certification Plan |
| Version | 1.0 |
| Level | Engineering Execution Plan |
| Status | Baseline — Ready for Execution |
| Phase | Phase 4 — Enterprise Hardening, Compliance & Multi-Region |
| Duration | Weeks 3–34 of Phase 4 |
| Owner | Security Architect / Security Lead |
| Governing Plan | KEI-ENG-400 — Phase 4 Engineering Execution Plan |
| Governing Architecture Documents | KEI-ARC-025, KEI-DES-036, KEI-OPS-040, KEI-OPS-041 |
| Predecessor | KEI-SPIKE-401 — Enterprise Hardening Prototype Plan |
| Next Plan File | KEI-MR-401 — Multi-Region & DR Certification Plan |

---

## 2. Executive Summary

Phase 4 must prove that Keirox is not only functionally correct and adoptable, but also **secure, governable, auditable, and compliance-ready** for regulated multi-tenant enterprises.

This plan defines the certification program for:

1. **Encryption and key management**
   - TLS/mTLS enforcement.
   - Envelope encryption using KMS.
   - DEK lifecycle management.
   - Key rotation.
   - Fail-secure behavior.

2. **Crypto-shredding and erasure**
   - GDPR/CCPA-style logical erasure.
   - Destroyed-key registry.
   - Backup interaction.
   - Erasure proof generation.

3. **Authorization and tenant isolation**
   - Default-deny ABAC.
   - Principal mapping across gateways and SDKs.
   - Tenant namespace enforcement.
   - Administrative operation controls.

4. **Audit and compliance evidence**
   - Tamper-evident audit trail.
   - Security event retention.
   - Access review support.
   - SOC2/ISO27001 readiness evidence.

5. **Security validation**
   - Security tests.
   - Chaos security tests.
   - Penetration testing coordination.
   - Threat model validation.

This document does **not** promise SOC2 or ISO certification by itself. It defines the engineering evidence and control readiness required for an external audit or customer security review.

---

## 3. Purpose and Scope

### 3.1 Purpose

The purpose of this plan is to:

1. Define security certification levels.
2. Define mandatory security acceptance criteria.
3. Define crypto-shredding certification evidence.
4. Define ABAC and tenant isolation tests.
5. Define audit trail validation requirements.
6. Define secure development and release controls.
7. Define penetration testing and remediation gates.
8. Produce the Phase 4 security and compliance evidence package.

### 3.2 Scope

**In scope:**

- KMS adapter certification.
- Envelope encryption certification.
- DEK cache security validation.
- Key rotation validation.
- Crypto-shredding certification.
- Destroyed-key registry validation.
- Backup and restore interaction with erasure.
- ABAC policy enforcement validation.
- Gateway principal mapping validation.
- Tenant isolation validation.
- Audit trail integrity validation.
- Secure logging validation.
- Secrets handling validation.
- Dependency scanning and supply-chain checks.
- Penetration testing coordination.
- SOC2/ISO readiness evidence mapping.

**Out of scope:**

- Formal external SOC2 Type II certification.
- Formal ISO27001 certification.
- Customer-specific legal acceptance of crypto-shredding.
- Multi-region DR certification — owned by KEI-MR-401.
- SQS/AMQP compatibility certification — owned by KEI-QUEUE-401.
- Jepsen-style consistency certification — owned by KEI-VAL-401.

---

## 4. Security Certification Principles

| ID | Principle | Requirement |
|---|---|---|
| SEC-CERT-1 | Fail secure | Encryption or KMS failure MUST deny unsafe access, never fall back to plaintext. |
| SEC-CERT-2 | Default deny | ABAC MUST deny unless explicitly allowed. |
| SEC-CERT-3 | Least privilege | Principals MUST receive only minimum required permissions. |
| SEC-CERT-4 | Tenant isolation by construction | Tenant boundaries MUST be enforced by namespace, key hierarchy, and policy. |
| SEC-CERT-5 | Erasure is cryptographic and auditable | Crypto-shredding MUST destroy keys and produce tamper-evident proof. |
| SEC-CERT-6 | Backups respect erasure | Restore MUST NOT resurrect destroyed data. |
| SEC-CERT-7 | Secrets are never exposed | Keys, tokens, and credentials MUST NOT appear in logs, errors, metrics, snapshots, or manifests. |
| SEC-CERT-8 | Evidence before trust | Security claims MUST be backed by repeatable tests and reports. |

---

## 5. Security Certification Levels

| Level | Name | Requirement |
|---|---|---|
| L1 | Encryption Certified | TLS/mTLS and encryption at rest validated |
| L2 | Key Management Certified | KMS adapter, DEK cache, rotation, fail-secure validated |
| L3 | Erasure Certified | Crypto-shredding and destroyed-key registry validated |
| L4 | Authorization Certified | ABAC, principal mapping, tenant isolation validated |
| L5 | Audit Certified | Tamper-evident audit trail and retention validated |
| L6 | Secure Delivery Certified | Dependency scanning, secret scanning, secure release gates validated |
| L7 | Attack Resistance Certified | Pen test and security chaos tests completed with critical/high findings resolved |

Phase 4 exit requires **L1 through L7**.

---

## 6. Encryption and Key Management Certification

### 6.1 Transport Encryption Requirements

| ID | Requirement |
|---|---|
| ENC-T-001 | All external client traffic MUST use TLS 1.3 or later. |
| ENC-T-002 | Internal cluster traffic MUST use mTLS. |
| ENC-T-003 | Weak cipher suites MUST be disabled. |
| ENC-T-004 | Certificate rotation MUST be supported without downtime. |
| ENC-T-005 | Plaintext fallback MUST be impossible by default. |

### 6.2 Encryption at Rest Requirements

| ID | Requirement |
|---|---|
| ENC-R-001 | Customer payload data MUST be encrypted at rest. |
| ENC-R-002 | WAL batches MUST be encrypted with AES-256-GCM or approved equivalent. |
| ENC-R-003 | Parquet lakehouse files MUST be encrypted. |
| ENC-R-004 | State snapshots and lease journals MUST be encrypted. |
| ENC-R-005 | Stream manifests and registry metadata MUST be protected according to sensitivity. |
| ENC-R-006 | Authenticated Additional Data MUST bind ciphertext to tenant/stream/sequence context. |

### 6.3 KMS Adapter Certification Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| KMS-T-001 | Generate DEK under Tenant KEK | Plaintext DEK returned only transiently; wrapped DEK stored |
| KMS-T-002 | Unwrap DEK with valid context | Decryption succeeds |
| KMS-T-003 | Unwrap DEK with invalid AAD/context | Decryption fails securely |
| KMS-T-004 | KMS unavailable with warm DEK cache | Cached reads continue within TTL; new writes requiring new DEK fail closed |
| KMS-T-005 | KMS unavailable with empty DEK cache | New writes fail secure |
| KMS-T-006 | Destroy DEK | Subsequent unwrap fails |
| KMS-T-007 | Rotate Tenant KEK | New DEKs wrapped under new KEK; old data remains readable |
| KMS-T-008 | Key metadata query | No key material exposed |

### 6.4 DEK Cache Certification

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| DEK-T-001 | Cache hit | Low-latency unwrap; no KMS call |
| DEK-T-002 | Cache expiry | Entry removed; KMS unwrap required |
| DEK-T-003 | Cache eviction | Memory zeroized |
| DEK-T-004 | Cache size limit | LRU behavior; bounded memory |
| DEK-T-005 | Destroyed key in cache | Entry invalidated; access denied |
| DEK-T-006 | Process crash | No DEK plaintext persists on disk |

---

## 7. Crypto-Shredding Certification

### 7.1 Erasure Requirements

| ID | Requirement |
|---|---|
| ERASE-001 | Erasure MUST be authorized and audited. |
| ERASE-002 | Legal hold MUST block erasure unless explicitly released. |
| ERASE-003 | Key destruction MUST be irreversible. |
| ERASE-004 | Destroyed keys MUST be recorded in a replicated registry. |
| ERASE-005 | Erasure tombstones MUST propagate to all regions. |
| ERASE-006 | Reads against destroyed data MUST fail securely. |
| ERASE-007 | Backups containing destroyed data MUST remain cryptographically inaccessible. |
| ERASE-008 | Physical deletion MAY occur later via lifecycle/compaction. |

### 7.2 Erasure Certification Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| ERASE-T-001 | Stream erasure | Stream DEK destroyed; stream data unreadable |
| ERASE-T-002 | Tenant erasure | Tenant KEK destroyed; tenant data unreadable |
| ERASE-T-003 | Batch erasure | Batch DEK destroyed; batch data unreadable |
| ERASE-T-004 | Erasure under legal hold | Blocked; audit event emitted |
| ERASE-T-005 | Erasure without authorization | Denied; security event logged |
| ERASE-T-006 | Read after erasure | Access error; no plaintext returned |
| ERASE-T-007 | Query lakehouse after erasure | Encrypted files inaccessible or filtered by destroyed-key policy |
| ERASE-T-008 | Restore backup after erasure | Destroyed data remains unreadable |
| ERASE-T-009 | Cross-region propagation | All regions reject access using destroyed keys |
| ERASE-T-010 | Erasure proof generation | Ticket, key IDs, receipts, timestamps, and operator recorded |

### 7.3 Erasure Evidence Package

Crypto-shredding certification MUST produce:

1. Erasure workflow diagram.
2. Erasure ticket schema.
3. KMS destruction receipts.
4. Destroyed-key registry entries.
5. Tombstone propagation confirmation.
6. Read-failure test results.
7. Backup restore test results.
8. Audit log excerpt.
9. Legal hold blocking evidence.
10. Operator runbook for erasure.

---

## 8. Authorization and Tenant Isolation Certification

### 8.1 ABAC Requirements

| ID | Requirement |
|---|---|
| AUTH-001 | ABAC MUST default deny. |
| AUTH-002 | Every operation MUST resolve to a PEF principal before execution. |
| AUTH-003 | Policies MUST be versioned and auditable. |
| AUTH-004 | Policy decisions MUST include reason codes. |
| AUTH-005 | Policy cache MUST have bounded TTL. |
| AUTH-006 | If no valid policy is available, operation MUST deny. |
| AUTH-007 | Administrative operations MUST require elevated authorization. |
| AUTH-008 | Destructive operations MUST require two-person approval where configured. |

### 8.2 Tenant Isolation Requirements

| ID | Requirement |
|---|---|
| ISO-001 | Tenant A MUST NOT read Tenant B streams. |
| ISO-002 | Tenant A MUST NOT write to Tenant B streams. |
| ISO-003 | Tenant A MUST NOT lease/ACK Tenant B queues. |
| ISO-004 | Tenant A MUST NOT access Tenant B lakehouse tables. |
| ISO-005 | Tenant A MUST NOT unwrap Tenant B keys. |
| ISO-006 | Tenant A MUST NOT view Tenant B audit events. |
| ISO-007 | Cross-tenant attempts MUST be denied and logged. |

### 8.3 Authorization Certification Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| AUTH-T-001 | Unauthenticated request | Rejected |
| AUTH-T-002 | Authenticated but unauthorized request | Denied with reason |
| AUTH-T-003 | Expired token | Rejected |
| AUTH-T-004 | Cross-tenant stream read | Denied and audited |
| AUTH-T-005 | Cross-tenant stream write | Denied and audited |
| AUTH-T-006 | Cross-tenant lease/ACK | Denied and audited |
| AUTH-T-007 | Cross-tenant DLQ redrive | Denied and audited |
| AUTH-T-008 | Cross-tenant key unwrap | Denied and audited |
| AUTH-T-009 | Admin operation without approval | Denied |
| AUTH-T-010 | Policy engine unavailable | Fail closed or bounded cached policy only |

---

## 9. Audit Trail Certification

### 9.1 Audit Requirements

| ID | Requirement |
|---|---|
| AUD-001 | Security events MUST be logged. |
| AUD-002 | Administrative events MUST be logged. |
| AUD-003 | Erasure events MUST be logged. |
| AUD-004 | Authorization denials MUST be logged. |
| AUD-005 | Key lifecycle events MUST be logged. |
| AUD-006 | Audit records MUST be tamper-evident. |
| AUD-007 | Audit records MUST include actor, action, resource, timestamp, result, and request ID. |
| AUD-008 | Audit retention MUST comply with configured policy. |
| AUD-009 | Audit logs MUST NOT contain secrets or full customer payload. |

### 9.2 Audit Certification Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| AUD-T-001 | Authentication failure | Audit event emitted |
| AUD-T-002 | Authorization denial | Audit event emitted |
| AUD-T-003 | Crypto-shredding request | Audit event emitted |
| AUD-T-004 | Key destruction | Audit event emitted |
| AUD-T-005 | Admin destructive operation | Audit event emitted |
| AUD-T-006 | Audit log tamper attempt | Tamper evidence detected |
| AUD-T-007 | Audit sink unavailable | Local buffering; critical operations blocked if buffer exhausted per policy |
| AUD-T-008 | Audit export | Export includes required fields |

---

## 10. Secure Delivery and Supply Chain Certification

### 10.1 Secure Development Requirements

| ID | Requirement |
|---|---|
| SDLC-001 | Dependency scanning MUST run in CI. |
| SDLC-002 | Secret scanning MUST run in CI. |
| SDLC-003 | Unsafe Rust usage MUST require explicit justification. |
| SDLC-004 | Debug logs MUST NOT leak secrets or payloads. |
| SDLC-005 | Release artifacts MUST be signed or checksummed. |
| SDLC-006 | Security-critical changes MUST receive security review. |
| SDLC-007 | Known vulnerabilities MUST be triaged within defined SLAs. |

### 10.2 Vulnerability SLAs

| Severity | Remediation Target |
|---|---|
| Critical | Immediate mitigation; blocks release |
| High | Fix before release or documented compensating control |
| Medium | Track and schedule |
| Low | Track and monitor |

### 10.3 Secure Delivery Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| SDLC-T-001 | Dependency CVE detected | CI alerts and blocks according to severity policy |
| SDLC-T-002 | Secret committed | CI blocks and alerts |
| SDLC-T-003 | Debug log contains token | Test fails |
| SDLC-T-004 | Release artifact checksum mismatch | Release rejected |
| SDLC-T-005 | Unsafe Rust introduced | Review gate enforced |

---

## 11. Penetration Testing Coordination

### 11.1 Pen Test Scope

Phase 4 penetration testing SHOULD cover:

1. Native gRPC/Arrow Flight API.
2. Kafka gateway.
3. SQS gateway.
4. AMQP gateway.
5. Admin/control-plane APIs.
6. Authentication flows.
7. Authorization bypass attempts.
8. Tenant isolation attacks.
9. Protocol parsing attacks.
10. Key management access paths.
11. Audit log protection.
12. Object storage access controls.

### 11.2 Pen Test Rules

| Rule | Requirement |
|---|---|
| PT-001 | Pen test MUST occur in isolated staging. |
| PT-002 | Pen test MUST NOT expose production tenant data. |
| PT-003 | All findings MUST be triaged. |
| PT-004 | Critical/High findings MUST be resolved or formally accepted before Gate 4C. |
| PT-005 | Retest MUST verify remediation. |
| PT-006 | Final report MUST be included in Phase 4 evidence package. |

---

## 12. Compliance Readiness Mapping

### 12.1 SOC2 Type II Readiness Controls

| Control Area | Keirox Evidence Source |
|---|---|
| Access control | ABAC tests, principal mapping, access review tooling |
| Encryption | TLS/mTLS tests, encryption at rest tests |
| Key management | KMS adapter tests, key rotation evidence |
| Audit logging | Audit trail validation, retention policy |
| Change management | PR traceability, ADRs, CI gates |
| Incident response | Incident runbooks, security alerts |
| Vendor management | KMS/cloud provider dependency controls |
| Data lifecycle | Retention, legal hold, erasure workflow |

### 12.2 ISO27001 Readiness Controls

| Control Area | Keirox Evidence Source |
|---|---|
| Information security policies | Security architecture and operational policies |
| Asset management | Data classification and tenant scoping |
| Access control | ABAC certification |
| Cryptography | Encryption and key management certification |
| Physical/environmental | Cloud provider responsibility boundary documentation |
| Operations security | Runbooks, capacity, alerting |
| Communications security | TLS/mTLS, network segmentation |
| Incident management | Incident response runbooks |
| Compliance | Legal hold, erasure, audit retention |

**Normative statement:** Keirox provides SOC2/ISO readiness controls and evidence. Formal certification depends on external audit scope, organizational processes, and customer-specific requirements.

---

## 13. Security Metrics and Alerts

### 13.1 Required Security Metrics

| Metric | Type | Purpose |
|---|---|---|
| `keirox_auth_failures_total` | Counter | Detect credential abuse |
| `keirox_authz_denials_total` | Counter | Detect policy misuse or attacks |
| `keirox_cross_tenant_denials_total` | Counter | Detect isolation attacks |
| `keirox_kms_errors_total` | Counter | Detect KMS availability issues |
| `keirox_dek_cache_hit_ratio` | Gauge | Monitor KMS load and cache health |
| `keirox_dek_cache_size` | Gauge | Monitor bounded cache memory |
| `keirox_crypto_shred_total` | Counter | Track erasure activity |
| `keirox_destroyed_key_registry_size` | Gauge | Track erasure governance |
| `keirox_audit_buffer_pressure` | Gauge | Detect audit sink degradation |
| `keirox_security_alert_total` | Counter | Track security events |

### 13.2 Required Alerts

| Alert | Condition | Severity |
|---|---|---|
| KMS unavailable | KMS errors exceed threshold | Critical |
| Plaintext fallback attempt | Any detected | Critical |
| Cross-tenant access spike | Denial rate exceeds threshold | Critical |
| Destroyed key access attempt | Access using destroyed key | Critical |
| Audit sink failure | Audit buffer full or sink unavailable | Critical |
| DEK cache exhaustion | Cache full with high eviction rate | Warning |
| Key rotation overdue | Rotation past policy window | Warning |
| Pen test finding unresolved | Critical/high open past SLA | Critical |

---

## 14. Deliverables and Milestones

| Deliverable | Description | Target Week |
|---|---|---:|
| D-SEC-001 | KMS adapter certification suite | Week 8 |
| D-SEC-002 | Encryption at rest validation suite | Week 10 |
| D-SEC-003 | DEK cache security tests | Week 12 |
| D-SEC-004 | Crypto-shredding certification suite | Week 14 |
| D-SEC-005 | Destroyed-key registry validation | Week 16 |
| D-SEC-006 | ABAC enforcement test suite | Week 18 |
| D-SEC-007 | Tenant isolation adversarial tests | Week 20 |
| D-SEC-008 | Audit trail validation suite | Week 22 |
| D-SEC-009 | Secure delivery pipeline checks | Week 24 |
| D-SEC-010 | Penetration test execution | Week 28 |
| D-SEC-011 | Pen test remediation and retest | Week 30 |
| D-SEC-012 | Compliance readiness evidence pack | Week 32 |
| D-SEC-013 | Final security certification report | Week 34 |

---

## 15. Security Certification Gates

### 15.1 Gate SEC-A — Encryption Prototype Gate

| Criterion | Mandatory |
|---|---|
| Encryption at rest works | Yes |
| KMS failure fails secure | Yes |
| DEK cache zeroizes on eviction | Yes |
| No plaintext fallback | Yes |

### 15.2 Gate SEC-B — Erasure and Authorization Gate

| Criterion | Mandatory |
|---|---|
| Crypto-shredding renders data unreadable | Yes |
| Backup restore respects destroyed keys | Yes |
| ABAC default-deny enforced | Yes |
| Cross-tenant access denied and audited | Yes |
| Audit trail captures security events | Yes |

### 15.3 Gate SEC-C — Final Security Certification Gate

| Criterion | Mandatory |
|---|---|
| All encryption tests pass | Yes |
| All key management tests pass | Yes |
| All erasure tests pass | Yes |
| All authorization tests pass | Yes |
| All tenant isolation tests pass | Yes |
| All audit tests pass | Yes |
| Secure delivery checks pass | Yes |
| Pen test critical/high findings resolved | Yes |
| Compliance readiness evidence approved | Yes |
| Security Review Board approval | Yes |

---

## 16. Risks and Mitigations

| Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|
| KMS provider integration complexity | High | Medium | Adapter pattern; support multiple backends; early prototype |
| Crypto-shredding misunderstood legally | High | Medium | Document technical erasure; require customer legal review |
| ABAC policy complexity causes false denials | Medium | Medium | Policy simulation; staged rollout; reason codes |
| Audit volume overwhelms storage | Medium | High | Sampling for non-security telemetry; full retention for security events |
| Pen test discovers critical bug | High | Medium | Schedule early; reserve remediation buffer |
| Secret leakage through logs | Critical | Low | Secret scanning; log redaction tests |
| Compliance evidence collection delayed | Medium | High | Start evidence collection early; automate where possible |
| Key rotation breaks historical reads | High | Low | Maintain old KEK versions; compatibility tests |

---

## 17. Evidence Package

The security certification evidence package MUST include:

1. Transport encryption report.
2. Encryption at rest report.
3. KMS adapter test report.
4. DEK cache security report.
5. Key rotation report.
6. Crypto-shredding certification report.
7. Destroyed-key registry validation report.
8. Backup restore erasure validation report.
9. ABAC enforcement report.
10. Tenant isolation report.
11. Audit trail validation report.
12. Secure delivery pipeline report.
13. Dependency and secret scanning report.
14. Penetration test report.
15. Pen test remediation report.
16. SOC2/ISO readiness evidence pack.
17. Security metrics and alert validation report.
18. Final security certification recommendation.

---

## 18. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial Security & Compliance Certification Plan. Defines encryption, key management, crypto-shredding, authorization, tenant isolation, audit, secure delivery, penetration testing, compliance readiness mapping, security gates, and evidence package. |