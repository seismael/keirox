//! Structured diagnostic error codes and operational telemetry taxonomy per `KEI-ARC-027` and `KEI-OPS-040`.

use std::fmt;

/// Standardized diagnostic error codes mapping to formal architecture runbooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticCode {
    /// Batch header magic or framing bytes are corrupted.
    InvalidBatchHeader,
    /// Batch payload or record CRC32C check failed.
    CrcMismatch,
    /// Consumer lease token is stale, invalid, or expired.
    StaleLeaseToken,
    /// Offset retry count exceeded max limit; evicted to DLQ.
    MaxRetriesExceeded,
    /// Tenant ingress throughput or lease quota exceeded.
    QuotaExceeded,
    /// Storage segment file or index unrecoverable corruption detected.
    StorageCorruption,
    /// Watermark attempted to advance backwards (violating monotonicity).
    WatermarkRegression,
    /// Record schema incompatible with promoted columnar schema.
    SchemaIncompatible,
    /// Underlying kernel I/O (io_uring / O_DIRECT) returned an unhandled error.
    IoFailure,
    /// Progressive backpressure ladder engaged due to capacity threshold.
    BackpressureEngaged,
    /// Raft quorum could not be achieved within the timeout window.
    RaftQuorumLost,
    /// Coordinator shard epoch is stale; request rejected to prevent split-brain.
    EpochFenced,
    /// Graceful leader transfer timed out.
    RaftLeaderTransferTimeout,
    /// Tier-1 S3 multipart upload failed after max retries.
    Tier1UploadFailed,
    /// Reconstructed state bitmap checksum does not match peer snapshot.
    StateReconciliationMismatch,
    /// Draining node failed to migrate coordinator shards.
    NodeDrainTimeout,
}

impl DiagnosticCode {
    /// Returns the canonical alphanumeric code per architecture specifications.
    #[must_use]
    pub const fn code_str(&self) -> &'static str {
        match self {
            Self::InvalidBatchHeader => "KEI-ERR-001",
            Self::CrcMismatch => "KEI-ERR-002",
            Self::StaleLeaseToken => "KEI-ERR-003",
            Self::MaxRetriesExceeded => "KEI-ERR-004",
            Self::QuotaExceeded => "KEI-ERR-005",
            Self::StorageCorruption => "KEI-ERR-006",
            Self::WatermarkRegression => "KEI-ERR-007",
            Self::SchemaIncompatible => "KEI-ERR-008",
            Self::IoFailure => "KEI-ERR-009",
            Self::BackpressureEngaged => "KEI-ERR-010",
            Self::RaftQuorumLost => "KEI-ERR-011",
            Self::EpochFenced => "KEI-ERR-012",
            Self::RaftLeaderTransferTimeout => "KEI-ERR-013",
            Self::Tier1UploadFailed => "KEI-ERR-014",
            Self::StateReconciliationMismatch => "KEI-ERR-015",
            Self::NodeDrainTimeout => "KEI-ERR-016",
        }
    }

    /// Returns the default remediation hint for operators.
    #[must_use]
    pub const fn default_remediation(&self) -> &'static str {
        match self {
            Self::InvalidBatchHeader => {
                "Verify WAL segment alignment and inspect disk block health."
            }
            Self::CrcMismatch => {
                "Segment corruption detected; run recovery reconciler or inspect NVMe sector."
            }
            Self::StaleLeaseToken => {
                "Consumer lease expired; retry message acquisition with a fresh lease token."
            }
            Self::MaxRetriesExceeded => {
                "Message poison pill evicted to virtual DLQ; inspect poison payload."
            }
            Self::QuotaExceeded => "Scale tenant token bucket or backoff ingress client requests.",
            Self::StorageCorruption => {
                "Isolate faulty node and trigger state reconstruction from peer replicas."
            }
            Self::WatermarkRegression => {
                "Severe invariant violation; check state machine coordinator fencing."
            }
            Self::SchemaIncompatible => {
                "Payload JSON fields diverged from schema; demoted to unstructured field."
            }
            Self::IoFailure => {
                "Inspect kernel dmesg, io_uring queue depth, and filesystem mount options."
            }
            Self::BackpressureEngaged => {
                "Ingress throttled; check downstream compaction and S3 upload backlog."
            }
            Self::RaftQuorumLost => {
                "Check cluster network partition or bring replacement peer nodes online."
            }
            Self::EpochFenced => {
                "Request routed to demoted coordinator; refresh shard routing from consistent hash ring."
            }
            Self::RaftLeaderTransferTimeout => {
                "Target leader replica lag too high; allow follower to catch up before transfer."
            }
            Self::Tier1UploadFailed => {
                "Inspect S3 credentials, bucket rate limits, and network egress connectivity."
            }
            Self::StateReconciliationMismatch => {
                "Bitmap divergence detected; force snapshot reload from Metadata Raft leader."
            }
            Self::NodeDrainTimeout => {
                "Active leases failed to drain within grace period; force shard eviction."
            }
        }
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code_str())
    }
}

/// Subsystem tags categorizing diagnostic events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubsystemTag {
    /// Physical storage and WAL engine.
    Storage,
    /// Roaring Bitmap state plane and consumption machine.
    StatePlane,
    /// Hot write ingress gateway and wire protocol.
    Ingress,
    /// Columnar transcoding, Arrow RecordBatching, and Parquet export.
    Export,
    /// Multi-Raft distributed consensus and coordinator sharding.
    Consensus,
    /// Operability, health, and capacity management.
    Operability,
}

impl fmt::Display for SubsystemTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage => write!(f, "STORAGE"),
            Self::StatePlane => write!(f, "STATE_PLANE"),
            Self::Ingress => write!(f, "INGRESS"),
            Self::Export => write!(f, "EXPORT"),
            Self::Consensus => write!(f, "CONSENSUS"),
            Self::Operability => write!(f, "OPERABILITY"),
        }
    }
}

/// Structured diagnostic event for system observability and runbook execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticEvent {
    /// Canonical diagnostic error code.
    pub code: DiagnosticCode,
    /// Subsystem origin.
    pub subsystem: SubsystemTag,
    /// Specific human-readable diagnostic message.
    pub message: String,
    /// Actionable remediation hint.
    pub remediation_hint: String,
    /// Nanosecond timestamp of the event.
    pub timestamp_ns: u64,
}

impl DiagnosticEvent {
    /// Create a new diagnostic event with the current timestamp or custom timestamp.
    pub fn new(
        code: DiagnosticCode,
        subsystem: SubsystemTag,
        message: impl Into<String>,
        timestamp_ns: u64,
    ) -> Self {
        let remediation_hint = code.default_remediation().to_string();
        Self {
            code,
            subsystem,
            message: message.into(),
            remediation_hint,
            timestamp_ns,
        }
    }

    /// Set a custom remediation hint.
    pub fn with_custom_remediation(mut self, hint: impl Into<String>) -> Self {
        self.remediation_hint = hint.into();
        self
    }
}

impl fmt::Display for DiagnosticEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] [{}] {}: {} (Remediation: {})",
            self.code, self.subsystem, self.timestamp_ns, self.message, self.remediation_hint
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_code_display_and_remediation() {
        let code = DiagnosticCode::CrcMismatch;
        assert_eq!(code.code_str(), "KEI-ERR-002");
        assert_eq!(format!("{code}"), "KEI-ERR-002");
        assert!(code.default_remediation().contains("Segment corruption"));

        let event = DiagnosticEvent::new(
            DiagnosticCode::MaxRetriesExceeded,
            SubsystemTag::StatePlane,
            "Offset 42 exceeded max retries (5)",
            1_700_000_000_000_000_000,
        );

        assert_eq!(event.code, DiagnosticCode::MaxRetriesExceeded);
        assert_eq!(event.subsystem, SubsystemTag::StatePlane);
        assert!(event.to_string().contains("KEI-ERR-004"));
        assert!(event.to_string().contains("STATE_PLANE"));
    }
}
