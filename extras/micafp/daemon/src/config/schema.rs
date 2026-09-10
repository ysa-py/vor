use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub tunnel: TunnelConfig,
    #[serde(default)]
    pub transport: TransportConfig,
    #[serde(default)]
    pub ai: AiConfig,
    #[serde(default)]
    pub intranet: IntranetConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelConfig {
    #[serde(default = "default_mtu")]
    pub mtu: u16,
    #[serde(default = "default_addr")]
    pub address: String,
    #[serde(default = "default_iran_path")]
    pub iran_ip_ranges_path: String,
    #[serde(default = "default_true")]
    pub split_tunnel_enabled: bool,
    #[serde(default = "default_true")]
    pub kill_switch_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    #[serde(default = "default_true")]
    pub chinese_cdn_primary: bool,
    #[serde(default)]
    pub cloudflare_worker_urls: Vec<String>,
    #[serde(default)]
    pub mqtt_brokers: Vec<String>,
    #[serde(default)]
    pub p2p_bootstrap_peers: Vec<String>,
    #[serde(default)]
    pub meek_bridges: Vec<String>,
    #[serde(default)]
    pub snowflake_broker: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    #[serde(default = "default_dpi_model")]
    pub dpi_model_path: String,
    #[serde(default = "default_pred_model")]
    pub predictor_model_path: String,
    #[serde(default = "default_threshold")]
    pub confidence_threshold: f32,
    #[serde(default = "default_alpha")]
    pub ucb_alpha: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntranetConfig {
    #[serde(default = "default_true")]
    pub auto_detect: bool,
    #[serde(default)]
    pub iranian_dns_servers: Vec<String>,
    #[serde(default = "default_check_interval")]
    pub check_interval_secs: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            tunnel: TunnelConfig::default(),
            transport: TransportConfig {
                chinese_cdn_primary: true,
                cloudflare_worker_urls: vec![],
                mqtt_brokers: vec!["broker.hivemq.com:1883".into()],
                p2p_bootstrap_peers: vec![],
                meek_bridges: vec!["azure".into()],
                snowflake_broker: "https://snowflake-broker.torproject.net/".into(),
            },
            ai: AiConfig::default(),
            intranet: IntranetConfig {
                auto_detect: true,
                iranian_dns_servers: vec!["10.202.10.10".into(), "78.157.42.100".into()],
                check_interval_secs: 30,
            },
        }
    }
}

impl Default for TunnelConfig {
    fn default() -> Self {
        Self {
            mtu: 1380,
            address: "172.19.0.1".into(),
            iran_ip_ranges_path: "/etc/unifiedshield/iran-ip-ranges.json".into(),
            split_tunnel_enabled: true,
            kill_switch_enabled: true,
        }
    }
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            chinese_cdn_primary: true,
            cloudflare_worker_urls: vec![],
            mqtt_brokers: vec!["broker.hivemq.com:1883".into()],
            p2p_bootstrap_peers: vec![],
            meek_bridges: vec!["azure".into()],
            snowflake_broker: "https://snowflake-broker.torproject.net/".into(),
        }
    }
}

impl Default for IntranetConfig {
    fn default() -> Self {
        Self {
            auto_detect: true,
            iranian_dns_servers: vec!["10.202.10.10".into(), "78.157.42.100".into()],
            check_interval_secs: 30,
        }
    }
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            dpi_model_path: "ai-models/models/dpi_classifier.onnx".into(),
            predictor_model_path: "ai-models/models/traffic_predictor.onnx".into(),
            confidence_threshold: 0.72,
            ucb_alpha: 1.414,
        }
    }
}

impl AppConfig {
    pub fn from_json(json: &str) -> Result<Self> {
        if json.trim() == "{}" || json.trim().is_empty() {
            return Ok(Self::default());
        }
        serde_json::from_str(json).context("Failed to parse config")
    }
}

fn default_mtu() -> u16 {
    1380
}
fn default_addr() -> String {
    "172.19.0.1".into()
}
fn default_iran_path() -> String {
    "/etc/unifiedshield/iran-ip-ranges.json".into()
}
fn default_true() -> bool {
    true
}
fn default_dpi_model() -> String {
    "ai-models/models/dpi_classifier.onnx".into()
}
fn default_pred_model() -> String {
    "ai-models/models/traffic_predictor.onnx".into()
}
fn default_threshold() -> f32 {
    0.72
}
fn default_alpha() -> f64 {
    1.414
}
fn default_check_interval() -> u64 {
    30
}

// ── ShieldConfig alias — used in main.rs ────────────────────────────────────
/// Top-level configuration wrapper. `ShieldConfig` is the canonical public name;
/// internally it wraps `AppConfig` with file-system load/save helpers.
pub type ShieldConfig = AppConfig;

impl ShieldConfig {
    /// Load configuration from the standard path, or return defaults.
    pub fn load_or_default() -> anyhow::Result<Self> {
        let config_path = dirs::config_dir()
            .map(|d| d.join("unifiedshield").join("config.json"))
            .unwrap_or_else(|| std::path::PathBuf::from("/etc/unifiedshield/config.json"));

        if config_path.exists() {
            let json = std::fs::read_to_string(&config_path)?;
            Self::from_json(&json)
        } else {
            Ok(Self::default())
        }
    }
}
