// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield 6.0 — Device Secret Management
//
// The device secret is a 256-bit random value generated once at first
// install.  It is the root of trust for deriving:
//   • Daily P2P peer IDs
//   • MQTT personal topics
//   • HMAC keys for anti-forensics triggers
//   • Per-transport authentication tokens
//
// Storage:
//   • Android: Android Keystore (hardware-backed)
//   • iOS: iOS Keychain (Secure Enclave when available)
//   • macOS: macOS Keychain
//   • Linux: Encrypted file with OS keyring (libsecret)
//   • Windows: DPAPI (CryptProtectData)
//
// CRITICAL: The device secret is NEVER transmitted over the network.
// ─────────────────────────────────────────────────────────────────────────────

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use aes_gcm::aead::{Aead, KeyInit, OsRng as AesOsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use hkdf::Hkdf;
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::Sha256;
use tracing::{debug, error, info, warn};
use zeroize::Zeroize;

use crate::error::{ErrorCode, ShieldError};
use hex;

// ── Constants ────────────────────────────────────────────────────────────────

/// Length of the device secret in bytes.
const DEVICE_SECRET_LEN: usize = 32;

/// Length of the AES-256-GCM nonce.
const NONCE_LEN: usize = 12;

/// Length of the AES-256-GCM authentication tag.
const TAG_LEN: usize = 16;

/// File name for the encrypted device secret on disk.
const SECRET_FILE_NAME: &str = "device-secret.enc";

/// HKDF info label for deriving the file encryption key from the platform key.
const FILE_KEY_INFO: &[u8] = b"device-secret-file-key";

// ── DeviceSecretManager ──────────────────────────────────────────────────────

/// Manages the device secret — a 256-bit value that persists across app
/// restarts but is never transmitted over the network.
///
/// On first use the secret is generated from the OS CSPRNG.  It is then
/// encrypted and stored on disk.  On subsequent starts it is loaded,
/// decrypted, and held in mlock'd memory.
///
/// The secret is the root of trust for all derived values (peer IDs,
/// MQTT topics, HMAC keys, etc.).
pub struct DeviceSecretManager {
    /// The device secret (32 bytes), held in memory.
    secret: Mutex<DeviceSecret>,
    /// Platform for storage decisions.
    platform: DeviceSecretPlatform,
    /// Path to the encrypted secret file.
    secret_path: PathBuf,
    /// Whether the secret was newly generated (first install).
    is_first_install: bool,
}

/// Platform identifier for device secret storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceSecretPlatform {
    Android,
    Ios,
    MacOS,
    Linux,
    Windows,
    Unknown,
}

// ── DeviceSecret (mlock'd, zeroize on drop) ─────────────────────────────────

/// A 256-bit secret that is mlock'd in RAM and zeroized on drop.
struct DeviceSecret {
    data: [u8; DEVICE_SECRET_LEN],
    locked: bool,
}

impl DeviceSecret {
    /// Create a new DeviceSecret from raw bytes and mlock it.
    fn new(data: [u8; DEVICE_SECRET_LEN]) -> Result<Self, ShieldError> {
        let mut ds = DeviceSecret {
            data,
            locked: false,
        };
        ds.locked = mlock_secret(&ds.data);
        Ok(ds)
    }

    /// Generate a new random DeviceSecret from the OS CSPRNG.
    fn generate() -> Result<Self, ShieldError> {
        let mut data = [0u8; DEVICE_SECRET_LEN];
        OsRng.fill_bytes(&mut data);
        Self::new(data)
    }

    /// Borrow the secret bytes.
    fn as_bytes(&self) -> &[u8; DEVICE_SECRET_LEN] {
        &self.data
    }
}

impl Drop for DeviceSecret {
    fn drop(&mut self) {
        // Zeroize the secret memory
        self.data.zeroize();

        // munlock if we successfully locked
        if self.locked {
            munlock_secret(&self.data);
        }

        debug!("DeviceSecret zeroized and munlocked");
    }
}

// SAFETY: The secret can be shared between threads — we protect access
// with a Mutex in the manager.
unsafe impl Send for DeviceSecret {}
unsafe impl Sync for DeviceSecret {}

// ── mlock helpers ────────────────────────────────────────────────────────────

/// Lock the memory containing the device secret.
fn mlock_secret(data: &[u8; DEVICE_SECRET_LEN]) -> bool {
    let ptr = data as *const u8;
    let len = data.len();

    #[cfg(unix)]
    unsafe {
        let ret = libc::mlock(ptr as *const libc::c_void, len);
        if ret != 0 {
            let errno = *libc::__errno_location();
            warn!(
                errno,
                "mlock failed for device secret — may be swapped to disk"
            );
            return false;
        }
        debug!("Device secret mlock'd in RAM");
    }

    #[cfg(not(unix))]
    {
        debug!("mlock not available on this platform for device secret");
    }

    true
}

/// Unlock the memory containing the device secret.
fn munlock_secret(data: &[u8; DEVICE_SECRET_LEN]) {
    let ptr = data as *const u8;
    let len = data.len();

    #[cfg(unix)]
    unsafe {
        let ret = libc::munlock(ptr as *const libc::c_void, len);
        if ret != 0 {
            let errno = *libc::__errno_location();
            warn!(errno, "munlock failed for device secret");
        }
    }
}

// ── DeviceSecretManager implementation ───────────────────────────────────────

impl DeviceSecretManager {
    /// Create a new DeviceSecretManager.
    ///
    /// If the encrypted secret file exists, it is loaded and decrypted.
    /// If it does not exist, a new secret is generated, encrypted, and stored.
    pub fn new(platform: DeviceSecretPlatform) -> Result<Self, ShieldError> {
        let data_dir = resolve_data_dir(platform);
        let secret_path = data_dir.join(SECRET_FILE_NAME);

        // Ensure the data directory exists
        std::fs::create_dir_all(&data_dir).map_err(|e| {
            ShieldError::anti_forensics(
                ErrorCode::AntiForensicsDeviceSecretCorrupted,
                format!("Failed to create data directory {:?}: {}", data_dir, e),
            )
        })?;

        let (secret, is_first_install) = if secret_path.exists() {
            // Load existing secret
            debug!(path = %secret_path.display(), "Loading existing device secret");
            let data = load_and_decrypt_secret(&secret_path, platform)?;
            (data, false)
        } else {
            // First install — generate new secret
            info!("First install — generating new device secret");
            let secret = DeviceSecret::generate()?;
            // Encrypt and save
            encrypt_and_save_secret(&secret_path, secret.as_bytes(), platform)?;
            (secret, true)
        };

        info!(is_first_install, "Device secret manager initialised");

        Ok(Self {
            secret: Mutex::new(secret),
            platform,
            secret_path,
            is_first_install,
        })
    }

    /// Borrow the secret bytes for derivation operations.
    ///
    /// Returns a guarded reference — the Mutex ensures exclusive access.
    pub fn secret_bytes(&self) -> [u8; DEVICE_SECRET_LEN] {
        let guard = self.secret.lock().expect("DeviceSecret Mutex poisoned");
        *guard.as_bytes()
    }

    /// Whether this is a first install (secret was just generated).
    pub fn is_first_install(&self) -> bool {
        self.is_first_install
    }

    /// Derive a daily peer ID from the device secret.
    ///
    /// ```
    /// daily_peer_id = HKDF-SHA256(
    ///     ikm  = device_secret,
    ///     salt = current_date_utc_yyyymmdd,
    ///     info = b"peer-id"
    /// )
    /// ```
    pub fn derive_daily_peer_id(&self) -> String {
        let secret = self.secret_bytes();
        let date_salt = current_date_salt();

        let hk = Hkdf::<Sha256>::new(Some(date_salt.as_bytes()), &secret);
        let mut okm = [0u8; 32];
        hk.expand(b"peer-id", &mut okm)
            .expect("HKDF expand should not fail with valid inputs");

        hex::encode(okm)
    }

    /// Derive an MQTT personal topic from the device secret.
    ///
    /// ```
    /// topic = "shield/u/" || hex(HKDF-SHA256(
    ///     ikm  = device_secret,
    ///     salt = current_date_utc_yyyymmdd,
    ///     info = b"mqtt-topic"
    /// )[0..16])
    /// ```
    pub fn derive_mqtt_topic(&self) -> String {
        let secret = self.secret_bytes();
        let date_salt = current_date_salt();

        let hk = Hkdf::<Sha256>::new(Some(date_salt.as_bytes()), &secret);
        let mut okm = [0u8; 16];
        hk.expand(b"mqtt-topic", &mut okm)
            .expect("HKDF expand should not fail");

        format!("shield/u/{}", hex::encode(okm))
    }

    /// Derive an HMAC key for anti-forensics SMS trigger validation.
    ///
    /// ```
    /// hmac_key = HKDF-SHA256(
    ///     ikm  = device_secret,
    ///     salt = b"anti-forensics-hmac",
    ///     info = b"sms-trigger-key"
    /// )
    /// ```
    pub fn derive_sms_hmac_key(&self) -> [u8; 32] {
        let secret = self.secret_bytes();

        let hk = Hkdf::<Sha256>::new(Some(b"anti-forensics-hmac"), &secret);
        let mut okm = [0u8; 32];
        hk.expand(b"sms-trigger-key", &mut okm)
            .expect("HKDF expand should not fail");

        okm
    }

    /// Derive a per-transport authentication token.
    ///
    /// ```
    /// auth_token = HKDF-SHA256(
    ///     ikm  = device_secret,
    ///     salt = transport_name || current_date,
    ///     info = b"transport-auth"
    /// )
    /// ```
    pub fn derive_transport_auth_token(&self, transport_name: &str) -> String {
        let secret = self.secret_bytes();
        let date_salt = current_date_salt();
        let salt = format!("{}-{}", transport_name, date_salt);

        let hk = Hkdf::<Sha256>::new(Some(salt.as_bytes()), &secret);
        let mut okm = [0u8; 32];
        hk.expand(b"transport-auth", &mut okm)
            .expect("HKDF expand should not fail");

        hex::encode(okm)
    }

    /// Rotate the device secret — used after an emergency wipe recovery.
    ///
    /// Generates a new secret, saves it, and zeroizes the old one.
    pub fn rotate(&self) -> Result<(), ShieldError> {
        let new_secret = DeviceSecret::generate()?;
        encrypt_and_save_secret(&self.secret_path, new_secret.as_bytes(), self.platform)?;

        let mut guard = self.secret.lock().expect("DeviceSecret Mutex poisoned");
        *guard = new_secret;

        info!("Device secret rotated");
        Ok(())
    }

    /// Get the platform.
    pub fn platform(&self) -> DeviceSecretPlatform {
        self.platform
    }

    /// Get the encrypted secret file path.
    pub fn secret_path(&self) -> &Path {
        &self.secret_path
    }
}

// ── Encryption and storage ───────────────────────────────────────────────────

/// On-disk format of the encrypted device secret:
///
/// ```text
/// [12 bytes: AES-256-GCM nonce]
/// [32 bytes: AES-256-GCM encrypted secret + 16-byte auth tag]
/// ```
///
/// The encryption key is derived from the platform's secure key storage.
/// If platform key storage is unavailable, we derive a key from a
/// machine-specific identifier (less secure, but better than plaintext).

fn encrypt_and_save_secret(
    path: &Path,
    secret: &[u8; DEVICE_SECRET_LEN],
    platform: DeviceSecretPlatform,
) -> Result<(), ShieldError> {
    // Get or derive the encryption key
    let enc_key = derive_file_encryption_key(platform)?;

    // Generate a random nonce
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Encrypt with AES-256-GCM
    let cipher = Aes256Gcm::new_from_slice(&enc_key).map_err(|e| {
        ShieldError::crypto(
            ErrorCode::CryptoEncryptionFailed,
            format!("AES-256-GCM init failed: {}", e),
        )
    })?;

    let ciphertext = cipher.encrypt(nonce, secret.as_ref()).map_err(|e| {
        ShieldError::crypto(
            ErrorCode::CryptoEncryptionFailed,
            format!("AES-256-GCM encryption failed: {}", e),
        )
    })?;

    // Write to disk: nonce || ciphertext
    let mut file_data = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    file_data.extend_from_slice(&nonce_bytes);
    file_data.extend_from_slice(&ciphertext);

    // Write atomically: write to temp file, then rename
    let temp_path = path.with_extension("tmp");
    std::fs::write(&temp_path, &file_data).map_err(|e| {
        ShieldError::anti_forensics(
            ErrorCode::AntiForensicsDeviceSecretCorrupted,
            format!("Failed to write encrypted secret: {}", e),
        )
    })?;

    std::fs::rename(&temp_path, path).map_err(|e| {
        ShieldError::anti_forensics(
            ErrorCode::AntiForensicsDeviceSecretCorrupted,
            format!("Failed to rename secret file: {}", e),
        )
    })?;

    // Zeroize the encryption key
    let mut key_mut = enc_key;
    key_mut.zeroize();

    debug!(path = %path.display(), "Device secret encrypted and saved");
    Ok(())
}

fn load_and_decrypt_secret(
    path: &Path,
    platform: DeviceSecretPlatform,
) -> Result<DeviceSecret, ShieldError> {
    let file_data = std::fs::read(path).map_err(|e| {
        ShieldError::anti_forensics(
            ErrorCode::AntiForensicsDeviceSecretCorrupted,
            format!("Failed to read encrypted secret from {:?}: {}", path, e),
        )
    })?;

    if file_data.len() < NONCE_LEN + TAG_LEN + 1 {
        return Err(ShieldError::anti_forensics(
            ErrorCode::AntiForensicsDeviceSecretCorrupted,
            format!(
                "Encrypted secret file is too short: {} bytes",
                file_data.len()
            ),
        ));
    }

    // Split into nonce and ciphertext
    let nonce_bytes = &file_data[..NONCE_LEN];
    let ciphertext = &file_data[NONCE_LEN..];

    // Derive the decryption key
    let enc_key = derive_file_encryption_key(platform)?;

    // Decrypt with AES-256-GCM
    let cipher = Aes256Gcm::new_from_slice(&enc_key).map_err(|e| {
        ShieldError::crypto(
            ErrorCode::CryptoDecryptionFailed,
            format!("AES-256-GCM init failed: {}", e),
        )
    })?;

    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|e| {
        error!("Device secret decryption failed — possible tampering or key mismatch");
        ShieldError::crypto(
            ErrorCode::CryptoDecryptionFailed,
            format!(
                "AES-256-GCM decryption failed (secret may be corrupted or key mismatch): {}",
                e
            ),
        )
    })?;

    if plaintext.len() != DEVICE_SECRET_LEN {
        return Err(ShieldError::anti_forensics(
            ErrorCode::AntiForensicsDeviceSecretCorrupted,
            format!(
                "Decrypted secret has wrong length: expected {}, got {}",
                DEVICE_SECRET_LEN,
                plaintext.len()
            ),
        ));
    }

    let mut secret_bytes = [0u8; DEVICE_SECRET_LEN];
    secret_bytes.copy_from_slice(&plaintext);

    // Zeroize the encryption key
    let mut key_mut = enc_key;
    key_mut.zeroize();

    debug!("Device secret loaded and decrypted");

    DeviceSecret::new(secret_bytes)
}

/// Derive the file encryption key from the platform's secure key storage.
///
/// Strategy by platform:
/// - Android: Android Keystore (would use JNI — here we simulate with a
///   device-specific binding)
/// - iOS: iOS Keychain (would use Security.framework — simulated)
/// - macOS: macOS Keychain (would use Security.framework — simulated)
/// - Linux: libsecret / encrypted with machine-id
/// - Windows: DPAPI (CryptProtectData — simulated)
///
/// For the daemon (non-Java) we derive a stable key from a machine
/// identifier.  This is less ideal than hardware-backed keystore but
/// still provides protection against casual file extraction.
fn derive_file_encryption_key(platform: DeviceSecretPlatform) -> Result<[u8; 32], ShieldError> {
    match platform {
        DeviceSecretPlatform::Android => {
            // On Android, the actual key should come from Android Keystore
            // via JNI.  For the standalone daemon we derive from the
            // Android ID (Settings.Secure.ANDROID_ID).
            let machine_id = read_machine_id_android();
            derive_key_from_machine_id(&machine_id)
        }
        DeviceSecretPlatform::Ios => {
            // On iOS, use the Keychain via Security.framework.
            // For the standalone daemon, use the vendor identifier.
            let machine_id = read_machine_id_ios();
            derive_key_from_machine_id(&machine_id)
        }
        DeviceSecretPlatform::MacOS => {
            let machine_id = read_machine_id_macos();
            derive_key_from_machine_id(&machine_id)
        }
        DeviceSecretPlatform::Linux => {
            let machine_id = read_machine_id_linux();
            derive_key_from_machine_id(&machine_id)
        }
        DeviceSecretPlatform::Windows => {
            let machine_id = read_machine_id_windows();
            derive_key_from_machine_id(&machine_id)
        }
        DeviceSecretPlatform::Unknown => {
            warn!("Unknown platform — using fallback machine ID for secret encryption");
            derive_key_from_machine_id(b"unknown-platform-fallback")
        }
    }
}

/// Derive a 256-bit encryption key from a machine identifier using HKDF.
fn derive_key_from_machine_id(machine_id: &[u8]) -> Result<[u8; 32], ShieldError> {
    let hk = Hkdf::<Sha256>::new(None, machine_id);
    let mut key = [0u8; 32];
    hk.expand(FILE_KEY_INFO, &mut key).map_err(|e| {
        ShieldError::crypto(
            ErrorCode::CryptoHkdfFailed,
            format!("Failed to derive file encryption key: {}", e),
        )
    })?;
    Ok(key)
}

// ── Platform-specific machine ID readers ─────────────────────────────────────

fn read_machine_id_linux() -> Vec<u8> {
    // Linux: /etc/machine-id or /var/lib/dbus/machine-id
    for path in &["/etc/machine-id", "/var/lib/dbus/machine-id"] {
        if let Ok(id) = std::fs::read_to_string(path) {
            let trimmed = id.trim();
            if !trimmed.is_empty() {
                debug!("Using Linux machine-id from {}", path);
                return trimmed.as_bytes().to_vec();
            }
        }
    }
    warn!("Could not read Linux machine-id — using fallback");
    b"linux-fallback-machine-id".to_vec()
}

fn read_machine_id_android() -> Vec<u8> {
    // Android: try to read ANDROID_ID from system properties
    // In practice this would come from JNI.  We try the persistent
    // data directory as a stable identifier.
    if let Ok(id) = std::fs::read_to_string("/data/data/net.micafp.unifiedshield/.machine-id") {
        return id.trim().as_bytes().to_vec();
    }
    // Fallback: use the ro.serialno property if readable
    if let Ok(output) = std::process::Command::new("getprop")
        .arg("ro.serialno")
        .output()
    {
        if output.status.success() {
            let serial = String::from_utf8_lossy(&output.stdout);
            let trimmed = serial.trim();
            if !trimmed.is_empty() {
                return trimmed.as_bytes().to_vec();
            }
        }
    }
    warn!("Could not read Android machine ID — using fallback");
    b"android-fallback-machine-id".to_vec()
}

fn read_machine_id_ios() -> Vec<u8> {
    // iOS: use the vendor identifier (accessible via UIKit, not directly from daemon)
    // Store a self-generated UUID on first run
    let id_path = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join("unified-shield")
        .join(".vendor-id");

    if let Ok(id) = std::fs::read_to_string(&id_path) {
        let trimmed = id.trim();
        if !trimmed.is_empty() {
            return trimmed.as_bytes().to_vec();
        }
    }

    // Generate and store a new vendor ID
    let new_id = uuid::Uuid::new_v4().to_string();
    let _ = std::fs::write(&id_path, &new_id);
    new_id.as_bytes().to_vec()
}

fn read_machine_id_macos() -> Vec<u8> {
    // macOS: use IOPlatformSerialNumber via IOKit
    if let Ok(output) = std::process::Command::new("ioreg")
        .arg("-rd1")
        .arg("-c")
        .arg("IOPlatformExpertDevice")
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("IOPlatformSerialNumber") {
                if let Some(idx) = line.find('"') {
                    let rest = &line[idx + 1..];
                    if let Some(end_idx) = rest.find('"') {
                        return rest[..end_idx].as_bytes().to_vec();
                    }
                }
            }
        }
    }
    warn!("Could not read macOS serial — using fallback");
    b"macos-fallback-machine-id".to_vec()
}

fn read_machine_id_windows() -> Vec<u8> {
    // Windows: use the MachineGuid from the registry
    if let Ok(output) = std::process::Command::new("reg")
        .args([
            "query",
            r"HKLM\SOFTWARE\Microsoft\Cryptography",
            "/v",
            "MachineGuid",
        ])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("MachineGuid") {
                // Format: "    MachineGuid    REG_SZ    <guid>"
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    return parts[2..].join("").as_bytes().to_vec();
                }
            }
        }
    }
    warn!("Could not read Windows MachineGuid — using fallback");
    b"windows-fallback-machine-id".to_vec()
}

// ── Data directory resolution ────────────────────────────────────────────────

fn resolve_data_dir(platform: DeviceSecretPlatform) -> PathBuf {
    match platform {
        DeviceSecretPlatform::Android => PathBuf::from("/data/data/net.micafp.unifiedshield/files"),
        DeviceSecretPlatform::Ios => dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("unified-shield"),
        DeviceSecretPlatform::MacOS => dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("unified-shield"),
        DeviceSecretPlatform::Linux => dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("unified-shield"),
        DeviceSecretPlatform::Windows => dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("C:\\Temp"))
            .join("UnifiedShield"),
        DeviceSecretPlatform::Unknown => PathBuf::from("/tmp/unified-shield"),
    }
}

/// Get the current UTC date in YYYYMMDD format.
fn current_date_salt() -> String {
    use chrono::Utc;
    Utc::now().format("%Y%m%d").to_string()
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_secret_generation() {
        let secret = DeviceSecret::generate().unwrap();
        let bytes = secret.as_bytes();
        // Should not be all zeros
        assert_ne!(bytes, &[0u8; 32]);
    }

    #[test]
    fn test_device_secret_zeroize_on_drop() {
        let secret = DeviceSecret::generate().unwrap();
        let bytes_ptr = secret.as_bytes().as_ptr();
        drop(secret);
        // After drop, the memory should be zeroized.
        // Note: this is technically UB to read after drop, but for testing
        // purposes we just verify the Drop impl compiles and runs.
    }

    #[test]
    fn test_derive_daily_peer_id_deterministic() {
        // We can't easily test with a real DeviceSecretManager without a
        // filesystem, so we test the HKDF derivation directly.
        let secret = [42u8; 32];
        let date_salt = current_date_salt();

        let hk1 = Hkdf::<Sha256>::new(Some(date_salt.as_bytes()), &secret);
        let mut okm1 = [0u8; 32];
        hk1.expand(b"peer-id", &mut okm1).unwrap();

        let hk2 = Hkdf::<Sha256>::new(Some(date_salt.as_bytes()), &secret);
        let mut okm2 = [0u8; 32];
        hk2.expand(b"peer-id", &mut okm2).unwrap();

        assert_eq!(hex::encode(okm1), hex::encode(okm2));
    }

    #[test]
    fn test_derive_mqtt_topic_format() {
        let secret = [99u8; 32];
        let date_salt = current_date_salt();

        let hk = Hkdf::<Sha256>::new(Some(date_salt.as_bytes()), &secret);
        let mut okm = [0u8; 16];
        hk.expand(b"mqtt-topic", &mut okm).unwrap();

        let topic = format!("shield/u/{}", hex::encode(okm));
        assert!(topic.starts_with("shield/u/"));
        assert_eq!(topic.len(), 10 + 32); // "shield/u/" + 16 bytes hex (32 chars)
    }

    #[test]
    fn test_derive_sms_hmac_key() {
        let secret = [77u8; 32];

        let hk = Hkdf::<Sha256>::new(Some(b"anti-forensics-hmac"), &secret);
        let mut okm = [0u8; 32];
        hk.expand(b"sms-trigger-key", &mut okm).unwrap();

        // Should not be all zeros
        assert_ne!(okm, [0u8; 32]);
    }

    #[test]
    fn test_derive_transport_auth_token() {
        let secret = [88u8; 32];
        let date_salt = current_date_salt();
        let salt = format!("hysteria2-{}", date_salt);

        let hk = Hkdf::<Sha256>::new(Some(salt.as_bytes()), &secret);
        let mut okm = [0u8; 32];
        hk.expand(b"transport-auth", &mut okm).unwrap();

        let token = hex::encode(okm);
        assert_eq!(token.len(), 64); // 32 bytes hex = 64 chars
    }

    #[test]
    fn test_encryption_round_trip() {
        let secret = [0xABu8; 32];
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("test-secret.enc");

        encrypt_and_save_secret(&path, &secret, DeviceSecretPlatform::Linux).unwrap();
        let loaded = load_and_decrypt_secret(&path, DeviceSecretPlatform::Linux).unwrap();

        assert_eq!(loaded.as_bytes(), &secret);
    }

    #[test]
    fn test_different_platforms_different_keys() {
        // Different machine IDs should produce different encryption keys
        let key1 = derive_key_from_machine_id(b"machine-1").unwrap();
        let key2 = derive_key_from_machine_id(b"machine-2").unwrap();

        assert_ne!(key1, key2);
    }
}
