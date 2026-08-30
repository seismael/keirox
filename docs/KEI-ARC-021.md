# KEI-ARC-021 — Consumption State Plane Architecture (Roaring Bitmaps, Leases, Virtual DLQ)

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-ARC-021 |
| Title | Consumption State Plane Architecture |
| Version | 1.0 |
| Level | **L2 — Subsystem Architecture** |
| Pillars Covered | Pillar 2 (Log-Bitmap Duality), Pillar 5 (Distributed State Plane) |
| Status | Approved for Engineering |
| Classification | Internal / Engineering Confidential |
| Owner | Principal Engineer (Distributed Systems) |
| Required Reviewers | Chief Architect, Principal Engineer (Storage), SRE Lead |
| Depends On | KEI-ARC-010 (Conceptual Architecture), KEI-ARC-011 (NFRs), KEI-ARC-012 (ADRs), KEI-ARC-020 (Storage Engine) |
| Feeds | KEI-ARC-022 (Consensus & Coordination), KEI-ARC-024 (Gateways), KEI-DES-031 (State Plane Data Structures), KEI-DES-032 (Lease/ACK Protocol) |
| Keywords | MUST, MUST NOT, SHOULD, SHOULD NOT, MAY per RFC 2119 |

---

## 2. Purpose, Scope, and Audience

### 2.1 Purpose

This document specifies the **Consumption State Plane** — the subsystem that projects the immutable physical log into the polymorphic consumption modes defined by the Golden Invariant. It is the architectural heart of the queue/stream duality.

It elaborates:

- **Pillar 2 (Log-Bitmap Duality):** the state machine, Roaring Bitmap overlays, watermark advancement, and virtual DLQ.
- **Pillar 5 (Distributed State Plane):** deterministic coordinator sharding, epoch fencing, and lease/ACK durability from the state-management perspective.

### 2.2 Scope

**In scope:** the consumption state machine, Roaring Bitmap overlay model, sliding watermark, lease lifecycle and timing wheel, virtual DLQ and Sparse Exception Table, coordinator sharding, ACK durability modes, state persistence/spilling, and multi-mode consumption over a shared log.

**Out of scope:**
- Physical log persistence and tiering — owned by KEI-ARC-020.
- Raft consensus protocol internals and the two-tier Raft topology mechanics — owned by KEI-ARC-022.
- Arrow shredding and lakehouse projection — owned by KEI-ARC-023.
- Exact in-memory serialization of bitmaps and lease tables — owned by KEI-DES-031.
- Wire protocol for lease/ACK RPCs — owned by KEI-DES-032.

### 2.3 Position in the Architecture

```
                        ┌─────────────────────────────┐
                        │  Control Plane (group/quota  │
                        │  config, ABAC)               │
                        └──────────────┬──────────────┘
                                       │
   Producers ──►┌──────────────────────┴──────────────────────────────┐
                │             STORAGE ENGINE (KEI-ARC-020)            │
                │             immutable physical log                   │
                └──────────────────────┬──────────────────────────────┘
                                       │ append-only read(stream, offset)
                                       ▼
                ┌──────────────────────────────────────────────────────┐
                │            CONSUMPTION STATE PLANE (this doc)        │
                │  State Machine · Roaring Bitmaps · Watermarks        │
                │  Leases · Timing Wheel · Virtual DLQ · Coordinators  │
                └───────┬──────────────────────┬──────────────────────┘
                        │ state snapshots &    │ lease/ACK RPCs
                        │ lease journal        ▼
                        ▼               ┌─────────────────┐
                ┌─────────────────┐     │ Workers /        │
                │ CONSENSUS       │     │ Stream Consumers │
                │ KEI-ARC-022     │     │ (Gateway/SDK)    │
                │ (Raft, epochs)  │     └─────────────────┘
                └─────────────────┘
```

**Normative boundary:** The state plane reads the log append-only and never mutates it (INV-1). It mutates only its own replicated state overlays (GI-3).

---

## 3. Subsystem Responsibilities and Non-Responsibilities

### 3.1 Responsibilities

| ID | Responsibility |
|---|---|
| R1 | Maintain the per-state-shard consumption state machine over immutable offsets. |
| R2 | Represent state via hierarchical Roaring Bitmap overlays. |
| R3 | Advance the sliding base watermark `W_base` to bound memory. |
| R4 | Manage lease acquisition, renewal, timeout, and reaping via a timing wheel. |
| R5 | Enforce mandatory DLQ eviction to guarantee watermark progress. |
| R6 | Maintain the Sparse Exception Table for virtual DLQ views. |
| R7 | Shard consumer state deterministically across coordinators. |
| R8 | Provide ACK_FAST / ACK_DURABLE acknowledgment durability modes. |
| R9 | Persist and recover state via lease journals, snapshots, and spill SSTables. |
| R10 | Serve stream, queue, and DLQ consumption modes over the same log. |

### 3.2 Non-Responsibilities

| ID | Non-Responsibility | Owned By |
|---|---|---|
| N1 | Physical record durability | KEI-ARC-020 |
| N2 | Raft log replication protocol | KEI-ARC-022 |
| N3 | Columnar transformation | KEI-ARC-023 |
| N4 | Protocol wire encoding | KEI-DES-032 |
| N5 | Authorization decisions | KEI-ARC-025 |

---

## 4. Internal Component Decomposition

```
┌──────────────────────────────────────────────────────────────────────────┐
│                     CONSUMPTION STATE PLANE                              │
│                                                                          │
│  ┌──────────────────┐   ┌──────────────────┐                            │
│  │ S1. State Shard   │──►│ S2. Roaring       │                          │
│  │     Router        │   │     Overlay Store │                          │
│  └──────────────────┘   └───────┬──────────┘                            │
│                                 │                                        │
│        ┌────────────────────────┼─────────────────────────┐             │
│        ▼                        ▼                          ▼             │
│  ┌──────────────┐       ┌──────────────┐         ┌──────────────┐       │
│  │ S3. Watermark │       │ S4. Lease     │         │ S5. Virtual   │     │
│  │     Advancer  │       │     Manager + │         │     DLQ +     │    │
│  │               │       │     Timing    │         │     Sparse    │    │
│  │               │       │     Wheel     │         │     Exception │    │
│  └──────┬───────┘       └──────┬───────┘         │     Table     │    │
│         │                       │                  └──────────────┘       │
│         └───────────┬───────────┘                                        │
│                     ▼                                                     │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │ S6. Coordinator (shard owner, epoch-fenced)                       │  │
│  │     • Local fast-path state apply                                  │ │
│  │     • Lease journal writer                                         │ │
│  │     • State snapshot emitter                                       │ │
│  └──────────────────────────┬───────────────────────────────────────┘   │
│                              │                                            │
│  ┌──────────────────┐       │   ┌──────────────────┐                     │
│  │ S7. Spill Manager │◄─────┴──►│ S8. Recovery      │                    │
│  │     (bitmap SST)  │           │     Reconciler    │                   │
│  └──────────────────┘           └──────────────────┘                     │
│                                                                          │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │ S9. ACK Durability Controller (ACK_FAST / ACK_DURABLE)            │  │
│  └──────────────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────────────┘
```

| Component | Responsibility |
|---|---|
| **S1. State Shard Router** | Maps `(tenant, stream, group, offset)` to a state shard and its owning coordinator. |
| **S2. Roaring Overlay Store** | Holds the per-shard `acked`, `dlq`, and `leased` bitmaps. |
| **S3. Watermark Advancer** | Computes and advances `W_base`; triggers purge and spill. |
| **S4. Lease Manager + Timing Wheel** | Grants/renews/expires leases; O(1) timer management. |
| **S5. Virtual DLQ + Sparse Exception Table** | Indexes evicted offsets and failure metadata. |
| **S6. Coordinator** | Shard owner; applies fast-path state; writes lease journal; emits snapshots. |
| **S7. Spill Manager** | Spills inactive bitmap containers to NVMe state SSTables. |
| **S8. Recovery Reconciler** | Restores shard state from snapshot + lease journal on failover. |
| **S9. ACK Durability Controller** | Selects and enforces ACK_FAST vs ACK_DURABLE semantics. |

---

## 5. The Consumption State Machine

### 5.1 States and Invariant

Each offset within a state shard has exactly one state:

```
State(i) ∈ { READY, LEASED(τ), ACKED, EVICTED_DLQ }
```

`READY` is the implicit default for offsets that are not leased, acked, or evicted, and is not stored as an explicit bit (memory optimization).

### 5.2 State Transition Diagram

```
                         lease(τ)
              ┌──────────────────────────────┐
              │                              ▼
          ┌────────┐                    ┌──────────┐
          │ READY  │                    │  LEASED  │
          └────────┘                    └────┬─────┘
              ▲                              │
              │   timeout(τ) / NACK          │
              └──────────────────────────────┤
                                             │ ACK
                                             ▼
                                       ┌──────────┐
                                       │  ACKED   │
                                       └──────────┘

   READY or LEASED ──(retry_count ≥ R_max  OR  time_in_flight ≥ T_max)──► EVICTED_DLQ
```

### 5.3 Transition Table

| From | Event | To | Side Effect |
|---|---|---|---|
| READY | lease(τ) | LEASED | Start timing-wheel timer; record worker + retry_count. |
| LEASED | ACK | ACKED | Set `acked` bit; cancel timer; clear lease. |
| LEASED | NACK | READY | Cancel timer; increment retry_count; requeue. |
| LEASED | timeout(τ) | READY | Cancel timer; requeue at dispatch head for prioritized retry. |
| READY/LEASED | retry_count ≥ R_max | EVICTED_DLQ | Insert into Sparse Exception Table; enable W_base advance. |
| READY/LEASED | time_in_flight ≥ T_max | EVICTED_DLQ | Insert into Sparse Exception Table; enable W_base advance. |

**Normative rule:** No transition may mutate the physical log. All transitions mutate only the state overlay (GI-3, INV-1).

---

## 6. Roaring Bitmap Overlay Model

### 6.1 Per-Shard Bitmap Set

Each state shard maintains a set of Roaring Bitmaps (ADR-002):

| Bitmap | Tracks |
|---|---|
| `acked_bitmap` | Offsets in `ACKED`. |
| `dlq_bitmap` | Offsets in `EVICTED_DLQ`. |
| `leased_index` | Offsets currently `LEASED` (cross-referenced with the lease table). |

`READY` is derived: `READY = ¬acked ∧ ¬dlq ∧ ¬leased`.

### 6.2 Roaring Container Types

The hierarchical Roaring Bitmap uses three container types, selected automatically per 16-bit chunk to minimize memory:

| Container | Best For | Density |
|---|---|---|
| **Run-Length (RLE)** | Dense consecutive ACK runs | <2 bits/entry |
| **Bitset** (8 KB) | Mixed ACK/NACK regions over 65,536 contiguous offsets | Fixed 8 KB |
| **Array** (sorted 16-bit) | Sparse leases / failures | 2 bytes/offset |

**Normative rule:** Container selection MUST be automatic and MUST be re-evaluated on mutation, so that dense ACK regions compress to RLE and sparse lease regions stay in array form.

### 6.3 Consumer Group State (Conceptual)

The per-shard state structure is represented conceptually as:

```
ConsumerGroupShardState {
  group_id, tenant_id, stream_id, shard_bucket
  coordinator_node_id
  coordinator_epoch
  base_watermark (W_base)
  acked_bitmap : RoaringBitmap
  dlq_bitmap   : RoaringBitmap
  active_leases: Map<offset, Lease>
  timing_wheel : TimingWheel
  max_retries  : R_max
  window_size_limit : Δ_max
}

Lease {
  offset
  worker_id
  lease_expiry_ms
  retry_count
  first_leased_at
}
```

Exact in-memory layout and serialization are specified in KEI-DES-031.

---

## 7. Watermark Advancement and Memory Bounding

### 7.1 Sliding Base Watermark

```
W_base = max { k ∈ ℕ | ∀ i < k, State(i) = ACKED ∨ State(i) = EVICTED_DLQ }
```

All state for offsets `< W_base` MUST be purged from memory.

### 7.2 Active Bounded Delta Window

The active state window is `[W_base, W_base + Δ_max]`, bounded by `window_size_limit` (Δ_max). State within this window lives in the Roaring Bitmaps; state outside it is purged or spilled.

### 7.3 Mandatory DLQ Eviction (Anti-Stuck Guarantee)

This is the critical correctness mechanism (ADR-004) that guarantees `W_base` always advances:

```
IF retry_count ≥ max_retries (R_max)
   OR time_in_flight ≥ max_time_in_flight (T_max)
   OR lease_policy == FORCE_EVICT
THEN State(offset) = EVICTED_DLQ
     INSERT ⟨tenant, stream, offset, failure_metadata⟩ INTO Sparse Exception Table
     ALLOW W_base TO ADVANCE
```

**Normative rule (INV-4):** The system MUST NOT permit a non-terminal offset to block `W_base` indefinitely. Mandatory DLQ eviction is the enforcement mechanism.

### 7.4 Memory Bounding Flow

```
Offsets < W_base           ──► PURGE (deallocated)
Offsets in [W_base, +Δ_max] ──► in-memory Roaring Bitmap
Bitmap > spill_threshold    ──► SPILL inactive containers to NVMe SSTable
```

**Normative rule:** Bitmap memory per shard MUST be bounded by `max_bitmap_memory`; on exceed, inactive containers MUST spill to NVMe before any OOM is possible (MEM-003, INV-5).

---

## 8. Lease Lifecycle and Timing Wheel

### 8.1 Lease Lifecycle

```
Worker requests lease ──► Coordinator grants LEASED(τ)
                              │
         ┌────────────────────┼─────────────────────┐
         │                    │                     │
    worker ACKs          worker NACKs         timer expires (τ)
         │                    │                     │
         ▼                    ▼                     ▼
       ACKED               READY                 READY
    (cancel timer)     (retry_count++)      (requeue at head)
                              │
                     retry_count ≥ R_max?
                              │ yes
                              ▼
                        EVICTED_DLQ
```

### 8.2 Hierarchical Priority Timing Wheel (ADR-025)

Lease timeouts are managed by a hierarchical timing wheel providing:

- **O(1)** lease insertion.
- **O(1)** lease cancellation (on ACK/NACK).
- **O(1)** amortized expiration firing.

**Normative rule:** Lease expiration MUST be driven by the timing wheel, not by polling scans, to preserve O(1) cost at millions of concurrent leases.

### 8.3 Redelivery Ordering

Timed-out leases re-enter `READY` at the **head** of the dispatch queue for prioritized retry, so that partially-processed work is retried before new work.

### 8.4 Cold-Task Lease Index

For tasks whose payloads have been offloaded to Tier-1, the state plane retains a **Tier-0 Sparse Queue Index** of unacknowledged-task pointers and lease states in local NVMe, so that re-leasing an old task does not require a cold S3 metadata lookup (addresses the "cold task penalty").

---

## 9. Virtual DLQ and Sparse Exception Table

### 9.1 Virtual DLQ (ADR-003)

Poison pills are not physically copied. On eviction:

1. `State(offset) = EVICTED_DLQ`; the `dlq_bitmap` bit is set.
2. An entry is inserted into the **Sparse Exception Table**:

```
⟨ TenantID, StreamID, Offset, FailureMetadata ⟩
```

3. The physical payload remains in the immutable log.

### 9.2 DLQ Views and Redrive

- A DLQ consumer reads offsets where `dlq_bitmap` is set, fetching payloads from the log.
- Redrive transitions an offset from `EVICTED_DLQ` back to `READY`, with an audit event.

**Normative rule:** DLQ operations MUST be zero-copy with respect to payload. Only state bits and exception metadata are written.

---

## 10. Coordinator Sharding and Epoch Fencing

### 10.1 Deterministic State Sharding (ADR-023)

Consumer state is sharded to bound per-coordinator load:

```
StateShard      = hash(tenant_id, stream_id, group_id, shard_bucket)
CoordinatorNode = ConsistentHash(StateShard)
```

Each shard is owned by exactly one coordinator at a time.

### 10.2 Coordinator Responsibilities (State-Plane View)

- Apply fast-path lease/ACK transitions to local in-memory state.
- Append lease deltas to the lease journal.
- Emit periodic state snapshots.
- Fence stale requests via `coordinator_epoch`.

### 10.3 Epoch Fencing (ADR-024)

Every lease issuance and ACK carries a monotonic `coordinator_epoch`. On failover:

1. Successor coordinator increments `coordinator_epoch`.
2. Restores state from latest snapshot + lease journal (see §13).
3. Rejects any request carrying a stale epoch.

**Normative rule (INV-6, AVAIL-004):** A state shard MUST be owned by exactly one live coordinator. Under an unrecoverable partition, the system SHOULD prefer shard unavailability over issuing conflicting leases.

> The Raft replication mechanics that persist the lease journal and snapshots are specified in KEI-ARC-022. This document defines only the state-plane contract they serve.

---

## 11. ACK Durability Modes (ADR-020, ADR-021)

The state plane exposes two explicit acknowledgment durability modes (enforcing P4 and P5).

### 11.1 ACK_FAST (default)

```
Worker ACK ──► coordinator applies to local memory (<1ms)
           ──► success returned to worker
           ──► lease delta replicated to Raft asynchronously
```

- Sub-millisecond ACK latency.
- On coordinator failover before replication, the ACK MAY be lost → message MAY be redelivered.
- Loss window is bounded by `min(journal_batch_interval, max_unflushed_journal_bytes)` (DUR-003, Class D).

### 11.2 ACK_DURABLE

```
Worker ACK ──► coordinator applies to local memory
           ──► lease delta committed to Raft
           ──► success returned to worker
```

- Higher ACK latency.
- No known ACK loss after success (DUR-004, Class A).

**Normative rule:** The client API MUST expose the ACK mode. `ACK_FAST` is the default (ADR-021); workloads requiring no-ACK-loss MUST explicitly select `ACK_DURABLE`. The `ACK_FAST` loss window MUST be documented to the client.

### 11.3 Delivery Guarantee Interaction

Per ADR-022, the default delivery guarantee is **at-least-once**. Idempotent producers and idempotent consumers are required for effectively-once end-to-end behavior. The state plane does not and MUST NOT claim broker-side exactly-once.

---

## 12. Multi-Mode Consumption over a Shared Log

The same immutable log is consumed in three modes, differing only by overlay usage.

| Mode | Overlay Used | Offset Model | Mutates Overlay? |
|---|---|---|---|
| **Stream Replay** | Offset cursor only | Monotonic per-stream offset | Only offset commit |
| **Task Queue** | Lease + ACK bitmaps + timing wheel | Per-offset lease state | Yes |
| **Virtual DLQ View** | `dlq_bitmap` + Sparse Exception Table | Evicted offsets | No (read-only) |

**Normative rule:** A consumer group MAY independently choose its mode. Switching a group from queue mode to stream mode MUST NOT corrupt the immutable log; it only changes how the group's overlay is interpreted.

---

## 13. State Persistence, Recovery, and Spilling

### 13.1 Persistence Artifacts

| Artifact | Content | Purpose |
|---|---|---|
| **Lease Journal** | Ordered lease/ACK/NACK/eviction deltas | Fast-path durability; replay on failover. |
| **State Snapshot** | Periodic full bitmap + lease table | Bound journal replay length. |
| **Spill SSTable** | Compressed inactive bitmap containers | Offload memory under fragmentation. |

### 13.2 Recovery Reconciliation (S8)

On coordinator failover:

```
1. Load latest state snapshot for the shard
2. Replay lease journal deltas after the snapshot
3. Rebuild timing wheel from active leases
4. Increment coordinator_epoch; fence stale requests
5. Resume leasing
```

**Target:** coordinator-shard failover completes in **< 3.5 seconds** (AVAIL-003, Class B).

### 13.3 Adaptive Container Spilling (S7)

If a shard's active bitmap exceeds the spill threshold (e.g., 4 MB) due to fragmentation, inactive containers spill to local NVMe as compressed state SSTables and are lazily reloaded on access.

**Normative rule:** Spilling MUST preserve correctness — a spilled offset's state MUST be readable before any lease/ACK decision for that offset.

---

## 14. Memory Invariants and Quotas

| ID | Invariant / Quota | Enforcement |
|---|---|---|
| MI-1 | Per-shard bitmap ≤ `max_bitmap_memory`; spill on exceed. | S7 Spill Manager. |
| MI-2 | `W_base` always advances (no stuck offset). | Mandatory DLQ eviction (§7.3). |
| MI-3 | Active lease map bounded per shard; throttle on exceed. | S4 Lease Manager + quotas. |
| MI-4 | Offsets `< W_base` are purged promptly. | S3 Watermark Advancer. |
| MI-5 | Node state-plane memory within published budget. | KEI-ARC-010 §9.2 budget formula. |
| MI-6 | Per-group `window_size_limit` (Δ_max) enforced. | S2 Overlay Store. |

---

## 15. State-Plane Failure Handling

| Scenario | Defense (this subsystem) |
|---|---|
| Coordinator crash | Snapshot + lease journal replay; epoch fencing (§13.2). |
| Split-brain partition | Epoch fencing; prefer shard unavailability over double-lease (§10.3). |
| Deep consumer lag / millions of leases | Adaptive container spilling (§13.3). |
| Stuck / poison-pill offset | Mandatory DLQ eviction (§7.3). |
| ACK loss on fast path | Bounded, documented loss window; redelivery (§11.1). |
| Cold-task re-lease latency | Tier-0 Sparse Queue Index (§8.4). |
| Worker crash mid-lease | Timing-wheel timeout returns offset to READY. |

---

## 16. NFR Traceability (Owned by This Subsystem)

| NFR | Requirement | How This Subsystem Satisfies It |
|---|---|---|
| DUR-003 | ACK_FAST bounded loss window | Async lease journal replication (§11.1). |
| DUR-004 | ACK_DURABLE zero loss | Raft commit before success (§11.2). |
| AVAIL-003 | Coordinator failover <3.5s | Snapshot + journal replay (§13.2). |
| AVAIL-004 | No double-lease under partition | Epoch fencing (§10.3). |
| SCALE-004 | ≥100 consumer groups/stream | Overlay-per-group, shared log (§12). |
| SCALE-005 | ≥1M concurrent leases | Timing wheel + sharding (§8, §10). |
| SCALE-006 | Coordinator load bounded per shard | Deterministic sharding (§10.1). |
| MEM-003 | Bitmap bounded + spill | Spill Manager (§13.3). |
| MEM-004 | Watermark advances | Mandatory DLQ eviction (§7.3). |
| MEM-006 | Lease map bounded | Lease Manager quotas (§14). |
| PERF-011 | Lease acquisition ≤1ms fast path | Local fast-path apply (§11.1). |
| OPS-006 | DLQ operability | Virtual DLQ + redrive (§9). |

---

## 17. Interfaces

### 17.1 Provided Interfaces

| Interface | Consumer | Semantics |
|---|---|---|
| `lease(group, stream, max_msgs, τ, ack_mode)` | Worker / Gateway | Grant up to N leases. |
| `ack(group, stream, offset, ack_mode)` | Worker / Gateway | Acknowledge an offset. |
| `nack(group, stream, offset)` | Worker / Gateway | Negative-acknowledge; requeue. |
| `renewLease(group, stream, offset, τ)` | Worker / Gateway | Extend a lease. |
| `streamFetch(group, stream, offset, max)` | Stream consumer | Sequential replay. |
| `dlqList / dlqRedrive(group, stream, offsets)` | Operator | DLQ inspection and redrive. |
| `getWatermark(group, stream)` | Observability | Return `W_base` and lag. |

### 17.2 Consumed Interfaces

| Interface | Provider | Purpose |
|---|---|---|
| `read(stream, offset_range)` | KEI-ARC-020 | Fetch payloads for delivery/DLQ. |
| Lease journal commit | KEI-ARC-022 | Durable state replication. |
| State snapshot store | KEI-ARC-022 | Bounded recovery. |
| Group/quota config | Control Plane | Shard and quota setup. |
| Authorization decision | KEI-ARC-025 | ABAC enforcement. |

---

## 18. Open Questions and ADR Dependencies

| Item | Status | Resolution Path |
|---|---|---|
| Default `R_max`, `T_max`, and Δ_max values | Open | Tune under Profile P4 before Phase-2 exit. |
| Lease journal batch interval for ACK_FAST | Open | Benchmark loss window vs. latency under P4. |
| Spill threshold (4 MB) tuning | Open | Validate under fragmented-lag soak test. |
| State-shard bucket count | Open | ADR pending (KEI-ARC-012); derive from coordinator load model. |

Binding decisions already recorded: ADR-002, ADR-003, ADR-004, ADR-020, ADR-021, ADR-022, ADR-023, ADR-024, ADR-025.

---

## 19. Glossary (Additions)

| Term | Definition |
|---|---|
| State Shard | The unit of state-plane ownership: hash(tenant, stream, group, bucket). |
| W_base | Sliding base watermark below which all state is purged. |
| Δ_max | The bounded active delta window above W_base. |
| Sparse Exception Table | Index of EVICTED_DLQ offsets and failure metadata. |
| Lease Journal | Ordered, replicated log of lease/ACK deltas. |
| Spill SSTable | Compressed NVMe-backed inactive bitmap container. |
| ACK_FAST / ACK_DURABLE | Selectable acknowledgment durability modes. |
| Coordinator Epoch | Monotonic generation fencing stale coordinators. |

---

## 20. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial consumption state-plane architecture. Defines the state machine, Roaring overlay model, watermark advancement with mandatory DLQ eviction, lease lifecycle and timing wheel, virtual DLQ, coordinator sharding and epoch fencing, ACK durability modes, and state persistence/spilling. Aligns to ADR-002…004 and ADR-020…025, and to NFRs DUR/AVAIL/SCALE/MEM/PERF/OPS. |