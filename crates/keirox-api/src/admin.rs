//! Safe administrative introspection and state inspection services per `KEI-ARC-027`.

use keirox_core::model::{Offset, StreamId, TenantId};

/// Administrative inspection report for a stream registry entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamInspectionReport {
    /// Owning tenant ID.
    pub tenant_id: TenantId,
    /// Stream ID.
    pub stream_id: StreamId,
    /// Current logical sequence number.
    pub current_sequence: u64,
    /// Base offset in the active segment.
    pub base_offset: Offset,
    /// Active physical segment sequence.
    pub segment_sequence: u32,
    /// Number of sparse index entries recorded.
    pub sparse_index_count: usize,
}

impl StreamInspectionReport {
    /// Render stream inspection as structured JSON.
    #[must_use]
    pub fn render_json(&self) -> String {
        format!(
            r#"{{"tenant_id":"{}","stream_id":"{}","current_sequence":{},"base_offset":{},"segment_sequence":{},"sparse_index_count":{}}}"#,
            self.tenant_id,
            self.stream_id,
            self.current_sequence,
            self.base_offset,
            self.segment_sequence,
            self.sparse_index_count,
        )
    }
}

/// Administrative inspection report for a consumer group consumption state overlay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumerGroupInspectionReport {
    /// Tenant ID.
    pub tenant_id: TenantId,
    /// Consumer group name.
    pub group_id: String,
    /// Stream ID.
    pub stream_id: StreamId,
    /// Sliding base watermark offset ($W_{base}$).
    pub watermark_base: Offset,
    /// Number of currently active message leases.
    pub leased_count: usize,
    /// Total number of ACKed message offsets above $W_{base}$.
    pub acked_count: usize,
    /// Total poison pill offsets evicted to DLQ.
    pub dlq_evicted_count: usize,
    /// Sample of up to 100 evicted DLQ offsets.
    pub dlq_sample_offsets: Vec<Offset>,
}

impl ConsumerGroupInspectionReport {
    /// Render consumer group inspection as structured JSON.
    #[must_use]
    pub fn render_json(&self) -> String {
        let dlq_json = self
            .dlq_sample_offsets
            .iter()
            .map(|o| o.to_string())
            .collect::<Vec<_>>()
            .join(",");

        format!(
            r#"{{"tenant_id":"{}","group_id":"{}","stream_id":"{}","watermark_base":{},"leased_count":{},"acked_count":{},"dlq_evicted_count":{},"dlq_sample_offsets":[{}]}}"#,
            self.tenant_id,
            self.group_id,
            self.stream_id,
            self.watermark_base,
            self.leased_count,
            self.acked_count,
            self.dlq_evicted_count,
            dlq_json,
        )
    }
}

/// Storage engine statistics report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageStatsReport {
    /// Active open segment ID.
    pub active_segment_id: u32,
    /// Total sealed segments.
    pub sealed_segments_count: usize,
    /// Total bytes appended.
    pub total_bytes_appended: u64,
    /// Total sparse index checkpoints.
    pub sparse_index_count: usize,
}

impl StorageStatsReport {
    /// Render storage statistics as JSON.
    #[must_use]
    pub fn render_json(&self) -> String {
        format!(
            r#"{{"active_segment_id":{},"sealed_segments_count":{},"total_bytes_appended":{},"sparse_index_count":{}}}"#,
            self.active_segment_id,
            self.sealed_segments_count,
            self.total_bytes_appended,
            self.sparse_index_count,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_reports_render_json() {
        let stream_report = StreamInspectionReport {
            tenant_id: TenantId([1; 16]),
            stream_id: StreamId([2; 16]),
            current_sequence: 500,
            base_offset: 0,
            segment_sequence: 1,
            sparse_index_count: 5,
        };

        let json = stream_report.render_json();
        assert!(json.contains("tenant-01010101010101010101010101010101"));
        assert!(json.contains(r#""current_sequence":500"#));

        let cg_report = ConsumerGroupInspectionReport {
            tenant_id: TenantId([1; 16]),
            group_id: "order-consumers".to_string(),
            stream_id: StreamId([2; 16]),
            watermark_base: 450,
            leased_count: 10,
            acked_count: 440,
            dlq_evicted_count: 2,
            dlq_sample_offsets: vec![12, 34],
        };

        let cg_json = cg_report.render_json();
        assert!(cg_json.contains(r#""group_id":"order-consumers""#));
        assert!(cg_json.contains(r#""watermark_base":450"#));
        assert!(cg_json.contains(r#""dlq_sample_offsets":[12,34]"#));
    }
}
