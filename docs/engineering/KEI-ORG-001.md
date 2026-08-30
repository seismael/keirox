# KEI-ORG-001 — Team, Governance, and Delivery Plan

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-ORG-001 |
| Title | Team, Governance, and Delivery Plan |
| Version | 1.0 |
| Level | Engineering Execution Plan |
| Status | Baseline — Ready for Execution |
| Phase | Phase 1 Engineering Bridge + Full Phase 1 |
| Owner | VP Engineering / Chief Architect |
| Governing Plan | KEI-ENG-100 — Phase 1 Engineering Execution Plan |
| Related Plans | KEI-SPIKE-001, KEI-FORMAL-001, KEI-BENCH-001, KEI-RISK-001 |
| Governing Architecture Documents | KEI-ARC-001..027, KEI-DES-030..036, KEI-OPS-040..041, KEI-VAL-050..052 |
| Next Plan File | KEI-RISK-001 — Risk Reduction and Go/No-Go Plan |

---

## 2. Executive Summary

This document defines the team topology, engineering governance structure, delivery cadence, and operational processes required to execute Phase 1 of the Keirox Polymorphic Event Fabric successfully.

A world-class architecture without a world-class delivery organization will fail. This plan ensures that:

1. The right roles are staffed with the right expertise.
2. Engineering decisions are made through a clear governance process.
3. Delivery is tracked against evidence-based milestones.
4. Architecture compliance is enforced in every pull request.
5. Risks are surfaced early and escalated through defined channels.

---

## 3. Team Topology

### 3.1 Phase 1 Core Team (Minimum Viable)

| Role | Count | Seniority | Primary Responsibility |
|---|---:|---|---|
| Chief Architect | 1 | Principal+ | Architecture governance, ADR approval, conflict resolution. |
| Engineering Program Lead | 1 | Staff+ | Milestone tracking, cross-team coordination, risk escalation. |
| Storage Engine Lead | 1 | Senior+ | WAL, segments, CRC, recovery, io_uring. |
| State Plane Lead | 1 | Senior+ | Bitmaps, leases, timers, watermark, DLQ. |
| Data Platform Engineer | 1 | Senior+ | Arrow, Parquet, schema handling, export. |
| SRE / QA Lead | 1 | Senior+ | Benchmarks, chaos tests, soak tests, CI/CD. |
| Systems Engineer (Rust) | 1–2 | Mid–Senior | Implementation support across workstreams. |

**Minimum viable team size: 6 engineers.**  
**Recommended Phase 1 team size: 8–9 engineers.**

### 3.2 Extended Support (Part-Time / Advisory)

| Role | Allocation | Responsibility |
|---|---|---|
| Security Advisor | 10% | Review security boundaries; plan for Phase 4. |
| Formal Methods Advisor | 20% (Weeks 1–12) | TLA+ modeling per KEI-FORMAL-001. |
| Product / GTM Advisor | 10% | Validate prototype against market needs. |
| FinOps Advisor | 5% | Validate TCO model assumptions. |

### 3.3 Scaling Plan

| Phase | Team Size | Key Additions |
|---|---:|---|
| Phase 1 (Months 1–9) | 6–9 | Core systems engineers. |
| Phase 2 (Months 10–18) | 10–14 | Distributed systems engineers, consensus specialist. |
| Phase 3 (Months 19–27) | 14–18 | Gateway engineers, SDK engineers, lakehouse specialist. |
| Phase 4 (Months 28–36) | 18–24 | Security engineers, compliance specialist, multi-region specialist. |

---

## 4. Engineering Governance Structure

### 4.1 Decision Authority Matrix

| Decision Type | Authority | Escalation Path |
|---|---|---|
| Code implementation details | Workstream Lead | Engineering Program Lead |
| Crate/module boundary changes | Chief Architect | Architecture Review Board |
| New architectural decision (ADR) | Chief Architect + Architecture Review Board | CTO |
| Scope addition | Engineering Program Lead + Chief Architect | VP Engineering |
| Performance target change | Chief Architect + SRE Lead | Architecture Review Board |
| Security policy change | Security Advisor + Chief Architect | CTO + Compliance |
| Milestone delay >2 weeks | Engineering Program Lead | VP Engineering |
| Go/No-Go gate decision | Architecture Review Board | CTO + VP Engineering |

### 4.2 Architecture Review Board (ARB)

**Composition:**

- Chief Architect (Chair)
- Storage Engine Lead
- State Plane Lead
- Data Platform Engineer
- SRE / QA Lead
- Security Advisor (as needed)

**Cadence:** Weekly (60 minutes).

**Responsibilities:**

1. Review and approve/reject ADR candidates.
2. Resolve cross-workstream architectural conflicts.
3. Review benchmark evidence against NFR targets.
4. Approve milestone exit criteria.
5. Review and update the Requirements Traceability Matrix.

**Quorum:** 4 of 6 members.

**Decision rule:** Consensus preferred. If consensus fails, Chief Architect makes final call with documented dissent.

### 4.3 Engineering Standup

**Cadence:** Daily (15 minutes).

**Format:**

1. What did you complete since last standup?
2. What are you working on next?
3. Are there any blockers?

**Rule:** Standup is for coordination, not problem-solving. Technical discussions move to dedicated sessions.

### 4.4 Benchmark Review

**Cadence:** Weekly (30 minutes).

**Purpose:** Review latest benchmark results, detect regressions, and plan performance work.

**Inputs:**

- Latest benchmark report from `keirox-bench`.
- Memory profile report.
- Latency histogram comparison.
- Error rate summary.

### 4.5 Risk Review

**Cadence:** Biweekly (30 minutes).

**Purpose:** Review the risk register, update severity/likelihood, and assign mitigation owners.

**Inputs:**

- KEI-RISK-001 risk register.
- Open defects and their severity.
- Milestone progress tracking.

---

## 5. Delivery Cadence and Milestones

### 5.1 Sprint Structure

| Cadence | Duration | Purpose |
|---|---|---|
| Sprint | 2 weeks | Implementation work. |
| Milestone Review | End of each milestone | Formal acceptance. |
| Phase Gate Review | End of Phase 1 | Go/No-Go decision. |

### 5.2 Phase 1 Milestone Schedule

| Milestone | Target Weeks | Sprint Alignment |
|---|---|---|
| M1.0 Engineering Mobilization | 1–2 | Sprint 1 |
| M1.1 WAL Foundation | 3–6 | Sprint 2–3 |
| M1.2 State Plane Foundation | 5–8 | Sprint 3–4 |
| M1.3 Recovery and Invariants | 7–10 | Sprint 4–5 |
| M1.4 Prototype Evidence Gate | 11–12 | Sprint 6 |
| M1.5 Storage Hardening | 13–18 | Sprint 7–9 |
| M1.6 State Plane Hardening | 15–22 | Sprint 8–11 |
| M1.7 Columnar Export | 17–26 | Sprint 9–13 |
| M1.8 Scale and Soak | 23–30 | Sprint 12–15 |
| M1.9 Operational Readiness | 27–32 | Sprint 14–16 |
| M1.10 Phase 1 Certification | 33–36 | Sprint 17–18 |

### 5.3 Definition of Ready

A task is ready for sprint planning when:

1. It has a clear description and acceptance criteria.
2. It references the governing KEI document and section.
3. It has an assigned workstream.
4. Dependencies are identified and unblocked.
5. It has been estimated by the assigned engineer.

### 5.4 Definition of Done

A task is done when:

1. Code is implemented and passes all tests.
2. PR traceability annotations are complete.
3. Code review is approved by at least one other engineer.
4. CI passes.
5. No new invariant violations are introduced.
6. Documentation is updated if the task changes behavior.
7. Benchmark evidence is produced if the task affects performance.

---

## 6. Engineering Process and Standards

### 6.1 Branching Strategy

| Branch | Purpose | Merge Target |
|---|---|---|
| `main` | Production-ready code. | Release. |
| `develop` | Integration branch. | `main` at milestone exit. |
| `feature/*` | Feature development. | `develop` via PR. |
| `fix/*` | Bug fixes. | `develop` via PR. |
| `release/*` | Release candidates. | `main`. |

### 6.2 Pull Request Requirements

Every PR MUST include:

1. Clear title and description.
2. Link to the task/issue.
3. Architecture traceability annotations (see KEI-ENG-100 §13).
4. Test results.
5. Benchmark results if performance-sensitive.
6. At least one approving review.
7. All CI checks passing.

### 6.3 Code Review Standards

| Review Type | Required For | Reviewer |
|---|---|---|
| Peer Review | All PRs | Any engineer in the workstream. |
| Architecture Review | PRs affecting crate boundaries, data structures, or invariants | Chief Architect or designated reviewer. |
| Security Review | PRs touching auth, encryption, or secrets | Security Advisor. |
| Performance Review | PRs affecting hot paths | SRE Lead or Storage Lead. |

### 6.4 Commit Message Convention

```text
<type>(<scope>): <summary>

<body>

Architecture-Documents: KEI-ARC-xxx §section
Requirement-IDs: REQ-xxx-nnn
ADRs: ADR-xxx
Tests: test-name
```

Types: `feat`, `fix`, `perf`, `refactor`, `test`, `docs`, `chore`.

---

## 7. Communication Plan

### 7.1 Internal Communication

| Channel | Purpose | Cadence |
|---|---|---|
| Standup | Daily coordination | Daily |
| Architecture Review | Technical decisions | Weekly |
| Benchmark Review | Performance evidence | Weekly |
| Risk Review | Risk management | Biweekly |
| Milestone Review | Formal acceptance | Per milestone |
| Async (Slack/Teams) | Ad-hoc coordination | Continuous |

### 7.2 External Communication

| Audience | Content | Cadence |
|---|---|---|
| Executive Leadership | Phase progress, technical milestones, risk posture | Monthly |
| Product / GTM | Prototype readiness, feature timeline | Biweekly |
| Security / Compliance | Security posture, compliance readiness | Per milestone |
| Engineering Blog (optional) | Technical deep-dives | Quarterly |

### 7.3 Escalation Path

```text
Engineer
   ↓ (blocker >1 day)
Workstream Lead
   ↓ (cross-workstream conflict)
Engineering Program Lead
   ↓ (architecture conflict)
Chief Architect / ARB
   ↓ (scope/resource/timeline risk)
VP Engineering / CTO
```

---

## 8. Tooling and Infrastructure

### 8.1 Development Environment

| Tool | Purpose |
|---|---|
| Rust stable toolchain | Primary language. |
| cargo | Build and dependency management. |
| rustfmt | Code formatting. |
| clippy | Linting. |
| cargo-test | Unit and integration testing. |
| criterion | Benchmarking. |
| proptest | Property-based testing. |
| Docker | Containerized development. |

### 8.2 CI/CD Pipeline

| Stage | Purpose |
|---|---|
| Build | Compile all crates. |
| Format Check | Verify rustfmt compliance. |
| Lint | Run clippy with warnings-as-errors. |
| Unit Tests | Run all unit tests. |
| Integration Tests | Run integration test suite. |
| Benchmark Smoke | Run quick benchmark to detect regressions. |
| Security Scan | Run cargo-audit. |
| Documentation Build | Build rustdoc. |

### 8.3 Issue Tracking

Recommended: GitHub Issues, Linear, or Jira.

Required fields for every issue:

- Title.
- Description.
- Workstream.
- Governing KEI document reference.
- Priority (P0–P3).
- Assigned engineer.
- Milestone target.

### 8.4 Documentation Management

- All architecture documents stored in `docs/architecture/`.
- All engineering plans stored in `docs/engineering/`.
- All benchmark reports stored in `docs/benchmarks/`.
- All ADRs stored in `docs/adr/`.
- All meeting notes stored in `docs/meetings/`.

---

## 9. Onboarding Plan

### 9.1 New Engineer Onboarding (Week 1)

| Day | Activity |
|---|---|
| Day 1 | Repository access, environment setup, read KEI-ARC-001 and KEI-ARC-010. |
| Day 2 | Read KEI-ARC-020 (Storage) and KEI-DES-030 (WAL format). |
| Day 3 | Read KEI-ARC-021 (State Plane) and KEI-DES-031 (State structures). |
| Day 4 | Pair with workstream lead on a small task. |
| Day 5 | Complete first PR with full traceability annotations. |

### 9.2 Architecture Deep Dive (Week 2)

| Day | Activity |
|---|---|
| Day 6–7 | Read all L2 architecture documents. |
| Day 8 | Architecture Review Board shadow session. |
| Day 9 | Run benchmark suite locally. |
| Day 10 | Present understanding of Golden Invariant to team. |

---

## 10. Quality Gates and Release Process

### 10.1 Milestone Exit Criteria

Each milestone exit requires:

1. All deliverables completed.
2. All tests passing.
3. Benchmark evidence produced.
4. Architecture Review Board approval.
5. Updated RTM.
6. No blocking defects.

### 10.2 Phase 1 Exit Criteria

Phase 1 exit requires:

1. All Phase 1 acceptance criteria met (see KEI-ENG-100 §10).
2. Phase 1 evidence package complete.
3. Architecture Review Board certification.
4. Go/No-Go recommendation delivered.
5. Phase 2 entry plan approved.

### 10.3 Release Process

| Stage | Gate |
|---|---|
| Development | All tests pass, PR approved. |
| Integration | Merged to `develop`, CI passes. |
| Release Candidate | Merged to `release/*`, full test suite passes. |
| Production | Merged to `main`, release notes published. |

---

## 11. Resource & Infrastructure Planning

### 11.1 Infrastructure & Environment Requirements

| Environment | Hardware Profile | Purpose |
|---|---|---|
| Development workstation | 16+ cores, 64GB RAM, NVMe SSD | Daily engineering and unit testing. |
| Benchmark server | 32+ cores, 128GB RAM, Enterprise NVMe (io_uring + O_DIRECT) | Official baseline benchmarks and soak testing. |
| CI runner | 8+ cores, 32GB RAM | Automated linting, build, and test validation. |

### 11.2 Tooling & Software Stack

| Category | Tooling Specification | Purpose |
|---|---|---|
| Language & Compiler | Rust 1.78+ (Stable / Nightly for SIMD intrinsics) | Core runtime implementation. |
| CI / Automation | GitHub Actions runner fleet with ASan/TSan support | Automated quality gates. |
| Profiling & Tracing | `flamegraph`, `perf`, Prometheus, Grafana | Performance observability. |
| Formal Methods | TLC Model Checker, TLA+ Tools | Formal state machine validation. |

---

## 12. Risks and Mitigations

| Risk | Severity | Likelihood | Mitigation | Owner |
|---|---|---|---|---|
| Key engineer departure | High | Medium | Document all knowledge; cross-train; robust onboarding. | VP Engineering |
| Scope creep | High | High | Strict change control; ARB approval required. | Engineering Program Lead |
| Benchmark environment inconsistency | Medium | Medium | Dedicated benchmark server; full environment disclosure. | SRE Lead |
| Architecture disputes slow delivery | Medium | Medium | Clear decision authority matrix; ARB cadence. | Chief Architect |
| Rust expertise gap | Medium | Medium | Hiring plan; internal training; pair programming. | Engineering Program Lead |
| Burnout from aggressive timeline | High | Medium | Sustainable pace; no mandatory overtime; regular check-ins. | VP Engineering |

---

## 13. Success Metrics

| Metric | Target | Measurement |
|---|---|---|
| Milestone completion rate | ≥90% on time | Milestone tracking. |
| PR merge time | <24 hours for non-architecture PRs | Git analytics. |
| Test coverage | ≥80% for core crates | CI coverage reports. |
| Benchmark regression rate | <5% per sprint | Benchmark dashboard. |
| Defect escape rate | <2 defects per milestone | Issue tracker. |
| Team satisfaction | ≥4/5 | Quarterly survey. |

---

## 14. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial Team, Governance, and Delivery Plan. Defines team topology, governance structure, delivery cadence, engineering process, communication plan, tooling, onboarding, quality gates, resource and infrastructure planning, risks, and success metrics. |