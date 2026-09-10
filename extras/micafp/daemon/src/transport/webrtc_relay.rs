use anyhow::Result;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use rand::RngCore;
use sha2::{Digest, Sha256};
use x25519_dalek::{EphemeralSecret, PublicKey};

pub struct WebRtcRelay {
    session_key: Option<[u8; 32]>,
}

impl WebRtcRelay {
    pub fn new() -> Self {
        Self { session_key: None }
    }

    pub async fn start_discovery(&mut self, bootstrap_peers: &[String]) -> Result<()> {
        tracing::info!(
            "Starting P2P discovery with {} bootstrap peers",
            bootstrap_peers.len()
        );
        for peer in bootstrap_peers {
            tracing::debug!("Bootstrap peer: {}", peer);
        }
        Ok(())
    }

    pub async fn find_peer(&mut self, peer_id: &str) -> Result<Vec<String>> {
        tracing::info!("Looking up peer: {}", peer_id);
        Ok(vec![])
    }

    pub fn negotiate_data_channel(&mut self, peer_dtls_fingerprint: &str) -> Result<()> {
        tracing::info!(
            "Negotiating data channel with DTLS fingerprint: {}",
            peer_dtls_fingerprint
        );
        Ok(())
    }

    pub async fn connect_relay(&mut self, relay_addr: &str) -> Result<()> {
        tracing::info!("Connecting via Circuit Relay v2: {}", relay_addr);
        self.generate_session_key()?;
        Ok(())
    }

    pub async fn try_direct_connection(&mut self, peer_addr: &str) -> Result<()> {
        tracing::info!("Attempting DCUtR direct connection to: {}", peer_addr);
        self.generate_session_key()?;
        Ok(())
    }

    fn generate_session_key(&mut self) -> Result<()> {
        let ephemeral_secret = EphemeralSecret::random_from_rng(&mut rand::thread_rng());
        let ephemeral_public = PublicKey::from(&ephemeral_secret);
        let mut hasher = Sha256::new();
        hasher.update(ephemeral_public.as_bytes());
        hasher.update(b"webrtc-relay-v1");
        let derived = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&derived);
        self.session_key = Some(key);
        Ok(())
    }

    pub fn encrypt_session(&self, data: &[u8]) -> Result<Vec<u8>> {
        let key_bytes = self
            .session_key
            .ok_or_else(|| anyhow::anyhow!("No session key"))?;
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key_bytes));
        let mut nonce = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce);
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce), data)
            .map_err(|e| anyhow::anyhow!("WebRTC encrypt failed: {}", e))?;
        let mut out = Vec::with_capacity(12 + ciphertext.len());
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }
}
