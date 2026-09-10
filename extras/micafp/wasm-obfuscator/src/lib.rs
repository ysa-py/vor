//! UnifiedShield WASM Obfuscator
//!
//! Provides TLS fingerprint randomization, timing jitter calculation,
//! and packet padding for censorship circumvention. Compiled to WASM
//! and runs in the browser or Node.js.

use wasm_bindgen::prelude::*;
use sha2::{Sha256, Digest};
use chacha20poly1305::{
    ChaCha20Poly1305,
    Key,
    Nonce,
    aead::Aead,
    KeyInit,
};
use rand::Rng;

// TLS fingerprint profiles
const PROFILE_CHROME: u8 = 0;
const PROFILE_FIREFOX: u8 = 1;
const PROFILE_SAFARI: u8 = 2;
const PROFILE_EDGE: u8 = 3;
const PROFILE_RANDOM: u8 = 255;

// Chrome-like cipher suites (TLS 1.3 + TLS 1.2)
const CHROME_CIPHERS: [u16; 11] = [
    0x1301, 0x1302, 0x1303, // TLS 1.3
    0xc02b, 0xc02f, 0xc02c, 0xc030, // ECDHE
    0x009e, 0x009c, // AES-GCM
    0xcca9, 0xcca8, // ChaCha20
];

// Firefox-like cipher suites
const FIREFOX_CIPHERS: [u16; 15] = [
    0x1301, 0x1302, 0x1303,
    0xc02b, 0xc02f, 0xc024, 0xc028,
    0xc00a, 0xc009, 0xc013, 0xc014,
    0x009e, 0x009c, 0x0039, 0x0033,
];

// Safari-like cipher suites
const SAFARI_CIPHERS: [u16; 13] = [
    0x1301, 0x1302, 0x1303,
    0xc02c, 0xc030, 0xc02b, 0xc02f,
    0x009e, 0x009c,
    0xc024, 0xc028, 0xc00a, 0xc009,
];

// Edge-like cipher suites (same as Chrome)
const EDGE_CIPHERS: [u16; 11] = CHROME_CIPHERS;

// Chrome TLS extensions order
const CHROME_EXTENSIONS: [u16; 15] = [
    0x0000, 0x0005, 0x000a, 0x000b, 0x000d,
    0x0012, 0x0015, 0x0017, 0x001b, 0x0023,
    0x0029, 0x002b, 0x002d, 0x0033, 0xfe0d,
];

// Firefox TLS extensions order
const FIREFOX_EXTENSIONS: [u16; 13] = [
    0x0000, 0x0005, 0x000a, 0x000b, 0x000d,
    0x0015, 0x0017, 0x001b, 0x0023,
    0x002b, 0x002d, 0x0033, 0xfe0d,
];

/// Generate cryptographic random bytes using the WASM RNG
fn random_bytes(len: usize) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    (0..len).map(|_| rng.gen::<u8>()).collect()
}

/// Generate a random u32
fn random_u32() -> u32 {
    let mut rng = rand::thread_rng();
    rng.gen::<u32>()
}

/// Get cipher suites and extensions for a fingerprint profile
fn get_profile_data(profile: u8) -> (Vec<u16>, Vec<u16>) {
    match profile {
        PROFILE_CHROME => (CHROME_CIPHERS.to_vec(), CHROME_EXTENSIONS.to_vec()),
        PROFILE_FIREFOX => (FIREFOX_CIPHERS.to_vec(), FIREFOX_EXTENSIONS.to_vec()),
        PROFILE_SAFARI => (SAFARI_CIPHERS.to_vec(), SAFARI_EXTENSIONS.to_vec()),
        PROFILE_EDGE => (EDGE_CIPHERS.to_vec(), CHROME_EXTENSIONS.to_vec()),
        PROFILE_RANDOM => {
            let mut rng = rand::thread_rng();
            let profiles = [
                (CHROME_CIPHERS.to_vec(), CHROME_EXTENSIONS.to_vec()),
                (FIREFOX_CIPHERS.to_vec(), FIREFOX_EXTENSIONS.to_vec()),
                (SAFARI_CIPHERS.to_vec(), SAFARI_EXTENSIONS.to_vec()),
                (EDGE_CIPHERS.to_vec(), CHROME_EXTENSIONS.to_vec()),
            ];
            let idx = rng.gen_range(0..profiles.len());
            profiles[idx].clone()
        }
        _ => (CHROME_CIPHERS.to_vec(), CHROME_EXTENSIONS.to_vec()),
    }
}

// Safari extensions (defined separately to avoid borrow issues)
const SAFARI_EXTENSIONS: [u16; 14] = [
    0x0000, 0x0005, 0x000a, 0x000b, 0x000d,
    0x0012, 0x0015, 0x0017, 0x001b, 0x0023,
    0x002b, 0x002d, 0x0033, 0xfe0d,
];

/// Build a TLS ClientHello-like byte sequence with the specified fingerprint profile
fn build_client_hello(profile: u8, session_hash: &[u8]) -> Vec<u8> {
    let (ciphers, extensions) = get_profile_data(profile);

    let mut hello = Vec::with_capacity(512);

    // TLS Record Header
    hello.push(0x16); // Content type: Handshake
    hello.push(0x03); // Version major
    hello.push(0x01); // Version minor
    hello.extend_from_slice(&[0x00, 0x00]); // Length placeholder (filled later)

    // Handshake Header
    hello.push(0x01); // Handshake type: ClientHello
    hello.extend_from_slice(&[0x00, 0x00, 0x00]); // Length placeholder

    // Client Version (TLS 1.2 in record, 1.3 negotiated)
    hello.push(0x03);
    hello.push(0x03);

    // Client Random (32 bytes, derived from session hash + random)
    let mut client_random = [0u8; 32];
    let mut hasher = Sha256::new();
    hasher.update(session_hash);
    hasher.update(&random_bytes(16));
    let hash_result = hasher.finalize();
    client_random.copy_from_slice(&hash_result);
    hello.extend_from_slice(&client_random);

    // Session ID (32 bytes random)
    hello.push(0x20); // Session ID length
    hello.extend_from_slice(&random_bytes(32));

    // Cipher Suites
    let cipher_len = (ciphers.len() * 2) as u16;
    hello.push((cipher_len >> 8) as u8);
    hello.push((cipher_len & 0xff) as u8);
    for cipher in &ciphers {
        hello.push((cipher >> 8) as u8);
        hello.push((cipher & 0xff) as u8);
    }

    // Compression Methods
    hello.push(0x01); // Length
    hello.push(0x00); // No compression

    // Extensions
    let mut ext_data = Vec::with_capacity(256);
    for ext_id in &extensions {
        ext_data.push((ext_id >> 8) as u8);
        ext_data.push((ext_id & 0xff) as u8);

        // Extension-specific payload
        let payload = match *ext_id {
            0x0000 => {
                // Server Name Indication (SNI) - random placeholder
                let sni = b"example.com";
                let mut sni_ext = Vec::new();
                sni_ext.extend_from_slice(&((sni.len() as u16 + 3).to_be_bytes()));
                sni_ext.push(0x00); // host name type
                sni_ext.extend_from_slice(&(sni.len() as u16).to_be_bytes());
                sni_ext.extend_from_slice(sni);
                sni_ext
            }
            0x002b => {
                // Supported Versions (TLS 1.3)
                vec![0x03, 0x03, 0x04, 0x03, 0x03]
            }
            0x002d => {
                // PSK Key Exchange Modes
                vec![0x01, 0x01]
            }
            0x0033 => {
                // Key Share (x25519 placeholder)
                let key_share = random_bytes(32);
                let mut ks = Vec::new();
                ks.push(0x00); // Key share entry length (2 bytes)
                ks.push(0x26); // 38 bytes total
                ks.push(0x00); // Group: x25519
                ks.push(0x1d);
                ks.push(0x00); // Key length: 32
                ks.push(0x20);
                ks.extend_from_slice(&key_share);
                ks
            }
            0x0005 => {
                // Status Request
                vec![0x01, 0x00, 0x00, 0x00, 0x00]
            }
            0x000a => {
                // Supported Groups
                let groups: [u16; 4] = [0x001d, 0x0017, 0x0018, 0x0019]; // x25519, secp256r1, secp384r1, secp521r1
                let mut g = Vec::new();
                g.extend_from_slice(&((groups.len() as u16 * 2).to_be_bytes()));
                for group in &groups {
                    g.extend_from_slice(&group.to_be_bytes());
                }
                g
            }
            0x000d => {
                // Signature Algorithms
                let sigs: [u16; 6] = [0x0403, 0x0503, 0x0603, 0x0203, 0x0804, 0x0805];
                let mut sa = Vec::new();
                sa.extend_from_slice(&((sigs.len() as u16 * 2).to_be_bytes()));
                for sig in &sigs {
                    sa.extend_from_slice(&sig.to_be_bytes());
                }
                sa
            }
            _ => {
                // Generic random extension data
                let data = random_bytes(4);
                let mut ext = Vec::new();
                ext.extend_from_slice(&(data.len() as u16).to_be_bytes());
                ext.extend_from_slice(&data);
                ext
            }
        };
        ext_data.extend_from_slice(&payload);
    }

    // Write extensions length
    hello.push((ext_data.len() >> 8) as u8);
    hello.push((ext_data.len() & 0xff) as u8);
    hello.extend_from_slice(&ext_data);

    // Fill in lengths
    let handshake_len = hello.len() - 9; // After record header (5) + handshake type (1) + len (3)
    hello[6] = ((handshake_len >> 16) & 0xff) as u8;
    hello[7] = ((handshake_len >> 8) & 0xff) as u8;
    hello[8] = (handshake_len & 0xff) as u8;

    let record_len = hello.len() - 5; // After record header
    hello[3] = ((record_len >> 8) & 0xff) as u8;
    hello[4] = (record_len & 0xff) as u8;

    hello
}

/// Randomize a TLS ClientHello message to match a specific browser fingerprint.
///
/// # Arguments
/// * `data` - Original client hello data (used as seed for randomization)
/// * `fingerprint_profile` - Browser profile to simulate (0=Chrome, 1=Firefox, 2=Safari, 3=Edge, 255=Random)
///
/// # Returns
/// Randomized ClientHello bytes matching the specified fingerprint
#[wasm_bindgen]
pub fn randomize_client_hello(data: &[u8], fingerprint_profile: u8) -> Vec<u8> {
    // Hash the original data to use as session identifier
    let mut hasher = Sha256::new();
    hasher.update(data);
    let session_hash = hasher.finalize();

    build_client_hello(fingerprint_profile, &session_hash)
}

/// Calculate timing jitter for a given base delay.
///
/// Produces a natural-looking jittered delay that mimics human browsing
/// patterns and avoids statistical detection of regular timing intervals.
///
/// # Arguments
/// * `delay_ms` - Base delay in milliseconds
///
/// # Returns
/// Jittered delay in milliseconds
#[wasm_bindgen]
pub fn add_timing_jitter(delay_ms: u32) -> u32 {
    let mut rng = rand::thread_rng();

    // Jitter range: 5-50ms with exponential distribution favoring lower values
    let jitter_base: f64 = rng.gen::<f64>();
    let jitter_magnitude = -50.0 * (1.0 - jitter_base).ln(); // Exponential distribution, λ=50
    let jitter = jitter_magnitude.min(200.0) as i32; // Cap at 200ms

    // Additional micro-jitter for more natural distribution
    let micro_jitter = rng.gen_range(-3..=3);

    // Apply sign based on probability (slight bias towards adding delay)
    let sign: i32 = if rng.gen::<f64>() < 0.55 { 1 } else { -1 };

    let result = delay_ms as i32 + sign * jitter + micro_jitter;
    result.max(0) as u32
}

/// Pad a packet to a target size or a natural-looking random size.
///
/// Uses SHA-256-based padding to ensure the padding is indistinguishable
/// from legitimate content. Supports multiple padding strategies:
/// - If target_size is 0: Pad to a random size between 1.0x and 1.5x original
/// - If target_size > 0: Pad to exactly target_size bytes
///
/// Format: [original_data][padding_delimiter: 0x00][padding_length: u16_be][sha256_hash: 32 bytes][padding_data]
///
/// # Arguments
/// * `data` - Original packet data
/// * `target_size` - Target size (0 for automatic sizing)
///
/// # Returns
/// Padded packet data
#[wasm_bindgen]
pub fn pad_packet(data: &[u8], target_size: u32) -> Vec<u8> {
    let mut rng = rand::thread_rng();

    let padding_overhead = 1 + 2 + 32; // delimiter + length + hash
    let min_size = data.len() + padding_overhead;

    let final_size = if target_size == 0 {
        // Auto-size: random between 1.0x and 1.5x, aligned to 16-byte blocks
        let factor = 1.0 + rng.gen::<f64>() * 0.5;
        let raw_size = ((data.len() as f64) * factor) as usize;
        let aligned = ((raw_size + 15) / 16) * 16;
        aligned.max(min_size)
    } else if (target_size as usize) < min_size {
        min_size
    } else {
        // Align target to 16-byte boundary
        ((target_size as usize + 15) / 16) * 16
    };

    let padding_len = final_size - data.len() - padding_overhead;

    // Generate padding data using SHA-256 chain for cryptographic randomness
    let mut padding_data = Vec::with_capacity(padding_len);
    let mut hash_input = data.to_vec();
    hash_input.extend_from_slice(&padding_len.to_be_bytes());
    hash_input.extend_from_slice(&random_bytes(8));

    let mut remaining = padding_len;
    while remaining > 0 {
        let mut hasher = Sha256::new();
        hasher.update(&hash_input);
        let hash = hasher.finalize();
        let take = remaining.min(32);
        padding_data.extend_from_slice(&hash[..take]);
        hash_input = hash.to_vec();
        remaining -= take;
    }

    // Build padded packet
    let mut result = Vec::with_capacity(final_size);
    result.extend_from_slice(data);
    result.push(0x00); // Padding delimiter
    result.push(((padding_len >> 8) & 0xff) as u8); // Padding length high byte
    result.push((padding_len & 0xff) as u8); // Padding length low byte

    // Integrity hash over the padding
    let mut hasher = Sha256::new();
    hasher.update(&padding_data);
    let padding_hash = hasher.finalize();
    result.extend_from_slice(&padding_hash);

    result.extend_from_slice(&padding_data);

    result
}

/// Remove padding from a padded packet.
///
/// # Arguments
/// * `data` - Padded packet data
///
/// # Returns
/// Original packet data without padding
#[wasm_bindgen]
pub fn remove_padding(data: &[u8]) -> Vec<u8> {
    // Find the padding delimiter (0x00) followed by a valid padding length
    // Search from the end for efficiency
    if data.len() < 36 {
        // Minimum: 1 data + 1 delimiter + 2 length + 32 hash
        return data.to_vec();
    }

    // Try to find the last 0x00 delimiter that results in valid padding
    for i in (0..data.len().saturating_sub(35)).rev() {
        if data[i] == 0x00 && i + 3 <= data.len() {
            let padding_len = ((data[i + 1] as usize) << 8) | (data[i + 2] as usize);
            let expected_total = i + 1 + 2 + 32 + padding_len;

            if expected_total == data.len() && i + 35 + padding_len <= data.len() {
                // Verify padding hash
                let padding_start = i + 3 + 32;
                let padding_data = &data[padding_start..padding_start + padding_len];
                let stored_hash = &data[i + 3..i + 35];

                let mut hasher = Sha256::new();
                hasher.update(padding_data);
                let computed_hash = hasher.finalize();

                if stored_hash == computed_hash.as_slice() {
                    return data[..i].to_vec();
                }
            }
        }
    }

    // Fallback: search forward for first delimiter
    for i in 0..data.len().saturating_sub(35) {
        if data[i] == 0x00 && i + 3 <= data.len() {
            let padding_len = ((data[i + 1] as usize) << 8) | (data[i + 2] as usize);
            let expected_total = i + 1 + 2 + 32 + padding_len;

            if expected_total == data.len() {
                return data[..i].to_vec();
            }
        }
    }

    data.to_vec()
}

/// Encrypt data using ChaCha20-Poly1305 for local obfuscation.
///
/// # Arguments
/// * `data` - Plaintext data
/// * `key` - 32-byte encryption key
/// * `nonce` - 12-byte nonce
///
/// # Returns
/// Encrypted data (ciphertext + 16-byte tag)
#[wasm_bindgen]
pub fn encrypt_chacha20poly1305(data: &[u8], key: &[u8], nonce: &[u8]) -> Result<Vec<u8>, JsValue> {
    if key.len() != 32 {
        return Err(JsValue::from_str("Key must be 32 bytes"));
    }
    if nonce.len() != 12 {
        return Err(JsValue::from_str("Nonce must be 12 bytes"));
    }

    let key = Key::from_slice(key);
    let nonce = Nonce::from_slice(nonce);
    let cipher = ChaCha20Poly1305::new(key);

    cipher
        .encrypt(nonce, data)
        .map_err(|e| JsValue::from_str(&format!("Encryption failed: {}", e)))
}

/// Decrypt data using ChaCha20-Poly1305.
///
/// # Arguments
/// * `data` - Encrypted data (ciphertext + tag)
/// * `key` - 32-byte encryption key
/// * `nonce` - 12-byte nonce
///
/// # Returns
/// Decrypted plaintext data
#[wasm_bindgen]
pub fn decrypt_chacha20poly1305(data: &[u8], key: &[u8], nonce: &[u8]) -> Result<Vec<u8>, JsValue> {
    if key.len() != 32 {
        return Err(JsValue::from_str("Key must be 32 bytes"));
    }
    if nonce.len() != 12 {
        return Err(JsValue::from_str("Nonce must be 12 bytes"));
    }

    let key = Key::from_slice(key);
    let nonce = Nonce::from_slice(nonce);
    let cipher = ChaCha20Poly1305::new(key);

    cipher
        .decrypt(nonce, data)
        .map_err(|e| JsValue::from_str(&format!("Decryption failed: {}", e)))
}

/// Generate a random X25519-like key pair seed for ECDH operations.
///
/// Returns 32 bytes of cryptographic random data suitable for use
/// as an X25519 private key seed.
#[wasm_bindgen]
pub fn generate_key_seed() -> Vec<u8> {
    random_bytes(32)
}

/// Compute SHA-256 hash of input data.
#[wasm_bindgen]
pub fn sha256_hash(data: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

/// Get the version of the WASM obfuscator.
#[wasm_bindgen]
pub fn get_version() -> String {
    "1.0.0".to_string()
}

/// Get available fingerprint profile names.
#[wasm_bindgen]
pub fn get_fingerprint_profiles() -> JsValue {
    let profiles = serde_json::json!({
        "0": "Chrome",
        "1": "Firefox",
        "2": "Safari",
        "3": "Edge",
        "255": "Random"
    });
    JsValue::from_str(&profiles.to_string())
}

// Panic hook for better error messages in WASM
#[wasm_bindgen]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_randomize_client_hello() {
        let data = b"test_data";
        let result = randomize_client_hello(data, 0);
        assert!(result.len() > 100);
        assert_eq!(result[0], 0x16); // TLS Handshake
    }

    #[test]
    fn test_add_timing_jitter() {
        let base = 100;
        let result = add_timing_jitter(base);
        // Result should be non-negative and within reasonable range
        assert!(result < 500);
    }

    #[test]
    fn test_pad_remove_roundtrip() {
        let data = b"hello world";
        let padded = pad_packet(data, 256);
        assert!(padded.len() >= 256);
        let unpadded = remove_padding(&padded);
        assert_eq!(unpadded, data);
    }

    #[test]
    fn test_auto_sized_padding() {
        let data = b"test data for padding";
        let padded = pad_packet(data, 0);
        assert!(padded.len() >= data.len() + 35);
        let unpadded = remove_padding(&padded);
        assert_eq!(unpadded, data);
    }

    #[test]
    fn test_chacha20_encrypt_decrypt() {
        let data = b"secret message";
        let key = random_bytes(32);
        let nonce = random_bytes(12);

        let encrypted = encrypt_chacha20poly1305(data, &key, &nonce).unwrap();
        let decrypted = decrypt_chacha20poly1305(&encrypted, &key, &nonce).unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_sha256() {
        let result = sha256_hash(b"test");
        assert_eq!(result.len(), 32);
    }
}
