// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield VIP-ULTRA — Alert Manager
// Threshold-based alerting with escalation and deduplication.
// ─────────────────────────────────────────────────────────────────────────────

use parking_lot::Mutex;
use std::collections::HashMap;
use std::time::Instant;
use tracing::{info, warn};

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Alert {
    pub level: AlertLevel,
    pub source: String,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub count: u32,
}

/// Manages alerts with deduplication and escalation.
pub struct AlertManager {
    active: Mutex<HashMap<String, Alert>>,
    history: Mutex<Vec<Alert>>,
}

impl AlertManager {
    pub fn new() -> Self {
        Self {
            active: Mutex::new(HashMap::new()),
            history: Mutex::new(Vec::new()),
        }
    }

    /// Fire an alert. Deduplicates by source key, escalates on repeat.
    pub fn fire(&self, level: AlertLevel, source: &str, message: &str) {
        let mut active = self.active.lock();
        let key = source.to_string();

        if let Some(existing) = active.get_mut(&key) {
            existing.count += 1;
            // Escalate: Warning → Critical after 3 occurrences
            if existing.count >= 3 && existing.level == AlertLevel::Warning {
                existing.level = AlertLevel::Critical;
                tracing::error!("alert ESCALATED to CRITICAL: [{}] {}", source, message);
            } else {
                warn!(
                    "alert [{}] #{}: [{}] {}",
                    format!("{:?}", existing.level),
                    existing.count,
                    source,
                    message
                );
            }
        } else {
            let alert = Alert {
                level: level.clone(),
                source: source.to_string(),
                message: message.to_string(),
                timestamp: chrono::Utc::now(),
                count: 1,
            };
            match &alert.level {
                AlertLevel::Info => info!("alert INFO: [{}] {}", source, message),
                AlertLevel::Warning => warn!("alert WARNING: [{}] {}", source, message),
                AlertLevel::Critical => tracing::error!("alert CRITICAL: [{}] {}", source, message),
            }
            active.insert(key, alert);
        }
    }

    /// Clear an alert (resolved).
    pub fn resolve(&self, source: &str) {
        let mut active = self.active.lock();
        if let Some(alert) = active.remove(source) {
            info!(
                "alert RESOLVED: [{}] after {} occurrences",
                source, alert.count
            );
            self.history.lock().push(alert);
        }
    }

    pub fn active_alerts(&self) -> Vec<Alert> {
        self.active.lock().values().cloned().collect()
    }
}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new()
    }
}
