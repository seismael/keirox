# KEI-ENG-500 — Phase 5 Productization & Distribution Plan
## Cloud-Native Deployment, Migration Tooling, Supply Chain & Day-2 Operations

---

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-ENG-500 |
| Title | Phase 5 Productization & Distribution Plan |
| Version | 1.0 |
| Level | Engineering Execution Plan |
| Status | Baseline — Ready for Execution |
| Phase | Phase 5 — Productization, Distribution & Day-2 Operations |
| Duration | Months 37–42 (6 months) |
| Owner | VP Engineering / Chief Architect / Head of Platform Engineering |
| Governing Architecture | KEI-ARC-020..027, KEI-DES-030..036, KEI-OPS-040..041 |
| Predecessor | KEI-ENG-400 (Phase 4 Engineering Execution Plan) |
| Relationship | Phase 5 may overlap with Phase 4 Weeks 24–36 |

---

## 2. Executive Summary

Phases 1 through 4 built a world-class distributed systems engine: correct, durable, secure, and adversarially validated. But an engine without deployment tooling, migration paths, observability dashboards, and a secure release pipeline is a research artifact — not a product.

Phase 5 answers the **adoption question**:

> Can an enterprise platform team discover Keirox, evaluate it in a sandbox, migrate their existing Kafka workloads to it, deploy it on their Kubernetes infrastructure, monitor it with their existing Grafana/Datadog stack, manage it with a CLI and Web Console, and trust the binary supply chain — all without requiring a custom integration project?

Phase 5 delivers:

1. **Cloud-Native Distribution** — Kubernetes Operator, Helm Charts, Terraform Provider.
2. **CLI & Web Console** — `keirox-cli`, Admin API, visual operations dashboard.
3. **Migration Tooling** — Kafka-to-Keirox bridge, offset sync, schema migration.
4. **Secure Supply Chain** — SLSA Level 3, SBOM, Sigstore signing, Distroless images.
5. **Day-2 Observability** — Grafana dashboards, Prometheus rules, OTel auto-instrumentation.
6. **Optional: Managed Cloud Control Plane** — Multi-tenant provisioning, metering, billing.

Phase 5 is the bridge between "engineering complete" and "commercially shippable."

---

## 3. Phase 5 Mission

The mission of Phase 5 is:

1. Make Keirox deployable on any enterprise Kubernetes cluster in under 30 minutes.
2. Make Keirox manageable via CLI and Web Console without requiring gRPC expertise.
3. Make migration from Apache Kafka safe, incremental, and reversible.
4. Make the binary supply chain transparent, signed, and auditable.
5. Make Day-2 operations (debugging, monitoring, scaling) native to the platform.
6. Prepare Keirox for General Availability (GA) launch.

---

## 4. Phase 5 Scope

### 4.1 In Scope

| Workstream | Scope |
|---|---|
| Cloud-Native Distribution | K8s Operator, CRDs, Helm charts, Terraform provider, cert-manager integration |
| CLI & Admin Console | `keirox-cli`, Admin gRPC API, Web-based operations UI |
| Migration Tooling | Kafka-to-Keirox bridge, offset sync, schema registry migration, cutover playbooks |
| Secure Supply Chain | Cross-compilation, container images, SBOM, binary signing, SLSA provenance |
| Day-2 Observability | Grafana dashboards, Prometheus rules, OTel auto-instrumentation, alert templates |
| Release Automation | CI/CD pipelines, release notes, changelog generation, artifact publishing |
| Documentation Portal | Developer docs site, API reference, interactive tutorials |

### 4.2 Optional (Business-Model Dependent)

| Workstream | Scope |
|---|---|
| Managed Cloud Control Plane | Multi-tenant provisioning, metering, billing, centralized audit (if offering SaaS) |

### 4.3 Out of Scope

| Item | Reason |
|---|---|
| New core engine features | Engine is frozen at Phase 4 |
| New protocol gateways | Kafka/SQS/AMQP are complete; no new protocols in v1 |
| Jepsen testing | Complete in Phase 4 |
| Pen testing | Complete in Phase 4 |
| In-broker SQL | Excluded from v1 |

### 4.4 Phase 5 Constraints

1. Phase 5 MUST NOT modify the core engine binary format or consensus protocol.
2. All tooling MUST be open-source compatible (Apache 2.0 or BSL where appropriate).
3. The Kubernetes Operator MUST support air-gapped deployments.
4. Migration tooling MUST support zero-downtime cutover.
5. Release artifacts MUST be reproducible and verifiable.
6. Web Console MUST be read-only by default; write operations require explicit opt-in.

---

## 5. Work Package Definitions

### 5.1 WP-P5-A: Cloud-Native Distribution

**Objective:** Make Keirox deployable on any Kubernetes cluster via Helm or Terraform.

**Deliverables:**

| ID | Deliverable | Description |
|---|---|---|
| D-P5-A-001 | Kubernetes Operator | Custom Resource Definitions (CRDs) for KeiroxCluster, KeiroxStream, KeiroxConsumerGroup |
| D-P5-A-002 | Operator reconciliation logic | Handle node scaling, PVC provisioning, cert rotation, graceful pod disruption |
| D-P5-A-003 | Helm Chart | Production-grade Helm chart with values.yaml, schema validation, NOTES.txt |
| D-P5-A-004 | Terraform Provider | `keirox_cluster`, `keirox_stream`, `keirox_consumer_group` resources for AWS/GCP/Azure |
| D-P5-A-005 | cert-manager integration | Automated mTLS certificate issuance and rotation |
| D-P5-A-006 | Air-gapped deployment support | Offline Helm chart bundles, private registry mirroring |
| D-P5-A-007 | Pod Disruption Budgets | Ensure Raft quorum safety during node drains |
| D-P5-A-008 | Storage class abstraction | Support for local NVMe, EBS, GCE PD, Azure Disk |

**Key Design Decisions:**

| Decision | Choice | Rationale |
|---|---|---|
| Operator framework | kube-rs (Rust) or kubebuilder (Go) | Rust preferred for consistency; Go acceptable for operator ecosystem |
| CRD granularity | Cluster-level CRD + per-stream CRD | Balance between simplicity and flexibility |
| Helm chart structure | Single chart with subcharts | Simplicity for day-0; subcharts for optional components |
| Terraform provider | Custom provider using Terraform Plugin Framework | Native HCL experience for IaC teams |

---

### 5.2 WP-P5-B: CLI & Admin Console

**Objective:** Make Keirox manageable without gRPC expertise.

**Deliverables:**

| ID | Deliverable | Description |
|---|---|---|
| D-P5-B-001 | `keirox-cli` binary | Cross-platform CLI for cluster management, stream operations, DLQ management |
| D-P5-B-002 | CLI command structure | `cluster`, `stream`, `group`, `dlq`, `schema`, `migration`, `admin` |
| D-P5-B-003 | Admin gRPC API | Internal API for CLI and Web Console to query/manage cluster state |
| D-P5-B-004 | Web Console UI | React/TypeScript dashboard for stream inspection, DLQ viewing, PITR triggers |
| D-P5-B-005 | Web Console read-only mode | Default mode shows state without mutation capability |
| D-P5-B-006 | Break-glass UI workflow | Explicit confirmation + audit trail for destructive operations |
| D-P5-B-007 | CLI output formats | JSON, YAML, table, and wide output modes |
| D-P5-B-008 | Shell completions | Bash, Zsh, Fish, PowerShell completions |

**CLI Command Structure:**

```text
keirox cluster init|status|scale|drain|replace-node
keirox stream create|list|describe|delete|read
keirox group create|list|describe|offsets|commit
keirox dlq list|inspect|redrive|purge
keirox schema register|list|describe|diff|evolve
keirox migration kafka-init|kafka-sync|kafka-cutover|kafka-rollback
keirox admin backup|restore|pitr|failover|erasure|legal-hold
keirox config set|get|list
```

---

### 5.3 WP-P5-C: Migration Tooling

**Objective:** Enable zero-downtime migration from Apache Kafka to Keirox.

**Deliverables:**

| ID | Deliverable | Description |
|---|---|---|
| D-P5-C-001 | Kafka-to-Keirox Bridge | Reads from Kafka topics, writes to Keirox streams with offset tracking |
| D-P5-C-002 | Offset synchronization | Maintains consumer group offset parity between Kafka and Keirox |
| D-P5-C-003 | Schema Registry migration | Imports Confluent/Apicurio schemas into Keirox registry |
| D-P5-C-004 | Dual-write proxy mode | Optional proxy that writes to both Kafka and Keirox during validation |
| D-P5-C-005 | Consumer cutover playbook | Step-by-step runbook for switching consumers from Kafka to Keirox |
| D-P5-C-006 | Rollback playbook | Step-by-step runbook for reverting to Kafka if issues arise |
| D-P5-C-007 | Migration validation suite | Automated comparison of Kafka and Keirox data for consistency |

**Migration Strategy:**

```text
Phase A: Bridge Deployment
   └── Kafka → Keirox bridge starts consuming and writing
   └── Consumers continue reading from Kafka

Phase B: Validation
   └── Dual-read validation: compare Kafka and Keirox offsets
   └── Schema compatibility verification
   └── Performance benchmarking under production load

Phase C: Consumer Cutover
   └── Switch consumers from Kafka to Keirox gateway
   └── Monitor for errors, latency, data loss
   └── Kafka remains as fallback

Phase D: Decommission
   └── Remove Kafka bridge
   └── Decommission Kafka cluster
   └── Archive Kafka data
```

---

### 5.4 WP-P5-D: Secure Supply Chain & Release Engineering

**Objective:** Produce verifiable, signed, reproducible release artifacts.

**Deliverables:**

| ID | Deliverable | Description |
|---|---|---|
| D-P5-D-001 | Cross-compilation pipeline | Build for Linux x86_64, ARM64; macOS x86_64, ARM64 |
| D-P5-D-002 | Container image pipeline | Distroless/Chainguard base images; multi-stage builds |
| D-P5-D-003 | SBOM generation | CycloneDX/SPDX for every binary and image |
| D-P5-D-004 | Binary signing | Sigstore/Cosign signing for all artifacts |
| D-P5-D-005 | SLSA Level 3 provenance | Verifiable build provenance attestation |
| D-P5-D-006 | Release automation | Automated changelog, release notes, GitHub/GitLab releases |
| D-P5-D-007 | Artifact publishing | Publish to Docker Hub, GitHub Container Registry, Helm repo |
| D-P5-D-008 | Reproducible builds | Deterministic builds from source to binary |
| D-P5-D-009 | Version management | Semantic versioning, release branching, hotfix workflows |

**Build Targets:**

| Target | OS | Architecture | Use Case |
|---|---|---|---|
| `keirox-server` | Linux | x86_64, ARM64 | Production server binary |
| `keirox-cli` | Linux, macOS, Windows | x86_64, ARM64 | CLI tool |
| `keirox-operator` | Linux | x86_64, ARM64 | Kubernetes operator |
| `keirox-gateway` | Linux | x86_64, ARM64 | Protocol gateway sidecar |
| `keirox/keirox` | Container | x86_64, ARM64 | All-in-one container image |
| `keirox/operator` | Container | x86_64, ARM64 | Operator container image |

---

### 5.5 WP-P5-E: Day-2 Observability Packaging

**Objective:** Make Keirox observable in enterprise monitoring stacks out-of-the-box.

**Deliverables:**

| ID | Deliverable | Description |
|---|---|---|
| D-P5-E-001 | Grafana dashboard suite | Pre-built dashboards for cluster health, streams, state plane, lakehouse |
| D-P5-E-002 | Prometheus recording rules | Pre-computed SLO metrics, alert thresholds |
| D-P5-E-003 | Prometheus alert rules | Critical/Warning alerts mapped to runbooks |
| D-P5-E-004 | OpenTelemetry auto-instrumentation | SDK-level trace propagation, span generation |
| D-P5-E-005 | Datadog integration | Native Datadog check, dashboard JSON |
| D-P5-E-006 | New Relic integration | NRQL queries, dashboard templates |
| D-P5-E-007 | Loki log pipeline | Structured log format, log-to-trace correlation |
| D-P5-E-008 | SLO definitions | Error budget tracking, burn rate alerts |

**Grafana Dashboard Suite:**

| Dashboard | Panels |
|---|---|
| Cluster Overview | Node health, Raft quorum, replication lag, leader status |
| Stream Throughput | Ingest rate, read rate, backlog per stream |
| State Plane | Active leases, watermark lag, bitmap memory, DLQ count |
| Lakehouse | Iceberg commit latency, freshness, file count, manifest health |
| Gateway | Request rate by API/version, error rate, translation latency |
| Security | Auth failures, ABAC denials, KMS errors, crypto-shred events |
| Capacity | NVMe usage, S3 backlog, memory pressure, CPU utilization |

---

### 5.6 WP-P5-F: Documentation & Developer Portal

**Objective:** Provide world-class developer documentation.

**Deliverables:**

| ID | Deliverable | Description |
|---|---|---|
| D-P5-F-001 | Documentation site | Docusaurus or MkDocs-based documentation portal |
| D-P5-F-002 | Getting Started guide | 5-minute quickstart for local development |
| D-P5-F-003 | Architecture overview | Public-facing architecture explanation |
| D-P5-F-004 | API reference | Auto-generated from protobuf/OpenAPI definitions |
| D-P5-F-005 | SDK guides | Language-specific guides for Rust, Go, Python |
| D-P5-F-006 | Migration guide | Step-by-step Kafka-to-Keirox migration guide |
| D-P5-F-007 | Operations guide | Day-2 operations, troubleshooting, scaling |
| D-P5-F-008 | Interactive tutorials | Browser-based or CLI-based walkthroughs |
| D-P5-F-009 | Compatibility matrices | Public Kafka/SQS/AMQP compatibility documentation |

---

## 6. Phase 5 Milestone Schedule

Phase 5 runs for 6 months (24 weeks), with potential overlap starting at Phase 4 Week 24.

| Milestone | Target Weeks | Deliverables | Exit Criteria |
|---|---|---|---|
| M5.0 Mobilization | 1–2 | Team onboarding, toolchain setup, K8s test cluster | Phase 5 environment ready |
| M5.1 Kubernetes Operator Alpha | 3–8 | CRDs, operator reconciliation, Helm chart | Deploy 3-node cluster via Helm |
| M5.2 CLI & Admin API | 6–10 | `keirox-cli`, Admin gRPC API | CLI passes integration tests |
| M5.3 Migration Bridge | 8–14 | Kafka bridge, offset sync, schema migration | Bridge syncs 100K messages without loss |
| M5.4 Supply Chain Pipeline | 10–14 | Cross-compile, SBOM, signing, images | All artifacts signed and verifiable |
| M5.5 Observability Suite | 12–16 | Grafana, Prometheus, OTel, Datadog | Dashboards deployed in staging |
| M5.6 Web Console | 14–20 | React UI, read-only mode, break-glass | Console operational in staging |
| M5.7 Terraform Provider | 14–20 | AWS/GCP/Azure providers | Terraform apply creates working cluster |
| M5.8 Migration Cutover Validation | 18–22 | End-to-end cutover test, rollback test | Zero-downtime cutover demonstrated |
| M5.9 Documentation & Portal | 18–24 | Docs site, API reference, tutorials | Public documentation review complete |
| M5.10 GA Readiness | 23–24 | Final validation, release candidate | v1.0.0-rc1 tagged and signed |

---

## 7. Phase 5 Acceptance Criteria

### 7.1 Deployment Acceptance

| ID | Requirement |
|---|---|
| ACC-P5-DEP-001 | 3-node Keirox cluster deployable via Helm in <10 minutes |
| ACC-P5-DEP-002 | Cluster deployable via Terraform on AWS, GCP, and Azure |
| ACC-P5-DEP-003 | Operator handles node scaling without manual intervention |
| ACC-P5-DEP-004 | Air-gapped deployment works without internet access |
| ACC-P5-DEP-005 | Pod disruption budgets prevent Raft quorum loss during node drains |

### 7.2 Migration Acceptance

| ID | Requirement |
|---|---|
| ACC-P5-MIG-001 | Kafka-to-Keirox bridge syncs data without loss |
| ACC-P5-MIG-002 | Consumer offsets remain synchronized during migration |
| ACC-P5-MIG-003 | Zero-downtime cutover demonstrated |
| ACC-P5-MIG-004 | Rollback to Kafka demonstrated |
| ACC-P5-MIG-005 | Schema registry migration preserves all versions |

### 7.3 Supply Chain Acceptance

| ID | Requirement |
|---|---|
| ACC-P5-REL-001 | All binaries cross-compiled and tested |
| ACC-P5-REL-002 | Container images use Distroless/Chainguard base |
| ACC-P5-REL-003 | SBOM generated for every artifact |
| ACC-P5-REL-004 | All artifacts signed with Sigstore/Cosign |
| ACC-P5-REL-005 | SLSA Level 3 provenance attestation generated |
| ACC-P5-REL-006 | Builds are reproducible from source |

### 7.4 Observability Acceptance

| ID | Requirement |
|---|---|
| ACC-P5-OBS-001 | Grafana dashboards deploy and render correctly |
| ACC-P5-OBS-002 | Prometheus alerts fire correctly under test conditions |
| ACC-P5-OBS-003 | OpenTelemetry traces propagate across SDK and server |
| ACC-P5-OBS-004 | Datadog/New Relic integrations validated |

### 7.5 CLI & Console Acceptance

| ID | Requirement |
|---|---|
| ACC-P5-CLI-001 | CLI passes all integration tests |
| ACC-P5-CLI-002 | CLI works on Linux, macOS, and Windows |
| ACC-P5-CLI-003 | Web Console displays cluster state correctly |
| ACC-P5-CLI-004 | Break-glass operations require explicit confirmation |
| ACC-P5-CLI-005 | All console actions are audit-logged |

---

## 8. Phase 5 Gates

### 8.1 Gate 5A: Deployment Ready (Week 10)

| Criterion | Mandatory |
|---|---|
| Helm chart deploys 3-node cluster | Yes |
| Operator handles node replacement | Yes |
| CLI passes core integration tests | Yes |
| Container images build and pass security scan | Yes |

### 8.2 Gate 5B: Migration Ready (Week 16)

| Criterion | Mandatory |
|---|---|
| Kafka bridge syncs without data loss | Yes |
| Offset synchronization validated | Yes |
| Schema migration preserves versions | Yes |
| Supply chain pipeline produces signed artifacts | Yes |
| Grafana dashboards operational | Yes |

### 8.3 Gate 5C: GA Release Ready (Week 24)

| Criterion | Mandatory |
|---|---|
| Zero-downtime cutover demonstrated | Yes |
| Rollback demonstrated | Yes |
| Terraform provider works on AWS/GCP/Azure | Yes |
| Web Console operational with audit trail | Yes |
| Documentation portal complete | Yes |
| All supply chain artifacts signed and verifiable | Yes |
| Release candidate tagged: `v1.0.0-rc1` | Yes |
| Architecture Review Board approval | Yes |
| Executive GA launch approval | Yes |

---

## 9. Dependencies

### 9.1 Phase 4 Prerequisites

Phase 5 may begin overlapping with Phase 4 at Week 24, but full execution requires:

1. Phase 4 Gate 4C passed (or conditional pass with remediation).
2. Core engine API frozen (no breaking changes).
3. Security architecture finalized (ABAC, KMS, audit).
4. Multi-region architecture finalized.
5. Compatibility matrices finalized.

### 9.2 External Dependencies

| Dependency | Purpose | Risk |
|---|---|---|
| Kubernetes 1.28+ | Operator target platform | Version compatibility |
| Helm 3.12+ | Chart packaging | Chart schema changes |
| Terraform 1.5+ | IaC provider | Provider SDK changes |
| Sigstore/Cosign | Binary signing | Key management |
| Docker/OCI registries | Image distribution | Registry access |
| GitHub/GitLab | CI/CD pipelines | Runner availability |

---

## 10. Team Requirements

| Role | Count | Responsibility |
|---|---:|---|
| Platform / K8s Engineer | 2 | Operator, Helm, Terraform |
| DevOps / Release Engineer | 1–2 | CI/CD, supply chain, signing, images |
| Migration Engineer | 1–2 | Kafka bridge, offset sync, cutover |
| Frontend / Console Engineer | 1–2 | Web Console UI |
| CLI / Tooling Engineer | 1 | `keirox-cli`, Admin API |
| Observability Engineer | 1 | Grafana, Prometheus, OTel, Datadog |
| Technical Writer | 1 | Documentation portal, guides, tutorials |
| Product Manager | 1 | GA launch coordination, customer feedback |
| Chief Architect | 1 | Architecture governance, API freeze |
| Engineering Program Lead | 1 | Delivery coordination |

Estimated Phase 5 team size: **10–14 engineers/specialists**.

---

## 11. Risks and Mitigations

| Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|
| Kubernetes Operator complexity exceeds estimates | High | Medium | Use mature operator frameworks; limit CRD scope in v1 |
| Migration bridge data loss during cutover | Critical | Low | Dual-read validation; automated consistency checks; rollback playbook |
| Supply chain signing infrastructure failure | Medium | Low | Multi-registry publishing; offline signing capability |
| Web Console scope creep | Medium | High | Read-only default; strict feature gates; MVP-first approach |
| Terraform provider maintenance burden | Medium | Medium | Use Terraform Plugin Framework; limit resource count in v1 |
| Documentation lags implementation | Medium | High | Docs-as-code; documentation included in PR review |
| Air-gapped deployment edge cases | Medium | Medium | Test in isolated environment; document all dependencies |
| GA launch date pressure | High | Medium | Strict scope control; MVP features only in v1.0 |

---

## 12. Definition of v1.0 General Availability

Keirox v1.0 GA is achieved when:

1. All Phase 5 acceptance criteria pass.
2. All Phase 5 gates pass.
3. Release candidate `v1.0.0-rc1` is tagged, signed, and published.
4. Documentation portal is live.
5. Helm chart is published to public registry.
6. Container images are published to public registry.
7. Terraform provider is published to registry.
8. Migration guide is published.
9. Compatibility matrices are published.
10. Executive team approves GA launch.

---

## 13. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial Phase 5 Productization & Distribution Plan. Defines cloud-native distribution, CLI/console, migration tooling, secure supply chain, Day-2 observability, documentation portal, milestones, acceptance criteria, gates, team requirements, and risks. |