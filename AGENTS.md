### LOCAL PROJECT GOVERNANCE: KEIROX DISTRIBUTED RUNTIME

### [PROJECT_INVARIANTS]

### 1. DOCUMENTATION_CANONICAL_SOURCE_OF_TRUTH & FAST ROUTING

* **Sole_Authority**: The formal architecture suite in `docs/architecture/` (`KEI-INDEX` through `KEI-VAL-052`) is the sole absolute authority for all system contracts, invariants, binary layouts, protocols, and algorithms.
* **Fast_Routing_Protocol**: Always consult [`docs/architecture/INDEX.md`](docs/architecture/INDEX.md) §2 to identify the exact L2/L3 document for the active domain. Ingest ONLY the specific governing document (never ingest the full documentation suite) to preserve context economy.
* **Immutable_Docs_Lock**: Architecture documents in `docs/architecture/` MUST NOT be modified, refactored, or edited by agents without explicit, prior user authorization.
* **Zero_Divergence_Policy**: Implementation code, tests, schemas, algorithms, and configurations MUST NEVER diverge from `docs/architecture/`. Lower-level code/crates MUST NOT contradict higher-level architectural invariants (`L0` Vision / `L1` Conceptual).
* **Contradiction_Escalation_Protocol**: If any ambiguity, gap, or contradiction is detected between specifications or between code and docs, the agent MUST immediately pause, cite the exact document IDs/sections to the developer, and wait for human resolution before proceeding. Never resolve contradictions unilaterally.
* **Architectural_Change_Control**: Any evolution or required modification to the specification MUST be submitted to the developer, audited against the Golden Invariant, approved, and formalized via an Architecture Decision Record (ADR in [`docs/architecture/KEI-ARC-012.md`](docs/architecture/KEI-ARC-012.md)) prior to touching implementation code.

### 2. PRE_FLIGHT_IMPLEMENTATION_GATE (DEFINITION OF DONE)

Before generating or modifying any code, verify:
* **Traceability**: Task traces to an explicit Requirement ID in [`docs/architecture/KEI-VAL-051.md`](docs/architecture/KEI-VAL-051.md) (RTM) and ADR in [`docs/architecture/KEI-ARC-012.md`](docs/architecture/KEI-ARC-012.md).
* **Memory_Hygiene**: Verify zero dynamic heap allocations (`malloc`, `Box`, `Vec::new()`, dynamic closures) in hot write ingress and WAL append loops.
* **Type_Safety**: Verify 100% strict explicit type declarations in Rust. Zero untyped pointers (`void*`), loose casts, or undocumented `unsafe` blocks.
* **Hardware_Target**: Systems-level zero-cost abstractions targeting Linux kernel `io_uring` + `O_DIRECT`, local NVMe storage, and Arrow SIMD (AVX-512 / ARM Neon).
* **Validation_Mapping**: Unit, benchmark, or chaos tests match [`docs/architecture/KEI-OPS-041.md`](docs/architecture/KEI-OPS-041.md).

### 3. KEIROX_LAYER_ISOLATION

* **Domain_Layer**: Pure distributed models, causal DAG ordering, Roaring Bitmap state machines, and sliding watermark invariants. Zero OS, zero network, zero disk/`io_uring`/S3 dependencies. 100% deterministic and unit-testable in isolation (`crates/keirox-core`, `crates/keirox-state`).
* **Application_Layer**: Coordinate state transitions. Dispatch commands between Domain and Infrastructure via interfaces (`StorageEngine`, `ConsensusCoordinator`, `CatalogSync`). Manage leases via Hierarchical Timing Wheels (`crates/keirox-timer`, `crates/keirox-arena`).
* **Infrastructure_Layer**: Implement platform adapters (`io_uring` ring-buffer WAL, Direct NVMe block storage, Apache Arrow/Parquet transcoding, S3 chunk streaming, Raft quorum consensus, Kafka binary framing) (`crates/keirox-wal`, `crates/keirox-index`, `crates/keirox-arrow-elt`).
* **Presentation_Layer**: Expose client-facing endpoints (Kafka Wire Protocol gateway, SQS/AMQP translator, Native Arrow Flight gRPC server, embedded DataFusion SQL interface) (`crates/keirox-api`, `crates/keirox-server`).

### 4. HOT_PATH_MEMORY_&_IO_HYGIENE (<2ms SLA)

* **Zero_Allocations**: Hot write ingress and WAL flush paths must execute over pre-allocated lock-free row arenas (`crates/keirox-arena`) and static ring-buffer registers.
* **Static_Registers**: Pre-allocate packed data structures (`StreamRegistryEntry` 32-byte packed structs, 64-byte aligned `SSTableChunkHeader`).
* **Direct_IO**: Enforce `O_DIRECT` and kernel-bypass `io_uring` for all physical WAL appends. Never route hot ingress through OS page cache.
* **Thread_Pinning**: Dedicated CPU core pinning for ingress network I/O and WAL flush loops. Background compaction and Arrow transcoding isolated to separate worker pools.
* **Alignment**: Enforce 64-byte cache-line and SIMD register alignment on all columnar Arrow buffers for AVX-512 / ARM Neon vectorization.

### 5. STATE_AND_STORAGE_CONSISTENCY

* **Log_Immutability**: Physical storage log is strictly append-only. Zero in-place data mutations or deletions allowed on the storage plane.
* **Bitmap_State**: Consumer group state transitions (`READY`, `LEASED(τ)`, `ACKED`, `EVICTED_DLQ`) must execute via Roaring Bitmaps.
* **Watermark_Advance**: Monotonic sliding base watermark ($W_{base}$) purging state bits for offsets $< W_{base}$. Evict poison pills exceeding max retries to virtual DLQ index to prevent memory leaks.
* **Consensus_Quorum**: Tier-0 NVMe writes require synchronous 3-node local Raft quorum acknowledgment before returning to clients. Multi-region WAN replication uses Hybrid Logical Clocks (HLC) and vector lineage tags.
* **Crypto_Shredding**: GDPR/CCPA erasure destroys DEKs via KMS and registers them in the Destroyed-Key Registry. Never physically rewrite historical immutable logs during erasure.

### 6. TOKEN_&_EXECUTION_HYGIENE

* **Output**: Dense, targeted fragments, clean code blocks, or minimalist diffs only. Zero conversational preamble, pleasantries, apologies, or trailing restatements.
* **Diffs**: Bounded searches. Minimal file rewrites using targeted edits.
* **Context_Economy**: Use [`docs/architecture/INDEX.md`](docs/architecture/INDEX.md) to ingest only the specific L2/L3 module directly related to immediate task execution to prevent token bloat.