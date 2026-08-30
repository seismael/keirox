# Keirox Engineering Execution Plans

This directory contains the formal engineering execution plans, technical spikes, formal methods validation, benchmark specifications, organizational delivery frameworks, and risk reduction gates for the Keirox Polymorphic Event Fabric.

---

## ⚡ Fast Task Routing Map (Token-Optimized)

When working on engineering tasks, agents MUST consult this table to ingest **only the single relevant plan**:

### Phase 1 Suite — Single-Node Core Engine (`1xx`)

| Active Engineering Task | Primary Plan Document | Key Deliverables & Evidence |
|---|---|---|
| **Phase 1 Roadmap & Milestones** | [`KEI-ENG-100.md`](KEI-ENG-100.md) | Milestones M1.0–M1.10, workstreams WS-0..WS-5, DoD. |
| **Minimum Vertical Prototype (Spike)** | [`KEI-SPIKE-101.md`](KEI-SPIKE-101.md) | 12-week spike: Single-node WAL, Roaring Bitmaps, leases, ACKs, DLQ, Parquet. |
| **Formal State Machine Validation** | [`KEI-FORMAL-101.md`](KEI-FORMAL-101.md) | 5 TLA+ models: Lease lifecycle, watermark monotonicity, DLQ progress, test oracles. |
| **Benchmark Harness & Telemetry** | [`KEI-BENCH-101.md`](KEI-BENCH-101.md) | Profiles P1-Proto..P6-Proto, HDR histograms, environmental disclosure. |
| **Risk Management & Pivot Triggers** | [`KEI-RISK-101.md`](KEI-RISK-101.md) | 5x5 technical risk matrix, mitigations, Go/No-Go gates, pre-authorized pivots. |

### Phase 2 Suite — Distributed Durability & Coordinator Sharding (`2xx`)

| Active Engineering Task | Primary Plan Document | Key Deliverables & Evidence |
|---|---|---|
| **Phase 2 Roadmap & Milestones** | [`KEI-ENG-200.md`](KEI-ENG-200.md) | 3-node cluster, Multi-Raft Quorum, S3 streaming, failover <3.5s. |
| **Distributed Consensus Prototype** | [`KEI-SPIKE-201.md`](KEI-SPIKE-201.md) | 12-week spike: 3-node cluster formation, Raft replication, epoch fencing, S3 uploader. |
| **Distributed Formal Verification** | [`KEI-FORMAL-201.md`](KEI-FORMAL-201.md) | 5 TLA+ models: Data Raft, Meta Raft, Epoch Fencing, State Replication, Split-Brain. |
| **Multi-Node Benchmark & Failover** | [`KEI-BENCH-201.md`](KEI-BENCH-201.md) | Multi-node throughput, failover latency, leader election telemetry. |
| **Distributed Risk & Failover Gates** | [`KEI-RISK-201.md`](KEI-RISK-201.md) | Distributed risk matrix, split-brain defenses, S3 throttling backpressure gates. |

### Phase 3 Suite — Ecosystem Compatibility, Gateways & Lakehouse (`3xx`)

| Active Engineering Task | Primary Plan Document | Key Deliverables & Evidence |
|---|---|---|
| **Phase 3 Roadmap & Milestones** | [`KEI-ENG-300.md`](KEI-ENG-300.md) | Kafka/SQS/AMQP gateways, Apache Iceberg ELT, native SDKs. |
| **Compatibility & Gateway Spike** | [`KEI-SPIKE-301.md`](KEI-SPIKE-301.md) | Wire protocol translation, Apache Arrow flight, zero-copy transcoding. |
| **Wire Protocol Compatibility** | [`KEI-COMPAT-301.md`](KEI-COMPAT-301.md) | Kafka v0.11-v3.7, SQS, AMQP 1.0 conformance suites. |
| **Lakehouse & Iceberg Integration** | [`KEI-LAKE-301.md`](KEI-LAKE-301.md) | Iceberg REST catalog, snapshot commit, Snappy Parquet streaming. |
| **Native Client SDKs** | [`KEI-SDK-301.md`](KEI-SDK-301.md) | High-performance Rust, Go, Python SDKs with connection pooling. |
| **REST API & HTTP Gateway** | [`KEI-API-301.md`](KEI-API-301.md) | REST API surface, HTTP/REST-to-gRPC transcoding, OpenAPI 3.1, probes. |
| **Ecosystem & Gateway Risk Plan** | [`KEI-RISK-301.md`](KEI-RISK-301.md) | Protocol drift, translation overhead, and schema evolution risks. |

### Phase 4 Suite — Enterprise Hardening, Compliance & Multi-Region (`4xx`)

| Active Engineering Task | Primary Plan Document | Key Deliverables & Evidence |
|---|---|---|
| **Phase 4 Roadmap & Milestones** | [`KEI-ENG-400.md`](KEI-ENG-400.md) | ABAC, KMS envelope encryption, multi-region replication, Jepsen testing. |
| **Enterprise Hardening Spike** | [`KEI-SPIKE-401.md`](KEI-SPIKE-401.md) | Multi-region WAN replication, crypto-shredding, delayed scheduling. |
| **Security & Compliance Plan** | [`KEI-SEC-401.md`](KEI-SEC-401.md) | SOC 2 Type II, ISO 27001, GDPR/CCPA crypto-shredding, KMS integration. |
| **Multi-Region & DR Plan** | [`KEI-MR-401.md`](KEI-MR-401.md) | Multi-region replication, RPO=0 local / RPO<1s WAN, automatic failover. |
| **Advanced Queuing & Delay** | [`KEI-QUEUE-401.md`](KEI-QUEUE-401.md) | Hierarchical Timing Wheels, delayed message delivery, priority queues. |
| **Adversarial Verification & Jepsen** | [`KEI-VAL-401.md`](KEI-VAL-401.md) | Jepsen partition suites, adversarial pen testing, long-term soak. |
| **Enterprise Risk & GA Readiness** | [`KEI-RISK-401.md`](KEI-RISK-401.md) | Compliance, multi-region WAN latency, and certification risk matrix. |

### Phase 5 Suite — Productization, Distribution & Day-2 Operations (`5xx`)

| Active Engineering Task | Primary Plan Document | Key Deliverables & Evidence |
|---|---|---|
| **Phase 5 Roadmap & Milestones** | [`KEI-ENG-500.md`](KEI-ENG-500.md) | Cloud-native distribution, CLI/console, migration tooling, release supply chain. |
| **Kubernetes Operator & Terraform** | [`KEI-K8S-501.md`](KEI-K8S-501.md) | K8s Operator CRDs, Helm charts, Terraform provider, air-gapped deployment. |
| **Enterprise Migration & Bridge** | [`KEI-MIG-501.md`](KEI-MIG-501.md) | Kafka-to-Keirox migration bridge, offset sync, zero-downtime cutover. |
| **Secure Supply Chain & Release** | [`KEI-REL-501.md`](KEI-REL-501.md) | SLSA Level 3, SBOM, Sigstore signing, Distroless container images. |
| **Day-2 Observability & Console** | [`KEI-OPS-502.md`](KEI-OPS-502.md) | Grafana dashboard suite, Prometheus alerts, OTel, Web Operations Console. |
| **Phase 5 Risks & v1 GA Launch** | [`KEI-RISK-501.md`](KEI-RISK-501.md) | GA launch risks, adoption readiness, and production release gates. |

---

## 📋 Engineering Plan Registry

| Document ID | File Path | Scope & Purpose |
|---|---|---|
| **KEI-ENG-100** | [`KEI-ENG-100.md`](KEI-ENG-100.md) | **Phase 1 Master Plan**: Single-node engine roadmap, milestones (M1.0–M1.10), workstreams, and DoD. |
| **KEI-SPIKE-101** | [`KEI-SPIKE-101.md`](KEI-SPIKE-101.md) | **Minimum Vertical Prototype Plan**: 12-week execution spike for single-node core WAL, state plane, and Parquet. |
| **KEI-FORMAL-101** | [`KEI-FORMAL-101.md`](KEI-FORMAL-101.md) | **State Machine Validation Plan**: Formal TLA+ modeling of single-node state machine and test oracles. |
| **KEI-BENCH-101** | [`KEI-BENCH-101.md`](KEI-BENCH-101.md) | **Performance Validation Harness Plan**: Single-node canonical workload profiles and telemetry taxonomy. |
| **KEI-RISK-101** | [`KEI-RISK-101.md`](KEI-RISK-101.md) | **Risk Reduction and Go/No-Go Plan**: Technical risk matrix, mitigations, and pivot strategies. |
| **KEI-ENG-200** | [`KEI-ENG-200.md`](KEI-ENG-200.md) | **Phase 2 Master Plan**: 3-node cluster, Multi-Raft Quorum, S3 streaming, coordinator sharding. |
| **KEI-SPIKE-201** | [`KEI-SPIKE-201.md`](KEI-SPIKE-201.md) | **Distributed Consensus Prototype Plan**: 3-node cluster prototype, epoch fencing, S3 uploader, chaos tests. |
| **KEI-FORMAL-201** | [`KEI-FORMAL-201.md`](KEI-FORMAL-201.md) | **Distributed Consensus Verification Plan**: Formal TLA+ modeling of two-tier Raft, epoch fencing, and split-brain safety. |
| **KEI-BENCH-201** | [`KEI-BENCH-201.md`](KEI-BENCH-201.md) | **Multi-Node Failover & Benchmark Plan**: Cluster performance and failover measurement. |
| **KEI-RISK-201** | [`KEI-RISK-201.md`](KEI-RISK-201.md) | **Distributed Risk Reduction Plan**: Split-brain, network partition, and cloud storage risk management. |
| **KEI-ENG-300** | [`KEI-ENG-300.md`](KEI-ENG-300.md) | **Phase 3 Master Plan**: Wire protocol gateways (Kafka, SQS, AMQP), Iceberg ELT, native SDKs. |
| **KEI-SPIKE-301** | [`KEI-SPIKE-301.md`](KEI-SPIKE-301.md) | **Compatibility & Gateway Spike Plan**: Wire protocol translation, Arrow flight, zero-copy transcoding. |
| **KEI-COMPAT-301** | [`KEI-COMPAT-301.md`](KEI-COMPAT-301.md) | **Wire Protocol Compatibility Plan**: Kafka, SQS, AMQP conformance matrices and certification tests. |
| **KEI-LAKE-301** | [`KEI-LAKE-301.md`](KEI-LAKE-301.md) | **Lakehouse & Iceberg Integration Plan**: Iceberg REST catalog, snapshot commit, Snappy Parquet streaming. |
| **KEI-SDK-301** | [`KEI-SDK-301.md`](KEI-SDK-301.md) | **Native Client SDKs Plan**: High-performance Rust, Go, Python client SDKs. |
| **KEI-API-301** | [`KEI-API-301.md`](KEI-API-301.md) | **REST API & HTTP Gateway Plan**: REST API surface, HTTP/REST-to-gRPC transcoding, OpenAPI 3.1, probes. |
| **KEI-RISK-301** | [`KEI-RISK-301.md`](KEI-RISK-301.md) | **Ecosystem & Gateway Risk Plan**: Protocol drift, translation latency, and schema evolution risk management. |
| **KEI-ENG-400** | [`KEI-ENG-400.md`](KEI-ENG-400.md) | **Phase 4 Master Plan**: Enterprise hardening, compliance, multi-region replication, Jepsen verification. |
| **KEI-SPIKE-401** | [`KEI-SPIKE-401.md`](KEI-SPIKE-401.md) | **Enterprise Hardening Prototype Plan**: Multi-region WAN replication, crypto-shredding, timing wheels. |
| **KEI-SEC-401** | [`KEI-SEC-401.md`](KEI-SEC-401.md) | **Security & Compliance Certification Plan**: SOC 2, ISO 27001, GDPR/CCPA crypto-shredding, KMS. |
| **KEI-MR-401** | [`KEI-MR-401.md`](KEI-MR-401.md) | **Multi-Region & DR Certification Plan**: WAN replication, HLC ordering, cross-region failover. |
| **KEI-QUEUE-401** | [`KEI-QUEUE-401.md`](KEI-QUEUE-401.md) | **Advanced Queuing & Delay Plan**: Hierarchical Timing Wheels, delayed message delivery, priority queues. |
| **KEI-VAL-401** | [`KEI-VAL-401.md`](KEI-VAL-401.md) | **Adversarial & Jepsen Verification Plan**: Jepsen partition suites, adversarial pen testing, long-term soak. |
| **KEI-RISK-401** | [`KEI-RISK-401.md`](KEI-RISK-401.md) | **Enterprise Risk & GA Readiness Plan**: Compliance, multi-region WAN latency, and certification risk matrix. |
| **KEI-ENG-500** | [`KEI-ENG-500.md`](KEI-ENG-500.md) | **Phase 5 Master Plan**: Cloud-native distribution, CLI/console, migration tooling, release supply chain. |
| **KEI-K8S-501** | [`KEI-K8S-501.md`](KEI-K8S-501.md) | **Kubernetes Operator & Terraform Plan**: K8s Operator CRDs, Helm charts, Terraform provider, air-gap. |
| **KEI-MIG-501** | [`KEI-MIG-501.md`](KEI-MIG-501.md) | **Enterprise Migration & Bridge Plan**: Kafka-to-Keirox migration bridge, offset sync, zero-downtime cutover. |
| **KEI-REL-501** | [`KEI-REL-501.md`](KEI-REL-501.md) | **Secure Supply Chain & Release Plan**: SLSA Level 3, SBOM, Sigstore signing, Distroless container images. |
| **KEI-OPS-502** | [`KEI-OPS-502.md`](KEI-OPS-502.md) | **Day-2 Observability & Console Plan**: Grafana dashboard suite, Prometheus alerts, OTel, Web Operations Console. |
| **KEI-RISK-501** | [`KEI-RISK-501.md`](KEI-RISK-501.md) | **Phase 5 Risks & v1 GA Launch Plan**: GA launch risks, adoption readiness, and production release gates. |

---

## 🏛️ Relationship to Architecture Suite

All engineering plans in this directory strictly trace back to the authoritative architecture baseline in [`docs/architecture/`](../architecture/):
- **Conceptual Foundation**: [`KEI-ARC-010`](../architecture/KEI-ARC-010.md) (The Golden Invariant)
- **NFR Targets**: [`KEI-ARC-011`](../architecture/KEI-ARC-011.md) (PERF, DUR, SCALE, MEM)
- **Binding Decisions**: [`KEI-ARC-012`](../architecture/KEI-ARC-012.md) (ADR Index)
- **Detailed Specifications**: [`KEI-DES-030`](../architecture/KEI-DES-030.md) (WAL Framing), [`KEI-DES-031`](../architecture/KEI-DES-031.md) (State Plane Algorithms)
- **Validation Mapping**: [`KEI-OPS-041`](../architecture/KEI-OPS-041.md) (Test & Chaos Plan), [`KEI-VAL-051`](../architecture/KEI-VAL-051.md) (Requirements Traceability Matrix)
