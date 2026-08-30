# KEI-OPS-040 — Operations Runbooks, Upgrade & DR Procedures

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-OPS-040 |
| Title | Operations Runbooks, Upgrade & DR Procedures |
| Version | 1.0 |
| Level | **L3 — Operations Specification** |
| Subsystem Covered | Operations, Lifecycle Management, Disaster Recovery Execution |
| Status | Approved for Engineering |
| Classification | Internal / Operations Confidential |
| Owner | SRE Lead / Platform Operations Lead |
| Required Reviewers | Chief Architect, Principal Engineer (Storage), Principal Engineer (Distributed Systems), Security Lead, DR Owner |
| Depends On | KEI-ARC-020..027 (Subsystem Architectures), KEI-DES-030..036 (Detailed Design Specifications), KEI-ARC-011 (NFRs), KEI-ARC-012 (ADRs) |
| Consumed By | SRE, on-call engineers, release managers, DR owners, security incident responders, platform support, compliance auditors |
| Keywords | MUST, MUST NOT, SHOULD, SHOULD NOT, MAY per RFC 2119 |

---

## 2. Purpose, Scope, and Audience

### 2.1 Purpose

This document specifies the **operational runbooks, upgrade procedures, disaster recovery procedures, and maintenance workflows** required to operate the Polymorphic Event Fabric safely in production.

It operationalizes:

- KEI-ARC-020 storage recovery and backpressure behavior.
- KEI-ARC-021 state-plane failover and watermark safety.
- KEI-ARC-022 consensus, membership, and epoch fencing.
- KEI-ARC-023 lakehouse commit lag and quarantine handling.
- KEI-ARC-024 gateway compatibility degradation.
- KEI-ARC-025 security incident and crypto-shredding governance.
- KEI-ARC-026 multi-region replication and disaster recovery.
- KEI-ARC-027 observability, quotas, and lifecycle management.

### 2.2 Scope

**In scope:**

- Operational roles and responsibilities.
- Runbook governance and execution model.
- Incident severity classification.
- Routine maintenance procedures.
- Rolling upgrade procedures.
- Node replacement and cluster scaling.
- Coordinator shard failover.
- Planned and unplanned region failover.
- Backup validation, restore, and point-in-time recovery.
- Crypto-shredding execution runbook.
- S3 outage and compaction backlog response.
- Security incident response integration.
- Emergency shedding and recovery.
- Abort criteria and post-incident validation.

**Out of scope:**

- Internal subsystem design rationale — owned by L2 documents.
- Binary formats and internal algorithms — owned by L3 design specs.
- Benchmark and chaos test definitions — owned by KEI-OPS-041.
- Customer-specific support policies.

### 2.3 Audience

- SRE and platform operations engineers.
- On-call incident responders.
- Release managers.
- Disaster recovery owners.
- Security and compliance responders.
- Engineering escalation contacts.

---

## 3. Operational Design Principles

| ID | Principle | Required Behavior |
|---|---|---|
| OP-1 | **Safety before speed.** | No runbook may prioritize speed over data durability or split-brain prevention. |
| OP-2 | **Automate only what is provably safe.** | Automated actions must have guardrails, health checks, and abort criteria. |
| OP-3 | **Every destructive operation requires authorization.** | Deletion, crypto-shredding, region failover, and restore operations require explicit approval. |
| OP-4 | **Every runbook is testable.** | Each runbook must be exercised in DR drills or staging validation. |
| OP-5 | **Every incident produces evidence.** | Metrics, logs, audit events, and decision records must be preserved. |
| OP-6 | **Fail secure and fail observable.** | Failures must deny unsafe behavior and emit clear operational signals. |
| OP-7 | **No silent recovery.** | Recovery operations must be visible, audited, and validated. |
| OP-8 | **Legal hold overrides lifecycle automation.** | Legal hold suspends expiration, orphan cleanup, and destructive maintenance. |

---

## 4. Operational Roles and Responsibilities

| Role | Responsibility |
|---|---|
| **SRE On-Call** | First responder for alerts; executes approved runbooks; escalates as required. |
| **Incident Commander** | Owns incident coordination, communication, and decision sequencing. |
| **Storage Engineer Escalation** | Handles WAL, Tier-0/Tier-1, compaction, and recovery edge cases. |
| **State Plane Engineer Escalation** | Handles coordinator, lease, bitmap, and watermark anomalies. |
| **Security Lead** | Authorizes security-sensitive actions and leads key-compromise response. |
| **DR Owner** | Authorizes region failover and DR restoration. |
| **Compliance Officer** | Verifies legal hold, erasure approvals, and audit requirements. |
| **Release Manager** | Owns rolling upgrades and feature flag rollout approval. |
| **Chief Architect** | Final escalation for architecture-invariant violations. |

### 4.1 Two-Person Rule

The following operations MUST require two-person authorization:

- Crypto-shredding execution.
- Tenant deletion.
- Region failover.
- Restore over existing production data.
- Manual orphan-file deletion.
- Manual snapshot expiration.
- Emergency global shedding.
- Disabling encryption or security controls, which SHOULD be prohibited unless explicitly approved by security and compliance.

---

## 5. Runbook Governance Model

### 5.1 Runbook Lifecycle States

```text
DRAFT
  │
  ▼
REVIEWED
  │
  ▼
APPROVED
  │
  ▼
TESTED
  │
  ▼
PRODUCTION-READY
  │
  ▼
EXECUTED / ABORTED / COMPLETED
```

### 5.2 Automation Levels

| Level | Definition | Requirement |
|---:|---|---|
| L0 | Manual procedure with checklist | Operator executes every step. |
| L1 | Scripted procedure | Script executes steps but requires operator confirmation. |
| L2 | Semi-automated | Automated execution with approval gates and abort conditions. |
| L3 | Fully automated | Automated with guardrails, health checks, and audit logging. |

**Normative rule:** Destructive operations MUST NOT be fully automated without explicit policy approval and guardrails.

### 5.3 Runbook Required Fields

Every runbook MUST include:

1. Runbook ID.
2. Trigger alerts or conditions.
3. Severity classification.
4. Required roles and approvals.
5. Preconditions.
6. Step-by-step actions.
7. Verification checks.
8. Abort criteria.
9. Rollback or recovery path.
10. Evidence to collect.
11. Post-action validation.

---

## 6. Incident Severity Classification

| Severity | Definition | Response Time Target | Example |
|---|---|---|---|
| SEV-1 | Data loss risk, full outage, split-brain risk, security breach | Immediate | Quorum loss, region failure, key compromise. |
| SEV-2 | Degraded durability, failover degraded, SLO burn critical | <15 minutes | Coordinator failover failure, S3 backlog critical. |
| SEV-3 | Partial degradation, elevated latency, one subsystem impaired | <1 hour | Compaction lag, gateway throttling, bitmap spill pressure. |
| SEV-4 | Minor degradation or warning | <4 hours | Elevated cache misses, non-critical alert. |
| SEV-5 | Informational or planned maintenance | Scheduled | Version upgrade, DR drill. |

**Normative rule:** Any condition that risks loss of quorum-committed data or conflicting lease issuance MUST be treated as SEV-1 until proven otherwise.

---

## 7. Runbook Catalog

| Runbook ID | Title | Category | Target Automation |
|---|---|---|---|
| OPS-RB-001 | Storage Node Failure and Replacement | Storage | L2 |
| OPS-RB-002 | Coordinator Shard Failover | State Plane | L2 |
| OPS-RB-003 | Rolling Upgrade | Lifecycle | L2 |
| OPS-RB-004 | Feature Flag Rollout and Rollback | Lifecycle | L2 |
| OPS-RB-005 | Cluster Expansion | Capacity | L1 |
| OPS-RB-006 | Cluster Shrink | Capacity | L1 |
| OPS-RB-007 | Planned Region Failover | DR | L1 |
| OPS-RB-008 | Unplanned Region Failover | DR | L1 |
| OPS-RB-009 | Backup Validation | DR | L2 |
| OPS-RB-010 | Full Cluster Restore | DR | L1 |
| OPS-RB-011 | Point-in-Time Recovery | DR | L1 |
| OPS-RB-012 | Crypto-Shredding Execution | Security/Compliance | L1 |
| OPS-RB-013 | Suspected Key Compromise | Security | L1 |
| OPS-RB-014 | S3 Outage or Throttling | Storage/Lakehouse | L2 |
| OPS-RB-015 | Compaction or Iceberg Commit Backlog | Lakehouse | L2 |
| OPS-RB-016 | Quota Violation or Noisy Tenant | Multi-tenancy | L2 |
| OPS-RB-017 | Emergency Shedding and Recovery | Availability | L1 |
| OPS-RB-018 | Gateway Protocol Anomaly | Ecosystem | L1 |
| OPS-RB-019 | Stuck Watermark or Lease Leak | State Plane | L2 |
| OPS-RB-020 | Orphan File Cleanup | Lakehouse | L2 |

---

# 8. Core Runbooks

---

## 8.1 OPS-RB-001 — Storage Node Failure and Replacement

### Trigger

- Storage node heartbeat loss.
- Data-plane Raft member unreachable.
- NVMe I/O errors.
- Node crash or kernel panic.

### Severity

SEV-2 if quorum remains healthy.  
SEV-1 if quorum is degraded or multiple nodes failed.

### Preconditions

- At least one healthy data-plane replica remains.
- Cluster membership state is consistent.
- Tier-1 manifests are accessible.

### Procedure

1. Confirm node failure through membership and Raft health.
2. Verify data-plane quorum status.
3. If quorum is lost, stop writes to affected volumes and escalate to SEV-1.
4. Mark failed node as drained and removed from scheduling.
5. Provision replacement node with compatible version.
6. Join replacement node to cluster membership.
7. Restore node state:
   - Fetch Tier-1 manifests.
   - Reconstruct stream registry.
   - Replay active WAL delta from peers.
8. Validate checksums and segment headers.
9. Add replacement node as follower.
10. Allow catch-up replication.
11. Return node to active service.
12. Verify write path p99 latency and backlog metrics.

### Verification

- Node appears healthy in membership.
- Raft replication lag returns to zero.
- No checksum failures.
- No WAL replay invariant violations.
- Write latency returns to SLO band.

### Abort Criteria

- Tier-1 manifests unavailable.
- WAL delta unavailable and recovery gap detected.
- Checksum validation failures.
- Replacement node version incompatible.

### Evidence

- Membership events.
- Raft logs.
- Recovery duration metric.
- Checksum validation report.

---

## 8.2 OPS-RB-002 — Coordinator Shard Failover

### Trigger

- Coordinator node failure.
- Coordinator epoch mismatch.
- Lease journal replication lag critical.
- State shard unresponsive.

### Severity

SEV-2 normally.  
SEV-1 if split-brain or double-lease risk is detected.

### Preconditions

- Metadata Raft is healthy.
- Latest state snapshot and lease journal are available.
- Successor coordinator can be assigned.

### Procedure

1. Confirm coordinator failure.
2. Verify no other coordinator claims the same shard.
3. Increment coordinator epoch via metadata plane.
4. Assign shard to successor coordinator.
5. Restore shard state:
   - Load latest snapshot.
   - Replay lease journal after snapshot LSN.
   - Rebuild timing wheel from active leases.
6. Validate state invariants:
   - No leased offset is ACKED.
   - No leased offset is DLQ.
   - `W_base` points to first non-terminal offset.
7. Reject stale-epoch requests.
8. Resume lease and ACK processing.
9. Monitor watermark lag and lease age.

### Verification

- Coordinator failover completes within target.
- No duplicate leases observed.
- No watermark regression.
- No unbounded lease growth.

### Abort Criteria

- Snapshot corruption.
- Lease journal gap.
- Epoch conflict unresolved.
- Metadata Raft unhealthy.

### Evidence

- Coordinator epoch transition.
- Snapshot and journal replay report.
- Duplicate lease detection report.
- Watermark lag metric.

---

## 8.3 OPS-RB-003 — Rolling Upgrade

### Trigger

- Approved software release.
- Security patch.
- Feature enablement requiring binary upgrade.

### Severity

SEV-5 planned.

### Preconditions

- Cluster health green.
- Backup validation current.
- SLO error budget sufficient.
- Upgrade approved by release manager.
- Target version supports N-1 compatibility.

### Procedure

1. Freeze destructive lifecycle operations during upgrade window.
2. Validate current cluster health.
3. Select one node for upgrade.
4. Enter drain mode.
5. Stop new stream and coordinator assignments.
6. Wait for active leases to expire or transfer.
7. Flush state snapshots and lease journals.
8. Confirm no in-flight WAL batches remain uncommitted.
9. Stop node process.
10. Deploy new binary.
11. Start node and verify version.
12. Rejoin cluster.
13. Catch up Raft and manifests.
14. Validate health checks.
15. Exit drain mode.
16. Repeat for remaining nodes.
17. After all nodes upgraded, enable feature flags if approved.

### Verification

- All nodes report target version.
- No data loss.
- No lease inconsistency.
- No WAL format errors.
- Client error rate remains within threshold.

### Abort Criteria

- Health check failure after node upgrade.
- Invariant violation.
- Persistent client error spike.
- Raft quorum instability.

### Rollback

1. Stop feature flags.
2. Drain upgraded node.
3. Deploy previous supported binary.
4. Rejoin cluster.
5. Verify compatibility with remaining nodes.
6. Resume normal operations.

### Evidence

- Upgrade log per node.
- Version inventory.
- Health dashboards.
- Client error metrics.

---

## 8.4 OPS-RB-004 — Feature Flag Rollout and Rollback

### Trigger

- New capability release.
- Tenant-specific feature request.
- Emergency kill-switch activation.

### Severity

SEV-5 planned, or SEV-2 if emergency rollback.

### Procedure

1. Verify feature flag schema and default state.
2. Approve rollout scope:
   - Global, tenant, stream, percentage.
3. Enable flag in staging first.
4. Validate behavior.
5. Enable in production incrementally.
6. Monitor error rate, latency, and invariant metrics.
7. If anomaly detected, disable flag immediately.
8. Record audit event.

### Abort / Rollback Criteria

- Error rate increase beyond threshold.
- Latency SLO breach.
- State invariant violation.
- Security or quota anomaly.

### Evidence

- Feature flag change audit.
- Metric snapshots before and after.
- Rollback reason if triggered.

---

## 8.5 OPS-RB-005 — Cluster Expansion

### Trigger

- Capacity forecast threshold.
- New tenant onboarding.
- Sustained ingress growth.

### Preconditions

- Capacity predictor recommends expansion.
- Network and rack/AZ topology planned.
- Quorum configuration reviewed.

### Procedure

1. Validate current cluster health.
2. Provision new nodes with approved version.
3. Join nodes to membership.
4. Assign storage volumes and coordinator shard capacity.
5. Rebalance coordinator shards gradually.
6. Rebalance stream placement if required.
7. Monitor replication lag and latency.
8. Validate tenant quota headroom.
9. Update capacity dashboard.

### Verification

- No data loss.
- No lease conflicts.
- Rebalancing completes without SLO breach.
- Capacity metrics improve as expected.

### Abort Criteria

- Membership instability.
- Rebalance causes hotspot.
- Replication lag exceeds safe threshold.

---

## 8.6 OPS-RB-006 — Cluster Shrink

### Trigger

- Cost optimization.
- Decommissioning hardware.
- Reduced workload.

### Preconditions

- Capacity forecast confirms safe headroom.
- Backups validated.
- No legal hold blocks data movement.

### Procedure

1. Validate target capacity after shrink.
2. Drain nodes to be removed.
3. Migrate coordinator shards.
4. Reassign stream placement.
5. Ensure all WAL deltas are replicated or offloaded.
6. Verify Tier-1 manifest completeness.
7. Remove nodes from membership.
8. Decommission hardware or VMs.
9. Validate cluster health.

### Abort Criteria

- Insufficient capacity headroom.
- Data migration failure.
- Quorum risk.
- Legal hold violation.

---

## 8.7 OPS-RB-007 — Planned Region Failover

### Trigger

- Scheduled DR drill.
- Regional maintenance.
- Cost or capacity migration.

### Severity

SEV-5 planned.

### Preconditions

- Replica region replication lag within RPO target.
- Region epoch control available.
- DNS/service mesh redirect plan approved.
- DR owner authorization.

### Procedure

1. Verify replica region health.
2. Verify replication lag.
3. Freeze writes in primary region.
4. Wait for WAL delta transmission to complete.
5. Confirm final manifest synchronization.
6. Increment region epoch for replica.
7. Promote replica region to primary.
8. Demote old primary to replica/read-only.
9. Redirect producers and consumers.
10. Validate stream registry and coordinator assignments.
11. Resume writes.
12. Monitor RPO/RTO telemetry.

### Verification

- Planned failover completes within RTO target.
- No unreplicated writes accepted after freeze.
- No split-brain writes.
- Consumer redirects successful.

### Abort Criteria

- Replica lag exceeds safe threshold.
- Region epoch fencing unavailable.
- Redirect mechanism failure.
- Manifest inconsistency.

---

## 8.8 OPS-RB-008 — Unplanned Region Failover

### Trigger

- Primary region outage.
- Cloud provider regional failure.
- Network partition isolating primary.

### Severity

SEV-1.

### Preconditions

- Control plane can confirm primary unavailability or fence it.
- Replica region is healthy.
- DR owner authorization, unless automated policy permits emergency failover.

### Procedure

1. Declare regional incident.
2. Confirm primary region is unavailable or fenceable.
3. Verify replica region state:
   - Manifests available.
   - WAL delta buffer available.
   - Destroyed-key registry synchronized.
   - Legal holds synchronized.
4. If old primary cannot be fenced, prefer unavailability over split-brain.
5. Increment region epoch.
6. Promote replica region.
7. Fence old primary at network/control-plane level.
8. Recover last WAL delta if available.
9. Quarantine conflict branches if split-brain writes occurred.
10. Redirect clients.
11. Validate writes and reads.
12. Begin post-failover reconciliation.

### Verification

- Failover completes within unplanned RTO target.
- Data loss bounded by RPO target.
- No destroyed data becomes accessible.
- Conflict branches quarantined if present.

### Abort Criteria

- Cannot fence old primary.
- Replica manifest corruption.
- Destroyed-key registry unavailable.
- Legal hold integrity cannot be verified.

### Evidence

- Region epoch transition.
- Replication lag at failover.
- Data loss estimate.
- Conflict branch report.

---

## 8.9 OPS-RB-009 — Backup Validation

### Trigger

- Scheduled validation cadence.
- Before major upgrade.
- After major configuration change.

### Procedure

1. Select backup set.
2. Validate checksums for all artifacts.
3. Restore to isolated validation cluster.
4. Verify:
   - Stream manifests.
   - Raft snapshots.
   - Schema registry.
   - Destroyed-key registry.
   - Quota and policy configuration.
5. Verify destroyed keys remain inaccessible.
6. Run read-only smoke tests.
7. Record validation report.

### Success Criteria

- All checksums valid.
- Restore completes within target.
- No destroyed data resurrected.
- Smoke tests pass.

### Failure Handling

- If backup invalid, escalate to SEV-2.
- Initiate backup remediation.
- Verify alternative backup region.

---

## 8.10 OPS-RB-010 — Full Cluster Restore

### Trigger

- Total cluster loss.
- Catastrophic data corruption.
- DR declaration.

### Severity

SEV-1.

### Preconditions

- Backup set available.
- Replacement infrastructure provisioned.
- DR owner authorization.
- Security and compliance approval if production tenant data is involved.

### Procedure

1. Provision replacement cluster.
2. Restore metadata and Raft snapshots.
3. Restore schema registry.
4. Restore stream manifests.
5. Validate Tier-1 object storage availability.
6. Rebuild stream registry from manifests.
7. Replay WAL tails if available.
8. Validate checksums.
9. Verify destroyed-key registry.
10. Restore quota and policy configuration.
11. Validate encryption key availability for non-erased data.
12. Resume read-only traffic.
13. Validate data integrity.
14. Resume write traffic.
15. Monitor SLO recovery.

### Verification

- All targeted streams accessible.
- No destroyed data accessible.
- Checksum validation passes.
- Client traffic recovers.

### Abort Criteria

- Backup corruption.
- Destroyed-key registry unavailable.
- Object storage unavailable.
- Encryption key availability cannot be verified.

---

## 8.11 OPS-RB-011 — Point-in-Time Recovery

### Trigger

- Logical corruption.
- Accidental deletion.
- Bad producer data window.
- Compliance recovery request.

### Preconditions

- Target timestamp identified.
- Retention window covers target timestamp.
- Backup and WAL tail availability confirmed.
- Legal hold and erasure state verified.

### Procedure

1. Identify target timestamp `T`.
2. Select latest snapshot before `T`.
3. Restore metadata and manifests as of `T`.
4. Replay WAL tails up to `T`.
5. Exclude data committed after `T`.
6. Validate destroyed-key registry as of `T`.
7. Present recovered state to validation cluster.
8. Compare expected streams and offsets.
9. Approve promotion or data export.
10. If overwriting production, require two-person authorization.

### Verification

- Recovered state matches expected timestamp.
- No post-`T` data visible.
- No destroyed data resurrected.
- Audit trail complete.

### Abort Criteria

- WAL tail gap.
- Snapshot missing.
- Destroyed-key registry inconsistent.
- Legal hold conflict.

---

## 8.12 OPS-RB-012 — Crypto-Shredding Execution

### Trigger

- Approved GDPR/CCPA erasure request.
- Tenant deletion.
- Stream deletion with compliance requirement.

### Severity

SEV-3 scheduled, unless legal deadline makes it SEV-2.

### Preconditions

- Erasure request authorized.
- Legal hold check passed.
- Target stream/tenant identified.
- Security lead approval.
- Compliance officer approval.

### Procedure

1. Validate erasure request and approvals.
2. Check legal hold status.
3. Identify target keys:
   - Stream DEK, or
   - Stream-Batch DEKs, or
   - Tenant KEK.
4. Freeze writes to target streams.
5. Remove keys from DEK cache in all regions.
6. Command KMS to destroy keys.
7. Record destruction receipts.
8. Add keys to destroyed-key registry.
9. Write erasure tombstones.
10. Propagate tombstones to all regions.
11. Verify no region can unwrap destroyed keys.
12. Block future commits for tombstoned streams.
13. Schedule physical cleanup during lifecycle.
14. Generate erasure proof report.

### Verification

- Destroyed keys cannot be unwrapped.
- Reads against erased data fail securely.
- Destroyed-key registry updated globally.
- Audit evidence complete.

### Abort Criteria

- Legal hold active.
- Approval missing.
- KMS destruction fails.
- Region propagation incomplete.

### Evidence

- Erasure ticket.
- KMS destruction receipts.
- Tombstone metadata.
- Audit log references.

---

## 8.13 OPS-RB-013 — Suspected Key Compromise

### Trigger

- KMS anomaly.
- Unexpected DEK access.
- Host compromise.
- Insider threat alert.

### Severity

SEV-1.

### Procedure

1. Declare security incident.
2. Isolate affected hosts.
3. Identify affected keys and tenants.
4. Revoke cached DEKs.
5. Rotate Tenant KEK if necessary.
6. Destroy compromised DEKs if erasure is required.
7. Re-encrypt active data if retention and policy require.
8. Audit all access to affected keys.
9. Notify security, compliance, and customer stakeholders per policy.
10. Document incident and remediation.

### Verification

- Compromised keys unavailable.
- No unauthorized reads detected after mitigation.
- Rotation/destruction complete.
- Audit trail preserved.

---

## 8.14 OPS-RB-014 — S3 Outage or Throttling

### Trigger

- S3 503 errors.
- Upload backlog rising.
- Tier-1 offload lag.
- NVMe backlog ETA decreasing.

### Severity

SEV-3 initially.  
SEV-2 if NVMe backlog ETA below safe threshold.  
SEV-1 if durability risk emerges.

### Procedure

1. Confirm S3 outage/throttling scope.
2. Check NVMe backlog ETA.
3. Verify backpressure ladder status.
4. Reduce commit frequency for lakehouse if needed.
5. Increase upload batch size if appropriate.
6. Enable jittered exponential backoff.
7. Apply hash-prefix repartitioning if hotspot detected.
8. If NVMe >80%, engage TCP clamping.
9. If NVMe >95%, prepare emergency shedding.
10. Notify tenants if SLO impact expected.
11. Monitor recovery.

### Verification

- No NVMe corruption.
- No loss of committed writes.
- Upload backlog drains after S3 recovery.
- Backpressure stage returns to normal.

### Abort / Escalation

- NVMe backlog ETA below safe threshold.
- Backpressure fails to stabilize storage.
- Committed writes at risk.

---

## 8.15 OPS-RB-015 — Compaction or Iceberg Commit Backlog

### Trigger

- `keirox_compaction_lag_seconds` elevated.
- `keirox_iceberg_snapshot_age_seconds` above target.
- Pending Parquet files growing.
- Quarantine backlog increasing.

### Procedure

1. Check compaction CPU and arena residency.
2. Verify core isolation is intact.
3. Check Iceberg catalog latency.
4. Verify schema evolution failures.
5. If schema conflict, quarantine incompatible file sets.
6. If catalog overloaded, increase commit batch size.
7. If small-file count rising, verify aggregator target size.
8. If commits fail repeatedly, inspect catalog lock/conflict metrics.
9. Escalate to lakehouse engineer if unrecoverable.

### Verification

- Compaction lag decreasing.
- Snapshot age returning to target.
- No growing quarantine backlog.
- No data loss.

---

## 8.16 OPS-RB-016 — Quota Violation or Noisy Tenant

### Trigger

- Tenant quota rejection spike.
- Ingress rate exceeds tenant limit.
- State-plane memory pressure from one tenant.
- Gateway throttling concentrated by tenant.

### Procedure

1. Identify tenant and affected streams.
2. Verify quota configuration.
3. Check whether violation is expected growth or abuse.
4. Apply protocol throttling if not already active.
5. If critical, temporarily reduce tenant priority.
6. Notify tenant owner if required.
7. Adjust quota only through approved change process.
8. Monitor recovery.

### Verification

- Fabric stability preserved.
- Other tenants unaffected.
- Tenant action audited.

---

## 8.17 OPS-RB-017 — Emergency Shedding and Recovery

### Trigger

- NVMe >95%.
- Memory pressure critical.
- Severe compaction or S3 backlog.
- Imminent storage corruption.

### Severity

SEV-1.

### Preconditions

- Backpressure ladder already engaged.
- Emergency shedding approved by incident commander unless automated policy permits.

### Procedure

1. Confirm emergency condition.
2. Identify non-critical streams and tenants.
3. Enable priority shedding for low-priority traffic.
4. Preserve critical streams.
5. Reject new ingress where required.
6. Communicate service degradation.
7. Drain backlog.
8. Restore normal admission gradually.
9. Validate no committed data lost.
10. Produce incident report.

### Verification

- Storage pressure relieved.
- Critical streams continue.
- No WAL corruption.
- Committed data intact.

### Evidence

- Shedding decisions.
- Priority classifications.
- Affected streams/tenants.
- Recovery metrics.

---

## 8.18 OPS-RB-018 — Gateway Protocol Anomaly

### Trigger

- Unsupported request spike.
- Protocol version mismatch.
- Client compatibility regression.
- Gateway translation errors.

### Procedure

1. Identify affected protocol: Kafka, SQS, AMQP.
2. Identify client versions and API versions.
3. Check compatibility matrix.
4. If unsupported operation, return explicit error and log.
5. If gateway bug, disable affected feature flag if available.
6. If widespread, roll back gateway or disable experimental compatibility.
7. Notify affected clients if required.

### Verification

- Error rate returns to baseline.
- Supported clients unaffected.
- Unsupported behavior explicitly rejected.

---

## 8.19 OPS-RB-019 — Stuck Watermark or Lease Leak

### Trigger

- `keirox_watermark_lag_offsets` growing.
- Bitmap memory increasing without progress.
- Lease age p99 abnormal.
- DLQ eviction rate abnormal.

### Procedure

1. Identify affected state shard and consumer group.
2. Inspect oldest unacked offset.
3. Check lease expiry and retry counts.
4. Verify mandatory DLQ eviction policy.
5. If worker dead, wait for lease timeout or force lease reap if approved.
6. If poison pill, verify DLQ transition.
7. If watermark still stuck, validate bitmap integrity.
8. If corruption suspected, restore shard from snapshot + journal.
9. Escalate to state-plane engineer.

### Verification

- Watermark advances.
- Bitmap memory stabilizes.
- No lease duplication.
- No invariant violation.

---

## 8.20 OPS-RB-020 — Orphan File Cleanup

### Trigger

- Scheduled maintenance.
- Orphan file metric rising.
- Post-failure cleanup.

### Preconditions

- Catalog metadata healthy.
- No active commit reconciliation in progress.
- No legal hold blocks cleanup.
- Orphan grace period satisfied.

### Procedure

1. Run orphan scan in dry-run mode.
2. Review candidate list.
3. Exclude pending commits.
4. Exclude legal holds.
5. Exclude objects younger than grace period.
6. Approve deletion if required.
7. Execute rate-limited deletion.
8. Audit deleted objects.
9. Verify table metadata remains healthy.

### Verification

- Orphan count decreases.
- No active snapshot broken.
- Query engines can read active tables.

### Abort Criteria

- Catalog unhealthy.
- Pending commit uncertainty.
- Legal hold conflict.
- Dry-run mismatch unexpected.

---

# 9. Change Management

## 9.1 Change Types

| Change Type | Examples | Approval |
|---|---|---|
| Standard | Metrics threshold tuning, dashboard change | Peer review |
| Normal | Rolling upgrade, cluster expansion | Release manager + SRE lead |
| Major | Region failover, restore over production, crypto-shredding | DR owner + security/compliance |
| Emergency | SEV-1 mitigation, emergency shedding | Incident commander + available approver |

## 9.2 Change Freeze Conditions

Changes SHOULD be frozen when:

- Active SEV-1 or SEV-2 incident.
- SLO error budget exhausted.
- DR drill in progress.
- Legal hold instability detected.
- Backup validation failed.
- Quorum health degraded.

---

# 10. Observability Requirements for Operations

## 10.1 Mandatory Dashboards

| Dashboard | Key Panels |
|---|---|
| Cluster Health | Node status, Raft quorum, coordinator epochs, SLO burn. |
| Storage | WAL latency, NVMe usage, backlog ETA, S3 upload backlog. |
| State Plane | Active leases, watermark lag, bitmap memory, spill bytes. |
| Lakehouse | Snapshot age, commit latency, quarantine, orphan count. |
| Gateways | Request rate by API/version, unsupported requests, auth failures. |
| Security | KMS errors, DEK cache hits, crypto-shred events, authorization denials. |
| DR | Replication lag, region epoch, failover readiness. |

## 10.2 Alert-to-Runbook Mapping

| Alert | Runbook |
|---|---|
| Storage node down | OPS-RB-001 |
| Coordinator failover failure | OPS-RB-002 |
| Upgrade health failure | OPS-RB-003 |
| Capacity forecast threshold | OPS-RB-005 |
| Region replication lag | OPS-RB-007/008 |
| Backup validation failure | OPS-RB-009 |
| Crypto-shred propagation failure | OPS-RB-012 |
| S3 throttling | OPS-RB-014 |
| Iceberg commit backlog | OPS-RB-015 |
| Noisy tenant | OPS-RB-016 |
| Emergency shedding | OPS-RB-017 |
| Gateway unsupported spike | OPS-RB-018 |
| Stuck watermark | OPS-RB-019 |
| Orphan file growth | OPS-RB-020 |

---

# 11. DR Drill Requirements

| Drill | Frequency | Success Criteria |
|---|---|---|
| Coordinator failover drill | Monthly | Failover <3.5s, no double lease. |
| Storage node replacement drill | Quarterly | Recovery <5s, no data loss. |
| Planned region failover | Quarterly | RTO ≤1min, RPO ≤5s. |
| Unplanned region failover simulation | Quarterly | RTO ≤5min, RPO ≤60s degraded. |
| Backup restore drill | Quarterly | Restore validated, no destroyed data resurrected. |
| PITR drill | Quarterly | Correct state at target timestamp. |
| Crypto-shredding drill | Quarterly | Key destruction verified globally. |
| Chaos test integration | Per KEI-OPS-041 | No invariant violations. |

---

# 12. Post-Incident and Post-Execution Validation

After every runbook execution, the following MUST be verified:

1. Runbook objective achieved.
2. No data durability violation.
3. No unauthorized data exposure.
4. No legal hold violation.
5. Metrics returned to acceptable range.
6. Audit events recorded.
7. Evidence archived.
8. Follow-up actions created if required.

For SEV-1 and SEV-2 incidents, a post-incident review MUST be produced.

---

# 13. NFR Traceability

| NFR / Requirement | Source | How This Document Satisfies It |
|---|---|---|
| AVAIL-002 | Node recovery | OPS-RB-001. |
| AVAIL-003 | Coordinator failover | OPS-RB-002. |
| AVAIL-004 | Split-brain lease safety | Epoch fencing checks in OPS-RB-002/008. |
| AVAIL-005 | Rolling upgrade availability | OPS-RB-003. |
| REC-001..004 | RPO/RTO | OPS-RB-007/008. |
| REC-005 | PITR | OPS-RB-011. |
| REC-006 | Backup scope validation | OPS-RB-009. |
| REC-007 | Crypto-shred backups unrecoverable | OPS-RB-010/011/012. |
| OPS-005 | Rolling upgrade safety | OPS-RB-003. |
| COMP-001/002/004 | Erasure and audit | OPS-RB-012. |
| SEC incident response | KEI-ARC-025 | OPS-RB-013. |

---

# 14. Open Questions

| Item | Status | Resolution Path |
|---|---|---|
| Exact admin CLI/API surface | Open | Define control-plane admin API before Phase-4 exit. |
| Automated failover policy thresholds | Open | Validate with chaos tests in KEI-OPS-041. |
| Change management tooling integration | Open | Select ITSM/ticketing integration. |
| DR drill automation level | Open | Evaluate staged automation after initial drills. |
| Tenant notification templates | Open | Define support/comms templates. |
| Conflict branch reconciliation procedure | Open | Define with storage and state-plane teams. |

---

# 15. Glossary

| Term | Definition |
|---|---|
| Runbook | Approved operational procedure for a specific failure or maintenance scenario. |
| Drain Mode | State where a node stops accepting new work before maintenance. |
| Two-Person Rule | Requirement for two authorized approvers before destructive action. |
| SEV | Severity classification for incidents. |
| PITR | Point-in-Time Recovery. |
| DR Drill | Scheduled exercise validating disaster recovery capability. |
| Conflict Branch | Quarantined writes from a demoted primary during split-brain. |
| Emergency Shedding | Controlled rejection of lower-priority traffic to protect the fabric. |

---

# 16. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial operations runbook specification. Defines operational roles, runbook governance, severity model, upgrade procedures, cluster lifecycle, DR failover, backup/PITR restore, crypto-shredding execution, S3/compaction incident response, security incident integration, and post-action validation. |