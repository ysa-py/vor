use anyhow::{Context, Result};
use base64::Engine;
use rand::Rng;

pub struct DohTunnel {
    upstream_servers: Vec<String>,
    window_size: u8,
    next_seq: u16,
    send_buffer: Vec<(u16, Vec<u8>)>,
}

impl DohTunnel {
    pub fn new() -> Self {
        Self {
            upstream_servers: vec![
                "https://dns.google/dns-query".to_string(),
                "https://1.1.1.1/dns-query".to_string(),
            ],
            window_size: 8,
            next_seq: 0,
            send_buffer: Vec::new(),
        }
    }

    pub fn encode_upstream_data(&mut self, data: &[u8]) -> String {
        let b32: String = data.iter().map(|b| format!("{:02x}", b)).collect();
        let mut rng = rand::thread_rng();
        let prefix = format!(
            "{}.{:04x}.d",
            b32.chars().take(1).collect::<String>(),
            self.next_seq
        );
        let labels: Vec<String> = b32[1..]
            .chars()
            .collect::<Vec<char>>()
            .chunks(63)
            .map(|c| c.iter().collect())
            .collect();
        self.next_seq = self.next_seq.wrapping_add(1);
        format!("{}.{}", prefix, labels.join("."))
    }

    pub fn encode_downstream_data(data: &[u8]) -> Vec<u8> {
        let mut ech = Vec::with_capacity(4 + data.len());
        ech.extend_from_slice(&(data.len() as u32).to_be_bytes());
        ech.extend_from_slice(data);
        ech
    }

    pub fn decode_downstream_data(ech: &[u8]) -> Option<Vec<u8>> {
        if ech.len() < 4 {
            return None;
        }
        let len = u32::from_be_bytes(ech[0..4].try_into().ok()?) as usize;
        if ech.len() < 4 + len {
            return None;
        }
        Some(ech[4..4 + len].to_vec())
    }

    pub fn ack_packet(&self, seq: u16) -> Vec<u8> {
        let mut ack = Vec::with_capacity(4);
        ack.extend_from_slice(&seq.to_be_bytes());
        ack.push(0xAC); // ACK marker
        ack
    }

    pub async fn send_doh_query(&self, domain: &str, server: &str) -> Result<Vec<u8>> {
        let client = reqwest::Client::builder()
            .build()
            .context("HTTP client failed")?;
        let url = format!("{}?name={}&type=65", server, domain);
        let resp = client
            .get(&url)
            .header("Accept", "application/dns-json")
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .context("DoH query failed")?;
        let body = resp.bytes().await.context("DoH response read failed")?;
        Ok(body.to_vec())
    }

    pub async fn recv_doh_response(&self, raw: &[u8]) -> Result<Vec<u8>> {
        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(raw) {
            if let Some(answer) = json.get("Answer").and_then(|a| a.as_array()) {
                for record in answer {
                    if record.get("type").and_then(|t| t.as_u64()) == Some(65) {
                        if let Some(data) = record.get("data").and_then(|d| d.as_str()) {
                            let decoded = base64::engine::general_purpose::STANDARD
                                .decode(data)
                                .unwrap_or_default();
                            if let Some(payload) = Self::decode_downstream_data(&decoded) {
                                return Ok(payload);
                            }
                        }
                    }
                }
            }
        }
        Ok(vec![])
    }

    pub fn target_throughput_kbps(&self) -> f64 {
        12.0
    }
}
