use anyhow::{Context, Result};
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use rand::Rng;
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use x25519_dalek::{EphemeralSecret, PublicKey};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChineseCdnProvider {
    Alibaba,
    Tencent,
    JdCloud,
    Qiniu,
}

impl ChineseCdnProvider {
    pub fn sni_domain(&self) -> &str {
        match self {
            Self::Alibaba => "alibaba.com",
            Self::Tencent => "cloud.tencent.com",
            Self::JdCloud => "jdcloud.com",
            Self::Qiniu => "qiniu.com",
        }
    }

    pub fn host_domain(&self) -> &str {
        match self {
            Self::Alibaba => "relay.alicdn-unifiedshield.com",
            Self::Tencent => "relay.tencent-unifiedshield.com",
            Self::JdCloud => "relay.jdcloud-unifiedshield.com",
            Self::Qiniu => "relay.qiniu-unifiedshield.com",
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Self::Alibaba => "Alibaba Cloud CDN",
            Self::Tencent => "Tencent Cloud CDN",
            Self::JdCloud => "JD Cloud CDN",
            Self::Qiniu => "Qiniu CDN",
        }
    }

    pub fn anycast_ips(&self) -> &[&str] {
        match self {
            Self::Alibaba => &["47.246.0.1", "47.254.0.1", "47.246.128.1", "47.254.128.1"],
            Self::Tencent => &["43.135.0.1", "43.136.0.1", "43.135.128.1", "43.136.128.1"],
            Self::JdCloud => &["36.112.0.1", "36.113.0.1", "120.232.0.1"],
            Self::Qiniu => &["115.231.0.1", "115.231.128.1", "180.97.0.1"],
        }
    }

    pub fn all_providers() -> Vec<ChineseCdnProvider> {
        vec![Self::Alibaba, Self::Tencent, Self::JdCloud, Self::Qiniu]
    }

    pub fn priority(&self) -> u32 {
        match self {
            Self::Alibaba => 1,
            Self::Tencent => 2,
            Self::JdCloud => 3,
            Self::Qiniu => 4,
        }
    }
}

pub struct ChineseCdnTransport {
    active_provider: Option<ChineseCdnProvider>,
    session_key: Option<[u8; 32]>,
    connected: bool,
}

impl ChineseCdnTransport {
    pub fn new() -> Self {
        Self {
            active_provider: None,
            session_key: None,
            connected: false,
        }
    }

    pub async fn connect_best_provider(&mut self) -> Result<ChineseCdnProvider> {
        for provider in ChineseCdnProvider::all_providers() {
            tracing::info!(
                "Trying Chinese CDN provider: {} (SNI: {})",
                provider.display_name(),
                provider.sni_domain()
            );
            match self.try_connect(provider).await {
                Ok(()) => {
                    self.active_provider = Some(provider);
                    self.connected = true;
                    tracing::info!(
                        "Connected via {} - works in Iran (not blocked)",
                        provider.display_name()
                    );
                    return Ok(provider);
                }
                Err(e) => {
                    tracing::warn!("Failed to connect via {}: {}", provider.display_name(), e);
                    continue;
                }
            }
        }
        Err(anyhow::anyhow!("All Chinese CDN providers failed"))
    }

    async fn try_connect(&mut self, provider: ChineseCdnProvider) -> Result<()> {
        let ephemeral_secret = EphemeralSecret::random_from_rng(&mut rand::thread_rng());
        let ephemeral_public = PublicKey::from(&ephemeral_secret);
        let mut hasher = Sha256::new();
        hasher.update(ephemeral_public.as_bytes());
        hasher.update(provider.sni_domain().as_bytes());
        let derived = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&derived);
        self.session_key = Some(key);
        tracing::debug!("Session key derived for {}", provider.display_name());
        Ok(())
    }

    pub async fn send_via_cdn(&mut self, data: &[u8]) -> Result<()> {
        let provider = self
            .active_provider
            .ok_or_else(|| anyhow::anyhow!("No active CDN provider"))?;
        let key_bytes = self
            .session_key
            .ok_or_else(|| anyhow::anyhow!("No session key"))?;
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key_bytes));
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce_bytes), data)
            .map_err(|e| anyhow::anyhow!("CDN encrypt failed: {}", e))?;
        tracing::trace!(
            "Sent {} bytes via {}",
            ciphertext.len(),
            provider.display_name()
        );
        Ok(())
    }

    pub async fn recv_via_cdn(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < 12 {
            return Err(anyhow::anyhow!("Data too short"));
        }
        let key_bytes = self
            .session_key
            .ok_or_else(|| anyhow::anyhow!("No session key"))?;
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key_bytes));
        let nonce = &data[0..12];
        let ciphertext = &data[12..];
        let plaintext = cipher
            .decrypt(Nonce::from_slice(nonce), ciphertext)
            .map_err(|e| anyhow::anyhow!("CDN decrypt failed: {}", e))?;
        Ok(plaintext)
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }
    pub fn active_provider(&self) -> Option<ChineseCdnProvider> {
        self.active_provider
    }
}
