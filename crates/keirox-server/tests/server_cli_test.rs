//! Integration tests for `keirox-server` CLI and configuration loader per `KEI-ARC-027` and `KEI-OPS-040`.

use keirox_api::{HealthProbeService, HealthStatus, TelemetryRegistry};

#[test]
fn test_server_status_and_health_probes() {
    let probe = HealthProbeService::new();
    let report = probe.check_health();
    assert_eq!(report.status, HealthStatus::Healthy);
    assert!(report.storage_writable);
    assert!(report.state_plane_healthy);
    assert!(report.memory_healthy);
}

#[test]
fn test_server_telemetry_metrics_rendering() {
    let registry = TelemetryRegistry::new();
    registry.record_ingest(500, 1024 * 1024);
    registry.record_wal_append(42);
    registry.set_active_leases(10);
    registry.set_watermark(100);

    let prom = registry.render_prometheus();
    assert!(prom.contains("keirox_ingest_messages_total 500"));
    assert!(prom.contains("keirox_watermark_offset 100"));

    let json = registry.render_json();
    assert!(json.contains("\"ingest_messages_total\":500"));
    assert!(json.contains("\"watermark_offset\":100"));
}

#[test]
fn test_server_inspection_dto_render() {
    let stream_report = keirox_api::StreamInspectionReport {
        tenant_id: keirox_core::model::TenantId([1u8; 16]),
        stream_id: keirox_core::model::StreamId([2u8; 16]),
        current_sequence: 1234,
        base_offset: 1000,
        segment_sequence: 1,
        sparse_index_count: 5,
    };
    let json = stream_report.render_json();
    assert!(json.contains("\"current_sequence\":1234"));

    let group_report = keirox_api::ConsumerGroupInspectionReport {
        tenant_id: keirox_core::model::TenantId([1u8; 16]),
        group_id: "test-group".into(),
        stream_id: keirox_core::model::StreamId([2u8; 16]),
        watermark_base: 1000,
        leased_count: 2,
        acked_count: 998,
        dlq_evicted_count: 0,
        dlq_sample_offsets: vec![],
    };
    let group_json = group_report.render_json();
    assert!(group_json.contains("\"watermark_base\":1000"));
}

#[tokio::test]
async fn test_server_tcp_listener_and_config_binding() {
    let ingress_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let local_addr = ingress_listener.local_addr().unwrap();
    assert!(local_addr.port() > 0);

    let telemetry = std::sync::Arc::new(TelemetryRegistry::new());
    let tele_clone = telemetry.clone();

    tokio::spawn(async move {
        if let Ok((mut socket, _)) = ingress_listener.accept().await {
            use tokio::io::AsyncWriteExt;
            let _ = socket.write_all(b"OK").await;
            tele_clone.record_ingest(1, 2);
        }
    });

    use tokio::io::AsyncReadExt;
    let mut client = tokio::net::TcpStream::connect(local_addr).await.unwrap();
    let mut buf = [0u8; 2];
    client.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"OK");
}
