use anyhow::Result;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use sha2::{Digest, Sha256};
use x25519_dalek::{EphemeralSecret, PublicKey, StaticSecret};

pub struct WireGuardTunnel {
    private_key: StaticSecret,
    peer_public_key: PublicKey,
    sending_key: Option<[u8; 32]>,
    receiving_key: Option<[u8; 32]>,
    send_nonce: u64,
    recv_nonce: u64,
    last_handshake: std::time::Instant,
    rekey_interval: std::time::Duration,
}

impl WireGuardTunnel {
    pub fn new(private_key_bytes: [u8; 32], peer_public_key_bytes: [u8; 32]) -> Self {
        Self {
            private_key: StaticSecret::from(private_key_bytes),
            peer_public_key: PublicKey::from(peer_public_key_bytes),
            sending_key: None,
            receiving_key: None,
            send_nonce: 0,
            recv_nonce: 0,
            last_handshake: std::time::Instant::now(),
            rekey_interval: std::time::Duration::from_secs(120),
        }
    }
    pub async fn handshake(&mut self) -> Result<()> {
        let ephemeral_secret = EphemeralSecret::random_from_rng(&mut rand::thread_rng());
        let ephemeral_public = PublicKey::from(&ephemeral_secret);
        let shared = self.private_key.diffie_hellman(&self.peer_public_key);
        let mut hasher = Sha256::new();
        hasher.update(b"unifiedshield-wg-v1");
        hasher.update(shared.as_bytes());
        hasher.update(ephemeral_public.as_bytes());
        let derived = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&derived);
        self.sending_key = Some(key);
        self.receiving_key = Some(key);
        self.send_nonce = 0;
        self.recv_nonce = 0;
        self.last_handshake = std::time::Instant::now();
        tracing::info!("WireGuard handshake completed");
        Ok(())
    }
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let key_bytes = self
            .sending_key
            .ok_or_else(|| anyhow::anyhow!("No sending key"))?;
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key_bytes));
        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[4..12].copy_from_slice(&self.send_nonce.to_le_bytes());
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce_bytes), plaintext)
            .map_err(|e| anyhow::anyhow!("Encrypt failed: {}", e))?;
        self.send_nonce = self.send_nonce.wrapping_add(1);
        let mut out = Vec::with_capacity(8 + ciphertext.len());
        out.extend_from_slice(&self.send_nonce.to_le_bytes());
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }
    pub fn decrypt(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < 8 {
            return Err(anyhow::anyhow!("Packet too short"));
        }
        let nonce_val = u64::from_le_bytes(data[0..8].try_into()?);
        let key_bytes = self
            .receiving_key
            .ok_or_else(|| anyhow::anyhow!("No receiving key"))?;
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key_bytes));
        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[4..12].copy_from_slice(&nonce_val.to_le_bytes());
        let plaintext = cipher
            .decrypt(Nonce::from_slice(&nonce_bytes), &data[8..])
            .map_err(|e| anyhow::anyhow!("Decrypt failed: {}", e))?;
        self.recv_nonce = nonce_val.wrapping_add(1);
        Ok(plaintext)
    }
    pub fn needs_rekey(&self) -> bool {
        self.last_handshake.elapsed() > self.rekey_interval
    }
}
