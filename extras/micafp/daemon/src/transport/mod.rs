// ─────────────────────────────────────────────────────────────────────────────
// Transport subsystem — All 22 protocols from all 13 source projects
// MICAFP-UnifiedShield-vip-ultra-Quantum-ultra v8.0
// ─────────────────────────────────────────────────────────────────────────────

pub mod cdn_tunnel;
pub mod cdn_worker;
pub mod chinese_cdn;
pub mod cloudflare_worker;
pub mod doh_tunnel;
pub mod domain_fronting;
pub mod doq_tunnel;
pub mod dual_mode;
pub mod hysteria2;
pub mod icmp_tunnel;
pub mod manager;
pub mod meek;
pub mod mqtt_tunnel;
pub mod mqtt_ws;
pub mod multihop_chain;
pub mod naive_proxy;
pub mod pluggable_transport;
pub mod reality;
pub mod shadow_tls;
pub mod tuic_v5;
pub mod vless;
pub mod webrtc_relay;
pub mod webtransport;

use crate::error::ShieldError;
use async_trait::async_trait;
use std::net::SocketAddr;

pub use dual_mode::*;
pub use manager::TransportManager;
pub use multihop_chain::MultiHopChainTransport;

/// A connected transport byte-stream handle (trait).
pub trait TransportConnection: Send + Sync {
    fn send_bytes(
        &mut self,
        data: &[u8],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send + '_>>;
    fn recv_bytes(
        &mut self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<Vec<u8>>> + Send + '_>>;
    fn close_conn(
        &mut self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send + '_>>;
}

// ── Generic transport connection wrapper ──────────────────────────────────

/// Supertrait combining AsyncRead + AsyncWrite for trait object usage.
pub trait AsyncReadWrite: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Sync {}
impl<T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Sync> AsyncReadWrite for T {}

/// A boxed, type-erased read+write stream for transport connections.
pub type BoxedStream = std::pin::Pin<Box<dyn AsyncReadWrite>>;

struct StreamWrapper {
    stream: BoxedStream,
    sni_domain: String,
    transport_name: String,
}

impl TransportConnection for StreamWrapper {
    fn send_bytes(
        &mut self,
        data: &[u8],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send + '_>> {
        use tokio::io::AsyncWriteExt;
        let s = &mut self.stream;
        let owned_data = data.to_vec();
        Box::pin(async move {
            s.write_all(&owned_data).await?;
            Ok(())
        })
    }

    fn recv_bytes(
        &mut self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<Vec<u8>>> + Send + '_>>
    {
        use tokio::io::AsyncReadExt;
        let s = &mut self.stream;
        Box::pin(async move {
            let mut buffer = vec![0u8; 65536];
            let n = s.read(&mut buffer).await?;
            buffer.truncate(n);
            Ok(buffer)
        })
    }

    fn close_conn(
        &mut self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send + '_>> {
        Box::pin(async move { Ok(()) })
    }
}

/// Make a boxed transport connection from any AsyncRead+AsyncWrite stream.
pub fn make_connection<S>(stream: S, sni: String, name: String) -> Box<dyn TransportConnection>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + Sync + 'static,
{
    Box::new(StreamWrapper {
        stream: Box::pin(stream),
        sni_domain: sni,
        transport_name: name,
    })
}

/// Core transport trait — implemented by all 22 protocols
#[async_trait]
pub trait Transport: Send + Sync {
    fn name(&self) -> &str;
    fn priority(&self) -> u8;
    async fn connect(&self, addr: &SocketAddr)
        -> Result<Box<dyn TransportConnection>, ShieldError>;
    async fn is_available(&self) -> bool;
    fn last_error(&self) -> Option<&ShieldError>;
    fn current_sni_domain(&self) -> &str;
    async fn rotate_sni_domain(&self) -> Result<String, ShieldError>;
    fn active_connections(&self) -> usize;
    async fn shutdown(&self) -> Result<(), ShieldError>;
}

// ── Re-exports ────────────────────────────────────────────────────────

pub use crate::config::isp_profile::IspProfile;
pub use crate::ipc::BatteryState;
pub use crate::ai::ucb_bandit::CoreArm;

/// Statistics for a single transport endpoint.
#[derive(Debug, Clone, Default)]
pub struct EndpointStats {
    pub attempts: u32,
    pub successes: u32,
    pub failures: u32,
    pub avg_latency_ms: f64,
    pub last_attempt_ts: u64,
    pub last_success_ts: u64,
    pub total_attempts: u64,
    pub last_seen: u64,
    pub deprioritized: bool,
    pub latency_ms: f64,
    pub success_count: u64,
}

impl EndpointStats {
    pub fn new() -> Self { Self::default() }
    pub fn success_rate(&self) -> f64 {
        if self.attempts == 0 { 0.5 } else { self.successes as f64 / self.attempts as f64 }
    }
    pub fn record_success(&mut self, _latency_ms: f64) {
        self.successes += 1; self.attempts += 1; self.success_count += 1; self.total_attempts += 1;
    }
    pub fn record_failure(&mut self) {
        self.failures += 1; self.attempts += 1; self.total_attempts += 1;
    }

    pub fn check_deprioritization(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        if now.saturating_sub(self.last_seen) > 24 * 60 * 60 * 1000 {
            self.deprioritized = true;
        }
    }
}

/// Transport weight for the load balancer.
#[derive(Debug, Clone, Default)]
pub struct TransportWeight {
    pub name: String,
    pub weight: u32,
    pub current_weight: f64,
    pub effective_weight: f64,
    pub enabled: bool,
}

/// Resolve a domain to a SocketAddr (async helper).
pub async fn resolve_domain(domain: &str, port: u16) -> Result<SocketAddr, ShieldError> {
    use tokio::net::lookup_host;
    let addr_str = format!("{}:{}", domain, port);
    lookup_host(addr_str)
        .await
        .map_err(|e| ShieldError::DnsResolutionFailed(format!("{}: {}", domain, e)))?
        .next()
        .ok_or_else(|| ShieldError::DnsResolutionFailed(format!("No addresses for {}", domain)))
}

/// Exponential backoff with full jitter.
pub fn exponential_backoff_with_jitter(
    attempt: u32, base_ms: u64, max_ms: u64,
) -> std::time::Duration {
    use rand::Rng;
    let cap = (base_ms * 2u64.pow(attempt)).min(max_ms);
    let jitter = rand::thread_rng().gen_range(0..=cap);
    std::time::Duration::from_millis(jitter)
}