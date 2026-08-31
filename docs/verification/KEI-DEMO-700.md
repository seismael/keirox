# KEI-DEMO-700 — End-to-End Demo Scenarios & Acceptance Testing
## Real-World Application Testing for Enterprise Adopters

---

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-DEMO-700 |
| Title | End-to-End Demo Scenarios & Acceptance Testing |
| Version | 1.0 |
| Level | Verification & Acceptance |
| Status | Baseline — Ready for Execution |
| Purpose | Prove Keirox works in real-world adoption scenarios through detailed, reproducible demo workflows |
| Governing Documents | KEI-ARC-001..027, KEI-DES-030..036, KEI-OPS-040..041, KEI-VER-001 |
| Audience | Solutions engineers, design partners, QA teams, product managers, executive stakeholders |

---

## 2. Purpose and Philosophy

This document defines **10 real-world demo scenarios** that mirror exactly how enterprise adopters will use Keirox in production. Each scenario is:

- **Business-grounded:** Based on actual enterprise use cases.
- **End-to-end:** Covers producer → processing → consumption → analytics → operations.
- **Reproducible:** Every step includes exact CLI commands, API calls, and expected outputs.
- **Verifiable:** Every scenario has explicit acceptance criteria that must pass.
- **Adversarial:** Includes failure injection, rollback, and edge cases.

These demos are NOT unit tests. They are **adoption simulations** — they prove that a customer can deploy Keirox, migrate their workloads, and operate them confidently.

### Demo Environment Prerequisites

Before running any demo:

```bash
# 1. Deploy a 3-node Keirox cluster via Helm
helm install keirox-prod ./charts/keirox \
  --set cluster.replicas=3 \
  --set storage.wal.storageClassName=local-nvme \
  --set storage.wal.size=100Gi \
  --set storage.tier1.provider=aws-s3 \
  --set storage.tier1.bucket=keirox-demo-tier1 \
  --set security.tls.enabled=true \
  --set lakehouse.enabled=true

# 2. Verify cluster health
keirox cluster status
# Expected: 3 nodes healthy, Raft quorum formed, leader elected

# 3. Verify REST API is accessible
curl -s https://keirox.example.com/api/v1/health/healthz
# Expected: {"status":"healthy","version":"1.0.0"}

# 4. Verify Web Console is accessible
open https://keirox-console.example.com
# Expected: Cluster Overview dashboard loads
```

---

## 3. Demo Scenario 1: E-Commerce Order Processing Pipeline

### 3.1 Business Context

An e-commerce platform needs to:
- Ingest 10,000 orders/second during peak sales.
- Process payments asynchronously via a task queue.
- Track order status in real-time via stream replay.
- Analyze sales data in the lakehouse for business intelligence.
- Handle failed payments via a dead-letter queue for manual review.

### 3.2 Architecture

```text
┌──────────────┐     ┌──────────────────────────────────────────────────┐
│ Order Service│────►│              KEIROX CLUSTER                      │
│ (Producer)   │     │                                                  │
│ 10K orders/s │     │  Stream: orders                                  │
└──────────────┘     │  ├──► Payment Workers (Queue: lease/ACK)        │
                     │  ├──► Order Tracker (Stream: fetch by offset)    │
                     │  ├──► Fraud Detector (Queue: lease/ACK)          │
                     │  └──► Iceberg Table: orders (Analytics)          │
                     │                                                  │
                     │  DLQ: payment-failures (Manual Review)           │
                     └──────────────────────────────────────────────────┘
```

### 3.3 Step-by-Step Execution

#### Step 1: Create Stream and Consumer Groups

```bash
# Create the orders stream
keirox stream create orders \
  --tenant acme \
  --schema-policy INFERRED \
  --retention-tier0 24h \
  --retention-tier1 90d

# Expected output:
# Stream 'orders' created successfully.
# Stream ID: str-a1b2c3d4
# Schema Policy: INFERRED
# Retention: Tier-0 24h, Tier-1 90d

# Create payment processing consumer group
keirox group create payment-processor \
  --stream orders \
  --ack-mode ACK_FAST \
  --max-retries 3 \
  --lease-ttl 30s

# Create order tracking consumer group
keirox group create order-tracker \
  --stream orders \
  --ack-mode ACK_FAST \
  --max-retries 1 \
  --lease-ttl 60s

# Create fraud detection consumer group
keirox group create fraud-detector \
  --stream orders \
  --ack-mode ACK_FAST \
  --max-retries 5 \
  --lease-ttl 120s
```

#### Step 2: Produce Orders

```bash
# Produce a single order
keirox stream append orders \
  --key "order-12345" \
  --payload '{
    "order_id": "order-12345",
    "customer_id": "cust-9876",
    "amount": 149.99,
    "currency": "USD",
    "items": [
      {"sku": "WIDGET-A", "qty": 2, "price": 49.99},
      {"sku": "GADGET-B", "qty": 1, "price": 50.01}
    ],
    "status": "PENDING",
    "created_at": "2026-08-30T14:30:00Z"
  }'

# Expected output:
# Appended to stream 'orders' at offset 42.
# Offset: 42
# Stream: orders
# Key: order-12345

# Produce 10,000 orders at scale (load test)
keirox bench produce orders \
  --rate 10000 \
  --duration 60s \
  --payload-size 1KB \
  --key-pattern "order-{seq}"

# Expected output:
# Produced 600,000 messages in 60s.
# Throughput: 10,000 msg/s (10 MB/s)
# Latency p50: 0.8ms, p99: 1.9ms
# Errors: 0
```

#### Step 3: Process Payments (Queue Worker)

```bash
# Start a payment worker that leases and processes orders
keirox worker run payment-processor \
  --stream orders \
  --handler "python payment_handler.py" \
  --concurrency 10 \
  --lease-ttl 30s

# payment_handler.py:
# def handle(message):
#     order = json.loads(message.payload)
#     result = charge_payment(order["customer_id"], order["amount"])
#     if result.success:
#         return ACK
#     else:
#         return NACK  # Will retry up to 3 times, then DLQ

# Expected output (per worker):
# Worker payment-processor-1 started.
# Leased 10 messages from stream 'orders'.
# Processing order-12345... Payment successful. ACK.
# Processing order-12346... Payment failed. NACK (retry 1/3).
# Processing order-12347... Payment successful. ACK.
```

#### Step 4: Track Orders in Real-Time (Stream Consumer)

```bash
# Start a stream consumer that tracks order status
keirox stream consume orders \
  --group order-tracker \
  --from-offset 0 \
  --format json

# Expected output (continuous stream):
# {"offset": 42, "key": "order-12345", "payload": {"status": "PENDING", ...}}
# {"offset": 43, "key": "order-12346", "payload": {"status": "PENDING", ...}}
# {"offset": 44, "key": "order-12347", "payload": {"status": "PENDING", ...}}
# ...
```

#### Step 5: Inspect Failed Payments (DLQ)

```bash
# List DLQ entries for failed payments
keirox dlq list --stream orders --group payment-processor

# Expected output:
# ┌────────────┬──────────────┬────────────┬────────────┬──────────────────┐
# │ Entry ID   │ Stream       │ Offset     │ Retries    │ Reason           │
# ├────────────┼──────────────┼────────────┼────────────┼──────────────────┤
# │ dlq-001    │ orders       │ 156        │ 3/3        │ Payment declined │
# │ dlq-002    │ orders       │ 892        │ 3/3        │ Card expired     │
# │ dlq-003    │ orders       │ 1204       │ 3/3        │ Insufficient fund│
# └────────────┴──────────────┴────────────┴────────────┴──────────────────┘
# Total: 3 entries

# Inspect a specific DLQ entry
keirox dlq inspect dlq-001

# Expected output:
# Entry ID: dlq-001
# Stream: orders
# Offset: 156
# Retries: 3/3
# Last Error: Payment declined by processor (code: CARD_DECLINED)
# Payload: {"order_id": "order-12500", "amount": 299.99, ...}
# Evicted At: 2026-08-30T14:35:22Z

# Redrive a DLQ entry (after fixing the issue)
keirox dlq redrive dlq-001

# Expected output:
# Entry dlq-001 redriven to stream 'orders'.
# New offset: 600123
# Retry count reset to 0.
```

#### Step 6: Query Sales Data (Lakehouse)

```bash
# Wait for Iceberg commit (default freshness ≤60s)
sleep 60

# Query order data via DuckDB
duckdb -c "
SELECT
  date_trunc('hour', _keirox_ingest_time) AS hour,
  COUNT(*) AS total_orders,
  SUM(CAST(json_extract_string(payload, '$.amount') AS DOUBLE)) AS total_revenue,
  AVG(CAST(json_extract_string(payload, '$.amount') AS DOUBLE)) AS avg_order_value
FROM read_parquet('s3://keirox-demo-tier1/tenant-acme/events/*.parquet')
WHERE _keirox_stream_id = 'orders'
GROUP BY 1
ORDER BY 1;
"

# Expected output:
# ┌─────────────────────┬──────────────┬────────────────┬───────────────────┐
# │ hour                │ total_orders │ total_revenue  │ avg_order_value   │
# ├─────────────────────┼──────────────┼────────────────┼───────────────────┤
# │ 2026-08-30 14:00:00 │ 600000       │ 89940000.00    │ 149.90            │
# └─────────────────────┴──────────────┴────────────────┴───────────────────┘
```

#### Step 7: Verify via Web Console

```text
1. Open https://keirox-console.example.com
2. Navigate to Streams → orders
3. Verify:
   - Throughput graph shows ~10,000 msg/s
   - Active leases show payment-processor workers
   - Watermark is advancing
   - DLQ count matches CLI output
4. Navigate to DLQ Manager
5. Verify failed payments are visible with payload preview
6. Navigate to Lakehouse Explorer
7. Verify Iceberg table 'orders' shows recent snapshots
```

### 3.4 Acceptance Criteria

| ID | Criterion | Verification Method |
|---|---|---|
| DEMO-1-ACC-001 | 10,000 orders/s ingested without errors | Bench output shows 0 errors |
| DEMO-1-ACC-002 | Payment workers process orders via lease/ACK | Worker logs show ACK/NACK |
| DEMO-1-ACC-003 | Failed payments land in DLQ after 3 retries | DLQ list shows entries |
| DEMO-1-ACC-004 | DLQ entries are inspectable with payload | DLQ inspect shows payload |
| DEMO-1-ACC-005 | DLQ redrive requeues entry | Redrive output shows new offset |
| DEMO-1-ACC-006 | Stream consumer receives all orders in order | Stream consume output is sequential |
| DEMO-1-ACC-007 | Iceberg table is queryable within 60 seconds | DuckDB query returns results |
| DEMO-1-ACC-008 | Web Console shows real-time stream state | Console displays throughput and leases |
| DEMO-1-ACC-009 | Write latency p99 ≤ 2ms | Bench output shows p99 |
| DEMO-1-ACC-010 | Zero data loss (produced = consumed + DLQ) | Count reconciliation |

---

## 4. Demo Scenario 2: IoT Telemetry Ingestion at Scale

### 4.1 Business Context

A manufacturing company has 50,000 sensors producing telemetry every second. They need to:
- Ingest 50,000 messages/second (small payloads ~200 bytes).
- Store data cost-efficiently in the lakehouse.
- Query historical sensor data for predictive maintenance.
- Handle schema evolution as sensors are upgraded.

### 4.2 Step-by-Step Execution

#### Step 1: Create Stream with IoT-Optimized Settings

```bash
keirox stream create sensor-telemetry \
  --tenant factory \
  --schema-policy REGISTERED \
  --schema-id sch-sensor-v1 \
  --retention-tier0 6h \
  --retention-tier1 365d \
  --compaction-target-size 128MB

# Register sensor schema
keirox schema register \
  --name sensor-telemetry \
  --version 1 \
  --definition '{
    "type": "record",
    "fields": [
      {"name": "sensor_id", "type": "string"},
      {"name": "temperature", "type": "double"},
      {"name": "pressure", "type": "double"},
      {"name": "vibration", "type": "double"},
      {"name": "timestamp", "type": "timestamp"}
    ]
  }'
```

#### Step 2: Produce Sensor Data (Micro-Batched)

```bash
# Produce 50,000 sensor readings/s (200-byte payloads, micro-batched)
keirox bench produce sensor-telemetry \
  --rate 50000 \
  --duration 300s \
  --payload-size 200B \
  --key-pattern "sensor-{seq % 50000}" \
  --batch-size 100 \
  --schema-id sch-sensor-v1

# Expected output:
# Produced 15,000,000 messages in 300s.
# Throughput: 50,000 msg/s (10 MB/s)
# Batch efficiency: 100 messages/batch → 500 batches/s
# Latency p50: 1.2ms, p99: 2.8ms
# WAL write amplification: 1.1 (micro-batching reduces overhead)
```

#### Step 3: Schema Evolution (Sensor Upgrade)

```bash
# Register new schema version with additional fields
keirox schema evolve sensor-telemetry \
  --version 2 \
  --add-field '{"name": "humidity", "type": "double", "default": null}' \
  --add-field '{"name": "firmware_version", "type": "string", "default": "1.0"}'

# Expected output:
# Schema 'sensor-telemetry' evolved to version 2.
# Added fields: humidity (double, nullable), firmware_version (string, nullable)
# Backward compatible: YES
# Historical data: readable (new fields return NULL)

# Produce data with new schema
keirox stream append sensor-telemetry \
  --schema-id sch-sensor-v2 \
  --payload '{
    "sensor_id": "sensor-42",
    "temperature": 72.5,
    "pressure": 1013.25,
    "vibration": 0.02,
    "humidity": 45.2,
    "firmware_version": "2.1",
    "timestamp": "2026-08-30T15:00:00Z"
  }'
```

#### Step 4: Query Historical + New Data

```bash
# Wait for Iceberg commit
sleep 60

# Query all sensor data (both schema versions)
duckdb -c "
SELECT
  json_extract_string(payload, '$.sensor_id') AS sensor_id,
  AVG(CAST(json_extract_string(payload, '$.temperature') AS DOUBLE)) AS avg_temp,
  MAX(CAST(json_extract_string(payload, '$.humidity') AS DOUBLE)) AS max_humidity,
  COUNT(*) AS readings
FROM read_parquet('s3://keirox-demo-tier1/tenant-factory/events/*.parquet')
WHERE _keirox_stream_id = 'sensor-telemetry'
  AND json_extract_string(payload, '$.sensor_id') = 'sensor-42'
GROUP BY 1;
"

# Expected output:
# ┌────────────┬──────────┬──────────────┬──────────┐
# │ sensor_id  │ avg_temp │ max_humidity │ readings │
# ├────────────┼──────────┼──────────────┼──────────┤
# │ sensor-42  │ 72.3     │ 45.2         │ 300      │
# └────────────┴──────────┴──────────────┴──────────┘
# Note: max_humidity is NULL for v1 readings, 45.2 for v2 readings
```

### 4.3 Acceptance Criteria

| ID | Criterion | Verification |
|---|---|---|
| DEMO-2-ACC-001 | 50,000 msg/s ingested with 200-byte payloads | Bench output |
| DEMO-2-ACC-002 | Micro-batching reduces WAL overhead | WAF ≤ 1.2 |
| DEMO-2-ACC-003 | Schema evolution is backward compatible | Old data readable with new schema |
| DEMO-2-ACC-004 | New fields return NULL for old data | DuckDB query shows NULL |
| DEMO-2-ACC-005 | 365-day retention configured | Stream describe shows retention |
| DEMO-2-ACC-006 | Iceberg query spans both schema versions | Query returns mixed data |

---

## 5. Demo Scenario 3: Kafka Migration (Zero-Downtime Cutover)

### 5.1 Business Context

A fintech company runs 20 Kafka topics with 50 consumer groups. They want to migrate to Keirox with:
- Zero downtime.
- Zero data loss.
- Ability to rollback within 5 minutes.

### 5.2 Step-by-Step Execution

#### Step 1: Deploy Migration Bridge

```bash
# Initialize migration bridge from Kafka cluster
keirox migration kafka-init \
  --kafka-bootstrap "kafka-broker-1:9092,kafka-broker-2:9092" \
  --topics "transactions,accounts,notifications" \
  --keirox-cluster "keirox-prod" \
  --offset-sync-interval 5s

# Expected output:
# Migration bridge initialized.
# Source: kafka-broker-1:9092 (3 topics)
# Target: keirox-prod
# Topics mapped:
#   kafka:transactions    → keirox:transactions
#   kafka:accounts        → keirox:accounts
#   kafka:notifications   → keirox:notifications
# Offset sync: every 5 seconds
# Bridge status: RUNNING
```

#### Step 2: Validate Data Consistency (Dual-Read)

```bash
# Run dual-read comparison for 10 minutes
keirox migration kafka-validate \
  --duration 600s \
  --sample-rate 0.1 \
  --compare-fields key,value,timestamp

# Expected output:
# Dual-read validation running (10% sample)...
# Compared 1,234,567 records.
# Match rate: 100.000%
# Key match: 100.000%
# Value match: 100.000%
# Timestamp delta: max 0.3s (within 1s threshold)
# Ordering match: 100.000%
# Result: PASS — Safe to proceed with cutover.
```

#### Step 3: Schema Registry Migration

```bash
# Migrate schemas from Confluent Schema Registry
keirox migration schema-import \
  --source-url "https://schema-registry:8081" \
  --subjects "transactions-value,accounts-value,notifications-value"

# Expected output:
# Schema migration complete.
# Imported 3 subjects, 12 versions.
# All schemas backward compatible.
# Keirox schema IDs:
#   transactions-value → sch-txn-v4
#   accounts-value     → sch-acct-v2
#   notifications-value → sch-notif-v7
```

#### Step 4: Consumer Cutover

```bash
# Execute consumer cutover (switch from Kafka to Keirox)
keirox migration kafka-cutover \
  --consumer-groups "payment-processor,account-service,notification-sender" \
  --dry-run

# Expected output (dry run):
# Cutover plan:
#   payment-processor: Kafka offset 45,230 → Keirox offset 45,230
#   account-service: Kafka offset 12,891 → Keirox offset 12,891
#   notification-sender: Kafka offset 98,442 → Keirox offset 98,442
# Estimated downtime: 0s (zero-downtime cutover)
# Rollback available: YES
# Proceed? [y/N]

# Execute actual cutover
keirox migration kafka-cutover \
  --consumer-groups "payment-processor,account-service,notification-sender" \
  --confirm

# Expected output:
# Cutover initiated...
# payment-processor: Switched to Keirox. Resumed from offset 45,230. ✓
# account-service: Switched to Keirox. Resumed from offset 12,891. ✓
# notification-sender: Switched to Keirox. Resumed from offset 98,442. ✓
# Cutover complete. All consumers reading from Keirox.
# Kafka cluster remains available as fallback.
```

#### Step 5: Verify Post-Cutover

```bash
# Verify consumers are processing from Keirox
keirox group describe payment-processor

# Expected output:
# Group: payment-processor
# Stream: transactions
# Committed Offset: 45,287
# Active Leases: 12
# Lag: 0
# Status: HEALTHY

# Verify no data loss
keirox migration kafka-verify-offsets

# Expected output:
# Offset verification:
#   payment-processor: Kafka 45,230 → Keirox 45,287 (57 new messages processed) ✓
#   account-service: Kafka 12,891 → Keirox 12,903 (12 new messages processed) ✓
#   notification-sender: Kafka 98,442 → Keirox 98,450 (8 new messages processed) ✓
# Data loss: 0 messages
# Result: PASS
```

#### Step 6: Rollback Test

```bash
# Simulate rollback (revert to Kafka)
keirox migration kafka-rollback \
  --consumer-groups "payment-processor" \
  --confirm

# Expected output:
# Rollback initiated for payment-processor...
# payment-processor: Switched back to Kafka. Resumed from offset 45,287. ✓
# Rollback complete in 3.2 seconds.
# Keirox remains available. Bridge re-syncing offsets.

# Rollback time: 3.2s (within 5-minute SLA)
```

### 5.3 Acceptance Criteria

| ID | Criterion | Verification |
|---|---|---|
| DEMO-3-ACC-001 | Migration bridge syncs data without loss | Dual-read validation 100% match |
| DEMO-3-ACC-002 | Schema registry migrated with all versions | Schema import output |
| DEMO-3-ACC-003 | Consumer cutover is zero-downtime | No gap in consumer processing |
| DEMO-3-ACC-004 | Offsets are correctly translated | Offset verification passes |
| DEMO-3-ACC-005 | Rollback completes in < 5 minutes | Rollback time measured |
| DEMO-3-ACC-006 | No data loss during cutover or rollback | Count reconciliation |

---

## 6. Demo Scenario 4: GDPR Erasure (Crypto-Shredding)

### 6.1 Business Context

A European customer exercises their GDPR Article 17 right to erasure. The platform must:
- Cryptographically erase all data associated with the customer.
- Prove erasure is irreversible.
- Ensure backups do not resurrect erased data.
- Generate an erasure proof for legal compliance.

### 6.2 Step-by-Step Execution

#### Step 1: Identify Customer Data

```bash
# Find all streams containing customer data
keirox stream list --tenant acme --filter "customer_id=cust-9876"

# Expected output:
# Streams containing customer_id=cust-9876:
#   orders (1,234 records)
#   payments (856 records)
#   support-tickets (12 records)
# Total: 2,102 records across 3 streams
```

#### Step 2: Initiate Erasure

```bash
# Initiate crypto-shredding for customer
keirox admin erasure \
  --tenant acme \
  --customer-id cust-9876 \
  --reason "GDPR Article 17 request" \
  --ticket-id "ERASURE-2026-0830-001" \
  --approver "compliance-officer@acme.com" \
  --confirm

# Expected output:
# Erasure initiated.
# Ticket: ERASURE-2026-0830-001
# Customer: cust-9876
# Streams affected: orders, payments, support-tickets
# Records affected: 2,102
# Keys to destroy: 3 (stream DEKs)
# Status: IN_PROGRESS
```

#### Step 3: Verify Erasure

```bash
# Check erasure status
keirox admin erasure status ERASURE-2026-0830-001

# Expected output:
# Ticket: ERASURE-2026-0830-001
# Status: COMPLETED
# Keys destroyed: 3/3
# Regions propagated: 2/2 (us-east-1, eu-west-1)
# Tombstones written: 3
# Completed at: 2026-08-30T15:30:00Z

# Attempt to read erased data
keirox stream read orders --offset 42

# Expected output:
# ERROR: Data for stream 'orders' at offset 42 is cryptographically erased.
# Error code: ERASED_DATA
# Erasure ticket: ERASURE-2026-0830-001
# This data cannot be recovered.

# Verify destroyed-key registry
keirox admin security destroyed-keys --tenant acme

# Expected output:
# ┌────────────────────┬──────────────┬──────────────────────┬──────────────────┐
# │ Key ID             │ Type         │ Destroyed At         │ Ticket           │
# ├────────────────────┼──────────────┼──────────────────────┼──────────────────┤
# │ dek-orders-acme    │ Stream DEK   │ 2026-08-30T15:29:58Z │ ERASURE-2026-0830│
# │ dek-payments-acme  │ Stream DEK   │ 2026-08-30T15:29:59Z │ ERASURE-2026-0830│
# │ dek-support-acme   │ Stream DEK   │ 2026-08-30T15:30:00Z │ ERASURE-2026-0830│
# └────────────────────┴──────────────┴──────────────────────┴──────────────────┘
```

#### Step 4: Verify Backup Safety

```bash
# Restore from backup taken BEFORE erasure
keirox admin restore \
  --backup-id "backup-2026-08-29" \
  --dry-run

# Expected output:
# Restore plan (dry run):
# Backup: backup-2026-08-29 (taken before erasure)
# Streams to restore: 15
# Streams with destroyed keys: 3 (orders, payments, support-tickets)
# WARNING: 3 streams will remain cryptographically inaccessible.
# Restorable data: 12 streams
# Result: SAFE — Destroyed keys prevent data resurrection.
```

#### Step 5: Generate Erasure Proof

```bash
# Generate compliance erasure proof
keirox admin erasure proof ERASURE-2026-0830-001 --format pdf

# Expected output:
# Erasure proof generated: ERASURE-2026-0830-001.pdf
# Contents:
#   - Erasure ticket details
#   - KMS key destruction receipts (3)
#   - Destroyed-key registry entries
#   - Cross-region propagation confirmation
#   - Tombstone metadata
#   - Audit trail excerpt
#   - Backup interaction verification
# File: /tmp/ERASURE-2026-0830-001.pdf
```

### 6.3 Acceptance Criteria

| ID | Criterion | Verification |
|---|---|---|
| DEMO-4-ACC-001 | Erasure destroys all customer keys | Destroyed-key registry shows 3 keys |
| DEMO-4-ACC-002 | Read after erasure fails securely | Error code ERASED_DATA returned |
| DEMO-4-ACC-003 | Erasure propagates to all regions | Cross-region confirmation |
| DEMO-4-ACC-004 | Backup restore does not resurrect data | Dry-run shows destroyed keys block access |
| DEMO-4-ACC-005 | Erasure proof is generated | PDF contains all required evidence |
| DEMO-4-ACC-006 | Audit trail records erasure | Audit log shows erasure events |

---

## 7. Demo Scenario 5: Multi-Region Disaster Recovery

### 7.1 Business Context

A global SaaS platform runs Keirox in us-east-1 (primary) with a replica in eu-west-1. A regional outage in us-east-1 requires failover to eu-west-1.

### 7.2 Step-by-Step Execution

#### Step 1: Verify Replication Health

```bash
# Check replication status
keirox admin replication status

# Expected output:
# Replication Mode: Mode A (Single-Writer Primary)
# Primary Region: us-east-1
# Replica Region: eu-west-1
# Replication Lag: 2.3 seconds
# Region Epoch: 42
# Status: HEALTHY
# Destroyed-Key Registry: Synchronized
```

#### Step 2: Simulate Regional Outage

```bash
# Simulate us-east-1 outage (kill all primary nodes)
keirox admin chaos inject-region-outage --region us-east-1

# Expected output:
# Region us-east-1 marked as UNAVAILABLE.
# All 3 nodes unreachable.
# Replica region eu-west-1 detected primary failure.
# Failover initiated...
```

#### Step 3: Execute Failover

```bash
# Monitor automatic failover
keirox admin failover status

# Expected output:
# Failover Status: IN_PROGRESS
# Step 1/5: Primary region confirmed unavailable ✓ (2.1s)
# Step 2/5: Region epoch incremented to 43 ✓ (0.3s)
# Step 3/5: Replica promoted to primary ✓ (1.2s)
# Step 4/5: WAL delta recovered (2.3s of data) ✓ (0.8s)
# Step 5/5: Client traffic redirected ✓ (0.5s)
# Total Failover Time: 4.9 seconds
# Data Loss: 2.3 seconds (within 60s RPO target)
# Status: COMPLETE

# Verify new primary accepts writes
keirox stream append orders --key "failover-test" --payload '{"test": true}'

# Expected output:
# Appended to stream 'orders' at offset 600,124.
# Region: eu-west-1 (new primary)
# Epoch: 43
```

#### Step 4: Verify Old Primary is Fenced

```bash
# Attempt write to old primary (should fail)
keirox stream append orders \
  --key "stale-write" \
  --payload '{"test": true}' \
  --region us-east-1

# Expected output:
# ERROR: Write rejected. Region epoch 42 is stale.
# Current epoch: 43
# Error code: STALE_EPOCH
# This region has been fenced. Writes are not accepted.
```

#### Step 5: Verify PITR

```bash
# Perform point-in-time recovery to 10 minutes before failover
keirox admin pitr \
  --timestamp "2026-08-30T15:20:00Z" \
  --target-region eu-west-1 \
  --dry-run

# Expected output:
# PITR Plan (dry run):
# Target timestamp: 2026-08-30T15:20:00Z
# Streams to recover: 15
# Records after target: 4,523 (will be excluded)
# Destroyed keys checked: 3 (will remain inaccessible)
# Estimated recovery time: 45 seconds
# Proceed? [y/N]
```

### 7.3 Acceptance Criteria

| ID | Criterion | Verification |
|---|---|---|
| DEMO-5-ACC-001 | Replication lag ≤ 5 seconds under normal conditions | Replication status output |
| DEMO-5-ACC-002 | Failover completes in ≤ 5 minutes (unplanned) | Failover time measured |
| DEMO-5-ACC-003 | Data loss bounded by RPO (≤60s degraded) | Data loss measured |
| DEMO-5-ACC-004 | Old primary is fenced (writes rejected) | STALE_EPOCH error returned |
| DEMO-5-ACC-005 | New primary accepts writes | Successful append to new primary |
| DEMO-5-ACC-006 | PITR excludes post-target data | Dry-run shows excluded records |

---

## 8. Demo Scenario 6: Task Queue with Priority Workers

### 8.1 Business Context

A video processing platform submits transcoding jobs to Keirox. Workers lease jobs, process them, and ACK. Failed jobs go to DLQ. Operators redrive after fixing issues.

### 8.2 Step-by-Step Execution

```bash
# Create task queue stream
keirox stream create video-jobs \
  --tenant media \
  --retention-tier0 4h

# Create worker group with aggressive retry
keirox group create transcoder-pool \
  --stream video-jobs \
  --ack-mode ACK_FAST \
  --max-retries 2 \
  --lease-ttl 300s

# Submit 100 jobs
for i in $(seq 1 100); do
  keirox stream append video-jobs \
    --key "job-$i" \
    --payload "{\"job_id\": \"job-$i\", \"video_url\": \"s3://videos/$i.mp4\", \"format\": \"h264\"}"
done

# Start 5 workers
for w in $(seq 1 5); do
  keirox worker run transcoder-pool \
    --handler "python transcode.py" \
    --concurrency 1 &
done

# Monitor progress
watch -n 5 "keirox group describe transcoder-pool"

# Expected output (after all jobs processed):
# Group: transcoder-pool
# Stream: video-jobs
# Committed Offset: 100
# Active Leases: 0
# DLQ Count: 3 (failed jobs)
# Status: IDLE (all jobs processed)

# Inspect and redrive failed jobs
keirox dlq list --stream video-jobs
keirox dlq redrive dlq-042
```

### 8.3 Acceptance Criteria

| ID | Criterion | Verification |
|---|---|---|
| DEMO-6-ACC-001 | 100 jobs submitted and leased to workers | Group describe shows processing |
| DEMO-6-ACC-002 | Workers ACK successful jobs | Committed offset advances |
| DEMO-6-ACC-003 | Failed jobs go to DLQ after 2 retries | DLQ count matches failures |
| DEMO-6-ACC-004 | DLQ redrive requeues jobs | Redrive output shows new offset |
| DEMO-6-ACC-005 | No job is processed twice (idempotent) | Job completion count = 100 |

---

## 9. Demo Scenario 7: Real-Time Fraud Detection

### 9.1 Business Context

A bank processes 5,000 transactions/second. Each transaction must be checked for fraud within 100ms. Transactions are also streamed to an audit trail and stored in the lakehouse for regulatory reporting.

### 9.2 Step-by-Step Execution

```bash
# Create transaction stream
keirox stream create transactions \
  --tenant bank \
  --schema-policy REGISTERED \
  --retention-tier0 12h \
  --retention-tier1 2555d  # 7 years for regulatory compliance

# Create fraud detection consumer group (low-latency)
keirox group create fraud-detector \
  --stream transactions \
  --ack-mode ACK_FAST \
  --max-retries 1 \
  --lease-ttl 5s

# Create audit trail consumer group
keirox group create audit-trail \
  --stream transactions \
  --ack-mode ACK_FAST \
  --max-retries 3 \
  --lease-ttl 30s

# Produce transactions at 5,000/s
keirox bench produce transactions \
  --rate 5000 \
  --duration 60s \
  --payload-size 512B \
  --key-pattern "txn-{seq}"

# Expected output:
# Produced 300,000 transactions in 60s.
# Throughput: 5,000 msg/s (2.5 MB/s)
# Latency p99: 1.8ms (within 100ms fraud detection SLA)

# Verify fraud detection latency
keirox group describe fraud-detector

# Expected output:
# Group: fraud-detector
# Stream: transactions
# Committed Offset: 300,000
# Active Leases: 0
# Average Lease-to-ACK Latency: 45ms
# Max Lease-to-ACK Latency: 89ms
# Status: HEALTHY (within 100ms SLA)
```

### 9.3 Acceptance Criteria

| ID | Criterion | Verification |
|---|---|---|
| DEMO-7-ACC-001 | 5,000 transactions/s ingested | Bench output |
| DEMO-7-ACC-002 | Fraud detection latency < 100ms | Group describe shows lease-to-ACK latency |
| DEMO-7-ACC-003 | Audit trail processes all transactions | Committed offset matches produced count |
| DEMO-7-ACC-004 | 7-year retention configured | Stream describe shows 2555d retention |
| DEMO-7-ACC-005 | Lakehouse table queryable for regulatory reports | DuckDB/Spark query succeeds |

---

## 10. Demo Scenario 8: Log Aggregation & Debugging

### 10.1 Business Context

A microservices platform ingests 100,000 log entries/second from 200 services. Developers need to replay logs for debugging, and the analytics team needs to run SQL queries over historical logs.

### 10.2 Step-by-Step Execution

```bash
# Create log stream
keirox stream create application-logs \
  --tenant platform \
  --retention-tier0 2h \
  --retention-tier1 30d

# Produce logs
keirox bench produce application-logs \
  --rate 100000 \
  --duration 60s \
  --payload-size 512B \
  --key-pattern "svc-{seq % 200}"

# Replay logs for a specific service (stream mode)
keirox stream consume application-logs \
  --from-offset 0 \
  --filter "key=svc-42" \
  --limit 100 \
  --format json

# Query logs via lakehouse
duckdb -c "
SELECT
  json_extract_string(payload, '$.level') AS level,
  json_extract_string(payload, '$.service') AS service,
  COUNT(*) AS count
FROM read_parquet('s3://keirox-tier1/tenant-platform/events/*.parquet')
WHERE _keirox_stream_id = 'application-logs'
  AND json_extract_string(payload, '$.level') = 'ERROR'
GROUP BY 1, 2
ORDER BY count DESC
LIMIT 10;
"
```

---

## 11. Demo Scenario 9: Kubernetes Deployment & Day-2 Operations

### 11.1 Business Context

A platform team deploys Keirox on Kubernetes and performs Day-2 operations: scaling, upgrading, node replacement, and monitoring.

### 11.2 Step-by-Step Execution

```bash
# Deploy via Helm
helm install keirox ./charts/keirox -f production-values.yaml

# Verify deployment
kubectl get keiroxcluster
kubectl get pods -l app=keirox-server
kubectl get pdb keirox-server-pdb

# Scale from 3 to 5 nodes
kubectl patch keiroxcluster keirox-prod -p '{"spec":{"replicas":5}}'

# Monitor scaling
kubectl get pods -w
keirox cluster status

# Perform rolling upgrade
helm upgrade keirox ./charts/keirox --set cluster.version=1.1.0

# Monitor upgrade
keirox cluster status
# Expected: Nodes upgraded one at a time, quorum maintained

# Replace a failed node
kubectl delete pod keirox-server-2
keirox cluster status
# Expected: Node replaced automatically, state recovered from S3

# Check Grafana dashboards
open http://grafana.example.com/d/keirox-cluster-overview
```

---

## 12. Demo Scenario 10: End-to-End Supply Chain Verification

### 12.1 Business Context

A security team verifies that the Keirox binary they are deploying is authentic, untampered, and free of known vulnerabilities.

### 12.2 Step-by-Step Execution

```bash
# Download release artifacts
curl -LO https://releases.keirox.io/v1.0.0/keirox-server-v1.0.0-linux-amd64
curl -LO https://releases.keirox.io/v1.0.0/keirox-server-v1.0.0-linux-amd64.sig
curl -LO https://releases.keirox.io/v1.0.0/keirox-server-v1.0.0-linux-amd64.pem

# Verify binary signature
cosign verify-blob \
  --certificate-identity="keirox-release@keirox.io" \
  --certificate-oidc-issuer="https://token.actions.githubusercontent.com" \
  --signature keirox-server-v1.0.0-linux-amd64.sig \
  --certificate keirox-server-v1.0.0-linux-amd64.pem \
  keirox-server-v1.0.0-linux-amd64

# Expected output:
# Verified OK

# Download and verify SBOM
curl -LO https://releases.keirox.io/v1.0.0/keirox-server-v1.0.0-sbom.cdx.json
cat keirox-server-v1.0.0-sbom.cdx.json | jq '.components | length'
# Expected: 142 (number of dependencies)

# Verify SLSA provenance
cosign verify-attestation \
  --type slsaprovenance \
  --certificate-identity="keirox-release@keirox.io" \
  keirox/keirox-server:v1.0.0

# Scan for vulnerabilities
trivy image keirox/keirox-server:v1.0.0
# Expected: 0 CRITICAL, 0 HIGH vulnerabilities

# Verify container image security
docker inspect keirox/keirox-server:v1.0.0 | jq '.[0].Config.User'
# Expected: "65532" (non-root)

docker run --rm keirox/keirox-server:v1.0.0 /bin/sh
# Expected: Error — no shell available (distroless)
```

---

## 13. Demo Execution Summary

| Demo | Scenario | Duration | Critical Checks |
|---|---|---|---|
| Demo 1 | E-Commerce Order Processing | 30 min | 10 |
| Demo 2 | IoT Telemetry at Scale | 20 min | 6 |
| Demo 3 | Kafka Migration (Zero-Downtime) | 45 min | 6 |
| Demo 4 | GDPR Erasure (Crypto-Shredding) | 15 min | 6 |
| Demo 5 | Multi-Region Disaster Recovery | 20 min | 6 |
| Demo 6 | Task Queue with Priority Workers | 15 min | 5 |
| Demo 7 | Real-Time Fraud Detection | 15 min | 5 |
| Demo 8 | Log Aggregation & Debugging | 15 min | 4 |
| Demo 9 | Kubernetes Deployment & Day-2 Ops | 30 min | 8 |
| Demo 10 | Supply Chain Verification | 10 min | 6 |
| **Total** | **10 scenarios** | **~3.5 hours** | **62 acceptance criteria** |

---

## 14. Demo Environment Checklist

Before executing demos, verify:

| # | Item | Status |
|---|---|---|
| 1 | 3-node Keirox cluster deployed and healthy | ☐ |
| 2 | REST API accessible | ☐ |
| 3 | Web Console accessible | ☐ |
| 4 | Grafana dashboards deployed | ☐ |
| 5 | S3/GCS bucket configured | ☐ |
| 6 | Iceberg catalog configured | ☐ |
| 7 | DuckDB/Polars installed | ☐ |
| 8 | Kafka cluster available (for Demo 3) | ☐ |
| 9 | Multi-region replica configured (for Demo 5) | ☐ |
| 10 | Cosign/Trivy installed (for Demo 10) | ☐ |
| 11 | kubectl configured (for Demo 9) | ☐ |
| 12 | Helm installed (for Demo 9) | ☐ |

---

## 15. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial End-to-End Demo Scenarios & Acceptance Testing document. Defines 10 real-world demo scenarios with step-by-step execution, CLI commands, expected outputs, and 62 acceptance criteria covering e-commerce, IoT, Kafka migration, GDPR erasure, disaster recovery, task queues, fraud detection, log aggregation, Kubernetes operations, and supply chain verification. |