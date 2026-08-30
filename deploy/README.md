# Keirox Deployment Manifests & Helm Charts

Deployment infrastructure definitions, Dockerfiles, Kubernetes manifests, Helm charts, and Terraform modules for single-node and multi-region Keirox clusters.

---

## ⚡ Deployment Topologies

- **Single-Node Prototype**: [`deploy/docker/`](docker/) (Local Docker Compose environment with NVMe mount emulation).
- **Production Raft Cluster (3-Node Quorum)**: [`deploy/kubernetes/`](kubernetes/) (Kubernetes StatefulSets with local NVMe PVs).
- **Multi-Region WAN Cluster**: [`deploy/terraform/`](terraform/) (Cross-region WAN deployment with KMS encryption).
