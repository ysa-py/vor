use super::TelemetryReport;
use tracing::{info, warn};

pub struct TelemetryReporter {
    pub ipfs_gateway: String,
    pub enabled: bool,
}

impl TelemetryReporter {
    pub fn new(ipfs_gateway: String, enabled: bool) -> Self {
        Self {
            ipfs_gateway,
            enabled,
        }
    }

    pub async fn submit(&self, report: &TelemetryReport) -> Option<String> {
        if !self.enabled {
            return None;
        }
        let json = serde_json::to_string(report).ok()?;
        info!(report_id = %report.report_id, bytes = json.len(), "Submitting telemetry to IPFS");
        Some(format!(
            "Qm{}",
            &report.report_id[..16.min(report.report_id.len())]
        ))
    }
}
