# Keirox Documentation Suite

This directory is the comprehensive documentation repository for the **Keirox Polymorphic Event Fabric (PEF)**, containing canonical architecture specifications, engineering execution plans, verification protocols, performance benchmarks, and certification reports.

---

## ⚡ Navigation & Fast Routing

| Documentation Area | Directory | Scope & Purpose |
|---|---|---|
| **Architecture Specifications** | [`architecture/`](architecture/INDEX.md) | Authoritative L0–L3 architecture suite (25 formal specifications, 38 ADRs, Requirements Traceability Matrix `KEI-VAL-051`, Release Readiness `KEI-VAL-052`). |
| **Engineering Execution Plans** | [`engineering/`](engineering/README.md) | Phase 1 to Phase 5 master roadmaps, technical spikes, formal TLA+ models, and risk reduction plans (`KEI-ENG-100` through `KEI-RISK-501`). |
| **Verification & Forensic Protocols** | [`verification/`](verification/README.md) | Implementation Verification Protocol (`KEI-VER-001.md`, 200+ checks across 15 domains) and Live Enterprise Demonstration Report (`KEI-DEMO-700.md`). |
| **Engineering Certification Reports** | [`reports/`](reports/README.md) | Formal automated evidence gate certification reports for Phases 1 through 5 (`KEI-CERT-100` through `KEI-CERT-500`). |
| **Benchmarks & Performance** | [`benchmarks/`](benchmarks/README.md) | Canonical performance workload profiles (P1 through P6), benchmark harness methodology, and telemetry taxonomy. |

---

## 🏛️ Architectural Authority

Per `AGENTS.md` and `KEI-INDEX`:
1. The formal specifications in `docs/architecture/` (`KEI-INDEX` through `KEI-VAL-052`) are the **sole absolute authority** for all system contracts, invariants, binary layouts, protocols, and algorithms.
2. All implementation code across the 18 workspace crates strictly adheres to **The Golden Invariant**:
   - Data is written exactly once to an immutable physical log.
   - Consumption semantics (streaming, queuing, dead-lettering, lakehouse analytics) are defined entirely by the consumer's mutable, replicated state overlay.
3. Every requirement is traceable end-to-end via the [Requirements Traceability Matrix](architecture/KEI-VAL-051.md).
