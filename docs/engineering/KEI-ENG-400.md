# KEI-ENG-400 — Phase 4 Engineering Execution Plan
## Enterprise Hardening, Compliance, Multi-Region & Advanced Queue Gateways

---

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-ENG-400 |
| Title | Phase 4 Engineering Execution Plan |
| Version | 1.0 |
| Level | Engineering Execution Plan |
| Status | Baseline — Ready for Execution |
| Phase | Phase 4 — Enterprise Hardening, Compliance & Multi-Region |
| Duration | Months 28–36 (9 months / 36 weeks) |
| Owner | Engineering Program Lead / Chief Architect |
| Governing Architecture | KEI-ARC-025, KEI-ARC-026, KEI-DES-036, KEI-DES-035, KEI-OPS-040, KEI-OPS-041 |
| Predecessor | KEI-ENG-300 (Phase 3 Engineering Execution Plan) |
| Next Phase | v1 Production Release Readiness / Phase 5 Planning |

---

## 2. Executive Summary

Phase 1 proved the Golden Invariant. Phase 2 proved distributed durability. Phase 3 proved ecosystem adoption through gateways, SDKs, and lakehouse integration.

Phase 4 answers the enterprise question:

> Can Keirox be trusted by regulated, multi-tenant, production enterprises requiring encryption, erasure, authorization, auditability, disaster recovery, and adversarial consistency validation?

Phase 4 transforms Keirox from a technically correct and adoptable platform into an **enterprise-ready v1 system**.

The phase delivers:

1. **Production Security Hardening**
   - TLS/mTLS everywhere.
   - Envelope encryption via KMS.
   - Secure DEK cache and key lifecycle.
   - Crypto-shredding for GDPR/CCPA-style erasure.
   - Tamper-evident audit trail.

2. **Authorization and Tenant Governance**
   - Default-deny ABAC.
   - Protocol gateway principal mapping.
   - Tenant isolation enforcement.
   - Administrative operation controls.

3. **Multi-Region Mode A Replication**
   - Single-writer primary per stream.
   - Asynchronous replica region.
   - Region epoch fencing.
   - Cross-region destroyed-key propagation.
   - Data residency enforcement.
   - Planned and unplanned failover.

4. **Disaster Recovery and Point-in-Time Recovery**
   - Backup scope validation.
   - Restore procedures.
   - PITR.
   - Legal hold integration.
   - DR drill certification.

5. **Advanced Queue Gateways**
   - SQS translation gateway subset.
   - AMQP direct/default exchange gateway subset.
   - Compatibility matrices.
   - Conformance tests.
   - Negative unsupported-operation tests.

6. **Jepsen-Style Consistency Certification**
   - Network partitions.
   - Clock skew.
   - Process kills.
   - Disk stalls.
   - Split-brain fencing.
   - Consistency invariant validation.

7. **Compliance Readiness**
   - SOC2 Type II readiness controls.
   - ISO27001 readiness controls.
   - Erasure proof generation.
   - Incident response evidence.
   - Audit retention.

8. **Enterprise Operational Hardening**
   - Runbook certification.
   - Upgrade safety.
   - Capacity governance.
   - Alerting maturity.
   - Break-glass procedures.
   - Supportability hooks.

Phase 4 is the final engineering phase before v1 production release readiness.

---

## 3. Phase 4 Mission

The mission of Phase 4 is:

1. Make Keirox safe for regulated multi-tenant production workloads.
2. Make erasure cryptographically enforceable and auditable.
3. Make authorization default-deny and tenant-scoped.
4. Make multi-region recovery provable and operationally executable.
5. Extend queue compatibility to SQS and AMQP workloads.
6. Prove consistency under adversarial failure conditions.
7. Produce enterprise compliance evidence.
8. Prepare the platform for v1 release readiness review.

---

## 4. Phase 4 Scope

### 4.1 In Scope

| Workstream | Scope |
|---|---|
| Security Hardening | TLS/mTLS, encryption at rest, KMS adapter, DEK cache, key rotation, destroyed-key registry |
| Crypto-Shredding | Erasure workflow, tombstones, backup interaction, erasure proof |
| Authorization | ABAC PDP/PEP, principal mapping, policy enforcement, admin controls |
| Audit | Tamper-evident audit events, retention, security event correlation |
| Multi-Region | Mode A replication, HLC causal tags, region epoch fencing, residency, failover |
| Disaster Recovery | Backup validation, restore, PITR, legal hold, DR drills |
| SQS Gateway | SendMessage, ReceiveMessage, DeleteMessage, ChangeMessageVisibility, FIFO subset |
| AMQP Gateway | Direct/default exchange subset, publish/consume/ack/nack/reject |
| Compatibility Certification | Published compatibility matrices, negative tests, conformance reports |
| Jepsen-Style Validation | Partition, skew, kill, stall, split-brain, invariant checking |
| Compliance Readiness | SOC2/ISO readiness evidence, access reviews, incident response |
| Operational Hardening | Runbooks, capacity, alerting, upgrades, break-glass, support hooks |

### 4.2 Out of Scope

| Item | Reason |
|---|---|
| Active-active same-stream multi-writer replication | Excluded for v1 by ADR-060 |
| CXL/RDMA hardware disaggregation | Excluded from v1 |
| In-broker SQL or materialized views | Excluded from v1 |
| Full Kafka parity | Rejected by ADR-070 |
| Kafka transactions | Deferred |
| Complex AMQP exchange topologies | Deferred/excluded |
| Universal exactly-once side effects | Not a broker guarantee |
| Full SOC2/ISO certification | External audit dependent |
| Customer-specific legal erasure acceptance | Legal review dependent |

### 4.3 Phase 4 Constraints

1. Phase 3 MUST be certified, or conditional remediation completed.
2. All Phase 1/2/3 invariants MUST continue to hold.
3. Security failures MUST fail secure.
4. Encryption MUST NOT fall back to plaintext.
5. Erasure MUST propagate to backups and replicas.
6. Legal hold MUST suspend destructive lifecycle operations.
7. Multi-region failover MUST fence old primary before writes resume.
8. Unsupported gateway operations MUST return explicit errors.
9. Compliance readiness MUST be evidenced, not assumed.

---

## 5. Phase 4 Objectives

| ID | Objective | Success Metric |
|---|---|---|
| OBJ-P4-001 | Prove encryption at rest and in transit | Security tests pass; no plaintext fallback |
| OBJ-P4-002 | Prove crypto-shredding | Erased data unreadable; erasure proof generated |
| OBJ-P4-003 | Prove default-deny authorization | Unauthorized operations rejected |
| OBJ-P4-004 | Prove tenant isolation | Cross-tenant access denied and audited |
| OBJ-P4-005 | Prove Mode A multi-region replication | RPO/RTO targets evidenced |
| OBJ-P4-006 | Prove failover safety | No split-brain writes; destroyed keys respected |
| OBJ-P4-007 | Prove backup and PITR | Restore validated; destroyed data not resurrected |
| OBJ-P4-008 | Prove SQS gateway subset | Certified SQS operations pass |
| OBJ-P4-009 | Prove AMQP gateway subset | Certified AMQP operations pass |
| OBJ-P4-010 | Prove adversarial consistency | Jepsen-style tests pass |
| OBJ-P4-011 | Produce compliance readiness evidence | SOC2/ISO readiness evidence package approved |
| OBJ-P4-012 | Prepare v1 release readiness | Final release readiness checklist approved |

---

## 6. Phase 4 Delivery Strategy

Phase 4 is divided into six work packages executed over 9 months.

### 6.1 Work Package Overview

| Work Package | ID | Duration | Focus |
|---|---|---|---|
| Security Foundations | WP-P4-A | Weeks 3–16 | KMS, encryption, DEK cache, key rotation, destroyed-key registry |
| Authorization, Audit & Tenant Governance | WP-P4-B | Weeks 6–20 | ABAC, principal mapping, tenant isolation, audit trail |
| Multi-Region Mode A & DR | WP-P4-C | Weeks 8–26 | Replication, failover, PITR, residency, DR drills |
| Advanced Queue Gateways | WP-P4-D | Weeks 10–28 | SQS and AMQP translation gateways |
| Compliance & Operational Hardening | WP-P4-E | Weeks 16–32 | SOC2/ISO readiness, runbooks, capacity, alerting |
| Certification & Release Readiness | WP-P4-F | Weeks 24–36 | Jepsen-style tests, pen test, evidence package, release review |

### 6.2 Overlap Strategy

- Weeks 6–16: Security and authorization overlap.
- Weeks 8–26: Multi-region and DR overlap with security erasure propagation.
- Weeks 10–28: Queue gateways overlap with operational hardening.
- Weeks 24–36: Certification, compliance evidence, and release readiness overlap.

---

## 7. Work Package A — Security Foundations

### 7.1 Objective

Implement production-grade encryption, key management, crypto-shredding, and secure failure behavior.

### 7.2 Deliverables

| ID | Deliverable | Description |
|---|---|---|
| D-P4-A-001 | KMS adapter | AWS KMS / Vault / GCP KMS abstraction |
| D-P4-A-002 | Envelope encryption engine | Root → Tenant KEK → Stream/Batch DEK |
| D-P4-A-003 | DEK cache | Bounded TTL cache with zeroization |
| D-P4-A-004 | WAL encryption integration | Encrypted WAL batches with AAD |
| D-P4-A-005 | Parquet encryption integration | Encrypted lakehouse files |
| D-P4-A-006 | State snapshot encryption | Encrypted state plane artifacts |
| D-P4-A-007 | Key rotation workflow | KEK/DEK rotation procedures |
| D-P4-A-008 | Destroyed-key registry | Replicated registry of destroyed keys |
| D-P4-A-009 | Crypto-shredding orchestrator | Erasure ticket, tombstone, propagation |
| D-P4-A-010 | Erasure proof generator | Audit evidence for key destruction |
| D-P4-A-011 | Fail-secure enforcement | No plaintext fallback |
| D-P4-A-012 | Security metrics | KMS errors, DEK cache hits, shred events |

### 7.3 Security Acceptance Criteria

| ID | Requirement |
|---|---|
| ACC-P4-SEC-001 | All external and internal traffic uses TLS 1.3/mTLS |
| ACC-P4-SEC-002 | Customer data is encrypted at rest |
| ACC-P4-SEC-003 | DEK plaintext is never persisted |
| ACC-P4-SEC-004 | DEK cache entries are zeroized on eviction |
| ACC-P4-SEC-005 | AAD validation prevents ciphertext substitution |
| ACC-P4-SEC-006 | Destroyed keys cannot unwrap data |
| ACC-P4-SEC-007 | Backup restore does not resurrect destroyed data |
| ACC-P4-SEC-008 | KMS unavailability fails secure |
| ACC-P4-SEC-009 | Crypto-shredding produces audit proof |
| ACC-P4-SEC-010 | Erasure propagates to all regions |

---

## 8. Work Package B — Authorization, Audit & Tenant Governance

### 8.1 Objective

Implement default-deny ABAC, tenant isolation, and tamper-evident audit logging.

### 8.2 Deliverables

| ID | Deliverable | Description |
|---|---|---|
| D-P4-B-001 | ABAC policy engine | Policy decision point |
| D-P4-B-002 | Policy enforcement points | Gateways, storage, state plane, admin APIs |
| D-P4-B-003 | Principal mapper | Kafka/SQS/AMQP/SDK identity mapping |
| D-P4-B-004 | Tenant namespace enforcement | Tenant-scoped streams, groups, tables, keys |
| D-P4-B-005 | Admin authorization controls | Two-person rule for destructive operations |
| D-P4-B-006 | Audit trail service | Tamper-evident security and admin events |
| D-P4-B-007 | Audit retention policy | Retention and export controls |
| D-P4-B-008 | Access review tooling | Periodic access certification support |
| D-P4-B-009 | Security telemetry | Auth failures, denials, cross-tenant attempts |

### 8.3 Authorization Acceptance Criteria

| ID | Requirement |
|---|---|
| ACC-P4-AUTH-001 | Default policy denies access |
| ACC-P4-AUTH-002 | Authenticated principals map to PEF identities |
| ACC-P4-AUTH-003 | Tenant A cannot access Tenant B data |
| ACC-P4-AUTH-004 | Cross-tenant attempts are denied and audited |
| ACC-P4-AUTH-005 | Destructive operations require approval |
| ACC-P4-AUTH-006 | Audit events are tamper-evident |
| ACC-P4-AUTH-007 | Audit events include actor, action, resource, result |
| ACC-P4-AUTH-008 | Authorization failures are observable |

---

## 9. Work Package C — Multi-Region Mode A & DR

### 9.1 Objective

Implement and certify Mode A multi-region replication, failover, backup, restore, and PITR.

### 9.2 Deliverables

| ID | Deliverable | Description |
|---|---|---|
| D-P4-C-001 | Region registry | Region roles and stream ownership |
| D-P4-C-002 | Mode A replication engine | Single-writer primary + async replica |
| D-P4-C-003 | HLC causal tagger | Cross-region causal ordering |
| D-P4-C-004 | Region epoch fencing | Split-brain write prevention |
| D-P4-C-005 | Cross-region key propagation | Destroyed-key registry replication |
| D-P4-C-006 | Residency enforcement | Region-bound data/key policies |
| D-P4-C-007 | Planned failover workflow | Graceful region failover |
| D-P4-C-008 | Unplanned failover workflow | Emergency region failover |
| D-P4-C-009 | Conflict branch quarantine | Orphaned write isolation |
| D-P4-C-010 | Backup manager | Backup scope, scheduling, integrity |
| D-P4-C-011 | Restore executor | Full cluster restore |
| D-P4-C-012 | PITR engine | Timestamp-based recovery |
| D-P4-C-013 | Legal hold integration | Suspension of destructive lifecycle |
| D-P4-C-014 | DR drill automation | Repeatable DR exercises |

### 9.3 Multi-Region Acceptance Criteria

| ID | Requirement | Target |
|---|---|---:|
| ACC-P4-MR-001 | RPO normal network | ≤5 seconds |
| ACC-P4-MR-002 | RPO degraded network | ≤60 seconds |
| ACC-P4-MR-003 | RTO planned failover | ≤1 minute |
| ACC-P4-MR-004 | RTO unplanned failover | ≤5 minutes |
| ACC-P4-MR-005 | Region epoch fencing | No split-brain writes |
| ACC-P4-MR-006 | Destroyed-key propagation | All regions confirm destruction |
| ACC-P4-MR-007 | Residency enforcement | No unauthorized cross-region transfer |
| ACC-P4-MR-008 | Backup restore | Destroyed data not resurrected |
| ACC-P4-MR-009 | PITR | Correct state at target timestamp |
| ACC-P4-MR-010 | Legal hold | Destructive operations blocked |

---

## 10. Work Package D — Advanced Queue Gateways

### 10.1 Objective

Implement SQS and AMQP translation gateways as compatibility-by-subset migration paths.

### 10.2 SQS Gateway Scope

| SQS Operation | Phase 4 Target |
|---|---|
| `SendMessage` | Certified |
| `SendMessageBatch` | Certified |
| `ReceiveMessage` | Certified |
| `DeleteMessage` | Certified |
| `DeleteMessageBatch` | Certified |
| `ChangeMessageVisibility` | Certified |
| `ChangeMessageVisibilityBatch` | Certified |
| `GetQueueAttributes` | Certified limited |
| `GetQueueUrl` | Certified limited |
| `ListQueues` | Certified limited |
| `PurgeQueue` | Certified with elevated authorization |
| FIFO `MessageGroupId` ordering | Certified |
| Content-based deduplication | Certified limited |
| Delay timers | Unsupported in v1 |
| DLQ configuration | Replaced by PEF virtual DLQ policy |

### 10.3 AMQP Gateway Scope

| AMQP Feature | Phase 4 Target |
|---|---|
| Direct exchange | Certified |
| Default exchange | Certified |
| Queue declare | Certified |
| Queue bind | Certified limited |
| Basic publish | Certified |
| Basic consume | Certified |
| Basic get | Certified |
| Basic ack | Certified |
| Basic nack | Certified |
| Basic reject | Certified |
| Basic qos/prefetch | Certified limited |
| Dead-letter routing | Mapped to PEF virtual DLQ |
| Topic exchange | Unsupported |
| Fanout exchange | Unsupported |
| Headers exchange | Unsupported |
| AMQP transactions | Unsupported |
| Publisher confirms | Deferred unless explicitly approved |

### 10.4 Queue Gateway Acceptance Criteria

| ID | Requirement |
|---|---|
| ACC-P4-Q-001 | Certified SQS operations pass conformance tests |
| ACC-P4-Q-002 | Certified AMQP operations pass conformance tests |
| ACC-P4-Q-003 | Unsupported operations return explicit errors |
| ACC-P4-Q-004 | FIFO group ordering preserved |
| ACC-P4-Q-005 | Visibility timeout maps to lease TTL |
| ACC-P4-Q-006 | Receipt handle maps to lease token |
| ACC-P4-Q-007 | Stale receipt handles are rejected |
| ACC-P4-Q-008 | Gateway identities map to ABAC principals |
| ACC-P4-Q-009 | Gateway metrics expose operation/version/status |
| ACC-P4-Q-010 | Compatibility matrices published |

---

## 11. Work Package E — Compliance & Operational Hardening

### 11.1 Objective

Prepare Keirox for enterprise operations and compliance readiness.

### 11.2 Deliverables

| ID | Deliverable | Description |
|---|---|---|
| D-P4-E-001 | SOC2 readiness evidence pack | Access control, encryption, audit, incident response |
| D-P4-E-002 | ISO27001 readiness evidence pack | ISMS-aligned controls mapping |
| D-P4-E-003 | Incident response runbooks | Security and operational incidents |
| D-P4-E-004 | Break-glass procedure | Emergency access with audit |
| D-P4-E-005 | Capacity governance | Forecasting, quotas, expansion rules |
| D-P4-E-006 | Alert maturity | Alert-to-runbook mapping |
| D-P4-E-007 | Upgrade certification | N/N-1 rolling upgrades under security controls |
| D-P4-E-008 | Supportability hooks | Diagnostics, safe debug endpoints, redaction |
| D-P4-E-009 | Compliance dashboard | Control evidence status |
| D-P4-E-010 | Data lifecycle governance | Retention, legal hold, erasure workflows |

### 11.3 Compliance Acceptance Criteria

| ID | Requirement |
|---|---|
| ACC-P4-COMP-001 | Access control evidence collected |
| ACC-P4-COMP-002 | Encryption evidence collected |
| ACC-P4-COMP-003 | Audit trail evidence collected |
| ACC-P4-COMP-004 | Incident response evidence collected |
| ACC-P4-COMP-005 | Erasure evidence collected |
| ACC-P4-COMP-006 | Retention and legal hold evidence collected |
| ACC-P4-COMP-007 | Security training/process evidence prepared |
| ACC-P4-COMP-008 | Vendor/KMS dependency controls documented |
| ACC-P4-COMP-009 | Operational runbooks tested |
| ACC-P4-COMP-010 | Break-glass access audited |

**Normative statement:** Keirox provides SOC2/ISO readiness controls. Formal certification depends on external audit scope and organizational processes.

---

## 12. Work Package F — Certification & Release Readiness

### 12.1 Objective

Produce the final v1 evidence package and prepare for release readiness review.

### 12.2 Deliverables

| ID | Deliverable | Description |
|---|---|---|
| D-P4-F-001 | Jepsen-style consistency report | Adversarial distributed tests |
| D-P4-F-002 | Security penetration test report | External or internal pen test |
| D-P4-F-003 | Crypto-shredding certification report | Erasure proof and restore validation |
| D-P4-F-004 | Multi-region DR certification report | RPO/RTO and failover evidence |
| D-P4-F-005 | SQS/AMQP compatibility certification | Conformance and negative tests |
| D-P4-F-006 | Operational readiness report | Runbooks, alerts, upgrades, capacity |
| D-P4-F-007 | Compliance readiness report | SOC2/ISO control evidence summary |
| D-P4-F-008 | Final release readiness checklist | v1 release gate |
| D-P4-F-009 | Known limitations register | Explicit v1 exclusions |
| D-P4-F-010 | Customer-facing safety statements | Supported guarantees and limitations |

### 12.3 Certification Acceptance Criteria

| ID | Requirement |
|---|---|
| ACC-P4-CERT-001 | Jepsen-style tests pass with zero invariant violations |
| ACC-P4-CERT-002 | Pen test findings triaged and critical/high resolved |
| ACC-P4-CERT-003 | Crypto-shredding validated end-to-end |
| ACC-P4-CERT-004 | Multi-region failover validated |
| ACC-P4-CERT-005 | Backup/PITR validated |
| ACC-P4-CERT-006 | SQS/AMQP certified subsets pass |
| ACC-P4-CERT-007 | Operational runbooks tested |
| ACC-P4-CERT-008 | Compliance evidence package approved |
| ACC-P4-CERT-009 | Known limitations documented |
| ACC-P4-CERT-010 | Release readiness checklist approved |

---

## 13. Phase 4 Milestone Schedule

| Milestone | Target Weeks | Deliverables | Exit Criteria |
|---|---|---|---|
| M4.0 Phase 4 Mobilization | 1–2 | Team onboarding, security environment, compliance tracker | Phase 4 environment ready |
| M4.1 Security Foundations | 3–12 | KMS, encryption, DEK cache, destroyed-key registry | Encryption tests pass |
| M4.2 Authorization & Audit | 6–18 | ABAC, principal mapping, audit trail | Default-deny enforced |
| M4.3 Multi-Region Replication | 8–20 | Mode A replication, epoch fencing, residency | Replication evidence produced |
| M4.4 DR & PITR | 14–24 | Backup, restore, PITR, legal hold | DR drill passes |
| M4.5 Queue Gateways | 12–26 | SQS/AMQP gateways and compatibility tests | Certified subsets pass |
| M4.6 Compliance & Ops Hardening | 18–30 | SOC2/ISO readiness, runbooks, alerting | Control evidence collected |
| M4.7 Jepsen & Security Certification | 24–34 | Jepsen-style tests, pen test | Certification reports approved |
| M4.8 Release Readiness | 35–36 | Final checklist, known limitations, release review | v1 readiness decision |

---

## 14. Phase 4 Gates

### 14.1 Gate 4A — Enterprise Prototype Evidence Gate (Week 12)

| Criterion | Mandatory |
|---|---|
| Encryption at rest works | Yes |
| KMS failure fails secure | Yes |
| Destroyed key prevents decryption | Yes |
| Basic ABAC deny works | Yes |
| Mode A replication basic flow works | Yes |
| SQS basic send/receive/delete works | Yes |
| No plaintext fallback | Yes |

### 14.2 Gate 4B — Mid-Phase Enterprise Review (Week 24)

| Criterion | Mandatory |
|---|---|
| Crypto-shredding erases target data | Yes |
| Backup restore respects destroyed keys | Yes |
| Region failover works in planned mode | Yes |
| ABAC default-deny enforced | Yes |
| Audit trail tamper-evident | Yes |
| SQS/AMQP alpha conformance passes | Yes |
| No unresolved Critical security defects | Yes |

### 14.3 Gate 4C — Phase 4 Certification Gate (Week 36)

| Criterion | Mandatory |
|---|---|
| All security acceptance criteria pass | Yes |
| All authorization acceptance criteria pass | Yes |
| All multi-region acceptance criteria pass | Yes |
| All DR/PITR acceptance criteria pass | Yes |
| All queue gateway acceptance criteria pass | Yes |
| Jepsen-style tests pass | Yes |
| Compliance readiness evidence approved | Yes |
| Operational readiness evidence approved | Yes |
| Pen test critical/high findings resolved | Yes |
| Final release readiness checklist approved | Yes |

---

## 15. Dependencies and Prerequisites

### 15.1 Phase 3 Prerequisites

Phase 4 implementation MUST NOT begin until:

1. Phase 3 Gate 3C is certified, or conditional remediation is complete.
2. Kafka gateway certified subset is stable.
3. Native SDK core APIs are stable.
4. Iceberg commit governance is stable.
5. Schema governance is stable.
6. Phase 3 evidence package is approved.

### 15.2 Architecture Dependencies

| Dependency | Document |
|---|---|
| Security architecture | KEI-ARC-025 |
| Encryption and crypto-shredding | KEI-DES-036 |
| Multi-region and DR | KEI-ARC-026 |
| Gateway compatibility matrices | KEI-DES-035 |
| Operations runbooks | KEI-OPS-040 |
| Validation and benchmark plan | KEI-OPS-041 |
| Release readiness checklist | KEI-VAL-052 |
| Requirements traceability | KEI-VAL-051 |

---

## 16. High-Level Team Requirements

| Role | Count | Responsibility |
|---|---:|---|
| Security Engineer | 2 | Encryption, ABAC, audit, erasure |
| Compliance Advisor | 1 | SOC2/ISO readiness evidence |
| Multi-Region/DR Engineer | 1–2 | Mode A replication, failover, PITR |
| Queue Gateway Engineer | 2 | SQS/AMQP gateways |
| SRE / Operations Engineer | 1–2 | Runbooks, alerting, capacity, upgrades |
| Chaos / Jepsen Engineer | 1 | Adversarial consistency tests |
| External Pen Test Provider | Contract | Security penetration testing |
| Security Architect | Advisory | Security design governance |
| Chief Architect | 1 | Cross-phase architecture governance |
| Engineering Program Lead | 1 | Delivery and gates |

Estimated Phase 4 team size: **12–16 engineers/specialists**, depending on Phase 3 carryover and external audit support.

---

## 17. Risks and Mitigations

| Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|
| KMS integration complexity | High | Medium | Use adapter pattern; support multiple providers; early spike |
| Crypto-shredding legal acceptance uncertainty | High | Medium | Document technical erasure; require customer legal review |
| Multi-region failover split-brain | Critical | Low | Region epoch fencing; chaos tests; prefer unavailability |
| Backup restore resurrects destroyed data | Critical | Low | Destroyed-key registry checks on restore; erasure tests |
| SQS/AMQP compatibility sprawl | High | High | Strict compatibility-by-subset; negative tests |
| Jepsen-style tests uncover deep bug | High | Medium | Start adversarial tests early; reserve remediation buffer |
| Compliance evidence collection delays | Medium | High | Start evidence collection early; automate control evidence |
| Security pen test finds critical defect | High | Medium | Schedule pen test before final gate; reserve remediation time |
| Operational runbooks not realistic | Medium | Medium | Run DR drills and incident simulations |
| Phase 4 scope creep into v2 features | High | High | Strict v1 exclusions; ARB change control |

---

## 18. Phase 4 Evidence Package

The Phase 4 evidence package MUST include:

1. Security test report.
2. Penetration test report.
3. Crypto-shredding certification report.
4. Destroyed-key registry validation report.
5. ABAC authorization report.
6. Tenant isolation report.
7. Audit trail validation report.
8. Multi-region replication report.
9. Planned and unplanned failover report.
10. RPO/RTO evidence report.
11. Backup validation report.
12. PITR validation report.
13. Legal hold validation report.
14. SQS compatibility certification report.
15. AMQP compatibility certification report.
16. Jepsen-style consistency report.
17. Compliance readiness evidence pack.
18. Operational readiness report.
19. Known limitations register.
20. Final v1 release readiness recommendation.

---

## 19. Phase 4 Outcomes

| Outcome | Meaning |
|---|---|
| PHASE 4 CERTIFIED | Proceed to v1 release readiness and controlled production rollout |
| CONDITIONALLY CERTIFIED | Proceed after defined remediation |
| EXTENDED | Additional Phase 4 work required |
| RE-SCOPE | v1 scope reduced to preserve safety |
| STOP | Critical enterprise trust assumption failed |

---

## 20. Definition of v1 Release Readiness

Phase 4 completion does not automatically mean public release. It means Keirox is ready for the **v1 Release Readiness Review**.

Release readiness requires:

1. All Phase 4 acceptance criteria pass.
2. All Critical and High risks are resolved or formally accepted.
3. All mandatory evidence is archived.
4. Known limitations are documented.
5. Support and incident response processes are tested.
6. Security and compliance stakeholders approve.
7. Architecture Review Board approves.
8. Product and executive stakeholders approve rollout strategy.

---

## 21. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial Phase 4 Engineering Execution Plan. Defines Phase 4 mission, scope, work packages, milestones, acceptance criteria, gates, dependencies, team requirements, risks, evidence package, and v1 release readiness definition. |