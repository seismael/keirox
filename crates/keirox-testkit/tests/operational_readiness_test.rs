//! Milestone M1.9 Operational Readiness Integration Test Suite per `KEI-ENG-100` §9.10 and `KEI-ARC-027`.

use keirox_api::{
    ConsumerGroupInspectionReport, HealthProbeService, HealthStatus, StorageStatsReport,
    StreamInspectionReport, TelemetryRegistry,
};
use keirox_core::diagnostics::{DiagnosticCode, DiagnosticEvent, SubsystemTag};
use keirox_core::model::{StreamId, TenantId};

#[test]
fn test_prometheus_and_json_metrics_pipeline() {
    let telemetry = TelemetryRegistry::new();

    // 1. Simulate ingress and write activity
    telemetry.record_ingest(500, 512_000);
    telemetry.record_wal_append(850);
    telemetry.record_wal_append(1150);
    telemetry.set_active_leases(42);
    telemetry.set_watermark(1250);
    telemetry.record_dlq_eviction();
    telemetry.record_segment_sealed();
    telemetry.record_parquet_export();
    telemetry.set_memory_usage(32 * 1024 * 1024);

    let snap = telemetry.snapshot();
    assert_eq!(snap.ingest_messages_total, 500);
    assert_eq!(snap.ingest_bytes_total, 512_000);
    assert_eq!(snap.wal_append_count, 2);
    assert_eq!(snap.wal_append_avg_latency_us, 1000);
    assert_eq!(snap.wal_append_max_latency_us, 1150);
    assert_eq!(snap.active_leases_count, 42);
    assert_eq!(snap.watermark_offset, 1250);
    assert_eq!(snap.dlq_evictions_total, 1);
    assert_eq!(snap.segments_sealed_total, 1);
    assert_eq!(snap.parquet_files_exported_total, 1);
    assert_eq!(snap.memory_usage_bytes, 32 * 1024 * 1024);

    // 2. Validate Prometheus text exposition format
    let prom = telemetry.render_prometheus();
    assert!(prom.contains("keirox_ingest_messages_total 500"));
    assert!(prom.contains("keirox_ingest_bytes_total 512000"));
    assert!(prom.contains("keirox_wal_append_operations_total 2"));
    assert!(prom.contains("keirox_wal_append_latency_avg_microseconds 1000"));
    assert!(prom.contains("keirox_active_leases_count 42"));
    assert!(prom.contains("keirox_watermark_offset 1250"));
    assert!(prom.contains("keirox_dlq_evictions_total 1"));
    assert!(prom.contains("keirox_segments_sealed_total 1"));
    assert!(prom.contains("keirox_parquet_files_exported_total 1"));
    assert!(prom.contains("keirox_memory_usage_bytes 33554432"));

    // 3. Validate structured JSON output
    let json = telemetry.render_json();
    assert!(json.contains(r#""ingest_messages_total":500"#));
    assert!(json.contains(r#""wal_append_avg_latency_us":1000"#));
    assert!(json.contains(r#""active_leases_count":42"#));
}

#[test]
fn test_health_probes_and_readiness_lifecycle() {
    let probe = HealthProbeService::new();

    // Initial state: Healthy & Serviceable
    let live = probe.check_live();
    assert_eq!(live.status, HealthStatus::Healthy);
    assert!(live.is_serviceable());

    let ready = probe.check_ready();
    assert_eq!(ready.status, HealthStatus::Healthy);
    assert!(ready.is_serviceable());
    assert!(ready.render_json().contains(r#""status":"HEALTHY""#));

    // Draining transition (e.g. graceful node rollout)
    probe.set_draining(true);
    let ready_draining = probe.check_ready();
    assert_eq!(ready_draining.status, HealthStatus::Degraded);
    assert!(ready_draining.is_serviceable());
    assert!(ready_draining
        .details
        .iter()
        .any(|d| d.contains("draining")));

    // Storage failure transition
    probe.set_draining(false);
    probe.set_storage_healthy(false);
    let ready_storage_fault = probe.check_ready();
    assert_eq!(ready_storage_fault.status, HealthStatus::Unhealthy);
    assert!(!ready_storage_fault.is_serviceable());
    assert!(ready_storage_fault
        .details
        .iter()
        .any(|d| d.contains("Storage WAL engine is not writable")));

    // State plane failure transition
    probe.set_storage_healthy(true);
    probe.set_state_plane_healthy(false);
    let ready_state_fault = probe.check_ready();
    assert_eq!(ready_state_fault.status, HealthStatus::Unhealthy);
    assert!(!ready_state_fault.is_serviceable());
}

#[test]
fn test_administrative_introspection_service() {
    let stream_report = StreamInspectionReport {
        tenant_id: TenantId([1; 16]),
        stream_id: StreamId([42; 16]),
        current_sequence: 10_000,
        base_offset: 0,
        segment_sequence: 3,
        sparse_index_count: 25,
    };

    let stream_json = stream_report.render_json();
    assert!(stream_json.contains("tenant-01010101010101010101010101010101"));
    assert!(stream_json.contains("stream-2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a"));
    assert!(stream_json.contains(r#""current_sequence":10000"#));
    assert!(stream_json.contains(r#""sparse_index_count":25"#));

    let cg_report = ConsumerGroupInspectionReport {
        tenant_id: TenantId([1; 16]),
        group_id: "order-billing-workers".to_string(),
        stream_id: StreamId([42; 16]),
        watermark_base: 9_950,
        leased_count: 15,
        acked_count: 9_935,
        dlq_evicted_count: 1,
        dlq_sample_offsets: vec![342],
    };

    let cg_json = cg_report.render_json();
    assert!(cg_json.contains(r#""group_id":"order-billing-workers""#));
    assert!(cg_json.contains(r#""watermark_base":9950"#));
    assert!(cg_json.contains(r#""dlq_sample_offsets":[342]"#));

    let storage_report = StorageStatsReport {
        active_segment_id: 3,
        sealed_segments_count: 2,
        total_bytes_appended: 128 * 1024 * 1024,
        sparse_index_count: 50,
    };

    let storage_json = storage_report.render_json();
    assert!(storage_json.contains(r#""active_segment_id":3"#));
    assert!(storage_json.contains(r#""sealed_segments_count":2"#));
}

#[test]
fn test_diagnostic_codes_and_event_tracer() {
    let codes = [
        (DiagnosticCode::InvalidBatchHeader, "KEI-ERR-001"),
        (DiagnosticCode::CrcMismatch, "KEI-ERR-002"),
        (DiagnosticCode::StaleLeaseToken, "KEI-ERR-003"),
        (DiagnosticCode::MaxRetriesExceeded, "KEI-ERR-004"),
        (DiagnosticCode::QuotaExceeded, "KEI-ERR-005"),
        (DiagnosticCode::StorageCorruption, "KEI-ERR-006"),
        (DiagnosticCode::WatermarkRegression, "KEI-ERR-007"),
        (DiagnosticCode::SchemaIncompatible, "KEI-ERR-008"),
        (DiagnosticCode::IoFailure, "KEI-ERR-009"),
        (DiagnosticCode::BackpressureEngaged, "KEI-ERR-010"),
    ];

    for (code, expected_str) in codes {
        assert_eq!(code.code_str(), expected_str);
        assert!(!code.default_remediation().is_empty());
    }

    let event = DiagnosticEvent::new(
        DiagnosticCode::BackpressureEngaged,
        SubsystemTag::Ingress,
        "Ingress queue depth exceeded 80%",
        1_700_000_000_000_000_000,
    );

    assert_eq!(event.code, DiagnosticCode::BackpressureEngaged);
    assert_eq!(event.subsystem, SubsystemTag::Ingress);
    assert!(event.to_string().contains("KEI-ERR-010"));
    assert!(event.to_string().contains("INGRESS"));
    assert!(event.to_string().contains("downstream compaction"));
}
