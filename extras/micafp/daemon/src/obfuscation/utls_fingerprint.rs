use sha2::{Sha256, Digest};
use rand::prelude::*;
use rand::RngCore;
use std::time::SystemTime;

#[derive(Debug, Clone, Copy)]
pub enum BrowserProfile {
    Chrome120,
    Firefox121,
    Safari17,
    Edge120,
    Ios17Safari,
}

pub struct TlsFingerprintRandomizer {
    rng: SmallRng,
}

impl TlsFingerprintRandomizer {
    pub fn new() -> Self {
        Self { rng: SmallRng::from_entropy() }
    }

    pub fn select_fingerprint(&mut self, sni: &[u8]) -> BrowserProfile {
        let hour = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() / 3600;
        let mut hasher = Sha256::new();
        hasher.update(sni);
        let sni_hash = hasher.finalize();
        let seed = hour ^ u64::from_be_bytes(sni_hash[0..8].try_into().unwrap_or([0u8;8]));
        let mut sel_rng = SmallRng::seed_from_u64(seed);
        let profiles = [
            BrowserProfile::Chrome120,
            BrowserProfile::Firefox121,
            BrowserProfile::Safari17,
            BrowserProfile::Edge120,
            BrowserProfile::Ios17Safari,
        ];
        profiles[sel_rng.gen_range(0..profiles.len())]
    }

    pub fn get_cipher_suites(profile: BrowserProfile) -> Vec<u16> {
        match profile {
            BrowserProfile::Chrome120 => vec![
                0x1301, 0x1302, 0x1303, 0xC02B, 0xC02F, 0xC02C, 0xC030,
                0xCCA9, 0xCCA8, 0xC013, 0xC014, 0x009C, 0x009D, 0x002F, 0x0035,
            ],
            BrowserProfile::Firefox121 => vec![
                0x1301, 0x1302, 0x1303, 0xC02B, 0xC02F, 0xCCA9, 0xCCA8,
                0xC02C, 0xC030, 0xC00A, 0xC009, 0xC013, 0xC014, 0x009C, 0x009D,
            ],
            BrowserProfile::Safari17 => vec![
                0x1301, 0x1302, 0x1303, 0xC02C, 0xC02B, 0xC030, 0xC02F,
                0xCCA9, 0xCCA8, 0xC024, 0xC023, 0xC00A, 0xC009, 0xC028, 0xC027,
            ],
            BrowserProfile::Edge120 => vec![
                0x1301, 0x1302, 0x1303, 0xC02B, 0xC02F, 0xC02C, 0xC030,
                0xCCA9, 0xCCA8, 0xC013, 0xC014, 0x009C, 0x009D, 0x002F, 0x0035,
            ],
            BrowserProfile::Ios17Safari => vec![
                0x1301, 0x1302, 0x1303, 0xC02C, 0xC02B, 0xC030, 0xC02F,
                0xCCA9, 0xCCA8, 0xC024, 0xC023, 0xC00A, 0xC009, 0xC028, 0xC027,
            ],
        }
    }

    pub fn get_extensions(profile: BrowserProfile) -> Vec<u16> {
        match profile {
            BrowserProfile::Chrome120 => vec![
                0x0000, 0x000D, 0x0010, 0x0012, 0x001B, 0x0015, 0x0017,
                0x002B, 0x002D, 0x0033, 0xFE0D, 0x001C, 0x001A, 0x0023,
            ],
            BrowserProfile::Firefox121 => vec![
                0x0000, 0x000D, 0x0010, 0x0012, 0x001B, 0x0017, 0x002B,
                0x002D, 0x0033, 0xFE0D, 0x001C, 0x001A, 0x0023,
            ],
            BrowserProfile::Safari17 => vec![
                0x0000, 0x000D, 0x0010, 0x0012, 0x001B, 0x0017, 0x002B,
                0x002D, 0x0033, 0xFE0D, 0x001C, 0x001A, 0x0023,
            ],
            BrowserProfile::Edge120 => vec![
                0x0000, 0x000D, 0x0010, 0x0012, 0x001B, 0x0015, 0x0017,
                0x002B, 0x002D, 0x0033, 0xFE0D, 0x001C, 0x001A, 0x0023,
            ],
            BrowserProfile::Ios17Safari => vec![
                0x0000, 0x000D, 0x0010, 0x0012, 0x001B, 0x0017, 0x002B,
                0x002D, 0x0033, 0xFE0D, 0x001C, 0x001A, 0x0023,
            ],
        }
    }

    pub fn get_alpn(profile: BrowserProfile) -> Vec<Vec<u8>> {
        match profile {
            BrowserProfile::Chrome120 | BrowserProfile::Edge120 => vec![
                b"h2".to_vec(), b"http/1.1".to_vec(),
            ],
            BrowserProfile::Firefox121 => vec![
                b"h2".to_vec(), b"http/1.1".to_vec(),
            ],
            BrowserProfile::Safari17 | BrowserProfile::Ios17Safari => vec![
                b"h2".to_vec(), b"http/1.1".to_vec(),
            ],
        }
    }

    pub fn build_client_hello(&mut self, sni: &str) -> Vec<u8> {
        let profile = self.select_fingerprint(sni.as_bytes());
        let ciphers = Self::get_cipher_suites(profile);
        let exts = Self::get_extensions(profile);
        let alpn = Self::get_alpn(profile);
        let mut hello = Vec::with_capacity(512);
        hello.push(0x16); // Handshake
        hello.push(0x03); hello.push(0x01); // TLS 1.0
        hello.extend_from_slice(&[0x00, 0x00]); // Length placeholder
        hello.push(0x01); // ClientHello
        hello.extend_from_slice(&[0x00, 0x00, 0x00]); // Length placeholder
        hello.extend_from_slice(&[0x03, 0x03]); // TLS 1.2
        let mut rng = rand::thread_rng();
        let mut random = [0u8; 32];
        rng.fill_bytes(&mut random);
        hello.extend_from_slice(&random);
        hello.push(0x20); // Session ID length
        let mut session_id = [0u8; 32];
        rng.fill_bytes(&mut session_id);
        hello.extend_from_slice(&session_id);
        hello.push(((ciphers.len() * 2) >> 8) as u8);
        hello.push(((ciphers.len() * 2) & 0xFF) as u8);
        for &c in &ciphers { hello.extend_from_slice(&c.to_be_bytes()); }
        hello.push(0x01); hello.push(0x00); // Compression
        let ext_len: usize = exts.len() * 4 + alpn.iter().map(|a| a.len() + 1).sum::<usize>() + sni.len() + 9 + 50;
        hello.push(((ext_len) >> 8) as u8);
        hello.push(((ext_len) & 0xFF) as u8);
        // SNI extension
        hello.extend_from_slice(&[0x00, 0x00]); // SNI type
        let sni_len = sni.len() + 5;
        hello.push(((sni_len + 2) >> 8) as u8);
        hello.push(((sni_len + 2) & 0xFF) as u8);
        hello.push(((sni_len) >> 8) as u8);
        hello.push(((sni_len) & 0xFF) as u8);
        hello.push(((sni.len() + 3) >> 8) as u8);
        hello.push(((sni.len() + 3) & 0xFF) as u8);
        hello.push(0x00); // host_name type
        hello.push(((sni.len()) >> 8) as u8);
        hello.push(((sni.len()) & 0xFF) as u8);
        hello.extend_from_slice(sni.as_bytes());
        // Supported versions
        hello.extend_from_slice(&[0x00, 0x2B, 0x00, 0x03, 0x02, 0x03, 0x01]);
        // Key share
        hello.extend_from_slice(&[0x00, 0x33]);
        let mut ks = Vec::new();
        ks.extend_from_slice(&[0x00, 0x1D]); // x25519
        ks.push(0x00); ks.push(0x20);
        let mut key = [0u8; 32];
        rng.fill_bytes(&mut key);
        ks.extend_from_slice(&key);
        hello.push(((ks.len() + 2) >> 8) as u8);
        hello.push(((ks.len() + 2) & 0xFF) as u8);
        hello.push(((ks.len()) >> 8) as u8);
        hello.push(((ks.len()) & 0xFF) as u8);
        hello.extend_from_slice(&ks);
        hello
    }
}
