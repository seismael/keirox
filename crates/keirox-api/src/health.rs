//! Health, Readiness, and Liveness probe services per `KEI-ARC-027` and `KEI-OPS-040`.

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Operational health classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Subsystem is fully functional and serving requests within SLA.
    Healthy,
    /// Subsystem is operating in degraded mode (e.g. backpressure or high memory).
    Degraded,
    /// Subsystem is unavailable or critical invariant failed.
    Unhealthy,
}

impl fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Healthy => write!(f, "HEALTHY"),
            Self::Degraded => write!(f, "DEGRADED"),
            Self::Unhealthy => write!(f, "UNHEALTHY"),
        }
    }
}

/// Detailed health probe evaluation report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeReport {
    /// Overall health status.
    pub status: HealthStatus,
    /// Process uptime in seconds.
    pub uptime_seconds: u64,
    /// Physical storage and WAL engine writable status.
    pub storage_writable: bool,
    /// Roaring Bitmap state plane operational status.
    pub state_plane_healthy: bool,
    /// Memory consumption within bounds.
    pub memory_healthy: bool,
    /// Diagnostic details or failure messages.
    pub details: Vec<String>,
}

impl ProbeReport {
    /// Returns true if the probe is in a Healthy or Degraded (serviceable) state.
    #[must_use]
    pub fn is_serviceable(&self) -> bool {
        self.status != HealthStatus::Unhealthy
    }

    /// Render probe report as JSON for HTTP endpoints (`/healthz`, `/readyz`, `/livez`).
    #[must_use]
    pub fn render_json(&self) -> String {
        let details_json = self
            .details
            .iter()
            .map(|d| format!(r#""{}""#, d.replace('"', "\\\"")))
            .collect::<Vec<_>>()
            .join(",");

        format!(
            r#"{{"status":"{}","uptime_seconds":{},"storage_writable":{},"state_plane_healthy":{},"memory_healthy":{},"details":[{}]}}"#,
            self.status,
            self.uptime_seconds,
            self.storage_writable,
            self.state_plane_healthy,
            self.memory_healthy,
            details_json,
        )
    }
}

/// Health and readiness probe manager coordinating subsystem health evaluations.
#[derive(Debug)]
pub struct HealthProbeService {
    start_time: Instant,
    is_draining: AtomicBool,
    storage_healthy: AtomicBool,
    state_plane_healthy: AtomicBool,
    memory_healthy: AtomicBool,
}

impl Default for HealthProbeService {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthProbeService {
    /// Initialize a new health probe service.
    #[must_use]
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            is_draining: AtomicBool::new(false),
            storage_healthy: AtomicBool::new(true),
            state_plane_healthy: AtomicBool::new(true),
            memory_healthy: AtomicBool::new(true),
        }
    }

    /// Mark node as entering draining mode for graceful maintenance.
    pub fn set_draining(&self, draining: bool) {
        self.is_draining.store(draining, Ordering::SeqCst);
    }

    /// Update storage engine health status.
    pub fn set_storage_healthy(&self, healthy: bool) {
        self.storage_healthy.store(healthy, Ordering::Relaxed);
    }

    /// Update state plane health status.
    pub fn set_state_plane_healthy(&self, healthy: bool) {
        self.state_plane_healthy.store(healthy, Ordering::Relaxed);
    }

    /// Update memory subsystem health status.
    pub fn set_memory_healthy(&self, healthy: bool) {
        self.memory_healthy.store(healthy, Ordering::Relaxed);
    }

    /// Evaluate Liveness (`/livez`): Checks if the core process is alive and responsive.
    #[must_use]
    pub fn check_live(&self) -> ProbeReport {
        ProbeReport {
            status: HealthStatus::Healthy,
            uptime_seconds: self.start_time.elapsed().as_secs(),
            storage_writable: self.storage_healthy.load(Ordering::Relaxed),
            state_plane_healthy: self.state_plane_healthy.load(Ordering::Relaxed),
            memory_healthy: self.memory_healthy.load(Ordering::Relaxed),
            details: vec!["Process alive".to_string()],
        }
    }

    /// Evaluate Readiness (`/readyz`): Checks if node can accept ingress and serve requests.
    #[must_use]
    pub fn check_ready(&self) -> ProbeReport {
        let uptime = self.start_time.elapsed().as_secs();
        let storage = self.storage_healthy.load(Ordering::Relaxed);
        let state = self.state_plane_healthy.load(Ordering::Relaxed);
        let memory = self.memory_healthy.load(Ordering::Relaxed);
        let draining = self.is_draining.load(Ordering::SeqCst);

        let mut details = Vec::new();
        let status = if draining {
            details.push("Node is draining".to_string());
            HealthStatus::Degraded
        } else if !storage {
            details.push("Storage WAL engine is not writable".to_string());
            HealthStatus::Unhealthy
        } else if !state {
            details.push("State plane invariant violation or degraded".to_string());
            HealthStatus::Unhealthy
        } else if !memory {
            details.push("Memory arena exhausted or high allocation pressure".to_string());
            HealthStatus::Degraded
        } else {
            details.push("Node is ready for traffic".to_string());
            HealthStatus::Healthy
        };

        ProbeReport {
            status,
            uptime_seconds: uptime,
            storage_writable: storage,
            state_plane_healthy: state,
            memory_healthy: memory,
            details,
        }
    }

    /// Evaluate Overall Health (`/healthz`): Comprehensive aggregate health summary.
    #[must_use]
    pub fn check_health(&self) -> ProbeReport {
        self.check_ready()
    }
}

/// Shared reference to health probe service.
pub type SharedHealthProbe = Arc<HealthProbeService>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_probe_lifecycle() {
        let probe = HealthProbeService::new();
        let live = probe.check_live();
        assert_eq!(live.status, HealthStatus::Healthy);
        assert!(live.is_serviceable());

        let ready = probe.check_ready();
        assert_eq!(ready.status, HealthStatus::Healthy);
        assert!(ready.render_json().contains(r#""status":"HEALTHY""#));

        probe.set_draining(true);
        let ready_draining = probe.check_ready();
        assert_eq!(ready_draining.status, HealthStatus::Degraded);
        assert!(ready_draining.is_serviceable());

        probe.set_storage_healthy(false);
        probe.set_draining(false);
        let ready_failed_storage = probe.check_ready();
        assert_eq!(ready_failed_storage.status, HealthStatus::Unhealthy);
        assert!(!ready_failed_storage.is_serviceable());
    }
}
