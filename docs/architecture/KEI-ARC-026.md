# KEI-ARC-026 — Multi-Region Replication & Disaster Recovery Architecture

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-ARC-026 |
| Title | Multi-Region Replication & Disaster Recovery Architecture |
| Version | 1.0 |
| Level | **L2 — Subsystem Architecture** |
| Pillars Covered | Cross-Cutting (Multi-Region & Disaster Recovery) |
| Status | Approved for Engineering |
| Classification | Internal / Engineering Confidential |
| Owner | Principal Engineer (Distributed Systems) / DR Owner |
| Required Reviewers | Chief Architect, Security Lead, SRE Lead, Platform Engineering Lead |
| Depends On | KEI-ARC-010 (Conceptual Architecture), KEI-ARC-011 (NFRs), KEI-ARC-012 (ADRs), KEI-ARC-020 (Storage Engine), KEI-ARC-022 (Consensus), KEI-ARC-025 (Security) |
| Feeds | KEI-OPS-040 (Operations Runbooks), KEI-OPS-041 (Validation & Test Plan) |
| Keywords | MUST, MUST NOT, SHOULD, SHOULD NOT, MAY per RFC 2119 |

---

## 2. Purpose, Scope, and Audience

### 2.1 Purpose

This document specifies the **Multi-Region Replication and Disaster Recovery (DR) subsystem** of the Polymorphic Event Fabric. It defines how the fabric replicates data across geographic boundaries, maintains causal consistency, executes region failover, and recovers from catastrophic datacenter or cloud-provider losses.

It enforces the **Mode A replication constraint** (ADR-060) to avoid unsolvable concurrent-write conflicts, and establishes the normative backup and restore topology required for enterprise business continuity.

### 2.2 Scope

**In scope:** Multi-region replication modes (Mode A and Regional Namespaces), Hybrid Logical Clock (HLC) causal ordering, RPO/RTO mechanisms, region failover and epoch fencing, backup/restore scope, point-in-time recovery (PITR), and cross-region security/residency enforcement.

**Out of scope:**
- Local cluster consensus and data-plane Raft — owned by KEI-ARC-022.
- Physical WAL persistence and Tier-1 S3 offload mechanics — owned by KEI-ARC-020.
- Encryption key management and crypto-shredding mechanics — owned by KEI-ARC-025.
- The step-by-step human runbooks for executing a failover — owned by KEI-OPS-040.

### 2.3 Position in the Architecture

```
                 ┌─────────────────────────────────────────────────────┐
                 │          CONTROL PLANE (Region Topology)            │
                 └───────────────────────┬─────────────────────────────┘
                                         │ region_epoch, residency rules
                                         ▼
┌────────────────────────────────────────────────────────────────────────┐
│               MULTI-REGION & DR SUBSYSTEM (this doc)                   │
│                                                                        │
│  HLC Causal Tagger ──► Async Chunk Replicator ──► Region Epoch Fencer │
│                                                                        │
│  Backup Scheduler ──► PITR Reconciler ──► Cross-Region Security Gate  │
└───────┬───────────────────┬───────────────────┬───────────────────────┘
        │                   │                   │
        ▼                   ▼                   ▼
┌──────────────┐    ┌──────────────┐    ┌──────────────────────┐
│ Storage      │    │ Consensus    │    │ Security             │
│ Engine (020) │    │ & HA (022)   │    │ & Privacy (025)      │
└──────────────┘    └──────────────┘    └──────────────────────┘
```

**Normative boundary:** This subsystem operates strictly *asynchronously* across regions. It MUST NOT gate the local Tier-0 write path (INV-7). Cross-region linearizability is NOT provided in v1 (ADR-031).

---

## 3. Subsystem Responsibilities and Non-Responsibilities

### 3.1 Responsibilities

| ID | Responsibility |
|---|---|
| R1 | Replicate sealed chunks, manifests, and active WAL deltas cross-region. |
| R2 | Tag events with Hybrid Logical Clocks (HLC) for causal ordering. |
| R3 | Execute region failover and enforce `region_epoch` fencing. |
| R4 | Manage backup scope, scheduling, and integrity validation. |
| R5 | Execute Point-in-Time Recovery (PITR) to new clusters. |
| R6 | Enforce data residency and cross-region transfer restrictions. |
| R7 | Propagate destroyed-key registries for cross-region crypto-shredding. |

### 3.2 Non-Responsibilities

| ID | Non-Responsibility | Owned By |
|---|---|---|
| N1 | Local quorum durability | KEI-ARC-022 |
| N2 | Tier-1 S3 upload mechanics | KEI-ARC-020 |
| N3 | Key destruction execution | KEI-ARC-025 |
| N4 | Human-in-the-loop failover decisions | KEI-OPS-040 |

---

## 4. Internal Component Decomposition

| Component | Responsibility |
|---|---|
| **D1. HLC Causal Tagger** | Assigns Hybrid Logical Clock tags to batches for cross-region causal ordering. |
| **D2. Async Chunk Replicator** | Pushes sealed Parquet chunks and manifests to replica regions. |
| **D3. WAL Delta Replicator** | Streams active Tier-0 WAL deltas for low-RPO recovery. |
| **D4. Region Epoch Fencer** | Issues and validates `region_epoch` to prevent split-brain writes. |
| **D5. Backup Scheduler** | Orchestrates periodic snapshots, manifest backups, and WAL tail archival. |
| **D6. PITR Reconciler** | Rebuilds cluster state from cold backups to a specific timestamp. |
| **D7. Residency & Security Gate** | Blocks cross-region replication that violates residency or destroyed-key policies. |

---

## 5. Multi-Region Replication Models

To avoid the unsolvable write-conflict problem of active-active same-stream replication, v1 supports two strict models (ADR-060).

### 5.1 Mode A: Single-Writer Primary + Async Replica (Default)

```
┌─────────────────────────────┐          async replication
│  PRIMARY REGION             │  ─────────────────────────►  ┌──────────────────────┐
│  • Owns writes for a stream │   sealed chunks + manifests   │  REPLICA REGION      │
│  • Data Plane Raft (local)  │   + WAL delta + HLC tags      │  • Read / Failover   │
└─────────────────────────────┘                               └──────────────────────┘
```

- A stream has exactly one writable primary region.
- Replica regions receive asynchronous updates and serve read-only consumers or standby failover.
- **Normative rule:** Concurrent writes to the same strictly-ordered stream from two different regions are NOT supported in v1.

### 5.2 Regional Namespaces (Alternative Active-Active)

For workloads that do not require global strict ordering:

- Each region writes to its own isolated stream namespace (e.g., `us-east.stream_A`, `eu-west.stream_A`).
- Consumers merge the streams analytically or via application-level causal logic.
- **Normative rule:** This is not same-stream replication; it is independent regional streams.

---

## 6. Causal Ordering and Conflict Resolution

### 6.1 Hybrid Logical Clocks (HLC)

Because physical clocks drift, cross-region causal ordering relies on HLCs.

- Every WAL batch is tagged with an HLC timestamp `(logical, physical)`.
- Replicas merge incoming batches using HLC comparison, ensuring that if Event A causally precedes Event B in the primary region, Event A is visible before Event B in the replica.

### 6.2 Conflict Resolution

Under Mode A, write conflicts are structurally impossible because only one region holds the write lease for a stream.

If a network partition causes a false failover and the old primary continues accepting writes (split-brain):
1. The `region_epoch` fencing mechanism (see §8) detects the anomaly when the partition heals.
2. The orphaned writes from the demoted primary are quarantined in a "conflict branch" manifest.
3. Administrative intervention is required to reconcile or discard the conflict branch.

**Normative rule:** The system MUST prefer quarantining orphaned writes over silently dropping them or corrupting the causal order.

---

## 7. RPO / RTO Mechanisms and Targets

### 7.1 Recovery Point Objective (RPO)

RPO defines the maximum acceptable data loss window.

| Scenario | Target | Mechanism | Class |
|---|---|---|---|
| Normal network | ≤ 5 seconds | WAL Delta Replicator (D3) + Chunk Replicator (D2). | D |
| Degraded network | ≤ 60 seconds | Exponential backoff; local buffering. | D |

**Normative rule:** RPO is bounded by the unreplicated WAL delta. If the primary region is physically destroyed before the WAL delta is transmitted, that delta is lost.

### 7.2 Recovery Time Objective (RTO)

RTO defines the maximum acceptable downtime during a failover.

| Scenario | Target | Mechanism | Class |
|---|---|---|---|
| Planned failover | ≤ 1 minute | Graceful drain, epoch advance, replica promotion. | B |
| Unplanned failover | ≤ 5 minutes | Automated fencing, manifest reload, consumer redirect. | B |

---

## 8. Region Failover and Epoch Fencing

### 8.1 The `region_epoch`

Every region operates under a monotonic `region_epoch`.

- When a region is promoted to primary, its `region_epoch` increments.
- All write requests and cross-region replication messages carry the epoch.
- Replicas and gateways reject requests carrying a stale epoch.

### 8.2 Failover Protocol

```
1. Detect primary region failure (control plane consensus).
2. Increment region_epoch for the replica region.
3. Replica promotes writable stream registry.
4. Fence old primary (network/ACL level).
5. Recover WAL delta if available; quarantine if conflicting.
6. Consumers reconnect to new primary via DNS / service mesh.
```

**Normative rule (Safety over Liveness):** If the control plane cannot confidently fence the old primary, it MUST NOT promote the replica. The system MUST prefer regional unavailability over split-brain writes.

---

## 9. Backup, Restore, and Point-in-Time Recovery (PITR)

### 9.1 Backup Scope

To guarantee recoverability from total cluster loss, the Backup Scheduler (D5) MUST persist:

1. **Tier-1 Object Storage:** Sealed Parquet chunks and Iceberg metadata (inherent in Tier-1 design).
2. **Stream Manifests:** The mapping of streams to chunks.
3. **Raft Snapshots:** Metadata and state-plane snapshots.
4. **Schema Registry:** All historical schema versions.
5. **WAL Tails:** Optional active WAL segments for tight RPO.
6. **Destroyed-Key Registry:** To prevent restoring crypto-shredded data.

### 9.2 Point-in-Time Recovery (PITR)

The PITR Reconciler (D6) rebuilds a cluster to a specific timestamp $T$:

```
1. Provision new cluster.
2. Restore Raft snapshots and Schema Registry prior to T.
3. Load Stream Manifests and filter chunks where ingest_time <= T.
4. Replay WAL tails up to T.
5. Validate checksums and destroyed-key registry.
```

### 9.3 Backup Interaction with Crypto-Shredding

If a stream was crypto-shredded at time $T_{shred}$, the destroyed-key registry MUST block the PITR Reconciler from restoring any data protected by that key, even if the ciphertext exists in the backup.

**Normative rule (COMP-001 / REC-007):** Restoring a backup MUST NOT resurrect cryptographically erased data.

---

## 10. Data Residency and Cross-Region Security

### 10.1 Residency Enforcement

The Residency & Security Gate (D7) evaluates replication requests against tenant residency policies.

```
IF stream.residency == "EU_ONLY" AND target_region == "US_EAST" THEN
    REJECT replication
    ALERT compliance officer
END
```

### 10.2 Cross-Region Key Replication

Encryption keys (KEKs/DEKs) MUST be replicated to replica regions via the KMS provider's native multi-region key replication, or via a secure control-plane key-escrow mechanism.

**Normative rule:** A replica region MUST NOT accept replicated ciphertext unless it possesses the corresponding DEK to serve reads, or the data is strictly cold-archived.

---

## 11. DR-Specific Failure Handling

| Scenario | Defense (this subsystem) |
|---|---|
| Primary region destroyed | Automated failover to replica; RPO bounded by last WAL delta sync. |
| Split-brain network partition | Epoch fencing; quarantine orphaned writes; prefer unavailability. |
| Backup corruption | Checksum validation on restore; multi-region backup redundancy. |
| Residency violation attempt | Replication blocked at the Residency Gate; audit logged. |
| Restoring shredded data | Destroyed-key registry blocks PITR reconciliation. |
| KMS region failure | Replica region fails open for reads if DEKs are cached; writes blocked. |

---

## 12. NFR Traceability (Owned by This Subsystem)

| NFR | Requirement | How This Subsystem Satisfies It |
|---|---|---|
| REC-001 | RPO ≤ 5s (normal) | WAL Delta Replicator (§7.1). |
| REC-002 | RPO ≤ 60s (degraded) | Local buffering + backoff (§7.1). |
| REC-003 | RTO ≤ 1m (planned) | Graceful drain + epoch advance (§8.2). |
| REC-004 | RTO ≤ 5m (unplanned) | Automated fencing + manifest reload (§8.2). |
| REC-005 | PITR support | PITR Reconciler + WAL tails (§9.2). |
| REC-006 | Backup scope completeness | Backup Scheduler scope (§9.1). |
| REC-007 | Crypto-shred backups unrecoverable | Destroyed-key registry gate (§9.3). |
| COMP-006 | Data residency | Residency & Security Gate (§10.1). |

---

## 13. Interfaces

### 13.1 Provided Interfaces

| Interface | Consumer | Semantics |
|---|---|---|
| `replicateChunk(chunk, hlc)` | KEI-ARC-020 | Push sealed chunk to replica. |
| `replicateWALDelta(batch, hlc)` | KEI-ARC-022 | Push active WAL delta. |
| `promoteRegion(region_id)` | Control Plane | Execute failover and epoch advance. |
| `triggerBackup(scope)` | Admin API | Initiate snapshot/manifest backup. |
| `restorePITR(timestamp)` | Admin API | Rebuild cluster to timestamp. |

### 13.2 Consumed Interfaces

| Interface | Provider | Purpose |
|---|---|---|
| Sealed chunks / manifests | KEI-ARC-020 | Data to replicate. |
| Raft snapshots / lease journals | KEI-ARC-022 | State to replicate. |
| Destroyed-key registry | KEI-ARC-025 | Prevent resurrection of erased data. |
| KMS multi-region API | External KMS | Key replication. |

---

## 14. Open Questions and ADR Dependencies

| Item | Status | Resolution Path |
|---|---|---|
| HLC implementation library | Open | Evaluate Rust HLC crates before Phase-4 start. |
| Conflict branch reconciliation UI | Open | Define admin tooling in KEI-OPS-040. |
| Cross-region DEK replication mechanism | Open | Specify per KMS provider in KEI-DES-036. |
| PITR granularity (chunk vs. record) | Open | Benchmark WAL tail replay cost. |

Binding decisions already recorded: ADR-031 (Causal WAN), ADR-060 (Mode A only).

---

## 15. Glossary (Additions)

| Term | Definition |
|---|---|
| Mode A | Single-writer primary + async replica multi-region mode. |
| HLC | Hybrid Logical Clock; combines physical time with logical counters for causal ordering. |
| region_epoch | Monotonic generation fencing a region to prevent split-brain writes. |
| PITR | Point-in-Time Recovery; rebuilding a cluster to a specific historical timestamp. |
| Conflict Branch | Quarantined writes from a demoted primary during a split-brain partition. |

---

## 16. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial multi-region and DR architecture. Defines Mode A replication, HLC causal ordering, RPO/RTO targets, region epoch fencing, backup/PITR scope, and cross-region residency/security enforcement. Aligns to ADR-031/060 and NFRs REC/COMP. |