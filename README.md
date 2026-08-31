<div align="center">

# Keirox: The Polymorphic Event Fabric (PEF)

**Next-Generation Unified Distributed Streaming, Task Queuing, and Columnar Lakehouse Runtime**

[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)
[![Language](https://img.shields.io/badge/Language-Rust-orange.svg)](https://www.rust-lang.org/)
[![Architecture](https://img.shields.io/badge/Architecture-Certified_L0--L3_Suite-emerald.svg)](docs/architecture/INDEX.md)
[![Verification](https://img.shields.io/badge/Verification-KEI--VER--001_Certified-success.svg)](docs/verification/KEI-VER-001.md)
[![Demos](https://img.shields.io/badge/Demos-KEI--DEMO--700_Validated-blue.svg)](docs/verification/KEI-DEMO-700.md)
[![SLA](https://img.shields.io/badge/Ingress_p99-≤2.0ms_(Profile_P1)-blueviolet.svg)](docs/architecture/KEI-ARC-011.md)
[![Status](https://img.shields.io/badge/Status-Production_Certified-success.svg)](docs/reports/KEI-CERT-500.md)

</div>

---

## 🚀 Overview

**Keirox** is an open-source, high-performance distributed systems runtime implementing the **Polymorphic Event Fabric (PEF)**.

Modern data infrastructures suffer from severe technological fragmentation: organizations routinely deploy and maintain separate broker clusters for **event streaming** (e.g., Apache Kafka), **message queuing / task distribution** (e.g., RabbitMQ, Amazon SQS), and **lakehouse ingestion pipelines** (e.g., Spark/Flink connectors writing to Apache Iceberg or Delta Lake). This fragmentation introduces massive operational overhead, infrastructure cost duplication, dual-write consistency hazards, and high network egress fees.

Keirox eliminates this complexity through a unified distributed storage and state architecture governed by **The Golden Invariant**:

> **$$\text{The Golden Invariant}$$**
> $$\text{Data is written exactly once to an immutable physical log.}$$
> $$\text{Consumption semantics (streaming, queuing, dead-lettering, lakehouse analytics) are defined}$$
> $$\text{entirely by the consumer's mutable, replicated state overlay.}$$

```mermaid
flowchart TD
    subgraph Ingress["Client Protocols & Gateways"]
        K[Kafka Clients] -->|Kafka Wire Protocol| GW[Protocol Gateways & SDK]
        S[SQS / AMQP Clients] -->|SQS / AMQP Translators| GW
        N[Native SDKs] -->|Apache Arrow Flight gRPC| GW
    end

    subgraph Core["Keirox Distributed Engine"]
        GW -->|Zero-Copy Batches| WAL["Tier-0: Append-Only WAL Ring-Buffer\nio_uring + O_DIRECT + NVMe Storage"]
        WAL -->|Raft Consensus| RAFT["Consensus & Quorum Plane (3-Node Quorum)"]
        
        WAL -->|State Overlay| STATE["Consumption State Plane\nRoaring Bitmaps (ACK/LEASE/DLQ)\nHierarchical Timing Wheels (O(1) TTL)"]
        
        WAL -->|Single-Pass Compaction| ELT["Internalized Columnar ELT\nArrow Vectorizer + Adaptive Shredding (64-field cap)"]
    end

    subgraph Storage["Storage & Lakehouse Tiers"]
        ELT -->|Batched Parquet Upload| S3["Tier-1: Cloud Object Storage (S3 / GCS / Azure Blob)"]
        ELT -->|Atomic Snapshots| ICEBERG["Apache Iceberg Shared Tenant Tables\ntenant_{id}.events"]
    end

    subgraph Security["Security & Governance"]
        KMS["KMS / Envelope Encryption"] -.->|AES-256-GCM DEKs| WAL
        KMS -.->|Crypto-Shredding Erasure| S3
        STATE -.->|Mandatory DLQ Eviction| VDLQ["Virtual DLQ Index (Zero WAL Duplication)"]
    end
```

---

## 📁 Repository Layout

```text
keirox/
├── Cargo.toml                  # Virtual workspace configuration (18 crates)
├── README.md                   # Repository overview, architecture & quick start
├── CONTRIBUTING.md             # Development standards & zero-allocation rules
├── AGENTS.md                   # AI Agent governance & zero-divergence protocol
├── AUDIT.md                    # Live end-to-end product audit & verification status
├── LICENSE                     # Apache License 2.0
├── docs/                       # Formal engineering documentation suite
│   ├── README.md               # Documentation routing & index map
│   ├── architecture/           # 25 certified architecture specifications (L0–L3)
│   ├── engineering/            # Implementation RFCs and design guides (Phase 1–5)
│   ├── verification/           # Implementation Verification Protocols & Demos (KEI-VER-001, KEI-DEMO-700)
│   ├── benchmarks/             # Benchmark plans, methodology, and raw results
│   ├── reports/                # Formal engineering certification reports (KEI-CERT-100..500)
│   └── archive/                # Non-authoritative historical concept drafts
├── crates/                     # Modular Rust workspace crates
│   ├── keirox-core/            # Domain models, identifiers, errors, traits, and invariants
│   ├── keirox-state/           # Roaring Bitmap state overlay & sliding watermarks (64-bit)
│   ├── keirox-timer/           # Hierarchical Timing Wheel for O(1) lease TTL & scheduling
│   ├── keirox-arena/           # Lock-free pre-allocated row arenas for hot ingress (<2ms)
│   ├── keirox-wal/             # io_uring + O_DIRECT WAL engine & 46B RecordEntry framing
│   ├── keirox-index/           # Packed 32-byte stream registry & SSTable indexes
│   ├── keirox-consensus/       # Multi-Raft quorum consensus, HardState & replication engine
│   ├── keirox-coordinator/     # Coordinator sharding, consistent hashing & epoch fencing
│   ├── keirox-arrow-elt/       # Arrow vectorizer, adaptive shredder (64-key cap) & Iceberg committer
│   ├── keirox-tier1/           # Tier-1 S3/GCS streaming, multipart uploader & manifests
│   ├── keirox-schema/          # Schema registry, compatibility governance & shredding policy
│   ├── keirox-api/             # Protobuf RPC schemas & gateway protocol definitions
│   ├── keirox-sdk/             # Native Arrow Flight & gRPC client SDK
│   ├── keirox-gateway/         # Kafka wire-protocol, SQS, AMQP gateways & migration bridge
│   ├── keirox-server/          # Distributed runtime daemon, CLI parser & Prometheus metrics
│   ├── keirox-bench/           # Canonical benchmark harness (P1–P6 workloads)
│   ├── keirox-chaos/           # Chaos engineering fault injection & Jepsen test suite
│   └── keirox-testkit/         # Test fixtures, mock storage, cluster harness & protocol test suites
├── scripts/                    # Build, formatting, and CI automation scripts
├── deploy/                     # Dockerfiles, Kubernetes manifests & Helm charts
└── tests/                      # End-to-end integration and verification test suites
```

---

## 📦 Workspace Crates

| Crate | Layer | Purpose & Key Invariants |
|---|---|---|
| [`keirox-core`](crates/keirox-core) | Domain | Pure domain models, error taxonomy, identifiers, and Golden Invariant contracts. |
| [`keirox-state`](crates/keirox-state) | Domain | `RoaringTreemap` 64-bit consumer state overlay (`READY`, `LEASED`, `ACKED`, `EVICTED_DLQ`). |
| [`keirox-timer`](crates/keirox-timer) | Application | Hierarchical Timing Wheel for sub-millisecond lease scheduling and timeout eviction. |
| [`keirox-arena`](crates/keirox-arena) | Application | Lock-free pre-allocated memory arenas guaranteeing zero heap allocations on hot paths. |
| [`keirox-wal`](crates/keirox-wal) | Infrastructure | `io_uring` + `O_DIRECT` NVMe write-ahead log engine, 128B headers, 46B `RecordEntry`, CRC32C framing. |
| [`keirox-index`](crates/keirox-index) | Infrastructure | Packed 32-byte `StreamRegistryEntry`, sparse exception table, SSTable chunk index. |
| [`keirox-consensus`](crates/keirox-consensus) | Infrastructure | Multi-Raft quorum consensus engine, Raft log, persistent `HardState`, and epoch fencing primitives. |
| [`keirox-coordinator`](crates/keirox-coordinator) | Application | Deterministic coordinator sharding, consistent hashing, and 24-byte epoch-fenced tokens. |
| [`keirox-arrow-elt`](crates/keirox-arrow-elt) | Infrastructure | In-broker Arrow vectorizer, adaptive shredder (64-field cap), Iceberg committer with OCC. |
| [`keirox-tier1`](crates/keirox-tier1) | Infrastructure | Tier-1 S3/GCS streaming, multipart uploader, manifest registry, and backpressure gating. |
| [`keirox-schema`](crates/keirox-schema) | Application | Schema registry, compatibility governance, and adaptive columnar shredding policy. |
| [`keirox-api`](crates/keirox-api) | Presentation | Protobuf definitions, Arrow Flight gRPC RPC, Kafka/SQS/AMQP gateway mappings, health & Prometheus telemetry. |
| [`keirox-sdk`](crates/keirox-sdk) | Presentation | Native Arrow Flight & gRPC client SDK with producer/consumer/task-queue abstractions. |
| [`keirox-gateway`](crates/keirox-gateway) | Presentation | Kafka wire-protocol compatibility ingest/fetch, SQS MD5 & handle encoding, AMQP, and migration bridge. |
| [`keirox-server`](crates/keirox-server) | Presentation | Production daemon binary, CLI parser, cluster coordinator, and Prometheus metrics exposition. |
| [`keirox-bench`](crates/keirox-bench) | Verification | Performance benchmark harness implementing canonical profiles P1 through P6. |
| [`keirox-chaos`](crates/keirox-chaos) | Verification | Fault injection engine for partitions, disk stalls, clock skew, and crash testing. |
| [`keirox-testkit`](crates/keirox-testkit) | Verification | Deterministic in-memory test fixtures, cluster runtime, and full verification test suites. |

---

## 📚 Complete Architecture Documentation Suite

The Keirox architecture is formally specified and verified across 25 comprehensive engineering documents in [`docs/architecture/`](docs/architecture/INDEX.md):

| Level | Document ID | Title | Summary |
|---|---|---|---|
| **Framework** | [`KEI-INDEX`](docs/architecture/INDEX.md) | Architecture Documentation Index & Routing Map | Single-entry architecture register, routing map, and precedence ladder. |
| **L0** | [`KEI-ARC-001`](docs/architecture/KEI-ARC-001.md) | Architecture Vision & System Context | System context, core problem statement, and unified fabric vision. |
| **L1** | [`KEI-ARC-010`](docs/architecture/KEI-ARC-010.md) | Conceptual Architecture & The Golden Invariant | Dual-plane separation, Log-Bitmap duality, and system boundaries. |
| **L1** | [`KEI-ARC-011`](docs/architecture/KEI-ARC-011.md) | Quality Attributes & NFRs | Concrete NFR catalog (PERF, DUR, AVAIL, SCALE, MEM, REC, SEC, COMP). |
| **L1** | [`KEI-ARC-012`](docs/architecture/KEI-ARC-012.md) | Architecture Principles & ADR Index | 38 binding Architecture Decision Records (ADRs) and governing principles. |
| **L2** | [`KEI-ARC-020`](docs/architecture/KEI-ARC-020.md) | Storage Engine Architecture | `io_uring` WAL ring buffer, Tier-0 NVMe, Tier-1 S3, single-pass compaction. |
| **L2** | [`KEI-ARC-021`](docs/architecture/KEI-ARC-021.md) | Consumption State Plane Architecture | Roaring Bitmap state transitions, lease management, virtual DLQ index. |
| **L2** | [`KEI-ARC-022`](docs/architecture/KEI-ARC-022.md) | Consensus, Coordination & HA Architecture | Multi-Raft quorum, deterministic coordinator sharding, epoch fencing. |
| **L2** | [`KEI-ARC-023`](docs/architecture/KEI-ARC-023.md) | Columnar ELT & Lakehouse Integration | Arrow vectorization, adaptive shredding, Parquet encoding, Iceberg integration. |
| **L2** | [`KEI-ARC-024`](docs/architecture/KEI-ARC-024.md) | Protocol Gateways & SDK Architecture | Native Arrow Flight gRPC, Kafka wire protocol, SQS/AMQP translation. |
| **L2** | [`KEI-ARC-025`](docs/architecture/KEI-ARC-025.md) | Security, Privacy & Compliance Architecture | AES-256-GCM envelope encryption, ABAC PDP, crypto-shredding, audit logs. |
| **L2** | [`KEI-ARC-026`](docs/architecture/KEI-ARC-026.md) | Multi-Region Replication & DR Architecture | Mode A WAN replication, HLC causal consistency, region failover. |
| **L2** | [`KEI-ARC-027`](docs/architecture/KEI-ARC-027.md) | Operability, Observability & Capacity Architecture | Metrics catalog, distributed tracing, backpressure ladder, rolling upgrades. |
| **L3** | [`KEI-DES-030`](docs/architecture/KEI-DES-030.md) | WAL Binary Framing & Storage Layout | 128-byte Batch Headers, 46-byte Record Entries, CRC32C, SIMD alignment. |
| **L3** | [`KEI-DES-031`](docs/architecture/KEI-DES-031.md) | State Plane Data Structures & Algorithms | `Roaring64Map`, Timing Wheel, watermark algorithms, lease journal binary format. |
| **L3** | [`KEI-DES-032`](docs/architecture/KEI-DES-032.md) | Producer/Consumer/Lease/ACK API Protocol | Protobuf RPC definitions, error taxonomy, idempotency keys. |
| **L3** | [`KEI-DES-033`](docs/architecture/KEI-DES-033.md) | Schema Registry & Adaptive Shredding | Schema inference scoring, field promotion/demotion, `_unstructured_payload`. |
| **L3** | [`KEI-DES-034`](docs/architecture/KEI-DES-034.md) | Iceberg Catalog Committer Specification | Atomic catalog commit protocol, commit ledger, snapshot lifecycle, orphan cleanup. |
| **L3** | [`KEI-DES-035`](docs/architecture/KEI-DES-035.md) | Gateway Wire-Protocol Compatibility Matrices | S0–S3 support tiers for Kafka, SQS, and AMQP protocols. |
| **L3** | [`KEI-DES-036`](docs/architecture/KEI-DES-036.md) | Encryption, Key Management & Crypto-Shredding | KMS adapters, DEK lifecycle, Destroyed-Key Registry, erasure workflow. |
| **OPS** | [`KEI-OPS-040`](docs/architecture/KEI-OPS-040.md) | Operations Runbooks, Upgrade & DR Procedures | 20 operational runbooks, rolling upgrades, DR failover, incident response. |
| **OPS** | [`KEI-OPS-041`](docs/architecture/KEI-OPS-041.md) | Validation, Benchmark & Chaos Test Plan | Canonical workload profiles (P1–P6), Jepsen test suite, release certification gates. |
| **VAL** | [`KEI-VAL-050`](docs/architecture/KEI-VAL-050.md) | Final Cross-Document Consistency Audit | Independent audit certifying zero contradictions and 100% NFR traceability. |
| **VAL** | [`KEI-VAL-051`](docs/architecture/KEI-VAL-051.md) | Requirements Traceability Matrix (RTM) | 113 requirements mapped end-to-end to design, operations, and tests. |
| **VAL** | [`KEI-VAL-052`](docs/architecture/KEI-VAL-052.md) | Release Readiness Checklist | Executive 5-gate certification checklist for Phase-1 engineering execution. |

---

## 🛠️ Engineering Execution Suite

Engineering execution is governed by the formal plans in [`docs/engineering/`](docs/engineering/README.md):

### Phase 1 Suite — Single-Node Core Engine (`1xx`)
| Document ID | Plan Title | Scope & Purpose |
|---|---|---|
| [`KEI-ENG-100`](docs/engineering/KEI-ENG-100.md) | Phase 1 Engineering Execution Plan | Master roadmap, milestones (M1.0–M1.10), workstreams, and DoD. |
| [`KEI-SPIKE-101`](docs/engineering/KEI-SPIKE-101.md) | Minimum Vertical Prototype Plan | 12-week spike: Single-node WAL, Roaring Bitmaps, leases, ACKs, DLQ, Parquet. |
| [`KEI-FORMAL-101`](docs/engineering/KEI-FORMAL-101.md) | State Machine Validation Plan | 5 TLA+ models: Lease lifecycle, watermark monotonicity, DLQ progress, test oracles. |
| [`KEI-BENCH-101`](docs/engineering/KEI-BENCH-101.md) | Performance Validation Harness Plan | Canonical workload profiles (P1-Proto..P6-Proto), telemetry taxonomy, and disclosures. |
| [`KEI-RISK-101`](docs/engineering/KEI-RISK-101.md) | Risk Reduction & Go/No-Go Plan | 5x5 technical risk matrix, mitigations, Go/No-Go gates, pre-authorized pivots. |

### Phase 2 Suite — Distributed Durability & Coordinator Sharding (`2xx`)
| Document ID | Plan Title | Scope & Purpose |
|---|---|---|
| [`KEI-ENG-200`](docs/engineering/KEI-ENG-200.md) | Phase 2 Engineering Execution Plan | 3-node cluster, Multi-Raft Quorum, S3 streaming, failover <3.5s. |
| [`KEI-SPIKE-201`](docs/engineering/KEI-SPIKE-201.md) | Distributed Consensus Prototype Plan | 3-node cluster prototype, epoch fencing, S3 uploader, chaos tests. |
| [`KEI-FORMAL-201`](docs/engineering/KEI-FORMAL-201.md) | Distributed Consensus Verification Plan | 5 TLA+ models: Data Raft, Meta Raft, Epoch Fencing, State Replication, Split-Brain. |
| [`KEI-BENCH-201`](docs/engineering/KEI-BENCH-201.md) | Multi-Node Failover & Benchmark Harness | Multi-node throughput, failover latency, leader election telemetry. |
| [`KEI-RISK-201`](docs/engineering/KEI-RISK-201.md) | Distributed Risk Reduction Plan | Split-brain, network partition, and cloud storage risk management. |

### Phase 3 Suite — Ecosystem Gateways, Native SDKs & Lakehouse (`3xx`)
| Document ID | Plan Title | Scope & Purpose |
|---|---|---|
| [`KEI-ENG-300`](docs/engineering/KEI-ENG-300.md) | Phase 3 Engineering Execution Plan | Kafka wire protocol gateway, native SDKs, schema registry, Iceberg ELT. |
| [`KEI-SPIKE-301`](docs/engineering/KEI-SPIKE-301.md) | Compatibility & Gateway Spike Plan | Wire protocol translation, Arrow flight, zero-copy transcoding. |
| [`KEI-COMPAT-301`](docs/engineering/KEI-COMPAT-301.md) | Wire Protocol Compatibility Plan | Kafka, SQS, AMQP conformance matrices and certification tests. |
| [`KEI-LAKE-301`](docs/engineering/KEI-LAKE-301.md) | Lakehouse & Iceberg Integration Plan | Iceberg REST catalog, snapshot commit, Snappy Parquet streaming. |
| [`KEI-SDK-301`](docs/engineering/KEI-SDK-301.md) | Native Client SDKs Plan | High-performance Rust, Go, Python client SDKs. |
| [`KEI-API-301`](docs/engineering/KEI-API-301.md) | REST API & HTTP Gateway Plan | REST API surface, HTTP/REST-to-gRPC transcoding, OpenAPI 3.1, probes. |
| [`KEI-RISK-301`](docs/engineering/KEI-RISK-301.md) | Ecosystem & Gateway Risk Plan | Protocol drift, translation latency, and schema evolution risk management. |

### Phase 4 Suite — Enterprise Hardening, Compliance & Multi-Region (`4xx`)
| Document ID | Plan Title | Scope & Purpose |
|---|---|---|
| [`KEI-ENG-400`](docs/engineering/KEI-ENG-400.md) | Phase 4 Engineering Execution Plan | Enterprise hardening, compliance, multi-region replication, Jepsen verification. |
| [`KEI-SPIKE-401`](docs/engineering/KEI-SPIKE-401.md) | Enterprise Hardening Prototype Plan | Multi-region WAN replication, crypto-shredding, timing wheels. |
| [`KEI-SEC-401`](docs/engineering/KEI-SEC-401.md) | Security & Compliance Certification Plan | SOC 2, ISO 27001, GDPR/CCPA crypto-shredding, KMS. |
| [`KEI-MR-401`](docs/engineering/KEI-MR-401.md) | Multi-Region & DR Certification Plan | WAN replication, HLC ordering, cross-region failover. |
| [`KEI-QUEUE-401`](docs/engineering/KEI-QUEUE-401.md) | Advanced Queuing & Delay Plan | Hierarchical Timing Wheels, delayed message delivery, priority queues. |
| [`KEI-VAL-401`](docs/engineering/KEI-VAL-401.md) | Adversarial & Jepsen Verification Plan | Jepsen partition suites, adversarial pen testing, long-term soak. |
| [`KEI-RISK-401`](docs/engineering/KEI-RISK-401.md) | Enterprise Risk & GA Readiness Plan | Compliance, multi-region WAN latency, and certification risk matrix. |

### Phase 5 Suite — Productization, Distribution & Day-2 Operations (`5xx`)
| Document ID | Plan Title | Scope & Purpose |
|---|---|---|
| [`KEI-ENG-500`](docs/engineering/KEI-ENG-500.md) | Phase 5 Master Plan | Cloud-native distribution, CLI/console, migration tooling, release supply chain. |
| [`KEI-K8S-501`](docs/engineering/KEI-K8S-501.md) | Kubernetes Operator & Terraform Plan | K8s Operator CRDs, Helm charts, Terraform provider, air-gap. |
| [`KEI-MIG-501`](docs/engineering/KEI-MIG-501.md) | Enterprise Migration & Bridge Plan | Kafka-to-Keirox migration bridge, offset sync, zero-downtime cutover. |
| [`KEI-REL-501`](docs/engineering/KEI-REL-501.md) | Secure Supply Chain & Release Plan | SLSA Level 3, SBOM, Sigstore signing, Distroless container images. |
| [`KEI-OPS-502`](docs/engineering/KEI-OPS-502.md) | Day-2 Observability & Console Plan | Grafana dashboard suite, Prometheus alerts, OTel, Web Operations Console. |
| [`KEI-RISK-501`](docs/engineering/KEI-RISK-501.md) | Phase 5 Risks & v1 GA Launch Plan | GA launch risks, adoption readiness, and production release gates. |

---

## 🔍 Verification & Demonstration Protocols

Keirox includes rigorous forensic verification and live demonstration suites in [`docs/verification/`](docs/verification/README.md):

| Protocol Document | Title | Description & Scope |
|---|---|---|
| [**`KEI-VER-001.md`**](docs/verification/KEI-VER-001.md) | **Implementation Verification Protocol** | 200+ forensic verification checks across 15 technical domains (Physical WAL bit-level corruption, 64-bit state overlays, Multi-Raft quorum, consumption semantics, lakehouse sync, envelope encryption, GDPR crypto-shredding, multi-region DR, protocol gateways, telemetry, benchmarking, and supply chain). |
| [**`KEI-DEMO-700.md`**](docs/verification/KEI-DEMO-700.md) | **Live Enterprise Demonstration Report** | 10 real-world production-mode enterprise adoption scenarios: E-Commerce Order Pipeline, IoT Telemetry, Kafka Zero-Downtime Migration, GDPR Article 17 Erasure, Multi-Region DR Failover, Task Queue Priority Workers, Real-Time Fraud Detection, Log Analytics, Kubernetes Operations, and Supply Chain Integrity. |

---

## 🏆 Formal Engineering Certification Reports

Each development phase is formally certified through an automated evidence gate in [`docs/reports/`](docs/reports/README.md):

| Phase | Certified Package | Key Certified Invariants & Milestones |
|---|---|---|
| **Phase 1** | [**`KEI-CERT-100`**](docs/reports/KEI-CERT-100.md) | Single-Node Core Engine, 128B WAL framing, 46B `RecordEntry`, Roaring Bitmap state plane, Parquet ELT. |
| **Phase 2** | [**`KEI-CERT-200`**](docs/reports/KEI-CERT-200.md) | 3-Node Multi-Raft Quorum, Coordinator Sharding, Epoch Fencing, Tier-1 S3 Streaming, <3.5s Failover. |
| **Phase 3** | [**`KEI-CERT-300`**](docs/reports/KEI-CERT-300.md) | Kafka Wire Protocol Gateway, Native Arrow Flight SDK, Schema Registry Governance, Iceberg OCC Committer. |
| **Phase 4** | [**`KEI-CERT-400`**](docs/reports/KEI-CERT-400.md) | KMS Envelope Encryption, GDPR/CCPA Crypto-Shredding, Default-Deny ABAC, SQS/AMQP Gateways, Multi-Region Mode A & PITR. |
| **Phase 5** | [**`KEI-CERT-500`**](docs/reports/KEI-CERT-500.md) | Kubernetes Operator & CRDs, Kafka Migration Bridge & Cutover, Distroless Packaging, Day-2 Observability, v1 GA Certification. |

---

## 🛠️ Getting Started

### Prerequisites
- **Rust Toolchain**: Stable Rust 1.78+ (`rustup default stable`).
- **Operating System**: Linux (kernel 5.10+ recommended for `io_uring` + `O_DIRECT`), macOS, or Windows.

### Building & Running Verification
```bash
# Clone repository
git clone https://github.com/seismael/keirox.git
cd keirox

# Validate formatting and linter hygiene (zero-warning policy)
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Run all unit, integration, and verification suites across the workspace
cargo test --workspace

# Run the formal KEI-VER-001 forensic verification protocol
cargo test --package keirox-testkit --test kei_ver_001_protocol_test

# Run the KEI-DEMO-700 enterprise adoption demo test suite
cargo test --package keirox-testkit --test kei_demo_700_scenarios_test
```

### Running the Distributed Runtime Daemon
```bash
# Start a single-node daemon with TCP ingress and Prometheus metrics
cargo run --package keirox-server -- start --bind-addr 127.0.0.1:9092 --metrics-addr 127.0.0.1:9100 --data-dir ./data

# Inspect health probes
curl http://127.0.0.1:9100/healthz
curl http://127.0.0.1:9100/readyz

# Query Prometheus metrics exposition
curl http://127.0.0.1:9100/metrics
```

---

## 🤝 Contributing

We welcome contributions from systems engineers and researchers! Keirox is governed by rigorous engineering invariants and clean architectural boundaries.

- Please read our [**Contributing Guide**](CONTRIBUTING.md) for details on code standards, hot-path memory hygiene, and pull request workflows.
- All architectural modifications must be audited against the Golden Invariant and formalized via an ADR aligned with [`docs/architecture/KEI-ARC-012.md`](docs/architecture/KEI-ARC-012.md).
- All participants must adhere to our [**Code of Conduct**](CODE_OF_CONDUCT.md).

---

## 📄 License

Keirox is licensed under the **[Apache License 2.0](LICENSE)**.

```text
Copyright 2026 Keirox Authors & Contributors

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```
