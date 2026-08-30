# Contributing to Keirox

Thank you for your interest in contributing to **Keirox**! 

Keirox is an open-source, high-performance distributed runtime implementing the **Polymorphic Event Fabric (PEF)**. It unifies durable event streaming, low-latency task queuing, virtual dead-lettering, and internalized columnar lakehouse materialization (via Apache Arrow and Apache Iceberg) into a single immutable storage and state engine.

We welcome contributions from systems engineers, distributed systems researchers, kernel enthusiasts, and data platform developers.

---

## 1. Architectural Governance & Canonical Source of Truth

The Keirox architecture is formally specified and strictly governed:

1. **Sole Source of Truth**: The architecture specification suite in [`docs/architecture/`](docs/architecture/) (`KEI-INDEX` through `KEI-VAL-052`) is the canonical authority for all system contracts, memory budgets, data layouts, and protocols.
2. **Zero Divergence**: Code, tests, schemas, and algorithms MUST NEVER diverge from the specifications in `docs/architecture/`.
3. **The Golden Invariant (KEI-ARC-010 §3)**:
   $$\text{Data is written exactly once to an immutable physical log. Consumption semantics are defined entirely by the consumer's mutable, replicated state overlay.}$$
4. **Architectural Change Control**: Any change to system behavior, protocols, or contracts MUST first be proposed and approved via an **Architecture Decision Record (ADR)** in [`docs/architecture/KEI-ARC-012.md`](docs/architecture/KEI-ARC-012.md) before writing implementation code.

---

## 2. Engineering Standards & Systems Invariants

All implementation code in Keirox must adhere to systems-level engineering discipline:

### 2.1 Hot-Path Memory & I/O Hygiene (<2ms p99 SLA)
- **Zero Allocations**: Hot write ingress and WAL append loops must execute over pre-allocated lock-free row arenas and static ring buffers (`io_uring`). Zero runtime heap allocations (`malloc`, `Box`, dynamic `Vec::new()`, or dynamic closures) on hot paths.
- **Direct I/O**: Enforce `O_DIRECT` and kernel-bypass `io_uring` for physical WAL appends. Never route hot ingress through the OS page cache.
- **Cache-Line & SIMD Alignment**: Enforce 64-byte alignment on critical structs (`StateShardKey`, `Lease`, `SSTableChunkHeader`) and Arrow columnar buffers for AVX-512 / ARM Neon vectorization.
- **Thread Pinning**: Dedicated CPU core pinning for ingress network I/O and WAL flush loops. Compaction and Arrow transcoding are isolated to separate background worker pools.

### 2.2 Strict Static Typing & Safety
- **100% Type Safety**: All domain models must have explicit types. Zero untyped pointers (`void*`) or loose casts.
- **`unsafe` Hygiene**: Unsafe blocks are forbidden unless required for kernel I/O (`io_uring`) or SIMD intrinsics. Every `unsafe` block MUST be documented with a `// SAFETY:` rationale and covered by isolated unit tests.
- **Fail-Fast Boundaries**: Parse and validate all inputs at the edge. Explicit domain error enums only; zero swallowed errors.

### 2.3 Layer Isolation
- **Domain Layer**: Pure distributed models, causal DAG ordering, Roaring Bitmap state machines, and sliding watermark invariants. Zero OS, disk, or network dependencies. 100% deterministic and unit-testable in isolation.
- **Application Layer**: State coordinators, command dispatching, lease management via Hierarchical Timing Wheels.
- **Infrastructure Layer**: Storage adapters (`io_uring` WAL, direct NVMe, Apache Parquet encoder, S3 streaming, Raft consensus, Kafka framing).
- **Presentation Layer**: Client gateways (Kafka Wire Protocol gateway, SQS/AMQP translators, native Arrow Flight gRPC server).

---

## 3. Getting Started

### 3.1 Prerequisites
- **Rust**: Latest stable or nightly toolchain (nightly required for AVX-512 / specific SIMD intrinsics and `io_uring` features).
- **Linux**: Kernel 5.10+ recommended for complete `io_uring` feature support.
- **Tools**: `cargo`, `rustfmt`, `clippy`, `cargo-audit`, `cargo-nextest`.

### 3.2 Setting Up the Repository
```bash
# Clone the repository
git clone https://github.com/seismael/keirox.git
cd keirox

# Verify toolchain
cargo --version
rustc --version
```

### 3.3 Quality Gates & Pre-Commit Verification
Before submitting code, ensure all quality gates pass cleanly:

```bash
# 1. Format check
cargo fmt --all -- --check

# 2. Strict linter check (Zero warning policy)
cargo clippy --all-targets --all-features -- -D warnings

# 3. Test execution
cargo test --all

# 4. Security audit
cargo audit
```

---

## 4. Development Workflow

### 4.1 Branching Strategy
- `main`: Production-ready, certified code.
- Feature / Fix branches: Branch from `main` using descriptive prefixes:
  - `feat/`: New subsystem feature or gateway implementation.
  - `fix/`: Bug fix or invariant violation correction.
  - `perf/`: Performance optimization with benchmark evidence.
  - `docs/`: Documentation enhancements or ADR proposals.
  - `test/`: Chaos tests, Jepsen-style suites, or benchmark additions.

### 4.2 Commit Message Convention
Keirox follows the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```text
<type>(<scope>): <short summary>

[optional body explaining context, non-obvious rationale, and invariant impact]

[optional footer referencing ADRs, issues, or NFRs]
```

**Examples**:
- `feat(storage): implement 64-byte aligned SSTableChunkHeader (ADR-013)`
- `fix(state): advance base watermark on mandatory DLQ eviction (ADR-004)`
- `perf(elt): vectorise Arrow RecordBatch shredding using AVX-512`
- `docs(adr): propose ADR-085 for dynamic tenant partition bucketing`

---

## 5. Pull Request & Review Process

1. **Self-Review**: Audit your diff against [AGENTS.md](AGENTS.md) and the applicable specifications in [`docs/architecture/`](docs/architecture/).
2. **Evidence-Gated PRs**: Performance optimizations or concurrency changes MUST include benchmark or chaos test results comparing before and after.
3. **Continuous Integration**: All GitHub Actions CI checks (Format, Clippy, Unit Tests, ASan/TSan memory tests) must pass.
4. **Maintainer Review**: Every PR requires review and approval from at least one subsystem owner and compliance with architectural invariants.
5. **Clean History**: PRs are squash-merged or rebased to keep git history linear and meaningful.

---

## 6. Code of Conduct

All contributors and maintainers are expected to adhere to our [Code of Conduct](CODE_OF_CONDUCT.md) (Contributor Covenant v2.1). Please report any unacceptable behavior to **conduct@keirox.io**.

---

## 7. License

By contributing to Keirox, you agree that your contributions will be licensed under the [Apache License 2.0](LICENSE).
