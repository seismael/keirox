# KEI-ARC-022 — Consensus, Coordination & High Availability Architecture

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-ARC-022 |
| Title | Consensus, Coordination & High Availability Architecture |
| Version | 1.0 |
| Level | **L2 — Subsystem Architecture** |
| Pillars Covered | Pillar 5 (Distributed State Plane) — consensus & coordination aspect |
| Status | Approved for Engineering |
| Classification | Internal / Engineering Confidential |
| Owner | Principal Engineer (Distributed Systems) |
| Required Reviewers | Chief Architect, Principal Engineer (Storage), SRE Lead, DR Owner |
| Depends On | KEI-ARC-010 (Conceptual Architecture), KEI-ARC-011 (NFRs), KEI-ARC-012 (ADRs), KEI-ARC-020 (Storage Engine), KEI-ARC-021 (State Plane) |
| Feeds | KEI-ARC-026 (Multi-Region & DR), KEI-OPS-040 (Runbooks), KEI-OPS-041 (Chaos Test Plan) |
| Keywords | MUST, MUST NOT, SHOULD, SHOULD NOT, MAY per RFC 2119 |

---

## 2. Purpose, Scope, and Audience

### 2.1 Purpose

This document specifies the **consensus and coordination layer** of the Polymorphic Event Fabric — the mechanisms that make the stateless, sharded subsystems defined in KEI-ARC-020 and KEI-ARC-021 durable, consistent, and highly available.

It owns:

- The **two-tier Raft topology** (data plane + metadata/state plane) — ADR-030.
- The **consistency model** (CP local quorum + causal WAN) — ADR-031.
- **Epoch fencing** and split-brain safety — ADR-024.
- **Lease journal replication** and state snapshot durability.
- The **failover protocols** for storage nodes, leaders, and coordinator shards.
- **Membership, rebalancing, and coordinator placement.**
- **Multi-region Mode A replication** with RPO/RTO — ADR-060.

### 2.2 Scope

**In scope:** consensus topology, replication mechanics, epoch fencing, failover protocols, membership/rebalancing, and multi-region causal replication.

**Out of scope:**
- Physical WAL persistence and segment lifecycle — owned by KEI-ARC-020.
- State machine semantics, watermarks, and lease lifecycle behavior — owned by KEI-ARC-021.
- The operational runbooks that execute failover — owned by KEI-OPS-040.
- The chaos test definitions — owned by KEI-OPS-041.

### 2.3 Position in the Architecture

```
                ┌────────────────────────────────────────────────────────┐
                │            CONSENSUS & COORDINATION (this doc)         │
                │                                                        │
                │   ┌────────────────────────┐  ┌─────────────────────┐  │
                │   │ DATA PLANE RAFT        │  │ METADATA & STATE    │  │
                │   │ (WAL segment heads)    │  │ RAFT                │  │
                │   │ 3-node sync quorum     │  │ (coordinator assign,│  │
                │   │ per storage volume     │  │  manifests, lease    │  │
                │   │                        │  │  journal, snapshots, │  │
                │   │                        │  │  committed W_base)   │  │
                │   └───────────┬────────────┘  └──────────┬──────────┘  │
                │               │                          │              │
                │               ▼                          ▼              │
                │   ┌────────────────────────────────────────────────┐  │
                │   │        EPOCH FENCING & MEMBERSHIP MANAGER      │  │
                │   └────────────────────────────────────────────────┘  │
                │               │                                        │
                │               ▼                                        │
                │   ┌────────────────────────────────────────────────┐  │
                │   │        MULTI-REGION REPLICATOR (Mode A)        │  │
                │   └────────────────────────────────────────────────┘  │
                └───────────┬────────────────────────────┬───────────────┘
                            │ durability confirm         │ lease journal
                            ▼                            ▼
                ┌───────────────────────┐   ┌────────────────────────┐
                │ STORAGE ENGINE        │   │ STATE PLANE            │
                │ KEI-ARC-020           │   │ KEI-ARC-021            │
                └───────────────────────┘   └────────────────────────┘
```

**Normative boundary:** This subsystem provides durability and coordination *services*. It does not define what is stored (KEI-ARC-020) nor the semantics of consumption state (KEI-ARC-021); it guarantees that whatever those subsystems commit is durable, consistent, and recoverable.

---

## 3. Subsystem Responsibilities and Non-Responsibilities

### 3.1 Responsibilities

| ID | Responsibility |
|---|---|
| R1 | Replicate WAL segment heads via a synchronous data-plane quorum. |
| R2 | Replicate coordinator assignments, manifests, lease journals, state snapshots, and committed `W_base` via the metadata/state plane. |
| R3 | Fence stale coordinators and leaders via monotonic epochs. |
| R4 | Execute leader election, transfer, and storage-node failover. |
| R5 | Manage cluster membership and coordinator shard rebalancing. |
| R6 | Bound ACK loss windows for the fast path and guarantee zero loss for the durable path. |
| R7 | Replicate sealed chunks and manifests cross-region under Mode A. |
| R8 | Provide the RPO/RTO mechanisms for disaster recovery. |

### 3.2 Non-Responsibilities

| ID | Non-Responsibility | Owned By |
|---|---|---|
| N1 | Record framing and segment lifecycle | KEI-ARC-020 |
| N2 | State machine transitions and watermark logic | KEI-ARC-021 |
| N3 | Arrow shredding and Iceberg commits | KEI-ARC-023 |
| N4 | Failover runbook steps | KEI-OPS-040 |
| N5 | Chaos/Jepsen test definitions | KEI-OPS-041 |

---

## 4. Internal Component Decomposition

| Component | Responsibility |
|---|---|
| **C1. Data Plane Raft Group** | Replicates WAL segment heads per storage volume with synchronous quorum. |
| **C2. Metadata & State Raft Group** | Replicates coordinator assignments, manifests, lease journals, state snapshots, committed `W_base`. |
| **C3. Leader & Membership Manager** | Elections, leader transfer, join/leave, health detection. |
| **C4. Epoch Fencing Authority** | Issues and validates coordinator and region epochs. |
| **C5. Coordinator Shard Allocator** | Deterministic consistent-hash placement and rebalancing. |
| **C6. Lease Journal Replicator** | Batches and commits lease/ACK deltas to the metadata plane. |
| **C7. State Snapshot Manager** | Periodic state snapshots to bound journal replay. |
| **C8. Multi-Region Replicator** | Mode A async replication with HLC ordering. |
| **C9. Failover Orchestrator** | Drives storage-node, leader, and coordinator recovery sequences. |

---

## 5. Two-Tier Raft Topology (ADR-030)

Consensus is split into two planes because data replication and coordination-state replication have different latency, throughput, and consistency profiles.

```
┌────────────────────────────────────────────────────────────────────────┐
│  TIER A — DATA PLANE RAFT                                              │
│   • One Raft group per storage volume                                  │
│   • 3-node synchronous quorum                                          │
│   • Replicates WAL segment heads (not full payloads to all replicas    │
│     unless configured; payload follows NVMe + Tier-1)                  │
│   • Gates producer ACK (INV-3)                                         │
├────────────────────────────────────────────────────────────────────────┤
│  TIER B — METADATA & STATE RAFT                                        │
│   • Replicates: coordinator assignments, stream manifests,             │
│     lease journals, bitmap state snapshots, committed W_base           │
│   • Lower throughput, higher consistency requirement                    │
│   • Accepts batched appends from Lease Journal Replicator              │
└────────────────────────────────────────────────────────────────────────┘
```

**Normative rules:**
- The two planes MUST NOT share a single Raft group, to prevent coordination chatter from degrading hot-path write latency.
- The data plane MUST gate producer ACKs (INV-3); the metadata plane MUST gate state visibility.

---

## 6. Data Plane Consensus

### 6.1 Write Quorum Flow

```
Producer batch
   │
   ▼
[Leader storage node] appends to local NVMe WAL segment
   │
   ├──► replicate segment head to Follower 1
   ├──► replicate segment head to Follower 2
   │
   ▼ (synchronous quorum: leader + 1 follower minimum, acks=all policy)
Quorum commit
   │
   ▼
Producer ACK issued  ──► record is DURABLE (DUR-002, INV-3)
```

### 6.2 Durability Guarantee (JML = 0)

- **Justified Maximum Loss (JML) = 0** for quorum-committed records (DUR-001, Class A).
- A record acknowledged to the producer MUST survive any single-node failure.
- Uncommitted records (present only on the failed leader) are not acknowledged and are therefore not lost from the client's perspective.

### 6.3 Latency Class

The data-plane write-latency target of ≤2 ms p99 is a **Class D conditional target** (ADR-062), valid under Profile P1 with defined hardware and same-rack/same-AZ quorum. It MUST be quoted with those conditions.

---

## 7. Metadata & State Plane Consensus

### 7.1 Replicated State

The metadata/state Raft group persists:

| Artifact | Purpose |
|---|---|
| Coordinator assignments | Which coordinator owns which state shard. |
| Stream manifests | Chunk index for recovery. |
| Lease journals | Ordered lease/ACK/NACK/eviction deltas. |
| State snapshots | Periodic bitmap + lease table checkpoints. |
| Committed `W_base` | Durable watermark per shard. |

### 7.2 Lease Journal Replication (C6)

This is the mechanism behind the ACK durability modes defined in KEI-ARC-021.

**ACK_FAST path:**
```
Worker ACK ──► coordinator applies to local memory (<1ms)
           ──► success returned
           ──► delta appended to lease journal batch
           ──► batch committed to metadata Raft asynchronously
```
- Loss window bounded by `min(journal_batch_interval, max_unflushed_bytes)` (DUR-003, Class D).

**ACK_DURABLE path:**
```
Worker ACK ──► coordinator applies to local memory
           ──► delta committed to metadata Raft synchronously
           ──► success returned
```
- Zero known ACK loss after success (DUR-004, Class A).

**Normative rule:** The Lease Journal Replicator MUST preserve the order of lease deltas within a shard so that replay reconstructs the correct state.

### 7.3 State Snapshots (C7)

- Snapshots are emitted periodically (e.g., every 30 s or every 256 MB journal).
- A snapshot bounds the journal replay length during recovery.
- Snapshots are themselves replicated to the metadata plane.

---

## 8. Epoch Fencing and Split-Brain Safety (ADR-024)

### 8.1 Epoch Model

Two epoch namespaces exist:

| Epoch | Scope | Issued By |
|---|---|---|
| `coordinator_epoch` | A coordinator shard | C4 on coordinator failover |
| `region_epoch` | A region (for Mode A failover) | C4 on region promotion |

### 8.2 Fencing Rule

Every lease issuance, ACK, and state mutation carries the current epoch. A receiving node:

```
IF request.epoch < local.known_epoch THEN
    REJECT request (stale)
ELSE IF request.epoch > local.known_epoch THEN
    ACCEPT and advance local.known_epoch
END
```

### 8.3 Split-Brain Safety Property

**Normative rule (AVAIL-004):** Under an unrecoverable network partition, the system MUST prefer unavailability of the affected shard over issuing conflicting leases. A minority partition MUST NOT grant leases.

This is a deliberate safety-over-liveness choice for the queue path, recorded as ADR-024.

---

## 9. Failover Protocols

### 9.1 Storage Node Failover

Triggered when a storage node (data-plane member) fails.

```
1. Membership Manager detects node loss (heartbeat timeout)
2. Data Plane Raft elects / confirms a new leader
3. Failover Orchestrator provisions a replacement node
4. Replacement reconstructs state from Tier-1 manifest + peer WAL delta
5. Replacement joins the Raft group as a follower, then catches up
```

**Target:** recovery in < 5 seconds (AVAIL-002, Class B). This relies on the stateless Tier-0 design in KEI-ARC-020.

### 9.2 Data Plane Leader Transfer

For planned maintenance:

```
1. Drain leader (stop accepting new writes)
2. Confirm followers are caught up
3. Transfer leadership via Raft leadership transfer
4. Old leader steps down gracefully
```

### 9.3 Coordinator Shard Failover

Triggered when a coordinator (state-plane owner) fails.

```
1. Membership Manager detects coordinator loss
2. Successor coordinator increments coordinator_epoch (fencing)
3. Restore shard state: latest snapshot + lease journal replay
4. Rebuild timing wheel from active leases
5. Resume leasing; reject stale-epoch requests
```

**Target:** coordinator-shard failover in < 3.5 seconds (AVAIL-003, Class B).

### 9.4 Recovery Consistency

Because the WAL is append-only and batch-framed with CRC32C, replay is deterministic. State-plane recovery reconciles the restored bitmap against the committed `W_base` from the metadata plane to ensure no state regresses below the committed watermark.

---

## 10. Membership, Rebalancing, and Coordinator Placement

### 10.1 Cluster Membership

- Nodes join/leave via the Membership Manager.
- Health is detected via heartbeat with bounded timeout.
- A node is considered failed only after quorum-confirmable evidence to avoid flapping.

### 10.2 Coordinator Shard Placement (C5)

Per ADR-023, state shards map to coordinators deterministically:

```
StateShard      = hash(tenant_id, stream_id, group_id, shard_bucket)
CoordinatorNode = ConsistentHash(StateShard)
```

- Consistent hashing bounds the number of shards that move when membership changes.
- Rebalancing transfers shard ownership (and epoch) without downtime, using a two-phase handoff: successor loads state, then the epoch advances atomically.

### 10.3 Rebalancing Safety

**Normative rule:** During a shard transfer, exactly one coordinator may hold the active epoch. The transfer MUST NOT create a window where two coordinators both accept leases for the same shard.

---

## 11. Multi-Region Replication — Mode A (ADR-060)

### 11.1 Mode A: Single-Writer Primary + Async Replica

v1 supports only single-writer-per-stream replication to avoid unsolvable same-stream write conflicts.

```
┌─────────────────────────────┐          async replication
│  PRIMARY REGION             │  ─────────────────────────►  ┌──────────────────────┐
│  • Owns writes for a stream │   sealed chunks + manifests   │  REPLICA REGION      │
│  • Data Plane Raft (local)  │   + WAL delta + HLC tags      │  • Read/failover     │
└─────────────────────────────┘                               └──────────────────────┘
```

### 11.2 Replication Unit

- Sealed columnar chunks and their manifests are replicated.
- Active WAL deltas are replicated for low RPO.
- Ordering uses Hybrid Logical Clocks (HLC) tags for causal consistency.

### 11.3 RPO / RTO Targets

| Scenario | Target | Class |
|---|---|---|
| RPO, normal network | ≤ 5 s | D |
| RPO, degraded network | ≤ 60 s | D |
| RTO, planned failover | ≤ 1 min | B |
| RTO, unplanned failover | ≤ 5 min | B |
| Data loss window | Unreplicated WAL delta only | — |

### 11.4 Region Failover

```
1. Detect primary region failure
2. Control plane increments region_epoch (fence old primary)
3. Replica promotes writable stream registry
4. Recover WAL delta if available
5. Consumers reconnect with new region_epoch
6. Replication direction reversed after failover
```

**Normative rule:** The old primary MUST be fenced by `region_epoch` before the replica accepts writes, preventing split-brain writes.

### 11.5 Out of Scope for v1

- Multi-writer same-stream active-active (requires global consensus/conflict resolution not provided by HLC alone).
- Regional namespaces (each region writes its own stream namespace) are an alternative pattern, not same-stream replication.

---

## 12. Consistency Model Summary (ADR-031)

| Scope | Model | Mechanism |
|---|---|---|
| Local cluster writes | CP (linearizable for committed records) | Data Plane Raft quorum. |
| Coordination state | CP | Metadata & State Raft. |
| Cross-region | Causal, asynchronous | HLC tags; Mode A replication. |

**Normative rule:** Cross-region linearizability is NOT provided in v1. Any component requiring global strong consistency MUST be confined to a single region.

---

## 13. Quorum Degradation and Backpressure

| Condition | Behavior |
|---|---|
| One data-plane follower lost | Quorum still holds (leader + 1); writes continue. |
| Quorum lost (2 of 3 down) | Writes to that volume pause (safety); reads may continue from committed state. |
| Metadata plane quorum degraded | State mutations pause; fast-path leases continue within bounded loss window. |
| Cross-region link down | Replication buffers locally; RPO degrades toward the degraded-network bound. |

**Normative rule:** Quorum loss MUST pause the affected write path rather than silently accept uncommitted writes (safety over availability for durability).

---

## 14. Consensus-Specific Failure Handling

| Scenario | Defense (this subsystem) |
|---|---|
| Leader crash | Raft election; JML=0 for committed records. |
| Storage node crash | Stateless recovery from Tier-1 manifest + WAL delta (§9.1). |
| Coordinator crash | Epoch-fenced successor; snapshot + journal replay (§9.3). |
| Split-brain partition | Epoch fencing; minority refuses leases (§8.3). |
| Network flap / false failure | Quorum-confirmable failure detection to avoid flapping. |
| Shard transfer race | Single active epoch during handoff (§10.3). |
| Region failover race | `region_epoch` fencing (§11.4). |

---

## 15. NFR Traceability (Owned by This Subsystem)

| NFR | Requirement | How This Subsystem Satisfies It |
|---|---|---|
| DUR-001 | JML = 0 for committed records | Data Plane Raft quorum (§6.2). |
| DUR-002 | ACK after quorum commit | Write quorum flow (§6.1). |
| DUR-003 | ACK_FAST bounded loss | Lease journal async batch (§7.2). |
| DUR-004 | ACK_DURABLE zero loss | Lease journal sync commit (§7.2). |
| AVAIL-001 | Service continuity on node loss | Raft election + recovery (§9.1). |
| AVAIL-002 | Node recovery <5s | Stateless recovery (§9.1). |
| AVAIL-003 | Coordinator failover <3.5s | Epoch-fenced replay (§9.3). |
| AVAIL-004 | No double-lease under partition | Epoch fencing (§8). |
| SCALE-006 | Coordinator load bounded per shard | Consistent-hash sharding (§10.2). |
| REC-001..004 | RPO/RTO | Mode A replication (§11.3). |
| REC-007 | Crypto-shred backups unrecoverable | Key destruction (coordination with KEI-ARC-025). |

---

## 16. Interfaces

### 16.1 Provided Interfaces

| Interface | Consumer | Semantics |
|---|---|---|
| `commitWrite(batch)` | KEI-ARC-020 | Quorum-commit a WAL batch; blocks until durable. |
| `appendLeaseJournal(deltas)` | KEI-ARC-021 | Append lease/ACK deltas (fast or durable). |
| `commitSnapshot(shard, state)` | KEI-ARC-021 | Persist a state snapshot. |
| `getCommittedWatermark(shard)` | KEI-ARC-021 | Return durable `W_base`. |
| `assignCoordinator(shard)` | Control Plane | Deterministic coordinator placement. |
| `fenceEpoch(scope)` | Failover Orchestrator | Advance and validate epochs. |
| `replicateRegion(chunks, manifests)` | KEI-ARC-026 | Mode A cross-region push. |

### 16.2 Consumed Interfaces

| Interface | Provider | Purpose |
|---|---|---|
| WAL segment append | KEI-ARC-020 | Data to replicate. |
| State shard state | KEI-ARC-021 | Snapshots and journals. |
| Cluster membership events | Control Plane | Join/leave/health. |
| KMS / secrets | KEI-ARC-025 | Encrypted replication channels. |

---

## 17. Open Questions and ADR Dependencies

| Item | Status | Resolution Path |
|---|---|---|
| Raft implementation choice (in-house vs. library) | Open | Evaluate before Phase-2 start. |
| Lease journal batch interval tuning | Open | Benchmark ACK_FAST loss vs. latency under P4. |
| Snapshot interval (30 s / 256 MB) tuning | Open | Validate replay-length bound under P4. |
| Region epoch propagation latency | Open | Measure under Mode A failover drill. |
| State-shard bucket count | Open | ADR pending (KEI-ARC-012); derive from coordinator load model. |

Binding decisions already recorded: ADR-012, ADR-023, ADR-024, ADR-030, ADR-031, ADR-060, ADR-061, ADR-062.

---

## 18. Glossary (Additions)

| Term | Definition |
|---|---|
| Data Plane Raft | The quorum replicating WAL segment heads per storage volume. |
| Metadata & State Raft | The quorum replicating coordination and consumption state. |
| Lease Journal | Ordered, replicated log of lease/ACK deltas. |
| State Snapshot | Periodic full checkpoint of a shard's bitmap and lease table. |
| coordinator_epoch | Monotonic generation fencing a coordinator shard. |
| region_epoch | Monotonic generation fencing a region in Mode A. |
| JML | Justified Maximum Loss; zero for committed records. |
| Mode A | Single-writer primary + async replica multi-region mode. |

---

## 19. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial consensus & HA architecture. Defines two-tier Raft topology, JML=0 durability, epoch fencing and split-brain safety, storage/leader/coordinator failover protocols, membership and rebalancing, and Multi-Region Mode A with RPO/RTO. Aligns to ADR-012/023/024/030/031/060/061/062 and NFRs DUR/AVAIL/SCALE/REC. |