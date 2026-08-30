# KEI-MR-401 — Multi-Region & Disaster Recovery Certification Plan

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-MR-401 |
| Title | Multi-Region & Disaster Recovery Certification Plan |
| Version | 1.0 |
| Level | Engineering Execution Plan |
| Status | Baseline — Ready for Execution |
| Phase | Phase 4 — Enterprise Hardening, Compliance & Multi-Region |
| Duration | Weeks 8–32 of Phase 4 |
| Owner | Multi-Region / Disaster Recovery Lead |
| Governing Plan | KEI-ENG-400 — Phase 4 Engineering Execution Plan |
| Governing Architecture Documents | KEI-ARC-026, KEI-DES-036, KEI-OPS-040, KEI-OPS-041 |
| Predecessor | KEI-SPIKE-401 — Enterprise Hardening Prototype Plan |
| Next Plan File | KEI-QUEUE-401 — SQS & AMQP Gateway Certification Plan |

---

## 2. Executive Summary

Phase 4 must prove that the Keirox Polymorphic Event Fabric can survive catastrophic regional failures, maintain cryptographic erasure across geographic boundaries, and recover deterministically without violating the Golden Invariant or resurrecting destroyed data.

This plan defines the certification program for:

1. **Multi-Region Mode A Replication** — validating single-writer primary, asynchronous WAL tail replication, and Hybrid Logical Clock (HLC) causal ordering.
2. **Region Epoch Fencing** — proving that split-brain network partitions cannot result in conflicting writes being accepted by the surviving topology.
3. **Disaster Recovery (DR)** — validating full cluster restore, Point-in-Time Recovery (PITR), and backup scope completeness.
4. **Legal Hold & Data Residency** — proving that geographic boundaries and compliance suspensions are strictly enforced.
5. **DR Drill Execution** — defining the mandatory, repeatable exercises required to certify operational readiness.

This document replaces informal "multi-region support" claims with strict, evidence-based RPO/RTO measurements and fencing validations.

---

## 3. Purpose and Scope

### 3.1 Purpose

The purpose of this plan is to:

1. Define the certification model for Mode A multi-region replication.
2. Define region epoch fencing and split-brain validation tests.
3. Define RPO (Recovery Point Objective) and RTO (Recovery Time Objective) measurement methodologies.
4. Define backup, restore, and PITR certification requirements.
5. Define legal hold and data residency enforcement tests.
6. Define the DR drill execution matrix.
7. Produce the Phase 4 multi-region and DR evidence package.

### 3.2 Scope

**In scope:**

- Mode A (Single-Writer Primary) replication validation.
- Asynchronous WAL tail replication.
- HLC causal tag validation.
- Region epoch fencing and split-brain quarantine.
- Planned and unplanned region failover.
- Backup scope validation (manifests, state, destroyed-key registry).
- Full cluster restore.
- Point-in-Time Recovery (PITR).
- Legal hold suspension of destructive lifecycle operations.
- Data residency enforcement (blocking unauthorized cross-region transfer).
- DR drill execution and automation.

**Out of scope:**

- Active-active multi-writer same-stream replication (Mode B) — explicitly excluded from v1 by ADR-060.
- Cross-region automatic conflict resolution — orphaned writes are quarantined, not merged.
- KMS and crypto-shredding internals — owned by KEI-SEC-401 (though cross-region key propagation is validated here).

---

## 4. Multi-Region Certification Principles

| ID | Principle | Requirement |
|---|---|---|
| MR-1 | Single-Writer Primary | v1 same-stream replication MUST use Mode A (one active primary region per stream). |
| MR-2 | Prefer Unavailability over Split-Brain | In a partition, the minority side MUST fence itself and reject writes rather than risk divergence. |
| MR-3 | Erasure is Global | Destroyed keys MUST propagate to all regions before erasure is considered complete. |
| MR-4 | Backups are Bounded | Backups MUST include the destroyed-key registry to prevent resurrection of erased data. |
| MR-5 | Residency is Enforced | Data tagged with geographic residency constraints MUST NOT replicate to unauthorized regions. |
| MR-6 | Fencing is Cryptographic | Region epoch transitions MUST be durable and validated before accepting writes. |

---

## 5. Replication & Epoch Fencing Certification

### 5.1 Mode A Replication Requirements

| ID | Requirement |
|---|---|
| REP-001 | Primary region MUST accept writes; Replica region MUST reject direct writes. |
| REP-002 | WAL tails MUST replicate asynchronously to the Replica region. |
| REP-003 | Replication MUST preserve HLC causal ordering. |
| REP-004 | Replication lag MUST be continuously measurable. |
| REP-005 | Replica region MUST be able to reconstruct state from replicated WAL tails and S3 manifests. |

### 5.2 Region Epoch Fencing Requirements

| ID | Requirement |
|---|---|
| FENCE-001 | Every region role transition MUST increment a monotonic `region_epoch`. |
| FENCE-002 | Writes MUST include the current `region_epoch`. |
| FENCE-003 | Nodes MUST reject writes with an epoch lower than the current known epoch. |
| FENCE-004 | A demoted primary MUST NOT accept writes after being fenced. |
| FENCE-005 | Orphaned writes from a split-brain primary MUST be quarantined, not merged. |

### 5.3 Replication and Fencing Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| REP-T-001 | Normal async replication | Replica lag ≤ RPO target (e.g., 5s). |
| REP-T-002 | Replica receives direct write | Write rejected (Read-Only/Fenced). |
| REP-T-003 | Network partition (Primary isolated) | Primary continues (if quorum holds); Replica stalls. |
| REP-T-004 | Split-brain heal (Old Primary reconnects) | Old Primary writes rejected (Epoch Fencing). |
| REP-T-005 | Epoch downgrade attempt | Rejected; security alert emitted. |
| REP-T-006 | HLC causal order validation | Cross-region reads respect causal dependencies. |

---

## 6. Failover Certification (RTO Measurement)

### 6.1 Planned Failover (Maintenance)

**Workflow:**
1. Freeze writes in Primary region.
2. Wait for WAL delta transmission to complete (Lag = 0).
3. Increment `region_epoch` for Replica.
4. Promote Replica to Primary.
5. Demote old Primary to Replica/Read-Only.
6. Redirect client traffic.
7. Resume writes.

**Target:** RTO ≤ 1 minute. Data Loss = 0.

### 6.2 Unplanned Failover (Disaster)

**Workflow:**
1. Detect Primary region unavailability (heartbeat timeout).
2. Verify old Primary is fenced or unreachable.
3. Increment `region_epoch`.
4. Promote Replica to Primary.
5. Recover last available WAL delta (if any).
6. Quarantine conflict branches if split-brain writes occurred.
7. Redirect client traffic.
8. Resume writes.

**Target:** RTO ≤ 5 minutes. Data Loss ≤ RPO target (e.g., 60s degraded).

### 6.3 Failover Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| FO-T-001 | Planned failover | RTO ≤ 1m; zero data loss; epoch incremented. |
| FO-T-002 | Unplanned failover (Primary killed) | RTO ≤ 5m; data loss bounded by RPO; epoch incremented. |
| FO-T-003 | Failover with destroyed keys | Erased data remains unreadable in new Primary. |
| FO-T-004 | Client redirect validation | Clients successfully reconnect to new Primary. |

---

## 7. Disaster Recovery (Backup & PITR) Certification

### 7.1 Backup Scope Requirements

A valid Keirox backup MUST include:

1. Tier-1 Object Storage data (Parquet chunks, WAL tails).
2. Stream manifests and sparse indexes.
3. Metadata Raft snapshots (coordinator assignments, schemas).
4. State Plane snapshots and lease journals.
5. **Destroyed-Key Registry** (Critical for compliance).
6. Audit trail logs.

### 7.2 Restore and PITR Requirements

| ID | Requirement |
|---|---|
| DR-001 | Full cluster restore MUST reconstruct the stream registry and state plane. |
| DR-002 | Restore MUST check the Destroyed-Key Registry before exposing data. |
| DR-003 | PITR MUST reconstruct state to a specific timestamp `T` without leaking post-`T` data. |
| DR-004 | Restore duration MUST be measured and bounded. |

### 7.3 DR Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| DR-T-001 | Full cluster loss and restore | Cluster recovers; all non-erased data accessible. |
| DR-T-002 | Restore of erased tenant | Data remains cryptographically inaccessible (destroyed key). |
| DR-T-003 | PITR to timestamp `T` | State matches `T`; no post-`T` records visible. |
| DR-T-004 | Corrupted backup artifact | Checksum validation fails; restore aborted safely. |

---

## 8. Legal Hold & Data Residency Certification

### 8.1 Legal Hold Requirements

| ID | Requirement |
|---|---|
| HOLD-001 | Legal hold MUST suspend snapshot expiration. |
| HOLD-002 | Legal hold MUST suspend orphan file cleanup. |
| HOLD-003 | Legal hold MUST suspend destructive schema migrations. |
| HOLD-004 | Legal hold release MUST be audited. |

### 8.2 Data Residency Requirements

| ID | Requirement |
|---|---|
| RES-001 | Streams tagged with residency constraints MUST NOT replicate to unauthorized regions. |
| RES-002 | Cross-region replication MUST validate residency policies before transferring data. |
| RES-003 | Residency violations MUST block replication and emit critical alerts. |

### 8.3 Compliance Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| COMP-T-001 | Snapshot expiration under legal hold | Blocked; audit event emitted. |
| COMP-T-002 | Orphan cleanup under legal hold | Blocked. |
| COMP-T-003 | Replicate EU-only stream to US region | Blocked; residency violation alerted. |

---

## 9. DR Drill Execution Plan

To maintain operational readiness, DR drills MUST be executed regularly. Phase 4 MUST automate and certify the following drills.

| Drill ID | Drill Name | Frequency | Success Criteria |
|---|---|---|---|
| DRILL-01 | Coordinator Failover | Monthly | <3.5s recovery, no double lease. |
| DRILL-02 | Storage Node Replacement | Quarterly | <5s recovery, no data loss. |
| DRILL-03 | Planned Region Failover | Quarterly | RTO ≤1m, RPO = 0. |
| DRILL-04 | Unplanned Region Failover | Biannually | RTO ≤5m, RPO ≤60s. |
| DRILL-05 | Full Backup Restore | Biannually | Restore validated, no resurrected erased data. |
| DRILL-06 | PITR Drill | Biannually | Correct state at target timestamp. |
| DRILL-07 | Crypto-Shredding Drill | Quarterly | Key destruction verified globally. |

Phase 4 exit requires successful execution and evidence collection for **DRILL-03, DRILL-04, DRILL-05, and DRILL-06**.

---

## 10. Metrics, Alerts & Observability

### 10.1 Required Metrics

| Metric | Type | Purpose |
|---|---|---|
| `keirox_replication_lag_seconds` | Gauge | Measure RPO. |
| `keirox_region_epoch` | Gauge | Track region role transitions. |
| `keirox_fenced_writes_total` | Counter | Detect split-brain attempts. |
| `keirox_failover_duration_seconds` | Histogram | Measure RTO. |
| `keirox_backup_success_total` | Counter | Track backup health. |
| `keirox_residency_violations_total` | Counter | Detect compliance breaches. |

### 10.2 Required Alerts

| Alert | Condition | Severity |
|---|---|---|
| Replication Lag Critical | Lag > RPO target | Critical |
| Region Epoch Mismatch | Node reports epoch < cluster epoch | Critical |
| Residency Viol | Unauthorized cross-region transfer | Critical |
| Backup Failure | Backup job fails | Critical |
| Legal Hold Bypass Attempt | Destructive op attempted on held data | Critical |

---

## 11. Certification Levels & Gates

| Level | Name | Requirement |
|---|---|---|
| L1 | Replication Certified | Mode A async replication and HLC ordering validated. |
| L2 | Fencing Certified | Region epoch fencing and split-brain quarantine validated. |
| L3 | Failover Certified | Planned and unplanned RTO/RPO targets met. |
| L4 | DR Certified | Full restore and PITR validated; destroyed keys respected. |
| L5 | Compliance Certified | Legal hold and data residency enforcement validated. |

Phase 4 exit requires **L1 through L5**.

---

## 12. Deliverables & Milestones

| Deliverable | Description | Target Week |
|---|---|---:|
| D-MR-001 | Mode A Replication Certification Suite | Week 14 |
| D-MR-002 | Region Epoch Fencing Chaos Tests | Week 18 |
| D-MR-003 | Failover RTO/RTO Measurement Harness | Week 20 |
| D-MR-004 | Backup and Restore Validation Suite | Week 22 |
| D-MR-005 | PITR Certification Suite | Week 24 |
| D-MR-006 | Legal Hold and Residency Tests | Week 26 |
| D-MR-007 | Automated DR Drill Framework | Week 28 |
| D-MR-008 | Final Multi-Region & DR Evidence Package | Week 32 |

---

## 13. Risks and Mitigations

| Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|
| Split-brain writes accepted during partition | Critical | Low | Strict epoch fencing; chaos testing; prefer unavailability. |
| Backup restore resurrects erased data | Critical | Low | Mandatory destroyed-key registry check on restore. |
| Replication lag exceeds RPO under load | High | Medium | Backpressure integration; S3 hash-prefixing; alert tuning. |
| HLC clock skew causes causal violations | High | Medium | NTP enforcement; HLC bounds checking; chaos clock skew tests. |
| DR drills cause production impact | Medium | High | Isolated staging environments for Phase 4 drills; runbooks for prod. |
| Residency policy misconfiguration | High | Medium | Strict default-deny residency; audit logging of policy changes. |

---

## 14. Evidence Package

The Multi-Region & DR evidence package MUST include:

1. Mode A replication lag report.
2. Region epoch fencing chaos report.
3. Planned failover RTO/RPO report.
4. Unplanned failover RTO/RPO report.
5. Full cluster restore report.
6. PITR validation report.
7. Destroyed-key backup interaction report.
8. Legal hold enforcement report.
9. Data residency enforcement report.
10. DR drill execution logs and sign-offs.

---

## 15. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial Multi-Region & DR Certification Plan. Defines Mode A replication, epoch fencing, RPO/RTO measurement, backup/PITR certification, legal hold, residency enforcement, and DR drill execution requirements. |