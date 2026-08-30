# KEIROX — END-TO-END PRODUCT AUDIT REPORT

| Field | Value |
|---|---|
| Document | AUDIT.md (project root) |
| Role | Main Product Auditor — robustness & correctness |
| Date | 2026-08-30 |
| Scope | Whole workspace: architecture docs, engineering docs, 18 crates, config, deploy, scripts, tests |
| Method | Read-only audit of code. Documentation alignment fixed where errors were *clear* (broken links, wrong counts/IDs); contradictions are escalated, not resolved (see §3). |
| Verification | `cargo check`, `cargo test`, `cargo clippy`, `cargo fmt --check` |

> **Scope status:** the project is in initial documentation-authoring / gap-closure phase. Clear documentation errors (broken links, wrong crate/ADR/doc counts, wrong record-entry size) were fixed. The 4-vs-5 phase / 36-vs-42-month contradiction was **re-escalated, not resolved** (docs are the sole source of truth; only clear errors are edited). Remaining items are **implementation/code** findings plus the open/deferred documentation items in §3.

---

## 1. RESOLUTION LOG (documentation — closed this session)

> **Re-audit note:** following a second pass, one prior edit was **reverted** — the roadmap phase-count change (see below). The 4-phase/36-month (architecture) vs 5-phase/42-month (engineering) divergence is a **pre-existing contradiction across two authoritative doc sets**, not a clear error, and is re-escalated rather than resolved. All other fixes below were re-verified as clear errors and retained.

| # | Finding | Resolution | Status |
|---|---|---|---|
| D-C2 | ~42 stale `KEI-*-001` references (broken links) | Swept `-001 → -101` across 13 engineering docs. | ✅ retained |
| D-H2 / E20 | Crate registry drift (12 vs 18) | `README.md` + `AGENTS.md` §3 list all 18 crates. | ✅ retained |
| D-H3 | Architecture doc count (22 vs 25) | `KEI-VAL-050` §2.1 corrected to **25 documents**. | ✅ retained |
| D-H4 | ADR range "001..083" | `architecture/INDEX.md` §3 corrected to **38 ADRs**. | ✅ retained |
| D-M1 / L2 | Record-entry "32 bytes" vs field list 46 | `KEI-DES-030` §6.1 + INDEX + README + VAL-050 → **46 bytes** (matches its own field list). | ✅ retained |
| E13 | SDK/gateway crate doc-reference mis-attribution | `keirox-sdk`/`keirox-gateway` `Cargo.toml` → `ARC-024` (was `ARC-023`/ELT). | ✅ retained |
| M1 | `cargo fmt` drift (33 diffs) | Resolved by developer (0 diffs). | ✅ retained |
| D-M7 | Engineering README registry stale | Verified already complete (Phases 1–5 listed). | ✅ retained |
| D-C1 | Roadmap 4 vs 5 phases / 36 vs 42 months | **REVERTED** — contradiction re-escalated (see §3). | ⚠️ open |
| D-M4 | Phase-3 naming drift | **REVERTED** — part of D-C1; ADR-081 restored to "Ecosystem Bridge". | ⚠️ open |
| — | ORG docs (staffing/cost/GTM) eliminated | Deleted `KEI-ORG-101/201/301/401/501`; removed all cross-references (README, INDEX, engineering README, BENCH/REL/SDK/VAL/RISK/ENG plans). | ✅ done |

---

## 2. REMAINING CODE / IMPLEMENTATION FINDINGS

### 2.1 CRITICAL

#### C1 — State plane truncates 64-bit offsets to 32-bit (correctness bug + spec divergence)
- Spec `KEI-DES-031 §5.1` mandates partitioned **`Roaring64Map`**.
- Code `keirox-state/src/state_machine.rs:126,189,206,234` (and `snapshot.rs:97,100`) uses `roaring::RoaringBitmap` (32-bit) with `offset as u32`.
- **Impact:** offsets ≥ 2³² truncate → bitmap collisions, wrong ACK/DLQ state, watermark corruption, silent state loss. Violates `REQ-STATE-002`.
- **Fix:** adopt `roaring::Roaring64Map` (or `BTreeMap<u32, Roaring32Bitmap>`); remove `as u32` casts; add ≥2³² regression test.

#### C2 — `RecordEntry` is 40 bytes, spec now mandates 46 bytes
- Spec `KEI-DES-030 §6.1` field list = 46 bytes (`producer_seq_delta` + `timestamp_delta_ms` included).
- Code `keirox-wal/src/framing.rs` implements a 40-byte struct that **omits** `producer_seq_delta` and `timestamp_delta_ms`, and **adds** `_reserved: u16`.
- **Impact:** on-disk WAL layout diverges from the governing L3 spec; CRC/golden-framing and cross-version compat at risk.
- **Fix:** add the two missing fields (or raise a spec change); update golden framing tests.

### 2.2 HIGH

#### H1 — `keirox-server` `start` is a no-op
- `server/src/main.rs:79-102`: parses `config`/`port` then only logs + records a metric. No socket bind, no WAL/state/consensus wiring, no config load.

#### H2 — Benchmark harness is synthetic
- `bench/src/runner.rs` times arbitrary closures; no real WAL/state/consensus measured. `KEI-CERT-100` perf claims (120 MB/s, 0.85 ms p99) unverifiable.

#### H3 — Raft does not persist `current_term` / `voted_for` / log
- `consensus/src/engine.rs` keeps all state in memory. Raft §5.2 requires durable term+vote before granting votes/committing. Restart can revert term → split-brain.

#### H4 — Unconditional `ack()` bypasses fencing (ADR-024)
- `state_machine.rs:186` `ack()` advances watermark without token validation; trait `acknowledge` routes through it. Stale-token / never-leased offsets can be ACKed.

### 2.3 MEDIUM

#### M2 — 64-field cap vs config 128
- `config/keirox.toml:44` `max_inferred_fields = 128`; spec + code default = **64** (`shredder.rs:11`).

#### M3 — Protobuf RPC not implemented
- `keirox-api` declares `tonic`/`prost` but has **zero `.proto` files**; `proto.rs` is hand-rolled DTOs.

#### M5 — deploy / scripts / tests are README-only stubs
- `deploy/{docker,kubernetes,terraform}/`, `scripts/`, `tests/{integration,golden,chaos,soak}/` contain only `README.md`. `scripts/README.md` references non-existent `check.sh`/`bench.sh`/`audit.sh`.

#### M7 — Doc-link validator gives false confidence
- `testkit/tests/doc_links_test.rs` only checks `*.md` link existence; empty stubs still "pass".

### 2.4 LOW

#### L1 — `get_state` conflates DLQ offsets below watermark
- `state_machine.rs:122-124` returns `Acked` for all `< base_watermark`, including `EvictedDlq` offsets.

#### L3 — Undocumented `unsafe` in WAL
- `segment.rs`/`writer.rs` use `from_raw_parts` without `// SAFETY`; no workspace `unsafe` policy. (Use `bytemuck`/`zerocopy`.)

#### L4 — Certification reports cite non-existent test files
- `KEI-CERT-100 §3.1` references `framing_test.rs`, `writer_test.rs`, etc. (tests are inline `mod tests`).

#### L5 — Unused dependencies
- `keirox-core`: `bytes`, `tracing`; `keirox-api`: `tonic`, `prost`, `bytes`.

#### L6 — Domain-layer purity
- `keirox-core` depends on `tracing` despite Domain "zero OS/network/disk" rule.

### 2.5 E-SERIES (exhaustive code pass)

**HIGH**
- **E2** — `ack_fenced` passes through when no active lease (`state_machine.rs:173-183`); `coordinator_node.rs:157-161` creates empty group state → `Ready` offsets ACKable.
- **E3** — `lease_offset`/`apply_delta` ignore `lease_with_token` failure (`coordinator_node.rs:121`, `lease_journal.rs:65`) → phantom leases.

**MEDIUM**
- **E4** — recovery logs "skip corrupt segment" but `return Err` (`recovery.rs:66-75`).
- **E5** — `InstallSnapshot` RPC defined but no `handle_install_snapshot`.
- **E6** — `last_applied` never advanced.
- **E7** — membership changes not consensus-replicated.
- **E8** — epoch token truncated to 16 bits; `offset` not bound into token.
- **E9** — failover does not reconstruct `timing_wheels` → orphaned lease expiries.
- **E10** — `expire_snapshots` is a no-op (counts but never removes).
- **E11** — timing wheel is `BTreeMap` O(log n), not O(1).
- **E12** — `append_replicated` ignores `_prev_index`; gap risk.

**LOW**
- **E14** — `check_live` hardcodes `memory_healthy: true`.
- **E15** — Parquet SNAPPY hardcoded (no config selection).
- **E16** — consistent-hash `shard_id` positional, not hash-derived.
- **E17** — `commit_modes` stored but never enforced.
- **E18** — `RwLock::unwrap()` poison panics; `match_indices[len - quorum_size]` underflow risk.
- **E19** — Kafka header silently drops malformed `client_id`; no flexible-version tags.

### 2.6 CROSS-CUTTING

- **E21** — unused deps (see L5); no unused-deps lint.
- **E22** — layer graph not enforced by any check.
- **E23** — no workspace `unsafe` allowlist.
- **E24** — `config/keirox.toml` never read by any crate; server ignores `--config`.
- **E25** — deploy/scripts/tests stubs (see M5).

---

## 3. REMAINING DOCUMENTATION ITEMS (deferred — not yet closed)

> These are *content-authoring* gaps and open contradictions. They remain open by design until a human resolves them.

- **D-C1 (OPEN — escalate)** — roadmap phase-count contradiction: architecture suite (`ADR-081`, `KEI-ARC-001`, `KEI-VAL-050/052`) states **4 phases / 36 months** ("Core Engine → Distributed Durability → Ecosystem Bridge → Enterprise Hardening"), while the engineering Phase-5 suite (`KEI-ENG-500`, `KEI-RISK-501`) states **5 phases / 42 months** ("… → Productization & Day-2 Operations"). These are two authoritative doc sets; the direction of reconciliation is a **human decision** (not yet resolved — a prior agent-side fix was reverted).
- **D-M4 (OPEN — part of D-C1)** — phase-3 naming drift: ADR-081 "Ecosystem Bridge" vs `KEI-ENG-300` "Ecosystem Compatibility Gateways & Lakehouse".
- **D-H1** — `KEI-CERT-200/300` assert "[GO] Certified / JML=0" without reproducible evidence; downgrade to Draft/Provisional until code blockers (H1–H3) close.
- **D-M2 / D-M3** — Phase 3/4 registers list verbose planned filenames and use a hybrid "register + embedded plan" structure.
- **D-M5** — loose ID ranges (`KEI-ARC-001..027`, `KEI-DES-030..037`) imply non-existent contiguous IDs.
- **D-M6** — "32 bytes/stream" (corrected overclaim) vs legitimate 32-byte `StreamRegistryEntry` terminology needs disambiguation.
- **D-L1** — one 0-byte Phase-5 stub (`KEI-OPS-502`) — content pending; other Phase-5 docs now populated.
- **D-L2 / D-L3** — milestone granularity undocumented; `reports/README.md` omits Phase 4/5.
- **G1–G16** — 16 operational-gap patches (resource isolation, bootstrap, monotonic clock, REST gateway, licensing, etc.) — accepted as gaps to close in governing docs, not yet authored.

---

## 4. VERIFIED-CORRECT (no issue)

- Binary framing: 128-byte `BatchHeader` (64-aligned), 4096-byte `SegmentHeader`/`Footer`, CRC32C hierarchy.
- `StreamRegistryEntry` (32B), `SparseIndexEntry` (16B), `StateShardKey` (64B) size invariants.
- `RoaringBitmap` snapshot round-trip (32-bit limitation = C1).
- Consistent-hash remap; epoch-fencing stale-token rejection; multipart backoff+jitter; hash-prefix URI.
- Raft 3-node election + median-of-3 quorum commit (in-memory case).
- `LeaseJournal` replay; `IcebergCatalogCommitter` optimistic-concurrency conflict detection.

---

## 5. REMEDIATION ORDER

1. **C1** offset-truncation (data-loss class) — fix before any scale claim.
2. **C2** record layout — reconcile code to 46-byte spec (or ADR the change).
3. **H4 / E2 / E3** — route ACK/lease through fenced paths.
4. **H3** — durable Raft term/vote persistence before Phase 2 exit.
5. **H1 / H2** — wire server/bench to real subsystems or downgrade cert claims.
6. **E4–E12** — recovery, snapshot, apply, membership, epoch, timing-wheel, snapshot-expiry, timing-wheel, log-conflict fixes.
7. **M2 / E24** — wire config (64-field cap, direct_io, compression).
8. **E14–E19, E21–E25** — hygiene (unused deps, unsafe policy, layer check, kafka header, deploy stubs).

---

## 6. GOVERNANCE NOTE

Documentation is the **sole source of truth**; only *clear errors* (broken links, wrong counts/IDs, internally-inconsistent sizes) were corrected under user authorization. The roadmap phase-count divergence (architecture 4/36 vs engineering 5/42) is a **contradiction across two authoritative doc sets** and was **reverted and re-escalated** rather than unilaterally resolved, per `AGENTS.md` Contradiction_Escalation_Protocol. No code was modified. Remaining code findings and open documentation items are listed in §2 and §3.
