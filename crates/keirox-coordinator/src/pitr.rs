//! Point-in-Time Recovery (PITR) and Disaster Recovery Engine per `KEI-MR-401` and `KEI-SEC-401 §9`.

use keirox_core::error::{KeiroxError, Result};
use keirox_core::model::{StreamId, TenantId};
use keirox_core::security::{DekId, DestroyedKeyRegistry};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::RwLock;

/// Legal Hold status for compliance and litigation readiness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegalHoldEntry {
    /// Target tenant ID.
    pub tenant_id: TenantId,
    /// Target stream ID.
    pub stream_id: StreamId,
    /// Active legal hold identifier.
    pub hold_id: String,
    /// Reason and court docket reference.
    pub reason: String,
    /// Applied timestamp.
    pub applied_at_ns: u64,
}

/// Point-in-Time Recovery (PITR) execution target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PitrRestoreTarget {
    /// Tenant ID to restore.
    pub tenant_id: TenantId,
    /// Stream ID to restore.
    pub stream_id: StreamId,
    /// Target restoration timestamp cutoff (nanoseconds).
    pub target_timestamp_ns: u64,
}

/// Verification report generated after executing PITR restore.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PitrRestoreReport {
    /// Target tenant ID.
    pub tenant_id: TenantId,
    /// Target stream ID.
    pub stream_id: StreamId,
    /// Target timestamp.
    pub target_timestamp_ns: u64,
    /// Total records successfully recovered up to target timestamp.
    pub records_recovered: u64,
    /// Number of records blocked from resurrection due to crypto-shredded DEKs.
    pub shredded_records_blocked: usize,
    /// Restore status.
    pub success: bool,
}

/// Point-in-Time Recovery Engine governing disaster recovery and legal hold invariants.
pub struct PitrRecoveryEngine {
    destroyed_registry: std::sync::Arc<DestroyedKeyRegistry>,
    active_legal_holds: RwLock<HashSet<(TenantId, StreamId)>>,
}

impl PitrRecoveryEngine {
    /// Create a new PITR recovery engine.
    pub fn new(destroyed_registry: std::sync::Arc<DestroyedKeyRegistry>) -> Self {
        Self {
            destroyed_registry,
            active_legal_holds: RwLock::new(HashSet::new()),
        }
    }

    /// Place a stream under Legal Hold to suspend destructive compaction and log deletion.
    pub fn apply_legal_hold(&self, hold: LegalHoldEntry) -> Result<()> {
        let mut set = self
            .active_legal_holds
            .write()
            .map_err(|_| KeiroxError::Internal("Legal holds lock poisoned".into()))?;
        set.insert((hold.tenant_id, hold.stream_id));
        Ok(())
    }

    /// Release an active Legal Hold on a stream.
    pub fn release_legal_hold(&self, tenant_id: TenantId, stream_id: StreamId) -> Result<bool> {
        let mut set = self
            .active_legal_holds
            .write()
            .map_err(|_| KeiroxError::Internal("Legal holds lock poisoned".into()))?;
        Ok(set.remove(&(tenant_id, stream_id)))
    }

    /// Check if a stream is protected under active Legal Hold.
    #[must_use]
    pub fn is_under_legal_hold(&self, tenant_id: TenantId, stream_id: StreamId) -> bool {
        if let Ok(set) = self.active_legal_holds.read() {
            set.contains(&(tenant_id, stream_id))
        } else {
            true // Fail secure
        }
    }

    /// Execute PITR restore simulation verifying that crypto-shredded records are never resurrected.
    pub fn execute_pitr_restore(
        &self,
        target: PitrRestoreTarget,
        records: &[(u64, Option<DekId>, Vec<u8>)], // (timestamp_ns, dek_id_opt, payload)
    ) -> Result<PitrRestoreReport> {
        let mut recovered_count = 0u64;
        let mut shredded_blocked = 0usize;

        for &(ts, dek_id_opt, _) in records {
            if ts > target.target_timestamp_ns {
                continue; // Past PITR cutoff
            }

            if let Some(dek_id) = dek_id_opt {
                if self
                    .destroyed_registry
                    .is_destroyed(target.tenant_id, dek_id)
                {
                    shredded_blocked += 1;
                    continue; // Invariant: shredded data MUST NOT be resurrected from cold backups
                }
            }

            recovered_count += 1;
        }

        Ok(PitrRestoreReport {
            tenant_id: target.tenant_id,
            stream_id: target.stream_id,
            target_timestamp_ns: target.target_timestamp_ns,
            records_recovered: recovered_count,
            shredded_records_blocked: shredded_blocked,
            success: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_pitr_restore_prevents_shredded_data_resurrection() {
        let registry = Arc::new(DestroyedKeyRegistry::new());
        let engine = PitrRecoveryEngine::new(registry.clone());

        let tenant = TenantId([0x01; 16]);
        let stream = StreamId([0x02; 16]);
        let dek_active = DekId(10);
        let dek_shredded = DekId(20);

        // Record destruction of dek_shredded
        registry
            .record_destruction(keirox_core::security::DestroyedKeyEntry {
                tenant_id: tenant,
                dek_id: dek_shredded,
                stream_id: Some(stream),
                destroyed_at_ns: 1500,
                operator_id: "sec-admin".into(),
                reason: "GDPR right to be forgotten".into(),
            })
            .unwrap();

        let backup_records = vec![
            (1000, Some(dek_active), b"Active record 1".to_vec()),
            (1200, Some(dek_shredded), b"Erased record 2".to_vec()),
            (1400, Some(dek_active), b"Active record 3".to_vec()),
            (2000, Some(dek_active), b"Future record 4".to_vec()),
        ];

        let target = PitrRestoreTarget {
            tenant_id: tenant,
            stream_id: stream,
            target_timestamp_ns: 1500,
        };

        let report = engine
            .execute_pitr_restore(target, &backup_records)
            .unwrap();
        assert_eq!(report.records_recovered, 2);
        assert_eq!(report.shredded_records_blocked, 1);
        assert!(report.success);
    }

    #[test]
    fn test_legal_hold_lifecycle() {
        let registry = Arc::new(DestroyedKeyRegistry::new());
        let engine = PitrRecoveryEngine::new(registry);

        let tenant = TenantId([0x01; 16]);
        let stream = StreamId([0x02; 16]);

        assert!(!engine.is_under_legal_hold(tenant, stream));

        engine
            .apply_legal_hold(LegalHoldEntry {
                tenant_id: tenant,
                stream_id: stream,
                hold_id: "HOLD-2026-SEC-09".into(),
                reason: "SEC Investigation #4401".into(),
                applied_at_ns: 1_700_000_000,
            })
            .unwrap();

        assert!(engine.is_under_legal_hold(tenant, stream));

        engine.release_legal_hold(tenant, stream).unwrap();
        assert!(!engine.is_under_legal_hold(tenant, stream));
    }
}
