//! Machine-readable Prometheus exposition format and JSON metrics telemetry engine per `KEI-ARC-027`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// In-memory lock-free operational telemetry registry for high-throughput counters and gauges.
#[derive(Debug, Default)]
pub struct TelemetryRegistry {
    ingest_messages_total: AtomicU64,
    ingest_bytes_total: AtomicU64,
    wal_append_count: AtomicU64,
    wal_append_latency_sum_us: AtomicU64,
    wal_append_latency_max_us: AtomicU64,
    active_leases_count: AtomicU64,
    watermark_offset: AtomicU64,
    dlq_evictions_total: AtomicU64,
    segments_sealed_total: AtomicU64,
    parquet_files_exported_total: AtomicU64,
    memory_usage_bytes: AtomicU64,
    raft_leader_status: AtomicU64,
    raft_current_term: AtomicU64,
    raft_commit_index: AtomicU64,
    coordinator_epoch: AtomicU64,
    s3_uploaded_bytes_total: AtomicU64,
    s3_backlog_bytes: AtomicU64,
}

/// Point-in-time immutable snapshot of all system telemetry metrics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricSnapshot {
    /// Total ingested message count.
    pub ingest_messages_total: u64,
    /// Total ingested payload bytes.
    pub ingest_bytes_total: u64,
    /// Total WAL batch append operations.
    pub wal_append_count: u64,
    /// Average WAL append latency in microseconds.
    pub wal_append_avg_latency_us: u64,
    /// Maximum observed WAL append latency in microseconds.
    pub wal_append_max_latency_us: u64,
    /// Current number of active consumer leases.
    pub active_leases_count: u64,
    /// Current monotonic sliding base watermark offset.
    pub watermark_offset: u64,
    /// Total messages evicted to virtual DLQ.
    pub dlq_evictions_total: u64,
    /// Total immutable storage segments sealed.
    pub segments_sealed_total: u64,
    /// Total Snappy Parquet files generated.
    pub parquet_files_exported_total: u64,
    /// Current allocated memory usage in bytes.
    pub memory_usage_bytes: u64,
    /// 1 if node is current Raft leader, 0 if follower.
    pub raft_leader_status: u64,
    /// Current consensus term.
    pub raft_current_term: u64,
    /// Current committed Raft log index.
    pub raft_commit_index: u64,
    /// Current active coordinator epoch.
    pub coordinator_epoch: u64,
    /// Total bytes uploaded to Tier-1 S3 storage.
    pub s3_uploaded_bytes_total: u64,
    /// Current NVMe backlog bytes pending S3 upload.
    pub s3_backlog_bytes: u64,
}

impl TelemetryRegistry {
    /// Create a new telemetry registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record message ingress.
    pub fn record_ingest(&self, messages: u64, bytes: u64) {
        self.ingest_messages_total
            .fetch_add(messages, Ordering::Relaxed);
        self.ingest_bytes_total.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record a WAL append operation with measured latency.
    pub fn record_wal_append(&self, latency_us: u64) {
        self.wal_append_count.fetch_add(1, Ordering::Relaxed);
        self.wal_append_latency_sum_us
            .fetch_add(latency_us, Ordering::Relaxed);
        self.wal_append_latency_max_us
            .fetch_max(latency_us, Ordering::Relaxed);
    }

    /// Update active lease gauge.
    pub fn set_active_leases(&self, count: u64) {
        self.active_leases_count.store(count, Ordering::Relaxed);
    }

    /// Update current base watermark offset gauge.
    pub fn set_watermark(&self, offset: u64) {
        self.watermark_offset.store(offset, Ordering::Relaxed);
    }

    /// Increment poison pill DLQ evictions counter.
    pub fn record_dlq_eviction(&self) {
        self.dlq_evictions_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment sealed segments counter.
    pub fn record_segment_sealed(&self) {
        self.segments_sealed_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment Parquet exported files counter.
    pub fn record_parquet_export(&self) {
        self.parquet_files_exported_total
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Update memory usage bytes gauge.
    pub fn set_memory_usage(&self, bytes: u64) {
        self.memory_usage_bytes.store(bytes, Ordering::Relaxed);
    }

    /// Update Raft cluster status gauges.
    pub fn set_raft_status(&self, is_leader: bool, term: u64, commit_index: u64) {
        self.raft_leader_status
            .store(if is_leader { 1 } else { 0 }, Ordering::Relaxed);
        self.raft_current_term.store(term, Ordering::Relaxed);
        self.raft_commit_index
            .store(commit_index, Ordering::Relaxed);
    }

    /// Update active coordinator epoch gauge.
    pub fn set_coordinator_epoch(&self, epoch: u64) {
        self.coordinator_epoch.store(epoch, Ordering::Relaxed);
    }

    /// Record Tier-1 S3 upload.
    pub fn record_s3_upload(&self, bytes: u64) {
        self.s3_uploaded_bytes_total
            .fetch_add(bytes, Ordering::Relaxed);
    }

    /// Update NVMe backlog bytes gauge.
    pub fn set_s3_backlog(&self, bytes: u64) {
        self.s3_backlog_bytes.store(bytes, Ordering::Relaxed);
    }

    /// Take an immutable snapshot of all metrics.
    #[must_use]
    pub fn snapshot(&self) -> MetricSnapshot {
        let count = self.wal_append_count.load(Ordering::Relaxed);
        let sum = self.wal_append_latency_sum_us.load(Ordering::Relaxed);
        let avg_latency = if count > 0 { sum / count } else { 0 };

        MetricSnapshot {
            ingest_messages_total: self.ingest_messages_total.load(Ordering::Relaxed),
            ingest_bytes_total: self.ingest_bytes_total.load(Ordering::Relaxed),
            wal_append_count: count,
            wal_append_avg_latency_us: avg_latency,
            wal_append_max_latency_us: self.wal_append_latency_max_us.load(Ordering::Relaxed),
            active_leases_count: self.active_leases_count.load(Ordering::Relaxed),
            watermark_offset: self.watermark_offset.load(Ordering::Relaxed),
            dlq_evictions_total: self.dlq_evictions_total.load(Ordering::Relaxed),
            segments_sealed_total: self.segments_sealed_total.load(Ordering::Relaxed),
            parquet_files_exported_total: self.parquet_files_exported_total.load(Ordering::Relaxed),
            memory_usage_bytes: self.memory_usage_bytes.load(Ordering::Relaxed),
            raft_leader_status: self.raft_leader_status.load(Ordering::Relaxed),
            raft_current_term: self.raft_current_term.load(Ordering::Relaxed),
            raft_commit_index: self.raft_commit_index.load(Ordering::Relaxed),
            coordinator_epoch: self.coordinator_epoch.load(Ordering::Relaxed),
            s3_uploaded_bytes_total: self.s3_uploaded_bytes_total.load(Ordering::Relaxed),
            s3_backlog_bytes: self.s3_backlog_bytes.load(Ordering::Relaxed),
        }
    }

    /// Render metrics in standard Prometheus exposition format (`text/plain; version=0.0.4`).
    #[must_use]
    pub fn render_prometheus(&self) -> String {
        let snap = self.snapshot();
        let mut out = String::with_capacity(2048);

        out.push_str("# HELP keirox_ingest_messages_total Total messages ingested\n");
        out.push_str("# TYPE keirox_ingest_messages_total counter\n");
        out.push_str(&format!(
            "keirox_ingest_messages_total {}\n\n",
            snap.ingest_messages_total
        ));

        out.push_str("# HELP keirox_ingest_bytes_total Total bytes ingested\n");
        out.push_str("# TYPE keirox_ingest_bytes_total counter\n");
        out.push_str(&format!(
            "keirox_ingest_bytes_total {}\n\n",
            snap.ingest_bytes_total
        ));

        out.push_str(
            "# HELP keirox_wal_append_operations_total Total WAL batch append operations\n",
        );
        out.push_str("# TYPE keirox_wal_append_operations_total counter\n");
        out.push_str(&format!(
            "keirox_wal_append_operations_total {}\n\n",
            snap.wal_append_count
        ));

        out.push_str(
            "# HELP keirox_wal_append_latency_avg_microseconds Average WAL append latency\n",
        );
        out.push_str("# TYPE keirox_wal_append_latency_avg_microseconds gauge\n");
        out.push_str(&format!(
            "keirox_wal_append_latency_avg_microseconds {}\n\n",
            snap.wal_append_avg_latency_us
        ));

        out.push_str("# HELP keirox_wal_append_latency_max_microseconds Max WAL append latency\n");
        out.push_str("# TYPE keirox_wal_append_latency_max_microseconds gauge\n");
        out.push_str(&format!(
            "keirox_wal_append_latency_max_microseconds {}\n\n",
            snap.wal_append_max_latency_us
        ));

        out.push_str("# HELP keirox_active_leases_count Current active consumer leases\n");
        out.push_str("# TYPE keirox_active_leases_count gauge\n");
        out.push_str(&format!(
            "keirox_active_leases_count {}\n\n",
            snap.active_leases_count
        ));

        out.push_str("# HELP keirox_watermark_offset Monotonic base watermark offset\n");
        out.push_str("# TYPE keirox_watermark_offset gauge\n");
        out.push_str(&format!(
            "keirox_watermark_offset {}\n\n",
            snap.watermark_offset
        ));

        out.push_str("# HELP keirox_dlq_evictions_total Total messages evicted to virtual DLQ\n");
        out.push_str("# TYPE keirox_dlq_evictions_total counter\n");
        out.push_str(&format!(
            "keirox_dlq_evictions_total {}\n\n",
            snap.dlq_evictions_total
        ));

        out.push_str(
            "# HELP keirox_segments_sealed_total Total immutable storage segments sealed\n",
        );
        out.push_str("# TYPE keirox_segments_sealed_total counter\n");
        out.push_str(&format!(
            "keirox_segments_sealed_total {}\n\n",
            snap.segments_sealed_total
        ));

        out.push_str("# HELP keirox_parquet_files_exported_total Total Parquet files exported\n");
        out.push_str("# TYPE keirox_parquet_files_exported_total counter\n");
        out.push_str(&format!(
            "keirox_parquet_files_exported_total {}\n\n",
            snap.parquet_files_exported_total
        ));

        out.push_str("# HELP keirox_memory_usage_bytes Current memory usage in bytes\n");
        out.push_str("# TYPE keirox_memory_usage_bytes gauge\n");
        out.push_str(&format!(
            "keirox_memory_usage_bytes {}\n\n",
            snap.memory_usage_bytes
        ));

        out.push_str("# HELP keirox_raft_leader_status 1 if node is cluster leader, 0 otherwise\n");
        out.push_str("# TYPE keirox_raft_leader_status gauge\n");
        out.push_str(&format!(
            "keirox_raft_leader_status {}\n\n",
            snap.raft_leader_status
        ));

        out.push_str("# HELP keirox_raft_current_term Current consensus term\n");
        out.push_str("# TYPE keirox_raft_current_term gauge\n");
        out.push_str(&format!(
            "keirox_raft_current_term {}\n\n",
            snap.raft_current_term
        ));

        out.push_str("# HELP keirox_raft_commit_index Current committed Raft log index\n");
        out.push_str("# TYPE keirox_raft_commit_index gauge\n");
        out.push_str(&format!(
            "keirox_raft_commit_index {}\n\n",
            snap.raft_commit_index
        ));

        out.push_str("# HELP keirox_coordinator_epoch Current active coordinator epoch\n");
        out.push_str("# TYPE keirox_coordinator_epoch gauge\n");
        out.push_str(&format!(
            "keirox_coordinator_epoch {}\n\n",
            snap.coordinator_epoch
        ));

        out.push_str("# HELP keirox_s3_uploaded_bytes_total Total bytes streamed to S3\n");
        out.push_str("# TYPE keirox_s3_uploaded_bytes_total counter\n");
        out.push_str(&format!(
            "keirox_s3_uploaded_bytes_total {}\n\n",
            snap.s3_uploaded_bytes_total
        ));

        out.push_str(
            "# HELP keirox_s3_backlog_bytes Current NVMe backlog bytes pending S3 upload\n",
        );
        out.push_str("# TYPE keirox_s3_backlog_bytes gauge\n");
        out.push_str(&format!(
            "keirox_s3_backlog_bytes {}\n",
            snap.s3_backlog_bytes
        ));

        out
    }

    /// Render metrics as structured JSON.
    #[must_use]
    pub fn render_json(&self) -> String {
        let snap = self.snapshot();
        format!(
            r#"{{"ingest_messages_total":{},"ingest_bytes_total":{},"wal_append_count":{},"wal_append_avg_latency_us":{},"wal_append_max_latency_us":{},"active_leases_count":{},"watermark_offset":{},"dlq_evictions_total":{},"segments_sealed_total":{},"parquet_files_exported_total":{},"memory_usage_bytes":{},"raft_leader_status":{},"raft_current_term":{},"raft_commit_index":{},"coordinator_epoch":{},"s3_uploaded_bytes_total":{},"s3_backlog_bytes":{}}}"#,
            snap.ingest_messages_total,
            snap.ingest_bytes_total,
            snap.wal_append_count,
            snap.wal_append_avg_latency_us,
            snap.wal_append_max_latency_us,
            snap.active_leases_count,
            snap.watermark_offset,
            snap.dlq_evictions_total,
            snap.segments_sealed_total,
            snap.parquet_files_exported_total,
            snap.memory_usage_bytes,
            snap.raft_leader_status,
            snap.raft_current_term,
            snap.raft_commit_index,
            snap.coordinator_epoch,
            snap.s3_uploaded_bytes_total,
            snap.s3_backlog_bytes,
        )
    }
}

/// Shared reference to telemetry registry.
pub type SharedTelemetry = Arc<TelemetryRegistry>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_recording_and_rendering() {
        let registry = TelemetryRegistry::new();
        registry.record_ingest(100, 10240);
        registry.record_wal_append(450);
        registry.record_wal_append(550);
        registry.set_active_leases(25);
        registry.set_watermark(99);
        registry.record_dlq_eviction();
        registry.record_segment_sealed();
        registry.record_parquet_export();
        registry.set_memory_usage(16 * 1024 * 1024);

        let snap = registry.snapshot();
        assert_eq!(snap.ingest_messages_total, 100);
        assert_eq!(snap.ingest_bytes_total, 10240);
        assert_eq!(snap.wal_append_count, 2);
        assert_eq!(snap.wal_append_avg_latency_us, 500);
        assert_eq!(snap.wal_append_max_latency_us, 550);
        assert_eq!(snap.active_leases_count, 25);
        assert_eq!(snap.watermark_offset, 99);
        assert_eq!(snap.dlq_evictions_total, 1);
        assert_eq!(snap.segments_sealed_total, 1);
        assert_eq!(snap.parquet_files_exported_total, 1);
        assert_eq!(snap.memory_usage_bytes, 16 * 1024 * 1024);

        let prom = registry.render_prometheus();
        assert!(prom.contains("keirox_ingest_messages_total 100"));
        assert!(prom.contains("keirox_wal_append_latency_avg_microseconds 500"));
        assert!(prom.contains("keirox_active_leases_count 25"));

        let json = registry.render_json();
        assert!(json.contains(r#""ingest_messages_total":100"#));
        assert!(json.contains(r#""wal_append_avg_latency_us":500"#));
    }
}
