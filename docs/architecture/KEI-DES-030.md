# KEI-DES-030 — WAL Binary Format & On-Disk Layout Specification

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-DES-030 |
| Title | WAL Binary Format & On-Disk Layout Specification |
| Version | 1.0 |
| Level | **L3 — Detailed Design Specification** |
| Subsystem Covered | Storage Engine — Tier-0 NVMe Write-Ahead Log |
| Status | Approved for Engineering |
| Classification | Internal / Engineering Confidential |
| Owner | Principal Engineer (Storage) |
| Required Reviewers | Chief Architect, Principal Engineer (Distributed Systems), Security Lead |
| Depends On | KEI-ARC-020 (Storage Engine Architecture), KEI-ARC-012 (ADRs), KEI-ARC-011 (NFRs) |
| Consumed By | Storage engine implementation, recovery manager, consensus replication, state plane reader |
| Keywords | MUST, MUST NOT, SHOULD, SHOULD NOT, MAY per RFC 2119 |

---

## 2. Purpose, Scope, and Audience

### 2.1 Purpose

This document specifies the **exact binary format, on-disk layout, and structural contracts** of the Tier-0 NVMe Write-Ahead Log (WAL). It provides implementation-level precision for engineers writing the WAL writer, recovery manager, consensus replication layer, and state plane reader.

It implements:

- ADR-013: Batch-oriented WAL framing with CRC32C.
- ADR-010: Multiplexed LSM-WAL over shared ring buffer.
- ADR-015: io_uring + O_DIRECT primary I/O path.
- ADR-050: Envelope encryption metadata placement.

### 2.2 Scope

**In scope:**

- Physical segment file layout and lifecycle.
- Batch frame binary format.
- Record entry binary format.
- CRC32C integrity boundaries.
- Page alignment and O_DIRECT constraints.
- Encryption metadata placement.
- Producer identity and idempotence fields.
- Transaction control records.
- Tombstone and special record types.
- Format versioning and forward compatibility.
- Rust `repr(C)` structural contracts.

**Out of scope:**

- WAL replication protocol — owned by KEI-ARC-022.
- Compaction and columnar transposition — owned by KEI-ARC-023.
- Consumption state overlays — owned by KEI-ARC-021.
- Tier-1 object storage format (Parquet/Iceberg) — owned by KEI-DES-034.

### 2.3 Audience

- Storage engine implementation engineers.
- Recovery and crash-recovery developers.
- Consensus replication engineers.
- Security engineers validating encryption metadata.
- Test engineers writing binary format validation.

---

## 3. Design Principles for Binary Format

| ID | Principle | Rationale |
|---|---|---|
| BF-1 | **Batch-oriented framing.** Common fields amortized into batch header; per-record entries carry only deltas. | Reduces overhead from ~72 bytes/record to ~32 bytes/record entry. |
| BF-2 | **CRC32C integrity at batch level.** | CRC16 is insufficient; CRC32C provides hardware-accelerated integrity. |
| BF-3 | **4096-byte page alignment.** | Required for O_DIRECT and io_uring zero-copy. |
| BF-4 | **Self-describing batches.** Each batch carries enough metadata for independent recovery. | Enables deterministic replay without external state. |
| BF-5 | **Forward-compatible versioning.** New fields append; old readers skip unknown fields. | Enables rolling upgrades without format breaks. |
| BF-6 | **Encryption metadata co-located.** DEK ID and nonce in batch header. | Enables decryption without external lookup. |
| BF-7 | **Immutable once sealed.** Sealed batches MUST NOT be rewritten. | Preserves Golden Invariant (GI-1). |

---

## 4. WAL Segment File Layout

### 4.1 Physical File Structure

A WAL segment is a preallocated 64 MB file.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        WAL SEGMENT FILE (64 MB)                             │
├─────────────────────────────────────────────────────────────────────────────┤
│ Offset 0x0000: Segment Header (4096 bytes)                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│ Offset 0x1000: Batch Frame 0                                                │
│ Offset 0x1000 + size(Batch 0): Batch Frame 1                               │
│ ...                                                                         │
│ Offset N: Batch Frame K                                                     │
├─────────────────────────────────────────────────────────────────────────────┤
│ Offset 0x3FFF000: Segment Footer (4096 bytes)                               │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 4.2 Segment Header (4096 bytes)

```rust
#[repr(C, align(4096))]
pub struct SegmentHeader {
    pub magic: u32,                    // 0x4B57414C ("KWAL")
    pub format_version: u16,           // Current: 1
    pub flags: u16,                    // Bit 0: sealed, Bit 1: encrypted
    pub segment_id: u64,              // Monotonic segment identifier
    pub volume_id: u32,               // Storage volume identifier
    pub node_id: u32,                 // Owning node at creation
    pub created_timestamp_ns: u64,    // Unix nanoseconds
    pub physical_seq_start: u64,      // First physical_seq in segment
    pub physical_seq_end: u64,        // Last physical_seq (filled on seal)
    pub batch_count: u32,             // Number of sealed batches
    pub record_count: u64,            // Total records across all batches
    pub segment_crc32c: u32,          // CRC32C of this header (excluding itself)
    pub reserved: [u8; 3968],         // Pad to 4096 bytes
}
```

**Normative rules:**

- `magic` MUST be `0x4B57414C` ("KWAL").
- `format_version` MUST be checked on read; unknown versions MUST trigger graceful rejection.
- `physical_seq_end` MUST be `0` until the segment is sealed.

### 4.3 Segment Footer (4096 bytes)

```rust
#[repr(C, align(4096))]
pub struct SegmentFooter {
    pub magic: u32,                    // 0x4B57414C ("KWAL")
    pub segment_id: u64,
    pub physical_seq_end: u64,
    pub batch_count: u32,
    pub record_count: u64,
    pub sealed_timestamp_ns: u64,
    pub footer_crc32c: u32,
    pub reserved: [u8; 4040],
}
```

**Normative rule:** The footer MUST be written only after the segment is sealed. Recovery MUST treat a segment without a valid footer as partially written.

---

## 5. Batch Frame Format

### 5.1 Batch Frame Layout

Each batch frame is the unit of CRC integrity, quorum replication, and recovery replay.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          BATCH FRAME                                        │
├─────────────────────────────────────────────────────────────────────────────┤
│ Batch Header (128 bytes)                                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│ Record Entry 0 (32 bytes)                                                   │
│ Record Entry 1 (32 bytes)                                                   │
│ ...                                                                         │
│ Record Entry N (32 bytes)                                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│ Payload Block (variable length, compressed or encrypted)                    │
├─────────────────────────────────────────────────────────────────────────────┤
│ CRC32C Trailer (16 bytes)                                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│ Padding to 4096-byte boundary                                               │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 5.2 Batch Header (128 bytes)

```rust
#[repr(C, align(64))]
pub struct BatchHeader {
    pub magic: u32,                    // 0x4B424154 ("KBAT")
    pub format_version: u16,           // Current: 1
    pub flags: u16,                    // See §5.3
    pub batch_physical_seq_start: u64, // First physical_seq in batch
    pub batch_record_count: u32,       // Number of records in batch
    pub batch_payload_len: u32,        // Total payload bytes (pre-compression)
    pub batch_compressed_len: u32,     // Payload bytes after compression
    pub tenant_id: u64,               // Tenant isolation identifier
    pub producer_id: u64,             // Producer identity (common to batch)
    pub producer_epoch: u32,          // Producer session epoch
    pub producer_seq_start: u64,      // First producer_seq in batch
    pub producer_seq_end: u64,        // Last producer_seq in batch
    pub schema_id: u32,               // Schema registry ID (common to batch)
    pub transaction_id: u64,          // 0 = non-transactional
    pub dek_id: u64,                  // Data Encryption Key identifier (0 = unencrypted)
    pub dek_version: u16,             // Key rotation version
    pub compression_type: u8,         // 0=none, 1=zstd, 2=lz4
    pub reserved: u8,
    pub batch_crc32c: u32,            // CRC32C of this header (excluding itself)
    pub reserved_pad: [u8; 12],       // Pad to 128 bytes
}
```

### 5.3 Batch Flags

| Bit | Name | Meaning |
|---|---|---|
| 0 | `ENCRYPTED` | Payload is encrypted; `dek_id` and `dek_version` are valid. |
| 1 | `COMPRESSED` | Payload is compressed; `compression_type` is valid. |
| 2 | `TRANSACTIONAL` | Batch is part of a transaction. |
| 3 | `TXN_COMMIT` | This batch is a transaction commit marker. |
| 4 | `TXN_ABORT` | This batch is a transaction abort marker. |
| 5 | `CONTAINS_TOMBSTONES` | At least one record is a tombstone. |
| 6 | `MULTI_STREAM` | Batch contains records from multiple streams. |
| 7 | `RECOVERY_DELTA` | Batch was written during recovery replay. |

**Normative rules:**

- If `ENCRYPTED` is set, `dek_id` MUST be non-zero.
- If `COMPRESSED` is set, `compression_type` MUST be non-zero.
- If `TRANSACTIONAL` is set, `transaction_id` MUST be non-zero.
- Bits 8–15 are reserved and MUST be zero.

---

## 6. Record Entry Format

### 6.1 Record Entry (32 bytes)

Each record entry is a lightweight pointer into the payload block. Common fields (producer, schema, tenant) are amortized into the batch header.

```rust
#[repr(C, packed)]
pub struct RecordEntry {
    pub stream_id: u128,              // Logical micro-stream identifier
    pub logical_offset: u64,          // Monotonic per-stream offset
    pub producer_seq_delta: u32,      // Delta from batch producer_seq_start
    pub payload_offset: u32,          // Byte offset into payload block
    pub payload_len: u32,             // Payload length in bytes
    pub timestamp_delta_ms: u32,      // Delta from batch timestamp (milliseconds)
    pub record_flags: u16,            // Per-record flags
    pub record_crc32c: u32,           // CRC32C of this entry (excluding itself)
}
```

### 6.2 Record Flags

| Bit | Name | Meaning |
|---|---|---|
| 0 | `TOMBSTONE` | Record is a deletion marker; payload is empty. |
| 1 | `COMPRESSED_INDIVIDUAL` | Individual record compressed (rare; usually batch-level). |
| 2 | `SCHEMA_OVERRIDE` | Record has a different schema than batch; schema_id in payload. |
| 3 | `CAUSAL_TAG` | Record carries a causal lineage tag. |
| 4 | `ENTITY_KEY_PRESENT` | Record carries an explicit entity key. |
| 5 | `SUB_ENTITY_KEY_PRESENT` | Record carries a sub-entity key. |

### 6.3 Multi-Stream Batches

When `MULTI_STREAM` flag is set in the batch header, each record entry carries its own `stream_id`. When `MULTI_STREAM` is not set, all records in the batch belong to the same stream, and `stream_id` in record entries MUST match the first record's stream.

**Normative rule:** The WAL writer SHOULD batch records from the same stream together to minimize per-record stream_id overhead.

---

## 7. Integrity Boundaries

### 7.1 CRC32C Hierarchy

Integrity is validated at three levels:

| Level | Scope | Validation |
|---|---|---|
| Segment | Entire segment file | Segment header CRC + footer CRC. |
| Batch | Single batch frame | Batch header CRC + batch payload CRC. |
| Record | Single record entry | Record entry CRC. |

### 7.2 Validation Order

On recovery replay, validation MUST proceed:

1. Validate segment header CRC.
2. Validate batch header CRC.
3. Validate batch payload CRC.
4. Validate individual record entry CRCs.

**Normative rule:** If a batch CRC fails, the entire batch MUST be treated as corrupt. Recovery MUST NOT attempt partial record recovery from a failed batch.

### 7.3 CRC32C Algorithm

CRC32C (Castagnoli) is used throughout because:

- Hardware acceleration via SSE4.2 / ARM CRC instructions.
- Better error detection than CRC32 for network/storage use.
- Compatible with io_uring and NVMe checksum offload.

**Normative rule:** CRC16 MUST NOT be used anywhere in the WAL format.

---

## 8. Page Alignment and O_DIRECT

### 8.1 Alignment Requirements

| Structure | Alignment | Rationale |
|---|---|---|
| Segment file | 4096 bytes | O_DIRECT requires block alignment. |
| Segment header | 4096 bytes | First block of file. |
| Batch frame start | 4096 bytes | Each batch starts on page boundary. |
| Batch padding | To 4096 bytes | Batch frames padded to page boundary. |
| I/O buffers | 4096 bytes | io_uring registered buffers. |
| Segment footer | 4096 bytes | Last block of file. |

### 8.2 Padding Rules

```text
batch_frame_size = header_size + (record_count × 32) + payload_len + crc_trailer_size
padded_size = ceil(batch_frame_size / 4096) × 4096
padding_bytes = padded_size - batch_frame_size
```

Padding bytes MUST be zero-filled.

**Normative rule:** Every batch frame MUST end on a 4096-byte boundary. The WAL writer MUST calculate padding before issuing the io_uring write.

---

## 9. Encryption Metadata Placement

### 9.1 Encrypted Batch Layout

When `ENCRYPTED` flag is set:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Batch Header (128 bytes) — contains dek_id, dek_version                    │
├─────────────────────────────────────────────────────────────────────────────┤
│ Record Entries (N × 32 bytes) — UNENCRYPTED (metadata only)                │
├─────────────────────────────────────────────────────────────────────────────┤
│ Encrypted Payload Block (variable length)                                   │
│   • AES-256-GCM ciphertext                                                 │
│   • 12-byte nonce prepended to ciphertext                                   │
│   • 16-byte authentication tag appended to ciphertext                       │
├─────────────────────────────────────────────────────────────────────────────┤
│ CRC32C Trailer (over ciphertext, not plaintext)                            │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 9.2 Authenticated Additional Data (AAD)

The AES-GCM AAD MUST include:

```text
tenant_id
stream_id (if single-stream batch)
batch_physical_seq_start
format_version
dek_id
```

**Normative rule:** Decryption MUST fail if AAD validation fails. This prevents ciphertext substitution attacks.

### 9.3 Record Entries and Encryption

Record entries are NOT encrypted. They contain only metadata (offsets, lengths, timestamps). Payload data is encrypted.

**Rationale:** Enables recovery and indexing without decryption. Payload confidentiality is preserved because entries contain no payload content.

---

## 10. Segment Lifecycle

### 10.1 Preallocation

```text
1. Preallocate 64 MB file via fallocate(FALLOC_FL_KEEP_SIZE)
2. Write SegmentHeader at offset 0
3. Set write pointer to offset 4096
4. Register file descriptor with io_uring
```

**Normative rule:** Segments MUST be preallocated before accepting writes. Allocation MUST NOT occur on the hot write path.

### 10.2 Sealing Triggers

A segment is sealed when:

| Trigger | Condition |
|---|---|
| Size | Accumulated batch data ≥ 64 MB. |
| Time | 500 ms elapsed since first write. |
| Quiesce | Explicit stream quiesce command. |
| Volume rotation | Storage volume rebalancing. |

### 10.3 Seal Procedure

```text
1. Stop accepting new batches for this segment
2. Write SegmentFooter with final physical_seq_end
3. Compute and write segment CRC
4. Flush to NVMe via io_uring with IOSYNC
5. Emit onSegmentSealed callback
6. Hand segment to compaction pipeline
```

**Normative rule:** A sealed segment MUST NOT accept new writes. Any write attempt MUST return an error.

---

## 11. Producer Identity and Idempotence

### 11.1 Producer Identity Fields

```text
producer_id      : u64  — Unique producer identifier
producer_epoch   : u32  — Session epoch (incremented on reconnect)
producer_seq     : u64  — Monotonic sequence within session
```

### 11.2 Idempotence Deduplication

The state plane maintains a sliding deduplication window per `producer_id`:

```text
DedupKey = (producer_id, producer_epoch, producer_seq)
```

**Normative rules:**

- `producer_seq` MUST be 64-bit (u64), not 32-bit.
- Duplicate produces within the dedup window MUST return the original offset without re-appending.
- The dedup window MUST survive coordinator failover via WAL replay.

---

## 12. Transaction Control Records

### 12.1 Transaction Lifecycle

```text
BEGIN_TXN   → No WAL record (state plane tracks)
APPEND      → Records carry transaction_id in batch header
COMMIT_TXN  → Batch with TXN_COMMIT flag and transaction_id
ABORT_TXN   → Batch with TXN_ABORT flag and transaction_id
```

### 12.2 Transaction Commit Batch

A commit batch contains:

```rust
// Batch header with TXN_COMMIT flag
// Zero or more record entries (may be empty for pure commit marker)
// Transaction metadata in payload (if any)
```

**Normative rules:**

- A transaction MUST have exactly one commit or abort batch.
- Records with `transaction_id != 0` MUST NOT be visible to `READ_COMMITTED` consumers until the commit batch is durable.
- Abort batches MUST mark all preceding transaction records as invisible.

---

## 13. Tombstone and Special Record Types

### 13.1 Tombstone Records

A tombstone is a deletion marker with empty payload.

```text
record_flags.TOMBSTONE = 1
payload_len = 0
```

**Use case:** Stream deletion, retention enforcement, crypto-shredding markers.

### 13.2 Causal Tag Records

When `CAUSAL_TAG` flag is set, the payload begins with a 32-byte HLC tag:

```rust
#[repr(C, packed)]
pub struct HlcTag {
    pub physical_time_ns: u64,
    pub logical_counter: u32,
    pub region_id: u32,
    pub padding: [u8; 16],
}
```

### 13.3 Entity Key Records

When `ENTITY_KEY_PRESENT` flag is set, the payload begins with:

```rust
#[repr(C, packed)]
pub struct EntityKeyPrefix {
    pub key_len: u16,
    pub key_bytes: [u8; key_len],  // Variable length
}
```

---

## 14. Format Versioning and Forward Compatibility

### 14.1 Version Field

`format_version` in segment header and batch header.

| Version | Status | Changes |
|---|---|---|
| 1 | Current | Initial release. |
| 2+ | Future | New fields appended; old fields preserved. |

### 14.2 Forward Compatibility Rules

- New fields MUST be appended to the end of fixed-size structures.
- Old readers MUST skip unknown fields using `reserved` space.
- Removing or reordering existing fields is PROHIBITED.
- Breaking changes MUST increment `format_version` and go through ADR.

**Normative rule:** Version N readers MUST be able to read version N-1 data. Version N-1 readers SHOULD gracefully reject version N data with a clear error.

---

## 15. Rust Type Contracts

### 15.1 Structure Alignment Summary

| Struct | Alignment | Size | Purpose |
|---|---|---|---|
| `SegmentHeader` | 4096 | 4096 | Segment metadata. |
| `SegmentFooter` | 4096 | 4096 | Seal metadata. |
| `BatchHeader` | 64 | 128 | Batch metadata. |
| `RecordEntry` | 1 (packed) | 32 | Record pointer. |
| `HlcTag` | 1 (packed) | 32 | Causal ordering tag. |
| `EntityKeyPrefix` | 1 (packed) | Variable | Entity key. |

### 15.2 Serialization Contract

- All integers are little-endian.
- All structs use `repr(C)` for C-compatible layout.
- Packed structs use `repr(C, packed)` to eliminate padding.
- Aligned structs use `repr(C, align(N))` for page alignment.

**Normative rule:** The WAL writer MUST validate struct sizes at compile time using `static_assert`:

```rust
const _: () = assert!(std::mem::size_of::<SegmentHeader>() == 4096);
const _: () = assert!(std::mem::size_of::<BatchHeader>() == 128);
const _: () = assert!(std::mem::size_of::<RecordEntry>() == 32);
```

---

## 16. Validation Rules

### 16.1 Write-Path Validation

Before writing a batch, the WAL writer MUST validate:

| Check | Action on Failure |
|---|---|
| `batch_record_count > 0` | Reject batch. |
| `batch_payload_len == sum(record.payload_len)` | Reject batch. |
| `producer_seq_end >= producer_seq_start` | Reject batch. |
| `dek_id != 0` if `ENCRYPTED` flag set | Reject batch. |
| `compression_type != 0` if `COMPRESSED` flag set | Reject batch. |
| `transaction_id != 0` if `TRANSACTIONAL` flag set | Reject batch. |
| All record entries fit within payload block | Reject batch. |

### 16.2 Read-Path Validation

On recovery replay, the reader MUST validate:

| Check | Action on Failure |
|---|---|
| Segment header CRC | Abort segment recovery. |
| Batch header CRC | Skip batch; log corruption. |
| Batch payload CRC | Skip batch; log corruption. |
| Record entry CRC | Skip record; log corruption. |
| `format_version` supported | Reject segment with clear error. |
| Encryption AAD | Reject batch; security incident. |

---

## 17. NFR Traceability

| NFR | Requirement | How This Specification Satisfies It |
|---|---|---|
| DUR-007 | Integrity detection | CRC32C at segment, batch, and record levels (§7). |
| PERF-004 | Framing overhead ≤8% | Batch-oriented framing amortizes headers (§5, §6). |
| PORT-002 | io_uring/O_DIRECT | 4096-byte alignment throughout (§8). |
| SEC-002 | Encryption at rest | AES-256-GCM with DEK metadata (§9). |
| AVAIL-002 | Node recovery <5s | Self-describing batches enable deterministic replay (§4, §5). |

---

## 18. Interfaces

### 18.1 Provided Interfaces

| Interface | Consumer | Semantics |
|---|---|---|
| `appendBatch(batch)` | WAL Writer | Write a batch frame to active segment. |
| `sealSegment(segment_id)` | Segment Manager | Seal and finalize a segment. |
| `readBatch(segment_id, batch_index)` | Recovery / Replication | Read a specific batch. |
| `readSegmentHeader(segment_id)` | Recovery | Read segment metadata. |
| `validateBatch(batch)` | Integrity Checker | Validate CRCs and structure. |

### 18.2 Consumed Interfaces

| Interface | Provider | Purpose |
|---|---|---|
| `allocateSegment()` | Filesystem | Preallocate 64 MB file. |
| `submitWrite(fd, buf, offset)` | io_uring | Async NVMe write. |
| `getDek(dek_id)` | Security Plane (KEI-ARC-025) | Retrieve encryption key. |
| `getSchema(schema_id)` | Schema Registry | Resolve schema for validation. |

---

## 19. Open Questions

| Item | Status | Resolution Path |
|---|---|---|
| Batch size target (records per batch) | Open | Benchmark under P1 to find optimal batch size. |
| Multi-stream batch overhead vs. single-stream | Open | Measure framing overhead trade-off. |
| Compression threshold (minimum payload size) | Open | Benchmark CPU vs. space trade-off. |
| Recovery parallelism (batch-level vs. segment-level) | Open | Benchmark recovery time under P1. |
| Format version migration tooling | Open | Design before Phase-2 exit. |

---

## 20. Glossary

| Term | Definition |
|---|---|
| Batch Frame | The unit of CRC integrity, quorum replication, and recovery replay. |
| Record Entry | A 32-byte pointer into the payload block. |
| Segment | A preallocated 64 MB WAL file. |
| Seal | The process of finalizing a segment and preventing further writes. |
| AAD | Authenticated Additional Data for AES-GCM. |
| O_DIRECT | Linux flag bypassing page cache for direct I/O. |
| io_uring | Linux async I/O interface. |

---

## 21. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial WAL binary format specification. Defines segment layout, batch framing, record entries, CRC32C integrity, page alignment, encryption metadata, producer identity, transaction records, and format versioning. Implements ADR-013 batch-oriented framing. |