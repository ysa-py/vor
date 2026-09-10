use super::wireguard::WireGuardTunnel;
use anyhow::Result;
use rand::Rng;
use rand::RngCore;

const AWG_MAGIC: [u8; 3] = [0x41, 0x57, 0x47];

pub struct AmneziaWGTunnel {
    inner: WireGuardTunnel,
    junk_count: u8,
    junk_min_size: u8,
    junk_max_size: u8,
}

impl AmneziaWGTunnel {
    pub fn new(
        priv_key: [u8; 32],
        peer_pub: [u8; 32],
        junk_count: u8,
        junk_min: u8,
        junk_max: u8,
    ) -> Self {
        Self {
            inner: WireGuardTunnel::new(priv_key, peer_pub),
            junk_count: junk_count.clamp(3, 12),
            junk_min_size: junk_min.clamp(40, 100),
            junk_max_size: junk_max.clamp(60, 200),
        }
    }
    pub fn generate_junk_packets(&self) -> Vec<Vec<u8>> {
        let mut rng = rand::thread_rng();
        (0..self.junk_count)
            .map(|_| {
                let size = rng.gen_range(self.junk_min_size..=self.junk_max_size);
                let mut p = vec![0u8; size as usize];
                rng.fill_bytes(&mut p);
                p
            })
            .collect()
    }
    pub fn obfuscate_header(&self, data: &mut [u8]) {
        if data.len() >= 3 {
            data[0..3].copy_from_slice(&AWG_MAGIC);
        }
    }
    pub fn verify_header(data: &[u8]) -> bool {
        data.len() >= 3 && data[0..3] == AWG_MAGIC
    }
    pub fn compute_mac2(cookie: &[u8], msg: &[u8]) -> [u8; 16] {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(b"mac2");
        h.update(cookie);
        h.update(msg);
        let r = h.finalize();
        let mut mac = [0u8; 16];
        mac.copy_from_slice(&r[0..16]);
        mac
    }
    pub async fn handshake(&mut self) -> Result<()> {
        let _junk = self.generate_junk_packets();
        self.inner.handshake().await?;
        tracing::info!(
            "AmneziaWG handshake completed with {} junk packets",
            self.junk_count
        );
        Ok(())
    }
    pub fn encrypt(&mut self, pt: &[u8]) -> Result<Vec<u8>> {
        let mut enc = self.inner.encrypt(pt)?;
        if enc.len() >= 3 {
            self.obfuscate_header(&mut enc);
        }
        Ok(enc)
    }
    pub fn decrypt(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        let mut d = data.to_vec();
        if Self::verify_header(&d) {
            d[0..3].copy_from_slice(&[0x04, 0x00, 0x00]);
        }
        self.inner.decrypt(&d)
    }
}
