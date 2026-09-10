use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IspProfile {
    pub id: String,
    pub name: String,
    pub name_fa: String,
    pub asn: Vec<u32>,
    pub dns_servers: Vec<String>,
    pub blocked_protocols: Vec<String>,
    pub working_protocols: Vec<String>,
    pub core_preference: Vec<String>,
    pub typical_latency_ms: u32,
    pub chinese_cdn_works: bool,
    pub cloudflare_works: bool,
}

pub struct IspProfileManager {
    profiles: HashMap<String, IspProfile>,
    detected_isp: Option<String>,
}

impl IspProfileManager {
    pub fn new() -> Self {
        Self {
            profiles: HashMap::new(),
            detected_isp: None,
        }
    }
    pub fn detect_isp(&mut self, dns_ip: &str) -> Option<&IspProfile> {
        None
    }
    pub fn get_profile(&self, id: &str) -> Option<&IspProfile> {
        self.profiles.get(id)
    }
}
