# KEI-K8S-501 — Kubernetes Operator & Terraform Certification Plan

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-K8S-501 |
| Title | Kubernetes Operator & Terraform Certification Plan |
| Version | 1.0 |
| Level | Engineering Execution Plan |
| Status | Baseline — Ready for Execution |
| Phase | Phase 5 — Productization, Distribution & Day-2 Operations |
| Duration | Weeks 3–20 of Phase 5 |
| Owner | Platform Engineering Lead / K8s Specialist |
| Governing Plan | KEI-ENG-500 — Phase 5 Productization & Distribution Plan |
| Governing Architecture Documents | KEI-ARC-022 (Consensus), KEI-ARC-026 (Multi-Region), KEI-OPS-040 (Runbooks) |
| Predecessor | KEI-ENG-500 (Phase 5 Master Plan) |
| Next Plan File | KEI-MIG-501 — Enterprise Migration & Bridge Plan |

---

## 2. Executive Summary

The Keirox Polymorphic Event Fabric is a multi-node, stateful distributed system with complex operational requirements: NVMe-backed WAL storage, Raft quorum consensus, S3 object storage streaming, and multi-region replication. Deploying and managing this system manually is error-prone and operationally expensive.

This plan defines the certification program for the **Keirox Kubernetes Operator**, **Helm Charts**, and **Terraform Provider** — the three deployment surfaces that enable enterprise platform teams to provision, scale, upgrade, and recover Keirox clusters using their existing infrastructure-as-code workflows.

The operator is the **primary deployment mechanism** for production environments. Helm provides the initial deployment and configuration layer. Terraform provides cloud infrastructure provisioning (S3 buckets, KMS keys, VPC peering, compute instances).

---

## 3. Purpose and Scope

### 3.1 Purpose

The purpose of this plan is to:

1. Define the Kubernetes Operator CRD schema and reconciliation logic.
2. Define the Helm chart structure and configuration surface.
3. Define the Terraform provider resources and data sources.
4. Define cert-manager integration for automated mTLS.
5. Define air-gapped deployment support.
6. Define deployment certification tests.
7. Produce the Phase 5 cloud-native distribution evidence package.

### 3.2 Scope

**In scope:**

- Kubernetes Operator CRD design and implementation.
- Operator reconciliation logic (scaling, upgrades, failure recovery).
- Helm chart packaging and validation.
- Terraform provider for AWS, GCP, and Azure.
- cert-manager integration for mTLS certificate lifecycle.
- Air-gapped / offline deployment support.
- Pod Disruption Budget management.
- Storage class abstraction (local NVMe, EBS, GCE PD, Azure Disk).
- Deployment certification tests.

**Out of scope:**

- Core engine implementation (frozen at Phase 4).
- Protocol gateway implementation (Phase 3/4).
- Web Console UI (owned by KEI-OPS-502).
- CLI tooling (owned by KEI-ENG-500 WP-P5-B).

---

## 4. Kubernetes Operator Design

### 4.1 Operator Architecture

```text
┌──────────────────────────────────────────────────────────────────┐
│                    KEIROX KUBERNETES OPERATOR                    │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                   CONTROLLER MANAGER                      │   │
│  │                                                          │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │   │
│  │  │ KeiroxCluster│  │ KeiroxStream │  │ KeiroxConsumer│   │   │
│  │  │ Reconciler   │  │ Reconciler   │  │ Group         │   │   │
│  │  │              │  │              │  │ Reconciler    │   │   │
│  │  └──────────────┘  └──────────────┘  └──────────────┘   │   │
│  │                                                          │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │   │
│  │  │ KeiroxGateway│  │ KeiroxBackup │  │ KeiroxRegion │   │   │
│  │  │ Reconciler   │  │ Reconciler   │  │ Replication   │   │   │
│  │  │              │  │              │  │ Reconciler    │   │   │
│  │  └──────────────┘  └──────────────┘  └──────────────┘   │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                   WEBHOOK SERVER                          │   │
│  │  - Validating webhooks (CRD validation)                   │   │
│  │  - Mutating webhooks (default injection)                  │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                   HEALTH PROBES                           │   │
│  │  - Liveness: /healthz                                     │   │
│  │  - Readiness: /readyz                                     │   │
│  │  - Metrics: /metrics (Prometheus)                         │   │
│  └──────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────┘
```

### 4.2 Custom Resource Definitions

#### KeiroxCluster CRD

The top-level resource representing a Keirox deployment.

```yaml
apiVersion: keirox.io/v1alpha1
kind: KeiroxCluster
metadata:
  name: production-cluster
  namespace: keirox
spec:
  version: "1.0.0"
  replicas: 3
  
  storage:
    walVolume:
      storageClassName: local-nvme
      size: 500Gi
    tier1:
      provider: aws-s3
      bucket: keirox-tier1-prod
      region: us-east-1
      prefix: production/
  
  consensus:
    raftReplicas: 3
    electionTimeoutMs: 3000
    heartbeatIntervalMs: 500
  
  statePlane:
    coordinatorShards: 16
    bitmapSpillEnabled: true
    bitmapSpillThresholdBytes: 268435456  # 256 MB
  
  lakehouse:
    enabled: true
    icebergCatalog:
      type: rest
      uri: http://iceberg-catalog:8181
    parquet:
      targetFileSizeBytes: 134217728  # 128 MB
      compressionCodec: zstd
  
  security:
    tls:
      enabled: true
      certManager:
        issuerRef:
          name: keirox-ca
          kind: ClusterIssuer
    kms:
      provider: aws
      keyId: alias/keirox-production
  
  observability:
    metrics:
      enabled: true
      port: 9090
    tracing:
      enabled: true
      exporter: otlp
      endpoint: otel-collector:4317
  
  resources:
    server:
      requests:
        cpu: "4"
        memory: 16Gi
      limits:
        cpu: "8"
        memory: 32Gi
  
  podDisruptionBudget:
    minAvailable: 2
  
  affinity:
    podAntiAffinity:
      requiredDuringSchedulingIgnoredDuringExecution:
        - labelSelector:
            matchLabels:
              app: keirox-server
          topologyKey: kubernetes.io/hostname
```

#### KeiroxStream CRD

Represents a logical stream within a cluster.

```yaml
apiVersion: keirox.io/v1alpha1
kind: KeiroxStream
metadata:
  name: orders-stream
  namespace: keirox
spec:
  clusterRef: production-cluster
  tenantId: tenant-acme
  streamName: orders
  schemaPolicy:
    mode: INFERRED
    maxShreddedFields: 64
  retention:
    tier0DurationHours: 24
    tier1DurationDays: 90
  consumerGroups:
    - name: order-processor
      ackMode: ACK_FAST
      maxRetries: 3
      leaseTtlSeconds: 30
```

#### KeiroxConsumerGroup CRD

Represents a consumer group with queue semantics.

```yaml
apiVersion: keirox.io/v1alpha1
kind: KeiroxConsumerGroup
metadata:
  name: order-processor
  namespace: keirox
spec:
  streamRef: orders-stream
  ackMode: ACK_FAST
  maxRetries: 3
  leaseTtlSeconds: 30
  dlqPolicy:
    enabled: true
    redriveEnabled: true
    maxDlqAgeHours: 168
```

### 4.3 Operator Reconciliation Logic

| Reconciler | Trigger | Actions |
|---|---|---|
| KeiroxCluster | Create/Update/Delete | Provision StatefulSet, Services, ConfigMaps, Secrets, PVCs |
| KeiroxStream | Create/Update/Delete | Call Admin API to create/update/delete stream |
| KeiroxConsumerGroup | Create/Update/Delete | Call Admin API to create/update/delete consumer group |
| KeiroxGateway | Create/Update/Delete | Deploy gateway sidecar or separate deployment |
| KeiroxBackup | Scheduled | Trigger backup via Admin API |
| KeiroxRegionReplication | Create/Update | Configure Mode A replication |

### 4.4 Scaling Behavior

| Operation | Operator Behavior |
|---|---|
| Scale up (add node) | Create new Pod, join Raft cluster, catch up replication, update Service endpoints |
| Scale down (remove node) | Drain node, transfer leadership if leader, remove from Raft, delete Pod and PVC (if configured) |
| Rolling upgrade | Upgrade one Pod at a time; wait for Raft catch-up before proceeding |
| Node failure | Detect via Pod status; if PVC persists, restart Pod; if PVC lost, provision new PVC and restore from Tier-1 |

### 4.5 Failure Recovery

| Failure Scenario | Operator Behavior |
|---|---|
| Pod crash | Kubernetes restarts Pod; operator verifies Raft membership |
| PVC loss | Operator provisions new PVC; node restores from S3 manifests + peer WAL delta |
| Raft leader loss | Raft elects new leader automatically; operator updates Service labels |
| S3 outage | Operator monitors S3 health; pauses Tier-1 offload; alerts via metrics |
| Network partition | Operator does NOT interfere with Raft fencing; monitors and alerts |

---

## 5. Helm Chart Design

### 5.1 Chart Structure

```text
keirox/
├── Chart.yaml
├── values.yaml
├── values.schema.json
├── templates/
│   ├── _helpers.tpl
│   ├── cluster/
│   │   ├── statefulset.yaml
│   │   ├── service.yaml
│   │   ├── service-headless.yaml
│   │   ├── configmap.yaml
│   │   ├── secret.yaml
│   │   ├── pvc.yaml
│   │   └── pdb.yaml
│   ├── gateway/
│   │   ├── deployment.yaml
│   │   ├── service.yaml
│   │   └── hpa.yaml
│   ├── operator/
│   │   ├── deployment.yaml
│   │   ├── rbac.yaml
│   │   └── crds.yaml
│   ├── cert-manager/
│   │   ├── certificate.yaml
│   │   └── issuer.yaml
│   ├── monitoring/
│   │   ├── servicemonitor.yaml
│   │   ├── prometheusrule.yaml
│   │   └── grafana-dashboard.yaml
│   └── NOTES.txt
├── charts/
│   └── keirox-gateway/  (subchart)
└── tests/
    └── test-connection.yaml
```

### 5.2 Values Schema

The `values.yaml` MUST support:

| Section | Parameters |
|---|---|
| `global` | Image registry, pull secrets, common labels |
| `cluster` | Replicas, version, resources, affinity, tolerations |
| `storage.wal` | Storage class, size, access mode |
| `storage.tier1` | S3/GCS/Azure config, bucket, prefix |
| `consensus` | Raft replicas, timeouts |
| `statePlane` | Coordinator shards, bitmap config |
| `lakehouse` | Iceberg catalog, Parquet config |
| `security` | TLS, cert-manager, KMS |
| `gateways.kafka` | Enabled, replicas, resources |
| `gateways.sqs` | Enabled, replicas, resources |
| `gateways.amqp` | Enabled, replicas, resources |
| `monitoring` | Prometheus, Grafana, alerting |
| `operator` | Enabled, replicas, resources |

### 5.3 Helm Certification Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| HELM-T-001 | `helm install` with default values | 3-node cluster deploys successfully |
| HELM-T-002 | `helm upgrade` with version bump | Rolling upgrade completes without data loss |
| HELM-T-003 | `helm install` with custom values | Custom storage class, replicas, and resources applied |
| HELM-T-004 | `helm install` with air-gapped registry | Deploys from private registry without internet |
| HELM-T-005 | `helm uninstall` | All resources cleaned up; PVCs optionally retained |
| HELM-T-006 | `helm template` with schema validation | Invalid values rejected by JSON schema |
| HELM-T-007 | `helm install` with cert-manager | TLS certificates issued automatically |

---

## 6. Terraform Provider Design

### 6.1 Provider Architecture

```text
terraform-provider-keirox/
├── main.go
├── internal/
│   ├── provider/
│   │   └── provider.go
│   ├── resources/
│   │   ├── cluster.go
│   │   ├── stream.go
│   │   ├── consumer_group.go
│   │   ├── gateway.go
│   │   └── replication.go
│   ├── data_sources/
│   │   ├── cluster.go
│   │   └── stream.go
│   └── client/
│       └── admin_client.go
├── docs/
├── examples/
└── Makefile
```

### 6.2 Terraform Resources

| Resource | Description |
|---|---|
| `keirox_cluster` | Provision a Keirox cluster (maps to Helm release or direct deployment) |
| `keirox_stream` | Create/manage a logical stream |
| `keirox_consumer_group` | Create/manage a consumer group |
| `keirox_gateway` | Deploy a protocol gateway |
| `keirox_replication` | Configure Mode A multi-region replication |
| `keirox_backup_policy` | Define backup schedule and retention |

### 6.3 Terraform Data Sources

| Data Source | Description |
|---|---|
| `keirox_cluster` | Read cluster status and health |
| `keirox_stream` | Read stream metadata and offsets |

### 6.4 Cloud-Specific Resources

For AWS:

```hcl
resource "keirox_cluster" "production" {
  name     = "production"
  version  = "1.0.0"
  replicas = 3
  
  storage {
    wal {
      storage_class = "gp3-nvme"
      size_gb       = 500
    }
    tier1 {
      provider = "aws_s3"
      bucket   = aws_s3_bucket.keirox_tier1.id
      region   = "us-east-1"
    }
  }
  
  security {
    tls_enabled = true
    kms_key_id  = aws_kms_key.keirox.arn
  }
}
```

### 6.5 Terraform Certification Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| TF-T-001 | `terraform plan` with valid config | Plan shows expected resources |
| TF-T-002 | `terraform apply` | Cluster provisions successfully |
| TF-T-003 | `terraform plan` after manual change | Drift detected |
| TF-T-004 | `terraform apply` with updated config | Cluster updates without data loss |
| TF-T-005 | `terraform destroy` | All resources cleaned up |
| TF-T-006 | AWS provider validation | Correct IAM roles and policies created |
| TF-T-007 | GCP provider validation | Correct service accounts and IAM bindings created |
| TF-T-008 | Azure provider validation | Correct managed identities and RBAC created |

---

## 7. cert-manager Integration

### 7.1 Certificate Architecture

```text
ClusterIssuer (self-signed or CA)
    │
    ├── Certificate: keirox-server-tls
    │   └── Used by: Keirox server nodes (mTLS)
    │
    ├── Certificate: keirox-gateway-tls
    │   └── Used by: Protocol gateways (TLS termination)
    │
    ├── Certificate: keirox-operator-tls
    │   └── Used by: Operator webhooks
    │
    └── Certificate: keirox-internal-mtls
        └── Used by: Internal cluster communication
```

### 7.2 Certificate Rotation

| Component | Rotation Policy | Downtime |
|---|---|---|
| Server mTLS | 30 days before expiry | Zero (hot reload) |
| Gateway TLS | 30 days before expiry | Zero (hot reload) |
| Operator webhook TLS | 30 days before expiry | Zero (hot reload) |

### 7.3 Certification Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| CERT-T-001 | Fresh install with cert-manager | Certificates issued automatically |
| CERT-T-002 | Certificate expiry simulation | Certificates rotated without downtime |
| CERT-T-003 | cert-manager unavailable | Existing certificates continue to work |
| CERT-T-004 | Custom CA issuer | Custom CA certificates used for mTLS |

---

## 8. Air-Gapped Deployment Support

### 8.1 Requirements

| ID | Requirement |
|---|---|
| AIR-001 | All container images MUST be mirrorable to private registries |
| AIR-002 | Helm chart MUST work without external repository dependencies |
| AIR-003 | Operator MUST NOT require internet access for reconciliation |
| AIR-004 | CLI MUST support offline mode for air-gapped environments |
| AIR-005 | SBOM and signatures MUST be verifiable offline |

### 8.2 Air-Gapped Bundle

```text
keirox-airgapped-v1.0.0/
├── images/
│   ├── keirox-server-v1.0.0.tar
│   ├── keirox-operator-v1.0.0.tar
│   ├── keirox-gateway-v1.0.0.tar
│   └── keirox-cli-v1.0.0.tar
├── helm/
│   └── keirox-1.0.0.tgz
├── terraform/
│   └── terraform-provider-keirox_v1.0.0_linux_amd64.zip
├── sbom/
│   └── keirox-v1.0.0-sbom.cdx.json
├── signatures/
│   ├── keirox-server-v1.0.0.sig
│   └── keirox-server-v1.0.0.pem
└── README.md
```

### 8.3 Air-Gapped Certification Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| AIR-T-001 | Deploy from private registry | Cluster deploys without internet |
| AIR-T-002 | Helm install from local chart | Chart installs without external repos |
| AIR-T-003 | Operator reconciliation without internet | Operator functions normally |
| AIR-T-004 | CLI operations in air-gapped mode | CLI works without external calls |
| AIR-T-005 | Verify signatures offline | Cosign verification works with local keys |

---

## 9. Pod Disruption & Scaling Safety

### 9.1 Pod Disruption Budget

The operator MUST create a PodDisruptionBudget that prevents Raft quorum loss:

```yaml
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: keirox-server-pdb
spec:
  minAvailable: 2  # For 3-node Raft quorum
  selector:
    matchLabels:
      app: keirox-server
```

**Normative rule:** The PDB `minAvailable` MUST be calculated as `(raft_replicas / 2) + 1` to maintain quorum.

### 9.2 Graceful Shutdown

When a Pod is terminated:

1. Pod receives SIGTERM.
2. Server stops accepting new writes.
3. Server flushes in-memory state to disk.
4. Server transfers Raft leadership if it is the leader.
5. Server closes all connections gracefully.
6. Pod terminates.

**Normative rule:** The graceful shutdown timeout MUST be configurable and default to 60 seconds.

---

## 10. Storage Class Abstraction

### 10.1 Supported Storage Classes

| Storage Class | Provider | Use Case |
|---|---|---|
| `local-nvme` | Local NVMe SSD | Highest performance; requires local volume provisioner |
| `gp3-nvme` | AWS EBS gp3 | Balanced performance and cost |
| `pd-ssd` | GCP Persistent Disk SSD | GCP environments |
| `managed-premium` | Azure Managed Disk Premium | Azure environments |
| `local-path` | Kubernetes local-path | Development/testing only |

### 10.2 Storage Class Certification

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| STOR-T-001 | Deploy with local-nvme | WAL writes use local NVMe; performance target met |
| STOR-T-002 | Deploy with gp3-nvme | WAL writes use EBS; performance within acceptable range |
| STOR-T-003 | PVC expansion | Volume expands without data loss |
| STOR-T-004 | Storage class migration | Data migrates between storage classes (if supported) |

---

## 11. Certification Levels

| Level | Name | Requirement |
|---|---|---|
| L1 | Operator Certified | CRDs reconcile correctly; scaling and upgrades work |
| L2 | Helm Certified | Chart deploys, upgrades, and uninstalls correctly |
| L3 | Terraform Certified | Provider creates and manages resources correctly |
| L4 | cert-manager Certified | TLS certificates issued and rotated automatically |
| L5 | Air-Gapped Certified | Deployment works without internet access |
| L6 | PDB Certified | Raft quorum preserved during node drains |

Phase 5 exit requires **L1 through L6**.

---

## 12. Deliverables and Milestones

| Deliverable | Description | Target Week |
|---|---|---:|
| D-K8S-001 | CRD schema design (KeiroxCluster, KeiroxStream, KeiroxConsumerGroup) | Week 4 |
| D-K8S-002 | Operator controller manager implementation | Week 6 |
| D-K8S-003 | KeiroxCluster reconciler | Week 8 |
| D-K8S-004 | Helm chart with values schema | Week 8 |
| D-K8S-005 | cert-manager integration | Week 10 |
| D-K8S-006 | Terraform provider (AWS) | Week 12 |
| D-K8S-007 | Terraform provider (GCP, Azure) | Week 14 |
| D-K8S-008 | Air-gapped deployment bundle | Week 16 |
| D-K8S-009 | PDB and graceful shutdown validation | Week 16 |
| D-K8S-010 | Deployment certification test suite | Week 18 |
| D-K8S-011 | Final cloud-native distribution evidence package | Week 20 |

---

## 13. Certification Gates

### 13.1 Gate K8S-A: Operator Alpha (Week 8)

| Criterion | Mandatory |
|---|---|
| KeiroxCluster CRD reconciles correctly | Yes |
| Helm chart deploys 3-node cluster | Yes |
| Pod Disruption Budget prevents quorum loss | Yes |
| Graceful shutdown works without data loss | Yes |

### 13.2 Gate K8S-B: Multi-Cloud Certified (Week 14)

| Criterion | Mandatory |
|---|---|
| Terraform provider works on AWS | Yes |
| Terraform provider works on GCP | Yes |
| Terraform provider works on Azure | Yes |
| cert-manager issues and rotates certificates | Yes |
| Air-gapped deployment works | Yes |

### 13.3 Gate K8S-C: Production Ready (Week 20)

| Criterion | Mandatory |
|---|---|
| All L1–L6 certification levels pass | Yes |
| Rolling upgrade preserves Raft quorum | Yes |
| Node failure recovery works automatically | Yes |
| PVC loss recovery works from S3 manifests | Yes |
| Evidence package complete | Yes |

---

## 14. Risks and Mitigations

| Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|
| Operator complexity exceeds estimates | High | Medium | Use mature frameworks (kube-rs, kubebuilder); limit v1 CRD scope |
| Helm chart maintenance burden | Medium | Medium | Use JSON schema validation; automated chart testing |
| Terraform provider API drift | Medium | Medium | Pin Terraform Plugin Framework version; comprehensive acceptance tests |
| Air-gapped edge cases | Medium | High | Test in fully isolated environment; document all dependencies |
| Storage class performance variance | Medium | Medium | Document performance expectations per storage class; benchmark each |
| cert-manager version incompatibility | Low | Medium | Test against cert-manager 1.12+; document supported versions |
| PDB misconfiguration causes quorum loss | Critical | Low | Operator calculates PDB automatically; validate in tests |

---

## 15. Evidence Package

The cloud-native distribution evidence package MUST include:

1. Operator CRD schema documentation.
2. Operator reconciliation test results.
3. Helm chart lint and test results.
4. Terraform provider acceptance test results (AWS, GCP, Azure).
5. cert-manager integration test results.
6. Air-gapped deployment test results.
7. PDB and graceful shutdown validation results.
8. Storage class benchmark results.
9. Rolling upgrade test results.
10. Node failure recovery test results.

---

## 16. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial Kubernetes Operator & Terraform Certification Plan. Defines CRD schema, operator reconciliation logic, Helm chart structure, Terraform provider resources, cert-manager integration, air-gapped deployment, PDB management, storage class abstraction, certification levels, and evidence requirements. |