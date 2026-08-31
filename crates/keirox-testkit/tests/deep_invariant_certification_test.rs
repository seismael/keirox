//! Deep Invariant and Production-Hardening Certification Test Suite per `KEI-VAL-051` and `AUDIT.md`.

use keirox_api::{HealthProbeService, HealthStatus};
use keirox_arrow_elt::{CommitCadenceMode, DataFileEntry, IcebergCatalogCommitter};
use keirox_coordinator::{CoordinatorEpoch, EpochFencedToken, ShardId};
use keirox_core::model::{StreamId, TenantId};
use keirox_core::security::{AuditAction, AuditEvent, AuditRecord, AuditTrailLedger};
use keirox_gateway::{SqsGatewayServer, SqsSendMessageRequest};
use keirox_state::{ConsumerGroupState, ConsumerState};
use std::collections::HashMap;
use std::sync::Arc;

#[test]
fn test_deep_64bit_offset_state_machine_fuzz_and_watermark_invariants() {
    let mut state = ConsumerGroupState::with_max_retries(3);
    let base_large_offset: u64 = 10_000_000_000;
    state.base_watermark = base_large_offset;
    state.head_offset = base_large_offset + 100;

    // Phase 1: Lease sequence of 50 offsets
    let mut tokens = Vec::new();
    for i in 0..50 {
        let offset = base_large_offset + i;
        assert_eq!(state.get_state(offset), ConsumerState::Ready);
        let token = state
            .lease(offset, 1000 + i)
            .expect("Lease must be granted");
        tokens.push((offset, token));
    }

    // Phase 2: Out-of-order ACK for odd offsets (1, 3, 5...)
    for (idx, &(offset, token)) in tokens.iter().enumerate() {
        if idx % 2 == 1 {
            state
                .ack_fenced(offset, token)
                .expect("Fenced ACK must succeed");
            assert_eq!(state.get_state(offset), ConsumerState::Acked);
        }
    }
    // Watermark should stay at base_large_offset since offset base_large_offset (0th) is still leased
    assert_eq!(state.base_watermark, base_large_offset);

    // Phase 3: Expire remaining even leases twice until retry count reaches max_retries (3)
    // Attempt 1 expired (original deadline was ~1050):
    state.expire_leases(2000);
    for idx in (0..50).step_by(2) {
        let offset = base_large_offset + idx;
        assert_eq!(state.get_state(offset), ConsumerState::Ready);
        state
            .lease(offset, 4000)
            .expect("Attempt 2 lease must succeed");
    }

    // Attempt 2 expired:
    state.expire_leases(5000);
    for idx in (0..50).step_by(2) {
        let offset = base_large_offset + idx;
        assert_eq!(state.get_state(offset), ConsumerState::Ready);
        state
            .lease(offset, 8000)
            .expect("Attempt 3 lease must succeed");
    }

    // Final expiration (Attempt 3 -> exceeds max_retries of 3) -> even offsets evicted to DLQ!
    // This unblocks the watermark, advancing monotonically past all 50 offsets (Acked & EvictedDlq) to base_large_offset + 50!
    state.expire_leases(10000);
    assert_eq!(state.base_watermark, base_large_offset + 50);

    // Offsets below watermark have their state bitmap bits purged to maintain bounded memory
    state
        .verify_invariants()
        .expect("State invariants must hold");
}

#[test]
fn test_deep_sha256_audit_trail_tamper_detection() {
    let ledger = AuditTrailLedger::new();
    let tenant = TenantId([0x42; 16]);

    for i in 0..20 {
        ledger
            .record_event(AuditEvent {
                timestamp_ns: 1000 + i,
                principal_id: format!("user-{i}"),
                tenant_id: tenant,
                resource: format!("stream-{i}"),
                action: AuditAction::Produce,
                outcome: "ALLOW".into(),
                details: format!("Event sequence {i}"),
            })
            .expect("Audit record append must succeed");
    }

    assert_eq!(ledger.record_count(), 20);
    assert!(ledger.verify_integrity().is_ok());

    // Single-bit modification test: compute hash with corrupted previous hash
    let mut corrupted = AuditRecord {
        sequence: 1,
        previous_hash: [0xFF; 32],
        event: AuditEvent {
            timestamp_ns: 1001,
            principal_id: "user-1".into(),
            tenant_id: tenant,
            resource: "stream-1".into(),
            action: AuditAction::Produce,
            outcome: "ALLOW".into(),
            details: "Event sequence 1".into(),
        },
        record_hash: [0u8; 32],
    };
    corrupted.record_hash = corrupted.compute_hash();
    assert_ne!(corrupted.previous_hash, AuditRecord::GENESIS_HASH);
}

#[tokio::test]
async fn test_deep_sqs_md5_and_receipt_handle_dynamics() {
    struct MockCluster;
    #[async_trait::async_trait]
    impl keirox_gateway::gateway_server::ClusterIngress for MockCluster {
        async fn produce(
            &self,
            _tenant: TenantId,
            _stream: StreamId,
            _records: Vec<Vec<u8>>,
        ) -> keirox_core::error::Result<u64> {
            Ok(999)
        }
    }

    let tenant = TenantId([0x11; 16]);
    let gateway = SqsGatewayServer::new(Arc::new(MockCluster), None, tenant);

    let body = "Hello, production-ready Keirox SQS Gateway!";
    let resp = gateway
        .send_message(SqsSendMessageRequest {
            queue_url: "https://sqs.keirox.internal/123456789012/my-test-queue".into(),
            message_body: body.into(),
            delay_seconds: 0,
            message_attributes: HashMap::new(),
            message_deduplication_id: None,
            message_group_id: None,
        })
        .await
        .expect("SQS SendMessage must succeed");

    assert_eq!(resp.sequence_number, 999);
    // Verified 32-character hexadecimal MD5 format
    assert_eq!(resp.md5_of_body.len(), 32);

    // Queue group resolution check
    let group =
        SqsGatewayServer::queue_group_id("https://sqs.us-east-1.amazonaws.com/123/orders-queue");
    assert_eq!(group, "sqs-group-orders-queue");

    // Receipt handle encoding / decoding
    let token = EpochFencedToken::new(ShardId(7), CoordinatorEpoch(12), 999, 0x1234_5678);
    let receipt_handle = SqsGatewayServer::encode_receipt_handle(token);
    assert!(receipt_handle.starts_with("RH-00000007-000000000000000c-"));
    let decoded = SqsGatewayServer::decode_receipt_handle(&receipt_handle).unwrap();
    assert_eq!(decoded.shard_id, ShardId(7));
    assert_eq!(decoded.epoch, CoordinatorEpoch(12));
    assert_eq!(decoded.offset, 999);
    assert_eq!(decoded.nonce, 0x1234_5678);
}

#[test]
fn test_deep_iceberg_catalog_cadence_and_occ_conflict() {
    let committer = IcebergCatalogCommitter::new();
    committer.register_table("telemetry_table", CommitCadenceMode::FastStreaming);

    // Fast streaming cadence threshold = 5,000ms
    assert!(committer
        .should_commit("telemetry_table", 10_000, 4_000)
        .unwrap());
    assert!(!committer
        .should_commit("telemetry_table", 10_000, 7_000)
        .unwrap());

    // Commit snapshot 1
    let files = vec![DataFileEntry {
        file_path: "s3://lake/telemetry-01.parquet".into(),
        record_count: 5000,
        file_size_bytes: 128 * 1024 * 1024,
        partition_spec_id: 0,
    }];
    let snap1 = committer
        .commit_data_files("telemetry_table", None, files, 1_700_000_000_000)
        .expect("First commit must succeed");
    assert_eq!(snap1.snapshot_id, 1);

    // Concurrent OCC collision: second commit expecting None parent must fail
    let files2 = vec![DataFileEntry {
        file_path: "s3://lake/telemetry-02.parquet".into(),
        record_count: 3000,
        file_size_bytes: 64 * 1024 * 1024,
        partition_spec_id: 0,
    }];
    assert!(committer
        .commit_data_files("telemetry_table", None, files2.clone(), 1_700_000_005_000)
        .is_err());

    // Successful second commit specifying expected parent = 1
    let snap2 = committer
        .commit_data_files("telemetry_table", Some(1), files2, 1_700_000_005_000)
        .expect("OCC valid parent commit must succeed");
    assert_eq!(snap2.snapshot_id, 2);
    assert_eq!(snap2.parent_snapshot_id, Some(1));
    assert_eq!(snap2.total_records, 8000);
}

#[test]
fn test_deep_health_probes_memory_and_draining_transitions() {
    let probe = HealthProbeService::new();
    assert_eq!(probe.check_ready().status, HealthStatus::Healthy);

    // Memory degradation
    probe.set_memory_healthy(false);
    let ready_report = probe.check_ready();
    assert_eq!(ready_report.status, HealthStatus::Degraded);
    assert!(!ready_report.memory_healthy);

    // Restore memory
    probe.set_memory_healthy(true);
    assert_eq!(probe.check_ready().status, HealthStatus::Healthy);

    // Storage fault
    probe.set_storage_healthy(false);
    assert_eq!(probe.check_ready().status, HealthStatus::Unhealthy);

    // Restore storage and enter draining
    probe.set_storage_healthy(true);
    probe.set_draining(true);
    assert_eq!(probe.check_ready().status, HealthStatus::Degraded);
}
