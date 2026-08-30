# Keirox Phase 1 Engineering Execution Plans

This directory contains the formal engineering execution plans, technical spikes, formal methods validation, benchmark specifications, organizational delivery frameworks, and risk reduction gates for **Phase 1** of the Keirox Polymorphic Event Fabric.

---

## ⚡ Fast Task Routing Map (Token-Optimized)

When working on engineering tasks, agents MUST consult this table to ingest **only the single relevant plan**:

| Active Engineering Task | Primary Plan Document | Key Deliverables & Evidence |
|---|---|---|
| **Phase 1 Roadmap & Milestones** | [`KEI-ENG-100.md`](KEI-ENG-100.md) | Milestones M1.0–M1.10, workstreams WS-0..WS-5, DoD. |
| **Minimum Vertical Prototype (Spike)** | [`KEI-SPIKE-001.md`](KEI-SPIKE-001.md) | 12-week spike: Single-node WAL, Roaring Bitmaps, leases, ACKs, DLQ, Parquet. |
| **Formal State Machine Validation** | [`KEI-FORMAL-001.md`](KEI-FORMAL-001.md) | 5 TLA+ models: Lease lifecycle, watermark monotonicity, DLQ progress, test oracles. |
| **Benchmark Harness & Telemetry** | [`KEI-BENCH-001.md`](KEI-BENCH-001.md) | Profiles P1-Proto..P6-Proto, HDR histograms, environmental disclosure. |
| **Team Topology & ARB Governance** | [`KEI-ORG-001.md`](KEI-ORG-001.md) | Decision matrix, ARB charter, sprint structure, hardware environments. |
| **Risk Management & Pivot Triggers** | [`KEI-RISK-001.md`](KEI-RISK-001.md) | 5x5 technical risk matrix, mitigations, Go/No-Go gates, pre-authorized pivots. |

---

## 📋 Engineering Plan Registry

| Document ID | File Path | Scope & Purpose |
|---|---|---|
| **KEI-ENG-100** | [`KEI-ENG-100.md`](KEI-ENG-100.md) | **Phase 1 Master Engineering Execution Plan**: Master roadmap, milestones (M1.0–M1.10), workstreams, acceptance criteria, and DoD. |
| **KEI-SPIKE-001** | [`KEI-SPIKE-001.md`](KEI-SPIKE-001.md) | **Minimum Vertical Prototype Plan**: 12-week execution spike for single-node core WAL, Roaring Bitmaps, leases, ACKs, watermarks, DLQ, and Parquet export. |
| **KEI-FORMAL-001** | [`KEI-FORMAL-001.md`](KEI-FORMAL-001.md) | **State Machine Validation Plan**: Formal TLA+ modeling of state machine, watermark monotonicity, lease uniqueness, and test oracle derivation. |
| **KEI-BENCH-001** | [`KEI-BENCH-001.md`](KEI-BENCH-001.md) | **Performance Validation Harness Plan**: Canonical workload profiles (P1-Proto..P6-Proto), telemetry taxonomy, and environmental disclosure rules. |
| **KEI-ORG-001** | [`KEI-ORG-001.md`](KEI-ORG-001.md) | **Team, Governance, and Delivery Plan**: Team topology, decision matrix, Architecture Review Board (ARB), resource planning, and quality gates. |
| **KEI-RISK-001** | [`KEI-RISK-001.md`](KEI-RISK-001.md) | **Risk Reduction and Go/No-Go Plan**: Risk scoring matrix, technical risk mitigations (io_uring, memory fragmentation, compaction jitter), and authorized pivot strategies. |

---

## 🏛️ Relationship to Architecture Suite

All engineering plans in this directory strictly trace back to the authoritative architecture baseline in [`docs/architecture/`](../architecture/):
- **Conceptual Foundation**: [`KEI-ARC-010`](../architecture/KEI-ARC-010.md) (The Golden Invariant)
- **NFR Targets**: [`KEI-ARC-011`](../architecture/KEI-ARC-011.md) (PERF, DUR, SCALE, MEM)
- **Binding Decisions**: [`KEI-ARC-012`](../architecture/KEI-ARC-012.md) (ADR Index)
- **Detailed Specifications**: [`KEI-DES-030`](../architecture/KEI-DES-030.md) (WAL Framing), [`KEI-DES-031`](../architecture/KEI-DES-031.md) (State Plane Algorithms)
- **Validation Mapping**: [`KEI-OPS-041`](../architecture/KEI-OPS-041.md) (Test & Chaos Plan), [`KEI-VAL-051`](../architecture/KEI-VAL-051.md) (Requirements Traceability Matrix)
