# KEI-DES-031 — State Plane Data Structures & Algorithms Specification

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-DES-031 |
| Title | State Plane Data Structures & Algorithms Specification |
| Version | 1.0 |
| Level | **L3 — Detailed Design Specification** |
| Subsystem Covered | Consumption State Plane |
| Status | Approved for Engineering |
| Classification | Internal / Engineering Confidential |
| Owner | Principal Engineer (Distributed Systems) |
| Required Reviewers | Chief Architect, Principal Engineer (Storage), SRE Lead, Security Lead |
| Depends On | KEI-ARC-021 (State Plane Architecture), KEI-ARC-022 (Consensus), KEI-ARC-027 (Operability), KEI-DES-030 (WAL Binary Format) |
| Consumed By | State plane implementation, coordinator implementation, recovery reconciler, chaos test suite |
| Keywords | MUST, MUST NOT, SHOULD, SHOULD NOT, MAY per RFC 2119 |

---

## 2. Purpose, Scope, and Audience

### 2.1 Purpose

This document specifies the **exact data structures, binary formats, and algorithms** used by the Consumption State Plane to project the immutable WAL into stream, queue, and virtual DLQ consumption modes.

It implements:

- ADR-002: Log-Bitmap duality.
- ADR-003: Virtual DLQ via flag.
- ADR-004: Mandatory DLQ eviction.
- ADR-020: ACK_FAST and ACK_DURABLE.
- ADR-023: Deterministic coordinator sharding.
- ADR-024: Epoch fencing.
- ADR-025: Hierarchical timing wheel.

### 2.2 Scope

**In scope:**

- State shard identity and layout.
- Roaring Bitmap representation for 64-bit offsets.
- Lease table and lease token design.
- Hierarchical timing wheel implementation.
- Watermark advancement algorithm.
- Lease acquisition, ACK, NACK, renewal, and timeout algorithms.
- Virtual DLQ and Sparse Exception Table structures.
- Lease journal binary format.
- State snapshot binary format.
- Bitmap spill SSTable format.
- Failover reconstruction algorithm.
- Memory quotas and validation rules.

**Out of scope:**

- WAL record binary format — owned by KEI-DES-030.
- Raft replication internals — owned by KEI-ARC-022.
- Client RPC wire protocol — owned by KEI-DES-032.
- Schema registry and shredding — owned by KEI-DES-033.
- Iceberg commit logic — owned by KEI-DES-034.

### 2.3 Audience

- State plane implementation engineers.
- Coordinator and failover engineers.
- Performance engineers optimizing bitmap and lease operations.
- Test engineers building correctness and chaos tests.
- Security engineers validating authorization and audit hooks.

---

## 3. Design Principles

| ID | Principle | Rationale |
|---|---|---|
| SP-1 | **The log is immutable; state is overlay.** | State transitions MUST NOT modify WAL records. |
| SP-2 | **Every mutable structure is bounded.** | Bitmaps, lease tables, timers, journals, and heaps MUST have quotas. |
| SP-3 | **Fast path and durable path are explicit.** | ACK_FAST and ACK_DURABLE MUST have separate code paths and semantics. |
| SP-4 | **Failover state is reconstructable.** | Snapshot + lease journal MUST fully restore shard state. |
| SP-5 | **Stale operations are fenced.** | Epoch and lease token validation MUST reject stale operations. |
| SP-6 | **Watermark advancement is guaranteed.** | Mandatory DLQ eviction MUST prevent stuck offsets. |
| SP-7 | **State operations are idempotent where possible.** | Duplicate ACKs and journal replays MUST NOT corrupt state. |

---

## 4. State Shard Identity

### 4.1 State Shard Key

```rust
#[repr(C, align(64))]
pub struct StateShardKey {
    pub tenant_id: u64,
    pub stream_id: u128,
    pub group_id: u64,
    pub shard_bucket: u16,
    pub reserved: [u8; 6],
}
```

Total size: 64 bytes after alignment.

### 4.2 Shard Assignment

```text
state_shard_key = (tenant_id, stream_id, group_id, shard_bucket)
coordinator_node = consistent_hash(state_shard_key)
```

**Normative rules:**

- Default `shard_bucket` MUST be `0`.
- Optional bucketing MAY be enabled by control plane for high-lease-volume groups.
- When bucketing is enabled, each bucket MUST own a disjoint offset subrange.
- Group-level progress MUST be computed as the minimum `W_base` across all buckets.

### 4.3 Offset Range Ownership

```rust
pub struct ShardOffsetRange {
    pub range_start_offset: u64,
    pub range_end_offset: u64, // exclusive
}
```

For default single-bucket groups:

```text
range_start_offset = 0
range_end_offset = u64::MAX
```

---

## 5. Roaring Bitmap Representation

### 5.1 64-Bit Offset Model

Offsets are 64-bit. Roaring Bitmaps natively cover 32-bit spaces. Therefore, PEF uses a partitioned 64-bit Roaring structure:

```rust
pub struct Roaring64Map {
    pub partitions: BTreeMap<u32, Roaring32Bitmap>,
}
```

Where:

```text
offset          = u64
high32          = (offset >> 32) as u32
low32           = (offset & 0xFFFF_FFFF) as u32
partition_key   = high32
low16_container = (low32 >> 16) as u16
low16_value     = (low32 & 0xFFFF) as u16
```

### 5.2 Bitmap Types Per State Shard

```rust
pub struct ShardBitmaps {
    pub acked: Roaring64Map,
    pub dlq: Roaring64Map,
    pub leased: Roaring64Map,
}
```

Derived state:

```text
READY(offset) = !acked.contains(offset)
              && !dlq.contains(offset)
              && !leased.contains(offset)
```

Terminal state:

```text
TERMINAL(offset) = acked.contains(offset) || dlq.contains(offset)
```

### 5.3 Container Types

Each 32-bit Roaring partition uses standard Roaring container types:

| Container | Use Case | Memory Behavior |
|---|---|---|
| Array container | Sparse leases, sparse DLQ entries | 2 bytes per offset. |
| Bitset container | Mixed ACK/NACK regions within a 65,536-offset block | Fixed 8 KiB. |
| Run container | Dense consecutive ACK or DLQ runs | ~4 bytes per run. |

### 5.4 Container Conversion Rules

- Array container converts to bitset when cardinality exceeds 4096 within a 16-bit container.
- Bitset container converts to array when cardinality falls below 4096.
- `run_optimize()` MUST be invoked after batch mutations to convert dense bitsets into run containers.
- Run optimization MUST be batched to avoid CPU spikes on the hot path.

### 5.5 Bitmap Memory Accounting

For each shard:

```text
M_bitmap =
    M_partition_maps
  + M_array_containers
  + M_bitset_containers
  + M_run_containers
  + M_spill_cache_index
```

Approximations:

```text
array_container  ≈ 2 bytes × cardinality + container overhead
bitset_container = 8192 bytes
run_container    ≈ 4 bytes × run_count + container overhead
```

**Normative rule:** Bitmap memory MUST be tracked per shard and exposed as `keirox_bitmap_memory_bytes`.

---

## 6. Consumer Group Shard State

### 6.1 Primary State Structure

```rust
pub struct ConsumerGroupShardState {
    pub shard_key: StateShardKey,
    pub coordinator_epoch: u64,
    pub base_watermark: u64,
    pub head_offset: u64,
    pub lease_cursor: u64,
    pub bitmaps: ShardBitmaps,
    pub active_leases: LeaseTable,
    pub retry_heap: RetryHeap,
    pub timing_wheel: TimingWheel,
    pub journal_buffer: JournalBuffer,
    pub spill_state: SpillState,
    pub quotas: ShardQuotas,
    pub dirty_flags: DirtyFlags,
}
```

### 6.2 Shard Quotas

```rust
pub struct ShardQuotas {
    pub max_window_size: u64,
    pub max_active_leases: u32,
    pub max_bitmap_memory_bytes: u64,
    pub max_retry_heap_entries: u32,
    pub max_journal_buffer_bytes: u32,
}
```

Defaults:

| Field | Default | Notes |
|---|---:|---|
| `max_window_size` | 1,000,000 | Offsets above `W_base`. |
| `max_active_leases` | 100,000 | Per shard. |
| `max_bitmap_memory_bytes` | 256 MB | Before spill pressure. |
| `max_retry_heap_entries` | 1,000,000 | Bounded retry queue. |
| `max_journal_buffer_bytes` | 16 MB | Fast-path batch buffer. |

### 6.3 Dirty Flags

```rust
pub struct DirtyFlags {
    pub bitmap_dirty: bool,
    pub lease_dirty: bool,
    pub watermark_dirty: bool,
    pub retry_heap_dirty: bool,
}
```

Dirty flags drive snapshot emission and journal flushing.

---

## 7. Lease Table

### 7.1 Lease Structure

```rust
#[repr(C, align(64))]
pub struct Lease {
    pub offset: u64,
    pub lease_token: u64,
    pub worker_id: u64,
    pub lease_expiry_ms: u64,
    pub first_leased_at_ms: u64,
    pub last_renewed_at_ms: u64,
    pub coordinator_epoch: u64,
    pub retry_count: u8,
    pub flags: u8,
    pub reserved: [u8; 6],
}
```

Total size: 64 bytes.

### 7.2 Lease Flags

| Bit | Name | Meaning |
|---|---|---|
| 0 | `RENEWED` | Lease has been renewed at least once. |
| 1 | `FAST_ACK_ELIGIBLE` | Worker may use ACK_FAST. |
| 2 | `DURABLE_ACK_REQUIRED` | Worker must use ACK_DURABLE. |
| 3 | `RETRY_CANDIDATE` | Offset has prior failed attempts. |
| 4 | `COLD_TASK` | Payload may reside in Tier-1. |

### 7.3 Lease Table Implementation

```rust
pub struct LeaseTable {
    pub by_offset: HashMap<u64, Lease>,
    pub count: u32,
}
```

**Normative rules:**

- The lease table MUST support O(1) lookup by offset.
- The lease table MUST reject insertion when `count >= max_active_leases`.
- Lease tokens MUST be unique per `(shard, offset)` within a coordinator epoch.

### 7.4 Lease Token Generation

```text
lease_token = monotonic shard-local u64
```

Or:

```text
lease_token = hash(coordinator_epoch, offset, worker_id, grant_timestamp)
```

The token MUST be validated on ACK, NACK, and renewal.

---

## 8. Hierarchical Timing Wheel

### 8.1 Purpose

The timing wheel manages lease expirations with O(1) insertion and amortized O(1) expiration.

### 8.2 Wheel Configuration

```rust
pub struct TimingWheelConfig {
    pub tick_ms: u64,
    pub level_slots: [u16; 4],
    pub max_lease_ttl_ms: u64,
}
```

Recommended default:

```text
tick_ms = 10 ms
level_slots = [256, 64, 64, 64]
```

Approximate coverage:

| Level | Slot Count | Resolution | Coverage |
|---|---:|---:|---:|
| 0 | 256 | 10 ms | 2.56 seconds |
| 1 | 64 | 2.56 s | ~164 seconds |
| 2 | 64 | 164 s | ~2.9 hours |
| 3 | 64 | 2.9 hours | ~7.8 days |

### 8.3 Timer Entry

```rust
#[repr(C, align(32))]
pub struct TimerEntry {
    pub offset: u64,
    pub lease_token: u64,
    pub expiry_ms: u64,
    pub wheel_level: u8,
    pub slot: u16,
    pub flags: u8,
    pub reserved: [u8; 4],
}
```

Total size: 32 bytes.

### 8.4 Lazy Cancellation

PEF uses lazy timer cancellation:

1. On ACK/NACK/renewal, the active lease is removed or updated.
2. Existing timer entry remains in wheel.
3. When timer fires, handler validates current lease state.
4. If lease token or expiry does not match, timer event is ignored.

**Normative rule:** Timer firing MUST NOT mutate state unless the active lease exactly matches the timer’s `offset`, `lease_token`, and `expiry_ms`.

### 8.5 Timer Rebuild on Failover

Timing wheels are not snapshotted. On recovery:

```text
for lease in active_leases:
    remaining = lease.lease_expiry_ms - current_wall_ms
    if remaining <= 0:
        schedule immediate expiration
    else:
        insert timer with remaining duration
```

---

## 9. Retry Heap

### 9.1 Purpose

The retry heap prioritizes redelivery of previously failed or timed-out tasks.

```rust
pub struct RetryHeap {
    pub heap: BinaryHeap<RetryEntry>,
    pub count: u32,
}

#[repr(C, align(32))]
pub struct RetryEntry {
    pub offset: u64,
    pub retry_count: u8,
    pub reserved: [u8; 7],
    pub enqueued_at_ms: u64,
    pub priority_score: u64,
}
```

### 9.2 Priority Rule

```text
priority_score = (retry_count << 56) | inverse_offset_weight
```

Higher retry count SHOULD be prioritized to resolve partially processed work earlier.

### 9.3 Retry Heap Rules

- Retry heap MUST be bounded by `max_retry_heap_entries`.
- If heap is full, the lowest-priority entry MAY be rejected and requeued via normal ready scan.
- Duplicate retry entries for the same offset MUST be suppressed.
- Entries whose offset is terminal MUST be discarded on pop.

---

## 10. Watermark Advancement Algorithm

### 10.1 Definition

```text
W_base = max { k | ∀ i < k, State(i) = ACKED ∨ State(i) = EVICTED_DLQ }
```

### 10.2 Algorithm

```rust
fn advance_watermark(shard: &mut ConsumerGroupShardState) {
    let terminal = shard.bitmaps.acked.or(&shard.bitmaps.dlq);

    let next_nonterminal = terminal.first_missing_from(shard.base_watermark);

    if next_nonterminal > shard.base_watermark {
        shard.purge_range(shard.base_watermark, next_nonterminal);
        shard.base_watermark = next_nonterminal;
        shard.dirty_flags.watermark_dirty = true;

        emit_metric!(
            "keirox_watermark_advanced_offsets",
            next_nonterminal - shard.base_watermark
        );
    }
}
```

### 10.3 Purge Rules

For offsets `< W_base`:

- ACK bits MUST be removed.
- DLQ bits MUST be removed from active bitmap but Sparse Exception Table metadata MAY be retained.
- Leased bits MUST already be absent because leased offsets are non-terminal.
- Retry heap entries below `W_base` MUST be discarded.
- Spilled containers below `W_base` MUST be marked eligible for deletion.

### 10.4 Mandatory DLQ Eviction

```rust
fn maybe_evict_to_dlq(
    shard: &mut ConsumerGroupShardState,
    offset: u64,
    retry_count: u8,
    time_in_flight_ms: u64,
) -> bool {
    if retry_count >= shard.quotas.max_retries
        || time_in_flight_ms >= shard.quotas.max_time_in_flight_ms
    {
        evict_to_dlq(shard, offset, DlqReason::RetryLimitExceeded);
        return true;
    }
    false
}
```

**Normative rule:** Eviction MUST set the DLQ bit, insert a Sparse Exception Table entry, and trigger watermark advancement.

---

## 11. Lease Acquisition Algorithm

### 11.1 LeaseNext Request

```rust
pub struct LeaseNextRequest {
    pub shard_key: StateShardKey,
    pub worker_id: u64,
    pub max_messages: u32,
    pub lease_ttl_ms: u32,
    pub ack_mode: AckMode,
    pub coordinator_epoch: u64,
}
```

### 11.2 Algorithm

```rust
fn lease_next(
    shard: &mut ConsumerGroupShardState,
    req: LeaseNextRequest,
) -> Result<Vec<Lease>, StateError> {
    if req.coordinator_epoch != shard.coordinator_epoch {
        return Err(StateError::StaleEpoch);
    }

    if shard.active_leases.count >= shard.quotas.max_active_leases {
        return Err(StateError::LeaseQuotaExceeded);
    }

    let mut granted = Vec::with_capacity(req.max_messages as usize);

    while granted.len() < req.max_messages as usize {
        let offset = next_lease_candidate(shard)?;

        match offset {
            None => break,
            Some(offset) => {
                if offset >= shard.base_watermark + shard.quotas.max_window_size {
                    break;
                }

                let lease = grant_lease(shard, offset, &req)?;
                granted.push(lease);
            }
        }
    }

    if !granted.is_empty() {
        shard.dirty_flags.lease_dirty = true;
        journal_grants(shard, &granted, req.ack_mode)?;
    }

    Ok(granted)
}
```

### 11.3 Candidate Selection

```rust
fn next_lease_candidate(
    shard: &mut ConsumerGroupShardState,
) -> Result<Option<u64>, StateError> {
    // 1. Prefer retry candidates.
    while let Some(retry_entry) = shard.retry_heap.pop() {
        if is_ready(shard, retry_entry.offset) {
            return Ok(Some(retry_entry.offset));
        }
    }

    // 2. Otherwise find next ready offset.
    let blocked = shard.bitmaps.acked
        .or(&shard.bitmaps.dlq)
        .or(&shard.bitmaps.leased);

    let from = max(shard.base_watermark, shard.lease_cursor);
    let next_ready = blocked.first_missing_from(from);

    if next_ready <= shard.head_offset {
        shard.lease_cursor = next_ready + 1;
        return Ok(Some(next_ready));
    }

    Ok(None)
}
```

### 11.4 Grant Lease

```rust
fn grant_lease(
    shard: &mut ConsumerGroupShardState,
    offset: u64,
    req: &LeaseNextRequest,
) -> Result<Lease, StateError> {
    let now = current_wall_ms();
    let token = next_lease_token(shard);

    let lease = Lease {
        offset,
        lease_token: token,
        worker_id: req.worker_id,
        lease_expiry_ms: now + req.lease_ttl_ms as u64,
        first_leased_at_ms: now,
        last_renewed_at_ms: now,
        coordinator_epoch: shard.coordinator_epoch,
        retry_count: get_retry_count(shard, offset),
        flags: lease_flags(req),
        reserved: [0; 6],
    };

    shard.bitmaps.leased.insert(offset);
    shard.active_leases.insert(offset, lease.clone())?;
    shard.timing_wheel.insert(offset, token, lease.lease_expiry_ms);

    Ok(lease)
}
```

---

## 12. ACK Algorithm

### 12.1 ACK Request

```rust
pub struct AckRequest {
    pub shard_key: StateShardKey,
    pub offset: u64,
    pub lease_token: u64,
    pub worker_id: u64,
    pub ack_mode: AckMode,
    pub coordinator_epoch: u64,
    pub idempotency_key: Option<u64>,
}
```

### 12.2 Fast-Path ACK

```rust
fn ack_fast(
    shard: &mut ConsumerGroupShardState,
    req: AckRequest,
) -> Result<AckResult, StateError> {
    validate_epoch(shard, req.coordinator_epoch)?;

    if shard.bitmaps.acked.contains(req.offset) {
        return Ok(AckResult::AlreadyAcked);
    }

    validate_active_lease(shard, req.offset, req.lease_token, req.worker_id)?;

    shard.bitmaps.acked.insert(req.offset);
    shard.bitmaps.leased.remove(req.offset);
    shard.active_leases.remove(req.offset);

    shard.dirty_flags.bitmap_dirty = true;
    shard.dirty_flags.watermark_dirty = true;

    journal_append(shard, JournalOp::Ack(req))?;

    advance_watermark(shard);

    Ok(AckResult::AcceptedFast)
}
```

### 12.3 Durable ACK

```rust
fn ack_durable(
    shard: &mut ConsumerGroupShardState,
    req: AckRequest,
) -> Result<AckResult, StateError> {
    validate_epoch(shard, req.coordinator_epoch)?;

    if shard.bitmaps.acked.contains(req.offset) {
        return Ok(AckResult::AlreadyAcked);
    }

    validate_active_lease(shard, req.offset, req.lease_token, req.worker_id)?;

    // Apply locally first.
    shard.bitmaps.acked.insert(req.offset);
    shard.bitmaps.leased.remove(req.offset);
    shard.active_leases.remove(req.offset);

    // Append and wait for metadata Raft commit.
    journal_append_durable(shard, JournalOp::Ack(req)).await?;

    advance_watermark(shard);

    Ok(AckResult::AcceptedDurable)
}
```

### 12.4 ACK Validation Rules

| Condition | Behavior |
|---|---|
| Offset already ACKED | Return success idempotently. |
| Offset in DLQ | Return error `OFFSET_EVICTED`. |
| Lease token mismatch | Return error `STALE_LEASE`. |
| Worker mismatch | Return error `WORKER_MISMATCH`. |
| Epoch mismatch | Return error `STALE_EPOCH`. |
| Offset not leased | Return error `LEASE_NOT_ACTIVE`. |

**Normative rule:** Duplicate ACKs for already-ACKED offsets MUST return success to preserve idempotent worker behavior.

---

## 13. NACK, Renewal, and Timeout Algorithms

### 13.1 NACK

```rust
fn nack(
    shard: &mut ConsumerGroupShardState,
    req: NackRequest,
) -> Result<NackResult, StateError> {
    validate_epoch(shard, req.coordinator_epoch)?;
    validate_active_lease(shard, req.offset, req.lease_token, req.worker_id)?;

    let mut lease = shard.active_leases.remove(req.offset)?;
    shard.bitmaps.leased.remove(req.offset);
    lease.retry_count += 1;

    if maybe_evict_to_dlq(
        shard,
        req.offset,
        lease.retry_count,
        time_in_flight(&lease),
    ) {
        return Ok(NackResult::EvictedToDlq);
    }

    shard.retry_heap.push(RetryEntry::from(&lease));
    shard.dirty_flags.retry_heap_dirty = true;

    journal_append(shard, JournalOp::Nack(req))?;

    Ok(NackResult::Requeued)
}
```

### 13.2 Renew Lease

```rust
fn renew_lease(
    shard: &mut ConsumerGroupShardState,
    req: RenewRequest,
) -> Result<RenewResult, StateError> {
    validate_epoch(shard, req.coordinator_epoch)?;

    let lease = shard.active_leases.get_mut(&req.offset)?;

    if lease.lease_token != req.lease_token {
        return Err(StateError::StaleLease);
    }

    let now = current_wall_ms();
    lease.lease_expiry_ms = now + req.new_ttl_ms as u64;
    lease.last_renewed_at_ms = now;
    lease.flags |= LeaseFlags::RENEWED;

    shard.timing_wheel.insert(
        lease.offset,
        lease.lease_token,
        lease.lease_expiry_ms,
    );

    journal_append(shard, JournalOp::Renew(req))?;

    Ok(RenewResult::Renewed)
}
```

### 13.3 Timer Expiration

```rust
fn on_timer_fire(
    shard: &mut ConsumerGroupShardState,
    entry: TimerEntry,
) {
    let Some(lease) = shard.active_leases.get(&entry.offset) else {
        return; // ACKed, NACKed, or removed.
    };

    if lease.lease_token != entry.lease_token
        || lease.lease_expiry_ms != entry.expiry_ms
    {
        return; // Stale timer.
    }

    let mut lease = shard.active_leases.remove(entry.offset).unwrap();
    shard.bitmaps.leased.remove(entry.offset);
    lease.retry_count += 1;

    if maybe_evict_to_dlq(
        shard,
        lease.offset,
        lease.retry_count,
        time_in_flight(&lease),
    ) {
        return;
    }

    shard.retry_heap.push(RetryEntry::from(&lease));
    shard.dirty_flags.retry_heap_dirty = true;

    journal_append(
        shard,
        JournalOp::LeaseTimeout {
            offset: lease.offset,
            lease_token: lease.lease_token,
            retry_count: lease.retry_count,
        },
    );
}
```

---

## 14. Virtual DLQ and Sparse Exception Table

### 14.1 DLQ Entry

```rust
#[repr(C, align(64))]
pub struct DlqExceptionEntry {
    pub tenant_id: u64,
    pub stream_id: u128,
    pub group_id: u64,
    pub offset: u64,
    pub reason: u8,
    pub retry_count: u8,
    pub reserved: [u8; 6],
    pub first_leased_at_ms: u64,
    pub evicted_at_ms: u64,
    pub last_worker_id: u64,
}
```

### 14.2 Sparse Exception Table

```rust
pub struct SparseExceptionTable {
    pub entries: BTreeMap<u64, DlqExceptionEntry>,
}
```

Keyed by offset.

### 14.3 DLQ Redrive

```rust
fn redrive(
    shard: &mut ConsumerGroupShardState,
    offsets: &[u64],
) -> Result<RedriveResult, StateError> {
    for offset in offsets {
        if !shard.bitmaps.dlq.contains(*offset) {
            continue;
        }

        shard.bitmaps.dlq.remove(*offset);
        shard.sparse_exception_table.remove(*offset)?;
        shard.retry_heap.push(RetryEntry::redrive(*offset));
    }

    shard.dirty_flags.bitmap_dirty = true;
    shard.dirty_flags.retry_heap_dirty = true;

    journal_append(shard, JournalOp::Redrive(offsets.to_vec()))?;

    Ok(RedriveResult::Accepted)
}
```

**Normative rule:** DLQ redrive MUST be authorized, audited, and idempotent.

---

## 15. Lease Journal Binary Format

### 15.1 Journal Frame

```rust
#[repr(C, align(64))]
pub struct JournalFrameHeader {
    pub magic: u32,              // 0x4B4A524E ("KJRN")
    pub format_version: u16,
    pub flags: u16,
    pub frame_lsn: u64,
    pub shard_key: StateShardKey,
    pub coordinator_epoch: u64,
    pub entry_count: u32,
    pub frame_payload_len: u32,
    pub created_timestamp_ms: u64,
    pub frame_crc32c: u32,
    pub reserved: [u8; 20],
}
```

### 15.2 Journal Entry Header

```rust
#[repr(C, packed)]
pub struct JournalEntryHeader {
    pub entry_len: u32,
    pub op_type: u8,
    pub reserved: [u8; 3],
    pub timestamp_ms: u64,
    pub entry_crc32c: u32,
}
```

### 15.3 Journal Operation Types

| Op Code | Name | Payload |
|---:|---|---|
| 1 | `LEASE_GRANT` | Offset, token, worker, TTL, retry count. |
| 2 | `LEASE_RENEW` | Offset, token, new expiry. |
| 3 | `ACK` | Offset, token, worker, ack mode. |
| 4 | `NACK` | Offset, token, retry count. |
| 5 | `LEASE_TIMEOUT` | Offset, token, retry count. |
| 6 | `EVICT_DLQ` | Offset, reason, retry count. |
| 7 | `REDRIVE` | Offset list. |
| 8 | `WATERMARK_COMMIT` | New `W_base`. |
| 9 | `SNAPSHOT_MARKER` | Snapshot ID, LSN. |

### 15.4 Journal Durability Rules

| Mode | Behavior |
|---|---|
| `ACK_FAST` | Journal entry appended to local buffer; batch-replicated asynchronously. |
| `ACK_DURABLE` | Journal entry MUST be committed to metadata Raft before success. |
| Lease grant | Default batch-replicated; durable lease mode MAY be configured. |
| Watermark commit | Replicated before being exposed as committed `W_base`. |

---

## 16. State Snapshot Format

### 16.1 Snapshot Header

```rust
#[repr(C, align(4096))]
pub struct StateSnapshotHeader {
    pub magic: u32,              // 0x4B535350 ("KSSP")
    pub format_version: u16,
    pub flags: u16,
    pub snapshot_id: u64,
    pub journal_lsn: u64,
    pub shard_key: StateShardKey,
    pub coordinator_epoch: u64,
    pub base_watermark: u64,
    pub head_offset: u64,
    pub lease_cursor: u64,
    pub acked_count: u64,
    pub dlq_count: u64,
    pub active_lease_count: u32,
    pub retry_heap_count: u32,
    pub created_timestamp_ms: u64,
    pub snapshot_crc32c: u32,
    pub reserved: [u8; 3968],
}
```

### 16.2 Snapshot Body

Snapshot body contains:

1. Serialized `acked` Roaring64Map.
2. Serialized `dlq` Roaring64Map.
3. Serialized active lease array.
4. Serialized retry heap.
5. Sparse Exception Table index.
6. Shard quota configuration.
7. Optional compression metadata.

### 16.3 Snapshot Rules

- Timing wheel MUST NOT be serialized.
- Journal buffer MUST be flushed before snapshot marker.
- Snapshot MUST include the last applied journal LSN.
- Snapshot MUST be checksummed with CRC32C.
- Snapshot MAY be compressed with zstd.

### 16.4 Snapshot Frequency

Default:

```text
Every 30 seconds OR every 256 MB journal, whichever occurs first.
```

---

## 17. Bitmap Spill SSTable Format

### 17.1 Spill Header

```rust
#[repr(C, align(4096))]
pub struct SpillSstableHeader {
    pub magic: u32,              // 0x4B53504C ("KSPL")
    pub format_version: u16,
    pub flags: u16,
    pub shard_key: StateShardKey,
    pub bitmap_type: u8,         // 1=acked, 2=dlq, 3=leased
    pub compression_type: u8,
    pub reserved: [u8; 2],
    pub range_start_offset: u64,
    pub range_end_offset: u64,
    pub container_count: u32,
    pub created_timestamp_ms: u64,
    pub spill_crc32c: u32,
    pub reserved_pad: [u8; 4040],
}
```

### 17.2 Spill Candidate Selection

A bitmap container MAY be spilled when:

```text
container_key NOT IN hot_range
AND last_access_ms < now - idle_threshold_ms
AND shard_bitmap_memory > spill_pressure_threshold
```

Hot range:

```text
hot_range = [
    W_base,
    min(W_base + hot_window, lease_cursor + hot_window)
]
```

Default `hot_window`: 1,000,000 offsets.

### 17.3 Spill Access Rules

- Spilled containers MUST be loaded before any state decision involving offsets in that container.
- Spill cache MUST be LRU-bounded.
- Spilled containers below `W_base` MUST be deleted during purge.
- Spill MUST NOT remove active lease containers unless explicitly idle and re-loadable.

---

## 18. Failover Reconstruction Algorithm

### 18.1 Recovery Inputs

```text
Latest valid state snapshot
All journal frames with LSN > snapshot.journal_lsn
Coordinator epoch assignment from metadata Raft
```

### 18.2 Algorithm

```rust
fn recover_shard(
    shard_key: StateShardKey,
    new_epoch: u64,
) -> Result<ConsumerGroupShardState, StateError> {
    let snapshot = load_latest_snapshot(shard_key)?;
    let mut state = ConsumerGroupShardState::from(snapshot)?;

    for frame in journal_frames_after(snapshot.journal_lsn) {
        validate_frame_crc(&frame)?;
        validate_frame_epoch(&frame, new_epoch)?;
        apply_journal_frame(&mut state, &frame)?;
    }

    state.coordinator_epoch = new_epoch;
    rebuild_timing_wheel(&mut state);
    advance_watermark(&mut state);
    validate_state_invariants(&state)?;

    Ok(state)
}
```

### 18.3 Validation Invariants

After recovery, the following MUST hold:

| Invariant | Check |
|---|---|
| No leased offset is ACKED. | `leased AND acked == empty` |
| No leased offset is DLQ. | `leased AND dlq == empty` |
| No active lease below `W_base`. | `active_leases.offsets >= W_base` |
| `W_base` points to first non-terminal offset. | `terminal.first_missing_from(W_base) == W_base` |
| Retry heap contains no terminal offsets. | Validate on pop. |
| Lease tokens are unique per active offset. | Lease table check. |

---

## 19. Concurrency Model

### 19.1 Shard Ownership

Each state shard is owned by exactly one active coordinator.

**Normative rule:** No two coordinators MAY concurrently apply state mutations for the same shard.

### 19.2 Internal Threading

Within a coordinator:

| Thread Pool | Responsibility |
|---|---|
| Shard mutation threads | Apply lease/ACK/NACK operations. |
| Journal writer thread | Batches and appends journal frames. |
| Timer wheel thread | Fires lease expirations. |
| Snapshot emitter thread | Produces periodic snapshots. |
| Spill manager thread | Spills and loads bitmap containers. |

### 19.3 Locking Rules

- Each shard SHOULD use a sharded mutex or single-writer event loop.
- Bitmap mutation MUST NOT occur concurrently for the same shard without synchronization.
- Read-only queries MAY use RCU-style snapshots where available.

---

## 20. Validation Rules

### 20.1 Write-Path State Validation

| Check | Failure Behavior |
|---|---|
| Coordinator epoch matches | Reject with `STALE_EPOCH`. |
| Offset within shard range | Reject with `INVALID_OFFSET`. |
| Lease token matches | Reject with `STALE_LEASE`. |
| ACK mode allowed | Reject with `INVALID_ACK_MODE`. |
| Lease quota not exceeded | Reject with `LEASE_QUOTA_EXCEEDED`. |
| Bitmap memory quota not exceeded | Trigger spill or backpressure. |

### 20.2 Recovery Validation

| Check | Failure Behavior |
|---|---|
| Snapshot CRC invalid | Use previous snapshot. |
| Journal frame CRC invalid | Stop replay at last valid LSN; alert. |
| Epoch mismatch in journal | Quarantine frame; alert. |
| Invariant violation after replay | Enter safe mode; require operator intervention. |

### 20.3 Compile-Time Size Assertions

```rust
const _: () = assert!(std::mem::size_of::<StateShardKey>() == 64);
const _: () = assert!(std::mem::size_of::<Lease>() == 64);
const _: () = assert!(std::mem::size_of::<TimerEntry>() == 32);
const _: () = assert!(std::mem::size_of::<StateSnapshotHeader>() == 4096);
const _: () = assert!(std::mem::size_of::<SpillSstableHeader>() == 4096);
```

---

## 21. NFR Traceability

| NFR | Requirement | How This Specification Satisfies It |
|---|---|---|
| DUR-003 | ACK_FAST bounded loss | Async journal batch replication (§15.4). |
| DUR-004 | ACK_DURABLE zero loss | Raft commit before success (§12.3). |
| AVAIL-003 | Coordinator failover <3.5s | Snapshot + journal recovery (§18). |
| AVAIL-004 | No double-lease under partition | Epoch + lease token fencing (§11, §12). |
| SCALE-004 | ≥100 consumer groups/stream | Shard-per-group model (§4). |
| SCALE-005 | ≥1M concurrent leases | Lease table + timing wheel + quotas (§7, §8). |
| SCALE-006 | Coordinator load bounded | Deterministic shard assignment (§4.2). |
| MEM-003 | Bitmap bounded + spill | Spill SSTable and quotas (§17). |
| MEM-004 | Watermark advances | Mandatory DLQ eviction (§10.4). |
| MEM-006 | Lease map bounded | Lease quota and rejection (§7.3). |
| PERF-011 | Lease acquisition ≤1ms fast path | Local in-memory mutation before journal (§11). |
| OPS-006 | DLQ operability | Sparse Exception Table and redrive (§14). |

---

## 22. Interfaces

### 22.1 Provided Interfaces

| Interface | Consumer | Semantics |
|---|---|---|
| `leaseNext(req)` | Worker / Gateway | Grant ready offsets. |
| `ack(req)` | Worker / Gateway | Acknowledge leased offset. |
| `nack(req)` | Worker / Gateway | Negative-acknowledge offset. |
| `renewLease(req)` | Worker / Gateway | Extend lease TTL. |
| `redrive(offsets)` | Operator API | Requeue DLQ offsets. |
| `getWatermark(shard)` | Observability | Return current `W_base`. |
| `snapshotShard(shard)` | Snapshot Manager | Emit state snapshot. |
| `recoverShard(shard, epoch)` | Failover Orchestrator | Reconstruct shard state. |

### 22.2 Consumed Interfaces

| Interface | Provider | Purpose |
|---|---|---|
| `read(stream, offset_range)` | Storage Engine | Payload delivery. |
| `appendJournal(frame)` | Metadata Raft | Durable state replication. |
| `commitSnapshot(snapshot)` | Metadata Raft / Backup | Durable snapshot storage. |
| `authorize(principal, op)` | Security Plane | ABAC enforcement. |
| `emitMetric(metric)` | Operability Plane | Observability. |

---

## 23. Open Questions

| Item | Status | Resolution Path |
|---|---|---|
| Default timing-wheel tick | Open | Benchmark lease churn under Profile P4. |
| Journal batch interval for ACK_FAST | Open | Measure loss window vs. latency. |
| Snapshot interval tuning | Open | Validate recovery time under shard size. |
| Retry heap priority formula | Open | Benchmark poison-pill resolution latency. |
| Bitmap spill hot-window size | Open | Tune under fragmented-lag soak test. |
| Optional bucketing strategy | Open | Requires ADR before production enablement. |

---

## 24. Glossary

| Term | Definition |
|---|---|
| State Shard | The unit of consumption-state ownership. |
| W_base | Sliding base watermark below which state is purged. |
| Lease Token | Unique token fencing stale lease operations. |
| Retry Heap | Bounded priority queue for failed or timed-out tasks. |
| Sparse Exception Table | Metadata index for virtual DLQ entries. |
| Spill SSTable | Compressed NVMe-backed bitmap container file. |
| Journal LSN | Log sequence number for lease journal frames. |
| Hot Range | Offset range kept resident for low-latency state decisions. |

---

## 25. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial state-plane data structures and algorithms specification. Defines Roaring64Map model, lease table, timing wheel, watermark advancement, ACK modes, DLQ eviction, journal format, snapshot format, spill SSTable format, and failover reconstruction. |