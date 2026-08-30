# KEI-MIG-501 — Enterprise Migration & Bridge Plan

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-MIG-501 |
| Title | Enterprise Migration & Bridge Plan |
| Version | 1.0 |
| Level | Engineering Execution Plan |
| Status | Baseline — Ready for Execution |
| Phase | Phase 5 — Productization, Distribution & Day-2 Operations |
| Duration | Weeks 8–22 of Phase 5 |
| Owner | Migration Engineering Lead / Ecosystem Lead |
| Governing Plan | KEI-ENG-500 — Phase 5 Productization & Distribution Plan |
| Governing Architecture Documents | KEI-ARC-024, KEI-DES-032, KEI-DES-035 |
| Predecessor | KEI-K8S-501 (Kubernetes Operator & Terraform) |
| Next Plan File | KEI-REL-501 — Secure Supply Chain & Release Engineering Plan |

---

## 2. Executive Summary

The most significant barrier to enterprise adoption of any new data platform is **migration risk**. Enterprises with existing Apache Kafka deployments have invested heavily in producer configurations, consumer group offsets, schema registries, monitoring dashboards, and operational runbooks. Asking them to perform a "big bang" migration is a non-starter.

This plan defines the **Enterprise Migration & Bridge** program — a set of tools, procedures, and validation suites that enable organizations to migrate from Apache Kafka (and compatible systems) to Keirox **incrementally, safely, and reversibly**.

The migration strategy follows a four-phase approach:

```text
Phase A: Bridge Deployment     → Dual-write from Kafka to Keirox
Phase B: Validation            → Compare data, offsets, schemas
Phase C: Consumer Cutover      → Switch consumers to Keirox
Phase D: Decommission          → Remove Kafka bridge and cluster
```

At every phase, a rollback path exists. At no point is the source Kafka cluster modified or put at risk.

---

## 3. Purpose and Scope

### 3.1 Purpose

The purpose of this plan is to:

1. Define the Kafka-to-Keirox migration bridge architecture.
2. Define consumer offset synchronization mechanisms.
3. Define schema registry migration procedures.
4. Define dual-write and dual-read validation strategies.
5. Define zero-downtime cutover procedures.
6. Define rollback procedures.
7. Define migration certification tests.
8. Produce the migration evidence package.

### 3.2 Scope

**In scope:**

- Kafka-to-Keirox data bridge (consumer-based replication).
- Consumer group offset synchronization.
- Schema registry migration (Confluent Schema Registry / Apicurio).
- Dual-write proxy mode for validation.
- Dual-read comparison for consistency verification.
- Zero-downtime cutover procedures.
- Rollback procedures.
- Migration validation suite.
- Migration runbooks and playbooks.

**Out of scope:**

- Kafka cluster decommissioning (customer responsibility).
- Migration from non-Kafka systems (SQS, AMQP, RabbitMQ) — these use native gateways.
- Keirox-to-Kafka reverse migration (rollback uses Kafka as source of truth).
- Data transformation or ETL during migration.

### 3.3 Migration Constraints

1. The source Kafka cluster MUST NOT be modified during migration.
2. Migration MUST support rollback at every phase.
3. Migration MUST NOT introduce data loss or duplication.
4. Consumer offset mapping MUST preserve ordering guarantees.
5. Schema compatibility MUST be validated before cutover.
6. Migration tools MUST support both cloud-managed Kafka (Confluent Cloud, MSK) and self-hosted Kafka.

---

## 4. Migration Architecture

### 4.1 Bridge Topology

```text
┌────────────────────────────────────────────────────────────────────────────┐
│                        MIGRATION BRIDGE TOPOLOGY                           │
│                                                                            │
│  ┌──────────────────┐                                                     │
│  │   SOURCE KAFKA   │                                                     │
│  │   CLUSTER        │                                                     │
│  │                  │                                                     │
│  │  Topic A         │──── Keirox Migration Bridge ────┐                  │
│  │  Topic B         │     (Kafka Consumer)             │                  │
│  │  Topic C         │                                   │                  │
│  └──────────────────┘                                   │                  │
│                                                         │                  │
│                                                         ▼                  │
│                                              ┌──────────────────────┐     │
│                                              │   KEIROX CLUSTER     │     │
│                                              │                      │     │
│                                              │  Stream A            │     │
│                                              │  Stream B            │     │
│                                              │  Stream C            │     │
│                                              └──────────────────────┘     │
│                                                         │                  │
│                                                         │                  │
│  ┌──────────────────┐                                   │                  │
│  │  CONSUMERS       │◄──── Cutover Switch ──────────────┘                  │
│  │                  │                                                      │
│  │  Consumer Group  │  (Initially reads from Kafka)                        │
│  │  1, 2, 3...      │  (After cutover, reads from Keirox)                 │
│  └──────────────────┘                                                      │
│                                                                            │
│  ┌──────────────────┐                                                     │
│  │  SCHEMA REGISTRY │──── Schema Migration Tool ────► Keirox Registry     │
│  └──────────────────┘                                                     │
└────────────────────────────────────────────────────────────────────────────┘
```

### 4.2 Bridge Components

| Component | Role |
|---|---|
| **Migration Bridge** | Kafka consumer that reads from source topics and writes to Keirox streams |
| **Offset Tracker** | Maintains mapping between Kafka consumer group offsets and Keirox stream offsets |
| **Schema Migrator** | Exports schemas from source registry and imports into Keirox registry |
| **Dual-Write Proxy** | Optional proxy that intercepts producer writes and sends to both Kafka and Keirox |
| **Validation Engine** | Compares Kafka and Keirox data for consistency |
| **Cutover Controller** | Orchestrates the consumer cutover process |

---

## 5. Migration Bridge Design

### 5.1 Bridge Architecture

The Migration Bridge is a standalone service that:

1. Consumes from source Kafka topics using standard Kafka consumer protocol.
2. Transforms Kafka records into Keirox append operations.
3. Tracks offsets to enable exactly-once semantics.
4. Supports multiple topics in parallel.
5. Reports metrics and health.

### 5.2 Record Mapping

| Kafka Concept | Keirox Mapping |
|---|---|
| Topic | Stream |
| Partition | Virtual partition (via `entity_key` hash) |
| Record Key | `entity_key` |
| Record Value | Payload |
| Record Headers | Metadata attributes |
| Partition Offset | Stream logical offset |
| Consumer Group | Consumer group |
| Consumer Group Offset | Stream offset commit |
| Timestamp | Ingest timestamp |

### 5.3 Offset Mapping Strategy

Consumer group offsets are the most critical state to preserve during migration. The bridge maintains a mapping table:

```text
OffsetMapping {
    kafka_topic: String,
    kafka_partition: i32,
    kafka_offset: i64,
    keirox_stream: String,
    keirox_offset: u64,
    migrated_at: Timestamp,
}
```

**Normative rules:**

- Offset mapping MUST be persisted durably (not in-memory only).
- Offset mapping MUST be idempotent (reprocessing the same Kafka offset produces the same Keirox offset).
- Offset mapping MUST support both earliest and latest starting positions.
- During cutover, consumer group offsets MUST be translated to Keirox offsets before consumers switch.

### 5.4 Exactly-Once Semantics

The bridge achieves exactly-once delivery through:

1. **Idempotent writes:** Each Kafka record is tagged with `(topic, partition, offset)` as the idempotency key.
2. **Offset tracking:** The bridge commits its own consumer group offset to Kafka only after the record is durably written to Keirox.
3. **Deduplication window:** Keirox idempotent produce deduplication prevents duplicates within the configured window.

```text
Bridge Processing Loop:
1. Poll records from Kafka
2. For each record:
   a. Check if (topic, partition, offset) already migrated
   b. If not, append to Keirox with idempotency key
   c. If yes, skip (idempotent)
3. Commit Kafka consumer group offset
4. Update offset mapping table
```

---

## 6. Schema Registry Migration

### 6.1 Migration Scope

| Source Registry | Supported Formats | Migration Method |
|---|---|---|
| Confluent Schema Registry | Avro, Protobuf, JSON Schema | REST API export/import |
| Apicurio Registry | Avro, Protobuf, JSON Schema, OpenAPI | REST API export/import |
| AWS Glue Schema Registry | Avro, Protobuf, JSON Schema | AWS SDK export/import |
| File-based schemas | Any | File-based import |

### 6.2 Migration Procedure

```text
1. Export all schemas from source registry
   └── Include: schema ID, version, subject, compatibility mode, schema definition

2. Transform schemas to Keirox format
   └── Map subject names to Keirox stream names
   └── Preserve schema versioning
   └── Validate compatibility modes

3. Import schemas into Keirox registry
   └── Register each schema with original version numbers
   └── Set compatibility mode per stream policy

4. Validate schema integrity
   └── Verify all versions are present
   └── Verify compatibility modes match
   └── Verify schema fingerprints match

5. Update producer/consumer configurations
   └── Point schema registry URL to Keirox
   └── Validate serialization/deserialization
```

### 6.3 Schema Migration Validation

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| SCHEMA-MIG-T-001 | Migrate 100 schemas | All 100 schemas imported with correct versions |
| SCHEMA-MIG-T-002 | Migrate schema with multiple versions | All versions preserved |
| SCHEMA-MIG-T-003 | Migrate incompatible schema | Warning emitted; manual review required |
| SCHEMA-MIG-T-004 | Validate fingerprint consistency | Source and target fingerprints match |
| SCHEMA-MIG-T-005 | Round-trip serialization | Data serialized with source schema deserializes correctly with Keirox schema |

---

## 7. Dual-Write Validation

### 7.1 Purpose

Before switching consumers to Keirox, the migration team must validate that Keirox receives the same data as Kafka. Dual-write mode enables this validation without modifying producers.

### 7.2 Dual-Write Modes

| Mode | Description | Use Case |
|---|---|---|
| **Bridge Mode** | Bridge consumes from Kafka and writes to Keirox | Default; no producer changes required |
| **Proxy Mode** | Producer writes to a proxy that forwards to both Kafka and Keirox | Validates producer-side behavior |
| **Application Mode** | Application code writes to both Kafka and Keirox | Full control; requires code changes |

### 7.3 Dual-Read Comparison

The Validation Engine performs dual-read comparison:

```text
1. For each topic/stream pair:
   a. Read N records from Kafka topic
   b. Read N records from Keirox stream
   c. Compare:
      - Record count
      - Record keys (entity_key mapping)
      - Record values (payload bytes)
      - Record timestamps
      - Record headers/metadata
   d. Report discrepancies
```

### 7.4 Comparison Metrics

| Metric | Target |
|---|---|
| Record count match | 100% |
| Key match | 100% |
| Value match | 100% |
| Timestamp delta | ≤1 second |
| Header match | 100% |
| Ordering match | 100% within partition/entity_key |

---

## 8. Consumer Cutover Procedure

### 8.1 Cutover Prerequisites

Before cutover can begin:

1. Bridge has been running for at least 7 days without errors.
2. Dual-read comparison shows 100% match for all topics.
3. Schema registry migration is complete and validated.
4. Consumer offset mapping is complete and validated.
5. Keirox cluster is healthy and passes all monitoring checks.
6. Rollback procedure has been tested.
7. Stakeholders have approved cutover.

### 8.2 Cutover Steps

```text
Step 1: Freeze Producers (Optional)
   └── If zero-tolerance for data gap, briefly pause producers
   └── Bridge catches up to latest Kafka offset

Step 2: Record Final Kafka Offsets
   └── For each consumer group, record current committed offsets
   └── Translate to Keirox offsets using offset mapping

Step 3: Switch Consumer Configuration
   └── Update consumer bootstrap servers to Keirox gateway
   └── Update consumer group offsets to translated Keirox offsets
   └── Update schema registry URL to Keirox

Step 4: Restart Consumers
   └── Consumers reconnect to Keirox
   └── Consumers resume from translated offsets
   └── Monitor for errors, latency, data gaps

Step 5: Validate
   └── Verify consumers are processing correctly
   └── Verify no data loss or duplication
   └── Verify latency within acceptable range

Step 6: Monitor (24-72 hours)
   └── Watch for delayed issues
   └── Keep Kafka cluster available as fallback

Step 7: Decommission Bridge
   └── Stop migration bridge
   └── Archive offset mapping table
   └── Update documentation
```

### 8.3 Cutover Rollback Procedure

If cutover fails:

```text
Step 1: Stop Keirox consumers
Step 2: Revert consumer configuration to Kafka
Step 3: Restart consumers against Kafka
Step 4: Resume bridge (if data gap exists)
Step 5: Investigate root cause
Step 6: Fix and re-attempt cutover
```

**Normative rule:** Rollback MUST be executable within 5 minutes of cutover failure detection.

---

## 9. Migration Validation Suite

### 9.1 Validation Test Categories

| Category | Tests | Purpose |
|---|---|---|
| Data Integrity | Record count, value, key comparison | No data loss or corruption |
| Offset Integrity | Offset mapping accuracy | Consumers resume correctly |
| Schema Integrity | Schema version and compatibility validation | Serialization works |
| Ordering Integrity | Per-partition/entity_key ordering | Ordering preserved |
| Performance | Throughput and latency comparison | Acceptable performance |
| Failure Recovery | Bridge crash, Kafka outage, Keirox outage | Migration survives failures |

### 9.2 Validation Tests

| Test ID | Scenario | Expected Behavior |
|---|---|---|
| MIG-T-001 | Bridge migrates 1M records | All records present in Keirox |
| MIG-T-002 | Bridge crash during migration | Bridge restarts; no data loss or duplication |
| MIG-T-003 | Kafka outage during migration | Bridge pauses; resumes when Kafka recovers |
| MIG-T-004 | Keirox outage during migration | Bridge retries; no data loss |
| MIG-T-005 | Offset mapping with 100 consumer groups | All offsets mapped correctly |
| MIG-T-006 | Cutover with 50 consumers | All consumers resume correctly |
| MIG-T-007 | Rollback after cutover | Consumers revert to Kafka successfully |
| MIG-T-008 | Schema migration with 500 schemas | All schemas migrated correctly |
| MIG-T-009 | Dual-read comparison with 10M records | 100% match |
| MIG-T-010 | End-to-end migration (bridge → validate → cutover) | Zero data loss, zero downtime |

---

## 10. Migration Runbooks

### 10.1 Runbook Catalog

| Runbook ID | Title | Trigger |
|---|---|---|
| MIG-RB-001 | Bridge Deployment | Migration project kickoff |
| MIG-RB-002 | Schema Registry Migration | Before consumer cutover |
| MIG-RB-003 | Dual-Write Validation | Before consumer cutover |
| MIG-RB-004 | Consumer Cutover | Scheduled maintenance window |
| MIG-RB-005 | Cutover Rollback | Cutover failure detected |
| MIG-RB-006 | Bridge Failure Recovery | Bridge crash or hang |
| MIG-RB-007 | Offset Reconciliation | Offset drift detected |
| MIG-RB-008 | Migration Decommission | Migration complete |

### 10.2 Runbook Requirements

Each runbook MUST include:

1. Prerequisites checklist.
2. Step-by-step procedures.
3. Validation checks at each step.
4. Rollback procedures.
5. Escalation contacts.
6. Estimated duration.
7. Risk assessment.

---

## 11. Migration Certification Levels

| Level | Name | Requirement |
|---|---|---|
| L1 | Bridge Certified | Bridge migrates data without loss or duplication |
| L2 | Offset Certified | Consumer group offsets map correctly |
| L3 | Schema Certified | Schema registry migration preserves all versions |
| L4 | Validation Certified | Dual-read comparison passes |
| L5 | Cutover Certified | Zero-downtime cutover demonstrated |
| L6 | Rollback Certified | Rollback demonstrated within 5 minutes |
| L7 | End-to-End Certified | Full migration cycle completed successfully |

Phase 5 exit requires **L1 through L7**.

---

## 12. Deliverables and Milestones

| Deliverable | Description | Target Week |
|---|---|---:|
| D-MIG-001 | Migration bridge architecture design | Week 9 |
| D-MIG-002 | Migration bridge implementation | Week 12 |
| D-MIG-003 | Offset tracking and mapping | Week 12 |
| D-MIG-004 | Schema registry migration tool | Week 14 |
| D-MIG-005 | Dual-write proxy (optional) | Week 14 |
| D-MIG-006 | Validation engine | Week 16 |
| D-MIG-007 | Cutover controller | Week 16 |
| D-MIG-008 | Migration validation test suite | Week 18 |
| D-MIG-009 | Migration runbooks | Week 20 |
| D-MIG-010 | End-to-end migration certification | Week 22 |

---

## 13. Certification Gates

### 13.1 Gate MIG-A: Bridge Certified (Week 14)

| Criterion | Mandatory |
|---|---|
| Bridge migrates 1M records without loss | Yes |
| Offset mapping is accurate | Yes |
| Bridge survives crash and restart | Yes |
| Schema migration preserves versions | Yes |

### 13.2 Gate MIG-B: Validation Certified (Week 18)

| Criterion | Mandatory |
|---|---|
| Dual-read comparison passes (100% match) | Yes |
| All validation tests pass | Yes |
| Performance within acceptable range | Yes |

### 13.3 Gate MIG-C: Cutover Certified (Week 22)

| Criterion | Mandatory |
|---|---|
| Zero-downtime cutover demonstrated | Yes |
| Rollback demonstrated within 5 minutes | Yes |
| End-to-end migration cycle completed | Yes |
| All runbooks reviewed and approved | Yes |
| Evidence package complete | Yes |

---

## 14. Risks and Mitigations

| Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|
| Offset mapping drift causes data gap | Critical | Medium | Continuous offset reconciliation; alerting on drift |
| Schema incompatibility discovered late | High | Medium | Migrate schemas early; validate before cutover |
| Consumer cutover causes processing delay | High | Medium | Test cutover in staging; measure consumer restart time |
| Bridge performance bottleneck | Medium | Medium | Horizontal bridge scaling; partition-level parallelism |
| Kafka cluster configuration differences | Medium | High | Pre-migration audit of Kafka cluster configuration |
| Customer-specific Kafka extensions | Medium | Medium | Pre-migration compatibility assessment |
| Rollback fails after cutover | Critical | Low | Test rollback procedure before cutover; keep Kafka running |
| Data ordering differences between Kafka and Keirox | Medium | Low | Validate per-partition ordering; document entity_key mapping |

---

## 15. Evidence Package

The migration evidence package MUST include:

1. Migration bridge architecture documentation.
2. Offset mapping specification.
3. Schema migration tool documentation.
4. Validation engine test results.
5. Dual-read comparison report.
6. Cutover procedure documentation.
7. Rollback procedure documentation.
8. End-to-end migration test report.
9. Migration runbooks.
10. Customer migration guide (public-facing).

---

## 16. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial Enterprise Migration & Bridge Plan. Defines migration bridge architecture, offset synchronization, schema registry migration, dual-write validation, zero-downtime cutover procedures, rollback playbooks, validation suite, runbooks, certification levels, and evidence requirements. |