# KEI-VER-001 — Implementation Verification Protocol
## Forensic Code-Level Audit for Architectural Compliance

---

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-VER-001 |
| Title | Implementation Verification Protocol |
| Version | 1.0 |
| Level | Verification & Audit |
| Status | Mandatory for all implementation reviews |
| Purpose | Ensure the coding agent implemented the exact flows, logic, and infrastructure specified in the architecture suite |
| Governing Documents | All 68 documents in INDEX.md v2.0 |
| Usage | Run this checklist against the codebase at every milestone gate |

---

## 2. How to Use This Protocol

Each verification item below is a **specific, testable, code-level check**. For each item:

1. **Locate** the relevant code in the codebase.
2. **Verify** the implementation matches the specification exactly.
3. **Test** the behavior under the described scenario.
4. **Record** PASS / FAIL / PARTIAL with evidence.
5. **Block** milestone progression on any FAIL in a Critical item.

Severity levels:

| Severity | Meaning |
|---|---|
| **CRITICAL** | Failure means data loss, corruption, security breach, or invariant violation. Blocks release. |
| **HIGH** | Failure means incorrect behavior, performance regression, or operational risk. Blocks milestone. |
| **MEDIUM** | Failure means degraded experience or missing feature. Tracked for fix. |
| **LOW** | Cosmetic or documentation issue. Tracked for backlog. |

---

## 3. WAL (Write-Ahead Log) Verification

### 3.1 Binary Format Compliance

| ID | Check | Severity | What to Verify in Code |
|---|---|---|---|
| WAL-V-001 | Batch header is exactly 128 bytes | CRITICAL | `#[repr(C)]` struct size assertion: `assert_eq!(std::mem::size_of::<WalBatchHeader>(), 128)` |
| WAL-V-002 | Record entry is exactly 32 bytes | CRITICAL | `assert_eq!(std::mem::size_of::<RecordEntry>(), 32)` |
| WAL-V-003 | Magic bytes are `0x4B454952` ("KEIR") | CRITICAL | Verify magic constant in header struct and parser validation |
| WAL-V-004 | Format version byte is present and validated | HIGH | Parser rejects unknown versions; test with version+1 |
| WAL-V-005 | CRC32C covers entire batch payload | CRITICAL | Verify CRC computation includes header + all record entries + payload block |
| WAL-V-006 | CRC32C trailer is at end of batch | HIGH | Verify trailer position in serialization code |
| WAL-V-007 | Batch is padded to 4096-byte boundary | HIGH | Verify padding calculation: `padded_len = (batch_len + 4095) & !4095` |
| WAL-V-008 | Segment footer contains segment metadata | HIGH | Verify footer struct includes segment_id, batch_count, byte_count, CRC |

### 3.2 Append-Only Guarantee

| ID | Check | Severity | What to Verify in Code |
|---|---|---|---|
| WAL-V-009 | No code path performs in-place mutation of WAL bytes | CRITICAL | Grep for `seek`, `overwrite`, `truncate` on WAL files; verify only `append` and `truncate_from_end` exist |
| WAL-V-010 | Segment sealing is irreversible | HIGH | After `seal()`, verify no write path exists for that segment |
| WAL-V-011 | WAL file is opened with `O_APPEND` or equivalent | CRITICAL | Verify file open flags in WAL writer |
| WAL-V-012 | No WAL bytes are modified after fsync | CRITICAL | Verify no code path writes to a byte offset that has been fsynced |

### 3.3 Durability & I/O

| ID | Check | Severity | What to Verify in Code |
|---|---|---|---|
| WAL-V-013 | `fsync` or `fdatasync` is called before ACK | CRITICAL | Trace the code path from `append()` to ACK; verify fsync occurs before ACK is sent |
| WAL-V-014 | `O_DIRECT` is used when configured | HIGH | Verify file open flags include `O_DIRECT` when direct I/O is enabled |
| WAL-V-015 | io_uring submission queue is used for async writes | HIGH | Verify io_uring integration in write path (not just sync write fallback) |
| WAL-V-016 | Write buffer is page-aligned for O_DIRECT | HIGH | Verify buffer allocation uses aligned allocation (e.g., `posix_memalign`) |
| WAL-V-017 | Partial write handling exists | HIGH | Verify code handles `EINTR`, short writes, and retries |

### 3.4 Segment Lifecycle

| ID | Check | Severity | What to Verify in Code |
|---|---|---|---|
| WAL-V-018 | Segment preallocation occurs on creation | MEDIUM | Verify `fallocate` or equivalent is called |
| WAL-V-019 | Active segment rolls when size threshold is reached | HIGH | Verify roll logic checks size and creates new segment |
| WAL-V-020 | Sealed segments are immutable | CRITICAL | Verify no write handle exists after sealing |
| WAL-V-021 | Truncated segments are unrecoverable | HIGH | Verify truncation calls `ftruncate` and updates manifest |

### 3.5 WAL Tests to Run

| Test | Scenario | Expected |
|---|---|---|
| WAL-T-001 | Append 1000 batches, kill process, replay | All 1000 batches recovered |
| WAL-T-002 | Flip 1 bit in batch payload, replay | CRC mismatch detected; batch rejected |
| WAL-T-003 | Truncate batch mid-write, replay | Partial batch detected; rejected safely |
| WAL-T-004 | Append with O_DIRECT, verify page alignment | No alignment errors |
| WAL-T-005 | Fill disk to 100%, attempt append | Graceful error; no corruption |

---

## 4. State Plane Verification

### 4.1 Roaring Bitmap Correctness

| ID | Check | Severity | What to Verify in Code |
|---|---|---|---|
| STA-V-001 | Bitmap uses Roaring Bitmap library (not custom) | HIGH | Verify dependency on `roaring` crate or equivalent |
| STA-V-002 | Container type selection is automatic | HIGH | Verify Array/Bitset/Run container transitions occur based on cardinality |
| STA-V-003 | Bitmap serialization is deterministic | HIGH | Serialize same state twice; bytes must be identical |
| STA-V-004 | Bitmap deserialization validates integrity | HIGH | Corrupt serialized bytes; verify error detected |
| STA-V-005 | Bitmap memory is bounded | CRITICAL | Verify spill threshold is enforced; test with 1M offsets |

### 4.2 State Machine Transitions

| ID | Check | Severity | What to Verify in Code |
|---|---|---|---|
| STA-V-006 | State enum has exactly 4 values: READY, LEASED, ACKED, EVICTED_DLQ | CRITICAL | Verify enum definition |
| STA-V-007 | READY → LEASED transition requires lease grant | CRITICAL | Verify no other code path sets LEASED |
| STA-V-008 | LEASED → ACKED transition requires valid lease token | CRITICAL | Verify token validation in ACK path |
| STA-V-009 | LEASED → READY transition occurs on NACK or timeout | CRITICAL | Verify both paths exist |
| STA-V-010 | LEASED → EVICTED_DLQ transition occurs when retry_count ≥ R_max | CRITICAL | Verify retry count check in timeout/NACK path |
| STA-V-011 | ACKED is terminal — no transition out of ACKED exists | CRITICAL | Grep for any code that changes ACKED state; must be zero |
| STA-V-012 | EVICTED_DLQ is terminal — no transition out of EVICTED_DLQ exists | CRITICAL | Grep for any code that changes EVICTED_DLQ state; must be zero |
| STA-V-013 | No state transition skips intermediate states | CRITICAL | Verify no READY → ACKED or READY → EVICTED_DLQ direct paths |

### 4.3 Watermark Invariants

| ID | Check | Severity | What to Verify in Code |
|---|---|---|---|
| STA-V-014 | `W_base` is monotonically non-decreasing | CRITICAL | Verify watermark update function: `new_wbase = max(old_wbase, computed)` |
| STA-V-015 | All offsets below `W_base` are terminal | CRITICAL | Write assertion: `for o in 0..W_base { assert!(is_terminal(o)) }` |
| STA-V-016 | No LEASED offset exists below `W_base` | CRITICAL | Write assertion: `for o in 0..W_base { assert!(state(o) != LEASED) }` |
| STA-V-017 | Watermark advancement is triggered after terminal transitions | HIGH | Verify watermark recalculation is called after ACK, DLQ eviction |
| STA-V-018 | Watermark is persisted durably | HIGH | Verify watermark is included in state snapshot and journal |

### 4.4 Mandatory DLQ Eviction

| ID | Check | Severity | What to Verify in Code |
|---|---|---|---|
| STA-V-019 | Stuck offset (retry_count ≥ R_max) is evicted to DLQ | CRITICAL | Verify eviction logic in timeout handler |
| STA-V-020 | DLQ eviction is automatic — no manual intervention required | CRITICAL | Verify no human approval is needed in eviction path |
| STA-V-021 | DLQ eviction advances watermark | CRITICAL | Verify watermark recalculation after eviction |
| STA-V-022 | DLQ eviction is logged and audited | HIGH | Verify audit event emission in eviction path |
| STA-V-023 | DLQ eviction is visible in DLQ list API | HIGH | Verify evicted offset appears in DLQ query results |

### 4.5 Lease Management

| ID | Check | Severity | What to Verify in Code |
|---|---|---|---|
| STA-V-024 | Lease token is globally unique | CRITICAL | Verify token generation uses UUID or atomic counter |
| STA-V-025 | Lease expiry uses monotonic clock | CRITICAL | Verify `Instant::now()` or equivalent; NOT `SystemTime` |
| STA-V-026 | Timing wheel uses O(1) insertion and expiry | HIGH | Verify timing wheel data structure; not a sorted list |
| STA-V-027 | Lease renewal extends expiry without changing token | HIGH | Verify renewal path preserves token |
| STA-V-028 | Expired lease returns offset to READY | CRITICAL | Verify timeout handler transitions state |
| STA-V-029 | Stale lease token is rejected | CRITICAL | Verify ACK/NACK with wrong token returns error |
| STA-V-030 | Duplicate ACK is idempotent | HIGH | Verify second ACK for same offset returns success without side effects |

### 4.6 State Plane Tests to Run

| Test | Scenario | Expected |
|---|---|---|
| STA-T-001 | Grant lease, ACK with correct token | Offset transitions to ACKED |
| STA-T-002 | Grant lease, ACK with wrong token | Error returned; state unchanged |
| STA-T-003 | Grant lease, wait for timeout | Offset returns to READY; retry_count incremented |
| STA-T-004 | NACK 3 times (R_max=3) | Offset transitions to EVICTED_DLQ |
| STA-T-005 | ACK offset below W_base | Idempotent success |
| STA-T-006 | Attempt to transition ACKED → READY | No code path exists; compile-time or runtime error |
| STA-T-007 | 1M offsets with 100K active leases | Bitmap memory bounded; no OOM |
| STA-T-008 | Kill process during lease grant, restart | Lease state recovered from journal |

---

## 5. Consensus / Raft Verification

### 5.1 Raft Safety Properties

| ID | Check | Severity | What to Verify in Code |
|---|---|---|---|
| RAF-V-001 | At most one leader per term | CRITICAL | Verify leader election logic; test with 3 nodes |
| RAF-V-002 | Log matching property holds | CRITICAL | If two logs have same index+term, all preceding entries match |
| RAF-V-003 | Committed entries are never lost | CRITICAL | Kill leader after commit; verify entry survives |
| RAF-V-004 | Leader completeness: leader has all committed entries | CRITICAL | Verify election restriction (candidate log must be up-to-date) |
| RAF-V-005 | Term is monotonically increasing | CRITICAL | Verify no code path decrements term |

### 5.2 ACK Gating

| ID | Check | Severity | What to Verify in Code |
|---|---|---|---|
| RAF-V-006 | Producer ACK is sent ONLY after quorum commit | CRITICAL | Trace code path: append → replicate → majority ACK → send producer ACK. Verify no early ACK. |
| RAF-V-007 | ACK_FAST mode: ACK after quorum commit, before full replication | HIGH | Verify ACK_FAST still waits for quorum, not all replicas |
| RAF-V-008 | ACK_DURABLE mode: ACK after full replication | HIGH | Verify ACK_DURABLE waits for all replicas |
| RAF-V-009 | No ACK is sent if quorum is not reached | CRITICAL | Kill 2 of 3 nodes; verify ACK is NOT sent; write is pending |

### 5.3 Epoch Fencing

| ID | Check | Severity | What to Verify in Code |
|---|---|---|---|
| RAF-V-010 | Coordinator epoch is monotonically increasing | CRITICAL | Verify epoch increment on failover |
| RAF-V-011 | Operations with stale epoch are rejected | CRITICAL | Send operation with epoch-1; verify rejection |
| RAF-V-012 | Epoch is included in all lease/ACK operations | HIGH | Verify epoch field in operation structs |
| RAF-V-013 | Epoch is persisted in Metadata Raft | HIGH | Verify epoch survives leader restart |

### 5.4 Consensus Tests to Run

| Test | Scenario | Expected |
|---|---|---|
| RAF-T-001 | 3-node cluster, kill leader | New leader elected; committed entries preserved |
| RAF-T-002 | 3-node cluster, kill 2 nodes | Writes pause; no corruption; recovery on restart |
| RAF-T-003 | Network partition (1 vs 2) | Majority continues; minority fenced |
| RAF-T-004 | Kill leader during append | Committed appends survive; uncommitted rejected |
| RAF-T-005 | Split-brain heal | Old leader writes rejected; state converges |

---

## 6. Consumption Semantics Verification

### 6.1 Delivery Guarantees

| ID | Check | Severity | What to Verify in Code |
|---|---|---|---|
| SEM-V-001 | Default delivery is at-least-once | CRITICAL | Verify no automatic ACK; consumer must explicitly ACK |
| SEM-V-002 | UnACKed messages are redelivered after lease timeout | CRITICAL | Verify timeout → READY → re-lease path |
| SEM-V-003 | Idempotent produce deduplicates within window | HIGH | Verify producer_id + sequence deduplication logic |
| SEM-V-004 | Duplicate produce returns original offset | HIGH | Verify idempotent response includes original offset |
| SEM-V-005 | NACK increments retry_count | HIGH | Verify retry counter in NACK path |
| SEM-V-006 | NACK with requeue=true returns offset to READY | HIGH | Verify state transition |
| SEM-V-007 | NACK with requeue=false evicts to DLQ | HIGH | Verify DLQ eviction path |

### 6.2 Ordering Guarantees

| ID | Check | Severity | What to Verify in Code |
|---|---|---|---|
| SEM-V-008 | Offsets are monotonically increasing per stream | CRITICAL | Verify offset assignment logic |
| SEM-V-009 | Independent entity_keys can be processed concurrently | HIGH | Verify no global lock on entity_key |
| SEM-V-010 | Same entity_key maintains ordering | CRITICAL | Verify entity_key → stream mapping preserves order |

### 6.3 Virtual DLQ

| ID | Check | Severity | What to Verify in Code |
|---|---|---|---|
| SEM-V-011 | DLQ is index-based (zero-copy) | HIGH | Verify DLQ references offsets in original WAL; no payload duplication |
| SEM-V-012 | DLQ list returns evicted entries | HIGH | Verify DLQ query returns correct entries |
| SEM-V-013 | DLQ redrive requeues entry | HIGH | Verify redrive transitions EVICTED_DLQ → READY |
| SEM-V-014 | DLQ redrive resets retry_count | HIGH | Verify retry counter reset on redrive |
| SEM-V-015 | DLQ purge removes entry permanently | HIGH | Verify purge requires elevated authorization |

---

## 7. Columnar ELT / Lakehouse Verification

### 7.1 Arrow / Parquet Export

| ID | Check | Severity | What to Verify in Code |
|---|---|---|---|
| ELT-V-001 | Arrow RecordBatch is generated from sealed WAL segments | HIGH | Verify export reads sealed segments, not active WAL |
| ELT-V-002 | Parquet file size target is 64–128 MB | HIGH | Verify file size configuration and rolling logic |
| ELT-V-003 | Parquet compression codec is configurable | MEDIUM | Verify codec selection (zstd, snappy, etc.) |
| ELT-V-004 | Export does not block hot append path | CRITICAL | Verify export runs on separate thread/core pool |
| ELT-V-005 | Export is idempotent | HIGH | Verify re-export of same segment produces same file |

### 7.2 Schema Shredding

| ID | Check | Severity | What to Verify in Code |
|---|---|---|---|
| ELT-V-006 | Maximum shredded fields is 64 | HIGH | Verify cap enforcement in shredding logic |
| ELT-V-007 | Fields beyond 64 route to `_unstructured_payload` | HIGH | Verify fallback routing |
| ELT-V-008 | Schema fingerprint is stable for same schema | HIGH | Verify fingerprint computation is deterministic |
| ELT-V-009 | Schema evolution preserves historical readability | CRITICAL | Verify old files readable after schema change |
| ELT-V-010 | Unsafe type changes require new schema version | HIGH | Verify type widening rules |

### 7.3 Iceberg Committer

| ID | Check | Severity | What to Verify in Code |
|---|---|---|---|
| ELT-V-011 | Commit ledger is durable | CRITICAL | Verify commit ledger is persisted before catalog commit |
| ELT-V-012 | Commit is idempotent | CRITICAL | Verify replay of same commit does not create duplicate snapshot |
| ELT-V-013 | Commit includes schema ID/version | HIGH | Verify commit metadata |
| ELT-V-014 | Manifest compaction is triggered when threshold exceeded | MEDIUM | Verify compaction logic |
| ELT-V-015 | Snapshot expiration respects legal hold | CRITICAL | Verify legal hold check before expiration |
| ELT-V-016 | Orphan cleanup has grace period | HIGH | Verify grace period configuration |
| ELT-V-017 | Orphan cleanup does not delete active files | CRITICAL | Verify cross-check against active manifests |

### 7.4 Lakehouse Tests to Run

| Test | Scenario | Expected |
|---|---|---|
| ELT-T-001 | Ingest 100K events, export to Parquet, query with DuckDB | All 100K events queryable |
| ELT-T-002 | Kill committer during commit, restart | No duplicate snapshot; ledger recovers |
| ELT-T-003 | Schema evolution: add nullable column | Old files return NULL; new files have column |
| ELT-T-004 | 100 fields in payload | 64 shredded; 36 in _unstructured_payload |
| ELT-T-005 | Iceberg catalog unavailable | Commits queue locally; no data loss |

---

## 8. Security Verification

### 8.1 Encryption

| ID | Check | Severity | What to Verify in Code |
|---|---|---|---|
| SEC-V-001 | WAL batches are encrypted with AES-256-GCM | CRITICAL | Verify encryption in WAL write path |
| SEC-V-002 | AAD includes tenant_id, stream_id, batch_seq | CRITICAL | Verify AAD construction |
| SEC-V-003 | Nonce is unique per (DEK, batch) pair | CRITICAL | Verify nonce generation (random, not counter) |
| SEC-V-004 | Decryption validates AAD | CRITICAL | Verify decryption fails on AAD mismatch |
| SEC-V-005 | Parquet files are encrypted | HIGH | Verify encryption in Parquet export path |
| SEC-V-006 | State snapshots are encrypted | HIGH | Verify encryption in snapshot path |

### 8.2 Key Management

| ID | Check | Severity | What to Verify in Code |
|---|---|---|---|
| SEC-V-007 | DEK plaintext is never written to disk | CRITICAL | Grep for DEK serialization; verify only wrapped DEK is persisted |
| SEC-V-008 | DEK cache entries are zeroized on eviction | CRITICAL | Verify `zeroize` crate or manual zeroing |
| SEC-V-009 | DEK cache has bounded size and TTL | HIGH | Verify LRU cache with TTL |
| SEC-V-010 | KMS failure blocks new writes (fail-secure) | CRITICAL | Simulate KMS outage; verify writes rejected |
| SEC-V-011 | No plaintext fallback exists | CRITICAL | Grep for any code path that writes unencrypted data |

### 8.3 Crypto-Shredding

| ID | Check | Severity | What to Verify in Code |
|---|---|---|---|
| SEC-V-012 | Erasure destroys key via KMS | CRITICAL | Verify KMS destroy call in erasure workflow |
| SEC-V-013 | Destroyed key is recorded in registry | CRITICAL | Verify registry entry creation |
| SEC-V-014 | Read after erasure fails securely | CRITICAL | Verify read path checks destroyed-key registry |
| SEC-V-015 | Backup restore checks destroyed-key registry | CRITICAL | Verify restore path checks registry before exposing data |
| SEC-V-016 | Erasure propagates to all regions | HIGH | Verify cross-region registry replication |
| SEC-V-017 | Legal hold blocks erasure | CRITICAL | Verify legal hold check before erasure |

### 8.4 Authorization

| ID | Check | Severity | What to Verify in Code |
|---|---|---|---|
| SEC-V-018 | Default policy is deny | CRITICAL | Verify ABAC engine returns deny when no policy matches |
| SEC-V-019 | Cross-tenant access is denied | CRITICAL | Test: Tenant A reads Tenant B stream → 403 |
| SEC-V-020 | All operations produce audit events | HIGH | Verify audit emission in all PEP paths |
| SEC-V-021 | Audit events are tamper-evident | HIGH | Verify audit log uses append-only storage or hash chain |
| SEC-V-022 | Secrets never appear in logs | CRITICAL | Grep for token/key logging; verify redaction |

---

## 9. Multi-Region / DR Verification

| ID | Check | Severity | What to Verify in Code |
|---|---|---|---|
| MR-V-001 | Mode A: only primary region accepts writes | CRITICAL | Verify replica region rejects direct writes |
| MR-V-002 | Region epoch is incremented on failover | CRITICAL | Verify epoch increment in failover workflow |
| MR-V-003 | Writes with stale region epoch are rejected | CRITICAL | Verify epoch validation in write path |
| MR-V-004 | WAL tails replicate asynchronously to replica | HIGH | Verify replication pipeline |
| MR-V-005 | Metadata Raft replicates to replica region | CRITICAL | Verify metadata replication (stream registry, schemas, offsets) |
| MR-V-006 | PITR restores state to exact timestamp | HIGH | Verify PITR replay logic |
| MR-V-007 | PITR does not expose post-target data | CRITICAL | Verify filtering of records after target timestamp |
| MR-V-008 | Legal hold blocks snapshot expiration | CRITICAL | Verify legal hold check |
| MR-V-009 | Data residency blocks unauthorized replication | CRITICAL | Verify residency policy enforcement |

---

## 10. Gateway Verification

### 10.1 Kafka Gateway

| ID | Check | Severity | What to Verify in Code |
|---|---|---|---|
| GW-V-001 | Certified Kafka APIs are implemented | HIGH | Verify Produce, Fetch, Metadata, ListOffsets, OffsetCommit, OffsetFetch |
| GW-V-002 | Unsupported Kafka APIs return explicit error | CRITICAL | Verify transactional APIs return error, not silent drop |
| GW-V-003 | Kafka partition maps to virtual partition | HIGH | Verify partition → entity_key mapping |
| GW-V-004 | Kafka idempotent produce maps to Keirox idempotence | HIGH | Verify producer_id + sequence mapping |
| GW-V-005 | Kafka consumer group maps to Keirox consumer group | HIGH | Verify group coordination mapping |

### 10.2 SQS Gateway

| ID | Check | Severity | What to Verify in Code |
|---|---|---|---|
| GW-V-006 | SendMessage maps to Keirox append | HIGH | Verify mapping |
| GW-V-007 | ReceiveMessage maps to LeaseNext | HIGH | Verify lease → receipt handle mapping |
| GW-V-008 | DeleteMessage maps to ACK with receipt handle validation | HIGH | Verify receipt handle → lease token validation |
| GW-V-009 | ChangeMessageVisibility maps to RenewLease | HIGH | Verify visibility timeout → lease TTL mapping |
| GW-V-010 | Stale receipt handle is rejected | CRITICAL | Verify expired lease token rejection |
| GW-V-011 | FIFO MessageGroupId maps to entity_key | HIGH | Verify ordering preservation |
| GW-V-012 | DelaySeconds returns explicit unsupported error | HIGH | Verify error response |

### 10.3 AMQP Gateway

| ID | Check | Severity | What to Verify in Code |
|---|---|---|---|
| GW-V-013 | Basic.publish maps to Keirox append | HIGH | Verify mapping |
| GW-V-014 | Basic.consume maps to LeaseNext | HIGH | Verify delivery-tag → lease token mapping |
| GW-V-015 | Basic.ack maps to ACK | HIGH | Verify delivery-tag validation |
| GW-V-016 | Basic.nack maps to NACK | HIGH | Verify requeue flag mapping |
| GW-V-017 | Topic/Fanout exchanges return NOT_IMPLEMENTED | HIGH | Verify error response |
| GW-V-018 | AMQP transactions return NOT_IMPLEMENTED | HIGH | Verify error response |

---

## 11. REST API / CLI / Console Verification

| ID | Check | Severity | What to Verify in Code |
|---|---|---|---|
| API-V-001 | `/healthz` returns 200 on healthy node | HIGH | Test with curl |
| API-V-002 | `/readyz` returns 503 on unhealthy node | HIGH | Simulate Raft quorum loss; test |
| API-V-003 | Admin endpoints require authentication | CRITICAL | Test without auth token → 401 |
| API-V-004 | Admin endpoints enforce ABAC | CRITICAL | Test with insufficient permissions → 403 |
| API-V-005 | Rate limiting returns 429 with Retry-After | HIGH | Exceed rate limit; verify response |
| API-V-006 | Error responses follow standard schema | HIGH | Verify error code, message, request_id, doc_url |
| API-V-007 | Pagination works with cursor tokens | HIGH | Test multi-page list operations |
| API-V-008 | OpenAPI spec is generated and valid | MEDIUM | Run spectral linter on generated spec |
| API-V-009 | CLI commands match API endpoints | HIGH | Test CLI → API mapping |
| API-V-010 | Web Console defaults to read-only mode | HIGH | Verify write operations require Admin role |

---

## 12. Performance Verification

| ID | Check | Severity | Target |
|---|---|---|---|
| PERF-V-001 | Tier-0 write latency p99 | CRITICAL | ≤2 ms (local, no quorum); ≤3 ms (with quorum) |
| PERF-V-002 | Sustained throughput | HIGH | ≥100 MB/s (single node); ≥100 MB/s (3-node cluster) |
| PERF-V-003 | Stream read latency p99 | HIGH | ≤2 ms for active data |
| PERF-V-004 | Lease acquisition latency p99 | HIGH | ≤1 ms |
| PERF-V-005 | ACK latency p99 | HIGH | ≤1 ms |
| PERF-V-006 | Write Amplification Factor | HIGH | ≤1.35 |
| PERF-V-007 | Bitmap memory per 100K offsets | HIGH | Measured and bounded |
| PERF-V-008 | Compaction interference on write path | HIGH | ≤5% p99 jitter |
| PERF-V-009 | Gateway translation overhead p99 | HIGH | ≤1 ms |
| PERF-V-010 | Iceberg default freshness | HIGH | ≤60 seconds |
| PERF-V-011 | Iceberg fast-mode freshness | MEDIUM | ≤5 seconds (tuned) |

---

## 13. Operational Verification

| ID | Check | Severity | What to Verify |
|---|---|---|---|
| OPS-V-001 | Backpressure ladder engages at correct thresholds | CRITICAL | Verify 80% → clamp, 90% → throttle, 95% → shed, 98% → reject |
| OPS-V-002 | Graceful shutdown flushes state | HIGH | Send SIGTERM; verify flush completes |
| OPS-V-003 | Rolling upgrade preserves quorum | CRITICAL | Upgrade 1 node at a time; verify no quorum loss |
| OPS-V-004 | Node replacement recovers state | HIGH | Kill node; replace; verify state reconstruction |
| OPS-V-005 | Coordinator failover < 3.5 seconds | HIGH | Measure failover time |
| OPS-V-006 | Metrics endpoint exposes all required metrics | HIGH | Verify all metrics from KEI-ARC-027 are present |
| OPS-V-007 | Alert rules link to runbooks | MEDIUM | Verify runbook_url in alert annotations |
| OPS-V-008 | PDB prevents quorum loss during node drain | CRITICAL | Drain node; verify quorum maintained |

---

## 14. Supply Chain Verification

| ID | Check | Severity | What to Verify |
|---|---|---|---|
| REL-V-001 | Builds are reproducible | HIGH | Build twice from same source; compare SHA-256 |
| REL-V-002 | SBOM is generated for every artifact | HIGH | Verify CycloneDX JSON exists |
| REL-V-003 | All binaries are signed with Cosign | CRITICAL | Verify signature with `cosign verify-blob` |
| REL-V-004 | All container images are signed with Cosign | CRITICAL | Verify signature with `cosign verify` |
| REL-V-005 | SLSA provenance attestation is generated | HIGH | Verify in-toto attestation |
| REL-V-006 | Container images use Distroless base | HIGH | Verify no shell, no package manager |
| REL-V-007 | Container images run as non-root | HIGH | Verify UID 65532 |
| REL-V-008 | Dependency scan passes with no critical vulns | CRITICAL | Run Trivy/Grype; verify zero critical |

---

## 15. Gap Closure Verification (15 Patches)

| GAP ID | Check | Severity | What to Verify |
|---|---|---|---|
| GAP-001 | Per-tenant CPU/IO quotas are enforced | HIGH | Verify cgroup or io_uring priority per tenant |
| GAP-002 | Cluster bootstrap protocol exists | HIGH | Verify `keirox cluster init` seeds Raft group |
| GAP-003 | Pod anti-affinity across zones is enforced | HIGH | Verify K8s operator sets topology spread |
| GAP-004 | Client SDK has bounded memory buffer | HIGH | Verify SDK buffer limit and blocking behavior |
| GAP-005 | Binary format has version byte | CRITICAL | Verify version field in all binary structs |
| GAP-006 | Degradation matrix is implemented | HIGH | Verify fallback behaviors for S3, KMS, Iceberg catalog outages |
| GAP-007 | Monotonic clock used for all internal timers | CRITICAL | Grep for `SystemTime` in state plane; must be zero |
| GAP-008 | No key-value compaction exists | CRITICAL | Verify no key-level deduplication in WAL path |
| GAP-009 | Metadata Raft replicates to replica region | CRITICAL | Verify cross-region metadata replication |
| GAP-010 | Schema cache exists at gateway edge | HIGH | Verify cached schema resolution when registry is down |
| GAP-011 | DLQ payload inspection requires ABAC permission | HIGH | Verify `dlq:inspect` permission check |
| GAP-012 | SDK micro-batches sub-512-byte payloads | MEDIUM | Verify batching logic in SDK |
| GAP-013 | Licensing split is documented | MEDIUM | Verify LICENSE files for core vs enterprise |
| GAP-014 | Metering events are emitted | MEDIUM | Verify metering topic emission |
| GAP-015 | Unknown protobuf fields are ignored | HIGH | Verify gRPC unknown field handling |

---

## 16. Verification Execution Protocol

### 16.1 When to Run

| Trigger | Scope |
|---|---|
| Every PR merge | Run relevant section tests (e.g., WAL PR → Section 3 tests) |
| Every milestone gate | Run ALL sections |
| Every release candidate | Run ALL sections + performance benchmarks |
| Every security review | Run Section 8 (Security) |
| Every DR drill | Run Section 9 (Multi-Region) |

### 16.2 Recording Results

For each verification item, record:

```text
Item ID: WAL-V-001
Status: PASS / FAIL / PARTIAL
Evidence: [screenshot, test output, code reference]
Reviewer: [name]
Date: [date]
Notes: [any observations]
```

### 16.3 Blocking Rules

| Severity | Blocking Rule |
|---|---|
| CRITICAL | Blocks ALL releases until resolved |
| HIGH | Blocks milestone gate until resolved |
| MEDIUM | Tracked; must be resolved before next milestone |
| LOW | Tracked; resolved in backlog |

---

## 17. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial Implementation Verification Protocol. Defines 200+ code-level verification items across WAL, State Plane, Consensus, Consumption Semantics, Lakehouse, Security, Multi-Region, Gateways, REST API, Performance, Operations, Supply Chain, and Gap Closure. |