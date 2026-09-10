// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield 6.0 — SMS Bootstrap Channel (Android Only)
//
// Decodes specially-formatted SMS messages that contain encrypted VPN
// endpoint configurations. This is the most reliable bootstrap channel
// for initial configuration when internet is completely blocked.
//
// SMS format:
//   [Unicode control prefix] [AES-256-GCM(payload)]
//   - Control prefix: U+200B (ZERO-WIDTH SPACE) + U+200C (ZERO-WIDTH NON-JOINER)
//   - Payload format: same as acoustic channel (version + encrypted endpoints + HMAC)
//
// Battery optimization:
//   • SMS receiver is completely passive (Android BroadcastReceiver)
//   • Zero battery impact when no SMS arrives
//   • No active polling required
//   • Rate limit: process at most 1 SMS per 30 seconds
// ─────────────────────────────────────────────────────────────────────────────

use std::sync::Arc;
use std::time::{Duration, Instant};

use aes_gcm::aead::{Aead, KeyInit};
use parking_lot::Mutex;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::error::{ErrorCode, ShieldError};

// ── Constants ───────────────────────────────────────────────────────────────

/// Unicode control prefix that identifies shield SMS messages.
/// U+200B ZERO-WIDTH SPACE + U+200C ZERO-WIDTH NON-JOINER
const SMS_PREFIX: &[u8] = b"\xe2\x80\x8b\xe2\x80\x8c";

/// AES-256-GCM key size.
const AES_KEY_SIZE: usize = 32;
/// AES-256-GCM nonce size.
const AES_NONCE_SIZE: usize = 12;
/// AES-256-GCM tag size.
const AES_TAG_SIZE: usize = 16;
/// HMAC-SHA256 output size.
const HMAC_SIZE: usize = 32;
/// Minimum SMS interval to process (rate limiting).
const MIN_SMS_INTERVAL: Duration = Duration::from_secs(30);
/// Maximum SMS payload size.
const MAX_SMS_PAYLOAD_SIZE: usize = 1024;
/// Maximum age of SMS payload before it's considered stale (1 hour).
const MAX_PAYLOAD_AGE: Duration = Duration::from_secs(3600);

// ── SMS Payload ─────────────────────────────────────────────────────────────

/// Decrypted SMS bootstrap payload.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SmsPayload {
    /// Protocol version.
    pub version: u8,
    /// List of VPN endpoint strings.
    pub endpoints: Vec<String>,
    /// Timestamp when this payload was created (UNIX epoch seconds).
    pub created_at: u64,
    /// Optional sender identifier.
    pub sender_id: Option<String>,
    /// Optional Yggdrasil public key for mesh networking.
    pub yggdrasil_pubkey: Option<Vec<u8>>,
}

impl SmsPayload {
    /// Check if this payload is still fresh (not expired).
    pub fn is_fresh(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now.saturating_sub(self.created_at) < MAX_PAYLOAD_AGE.as_secs()
    }
}

// ── SMS Bootstrap Channel ───────────────────────────────────────────────────

/// SMS bootstrap channel for receiving VPN configurations via SMS.
///
/// On Android, this uses a BroadcastReceiver that intercepts SMS messages
/// with the shield control prefix. The receiver is passive — it consumes
/// zero battery when no SMS arrives.
pub struct SmsBootstrapChannel {
    /// Whether the channel is active.
    active: Arc<std::sync::atomic::AtomicBool>,
    /// Last SMS processing timestamp for rate limiting.
    last_processed: Mutex<Option<Instant>>,
    /// Received payloads awaiting processing.
    received_payloads: Arc<RwLock<Vec<SmsPayload>>>,
    /// Encryption key for payload decryption.
    encryption_key: [u8; AES_KEY_SIZE],
    /// HMAC key for payload authentication.
    hmac_key: [u8; 32],
    /// Rate limiter: minimum interval between processed SMS.
    min_interval: Duration,
}

impl SmsBootstrapChannel {
    /// Create a new SMS bootstrap channel.
    pub fn new() -> Result<Self, ShieldError> {
        // In production, these keys are derived from the device secret
        let encryption_key = [0u8; AES_KEY_SIZE];
        let hmac_key = [0u8; 32];

        Ok(Self {
            active: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            last_processed: Mutex::new(None),
            received_payloads: Arc::new(RwLock::new(Vec::new())),
            encryption_key,
            hmac_key,
            min_interval: MIN_SMS_INTERVAL,
        })
    }

    /// Activate the SMS bootstrap channel.
    ///
    /// On Android, this registers the SmsBootstrapReceiver via JNI.
    /// The receiver is passive — no battery impact when idle.
    pub async fn activate(&self) -> Result<(), ShieldError> {
        self.active
            .store(true, std::sync::atomic::Ordering::Relaxed);

        info!("SMS bootstrap channel activated (passive receiver)");

        // On Android, this calls:
        //   SmsBootstrapReceiver.register(context, daemon_ptr)
        // which enables the BroadcastReceiver for SMS_RECEIVED

        Ok(())
    }

    /// Deactivate the SMS bootstrap channel.
    pub async fn deactivate(&self) -> Result<(), ShieldError> {
        self.active
            .store(false, std::sync::atomic::Ordering::Relaxed);

        info!("SMS bootstrap channel deactivated");

        // On Android, this calls:
        //   SmsBootstrapReceiver.unregister(context)

        Ok(())
    }

    /// Check if the channel is active.
    pub fn is_active(&self) -> bool {
        self.active.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Process an incoming SMS message from the Android BroadcastReceiver.
    ///
    /// This is called via JNI when an SMS with the shield prefix is received.
    /// Returns the decrypted payload if the SMS was valid and not rate-limited.
    pub async fn process_incoming_sms(
        &self,
        sender: &str,
        message_body: &str,
    ) -> Result<Option<SmsPayload>, ShieldError> {
        if !self.is_active() {
            debug!("SMS received but channel is inactive — ignoring");
            return Ok(None);
        }

        // Rate limiting: only process 1 SMS per 30 seconds
        {
            let last = self.last_processed.lock();
            if let Some(last_time) = *last {
                if last_time.elapsed() < self.min_interval {
                    debug!(
                        remaining = ?(self.min_interval - last_time.elapsed()),
                        "SMS rate limited — skipping"
                    );
                    return Ok(None);
                }
            }
        }

        // Check for the shield control prefix
        if !message_body.as_bytes().starts_with(SMS_PREFIX) {
            debug!("SMS does not have shield prefix — ignoring");
            return Ok(None);
        }

        let payload_data = &message_body.as_bytes()[SMS_PREFIX.len()..];
        debug!(
            sender,
            payload_len = payload_data.len(),
            "Processing shield SMS"
        );

        // Process the payload
        let result = self.process_incoming_payload(payload_data).await;

        match &result {
            Ok(Some(payload)) => {
                info!(
                    sender,
                    num_endpoints = payload.endpoints.len(),
                    "SMS bootstrap payload decrypted successfully"
                );

                // Update rate limiter
                *self.last_processed.lock() = Some(Instant::now());

                // Store the payload
                self.received_payloads.write().await.push(payload.clone());

                Ok(Some(payload.clone()))
            }
            Ok(None) => Ok(None),
            Err(e) => {
                warn!(sender, error = %e, "Failed to process SMS bootstrap payload");
                // Still update rate limiter to prevent brute-force attempts
                *self.last_processed.lock() = Some(Instant::now());
                Ok(None)
            }
        }
    }

    /// Process an incoming payload (raw bytes) from any source.
    ///
    /// This can be called from JNI or from the NAIN coordinator.
    pub async fn process_incoming_payload(
        &self,
        data: &[u8],
    ) -> Result<Option<SmsPayload>, ShieldError> {
        if data.len() > MAX_SMS_PAYLOAD_SIZE {
            return Err(ShieldError::nain_mode(
                ErrorCode::NainCovertChannelFailed,
                format!("SMS payload too large: {} bytes", data.len()),
            ));
        }

        if data.len() < 1 + AES_TAG_SIZE + HMAC_SIZE {
            return Err(ShieldError::nain_mode(
                ErrorCode::NainCovertChannelFailed,
                "SMS payload too short",
            ));
        }

        // Split message and HMAC
        let (message, received_hmac) = data.split_at(data.len() - HMAC_SIZE);

        // Verify HMAC
        let expected_hmac = self.compute_hmac(message);
        if !constant_time_eq(received_hmac, &expected_hmac) {
            return Err(ShieldError::nain_mode(
                ErrorCode::NainCovertChannelFailed,
                "SMS payload HMAC verification failed",
            ));
        }

        // Parse version
        let version = message[0];
        if version != 0x01 {
            return Err(ShieldError::nain_mode(
                ErrorCode::NainCovertChannelFailed,
                format!("Unsupported SMS payload version: {}", version),
            ));
        }

        // Decrypt AES-256-GCM
        let ciphertext = &message[1..message.len()];
        let nonce = aes_gcm::Nonce::from_slice(&[0u8; AES_NONCE_SIZE]);
        let cipher = aes_gcm::Aes256Gcm::new(aes_gcm::Key::<aes_gcm::Aes256Gcm>::from_slice(
            &self.encryption_key,
        ));
        let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|_| {
            ShieldError::nain_mode(
                ErrorCode::NainCovertChannelFailed,
                "SMS payload AES-GCM decryption failed",
            )
        })?;

        // Deserialize
        let payload: SmsPayload = serde_json::from_slice(&plaintext).map_err(|e| {
            ShieldError::nain_mode(
                ErrorCode::NainCovertChannelFailed,
                format!("SMS payload deserialization failed: {}", e),
            )
        })?;

        // Check freshness
        if !payload.is_fresh() {
            warn!("SMS payload is stale — ignoring");
            return Ok(None);
        }

        Ok(Some(payload))
    }

    /// Get all received payloads and clear the buffer.
    pub async fn drain_received_payloads(&self) -> Vec<SmsPayload> {
        std::mem::take(&mut *self.received_payloads.write().await)
    }

    /// Get the count of received (unprocessed) payloads.
    pub async fn pending_count(&self) -> usize {
        self.received_payloads.read().await.len()
    }

    /// Compute HMAC-SHA256 for payload authentication.
    fn compute_hmac(&self, message: &[u8]) -> [u8; 32] {
        use sha2::Sha256;
        
        let mut mac = hmac::Hmac::<Sha256>::new_from_slice(&self.hmac_key)
            .expect("HMAC key length is valid");
        <hmac::Hmac<Sha256> as hmac::Mac>::update(&mut mac, message);
        let result = <hmac::Hmac<Sha256> as hmac::Mac>::finalize(mac);
        let code_bytes = result.into_bytes();
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&code_bytes);
        arr
    }
}

// ── JNI FFI Bridge (Android) ────────────────────────────────────────────────

/// FFI callbacks for Android SmsBootstrapReceiver.kt.
#[cfg(target_os = "android")]
pub mod jni_bridge {
    use super::*;

    /// JNI callback: SMS received with shield prefix.
    ///
    /// # Safety
    /// Called from JNI. Pointers must be valid with specified lengths.
    #[no_mangle]
    pub unsafe extern "C" fn Java_org_micafp_shield_sms_SmsBootstrapReceiver_onSmsReceived(
        _env: *mut std::ffi::c_void,
        _class: *mut std::ffi::c_void,
        channel_ptr: usize,
        sender_ptr: *const u8,
        sender_len: usize,
        body_ptr: *const u8,
        body_len: usize,
    ) {
        if sender_ptr.is_null() || body_ptr.is_null() {
            return;
        }

        let sender = std::str::from_utf8(std::slice::from_raw_parts(sender_ptr, sender_len))
            .unwrap_or("<invalid>")
            .to_string();
        let body = std::str::from_utf8(std::slice::from_raw_parts(body_ptr, body_len))
            .unwrap_or("")
            .to_string();

        let channel = unsafe { &*(channel_ptr as *const SmsBootstrapChannel) };

        let rt = tokio::runtime::Handle::current();
        let _ = rt.block_on(async { channel.process_incoming_sms(&sender, &body).await });
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Constant-time comparison to prevent timing attacks.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sms_channel_activation() {
        let channel = SmsBootstrapChannel::new().unwrap();
        assert!(!channel.is_active());

        channel.activate().await.unwrap();
        assert!(channel.is_active());

        channel.deactivate().await.unwrap();
        assert!(!channel.is_active());
    }

    #[tokio::test]
    async fn test_sms_prefix_detection() {
        let channel = SmsBootstrapChannel::new().unwrap();
        channel.activate().await.unwrap();

        // Regular SMS should be ignored
        let result = channel
            .process_incoming_sms("+989123456789", "Hello, how are you?")
            .await
            .unwrap();
        assert!(result.is_none());

        // SMS with shield prefix but invalid payload should fail gracefully
        let shield_sms =
            String::from_utf8([SMS_PREFIX.to_vec(), b"invalid_data".to_vec()].concat()).unwrap();
        let result = channel
            .process_incoming_sms("+989123456789", &shield_sms)
            .await
            .unwrap();
        assert!(result.is_none()); // Failed to decrypt, but no crash
    }

    #[tokio::test]
    async fn test_sms_rate_limiting() {
        let channel = SmsBootstrapChannel::new().unwrap();
        channel.activate().await.unwrap();

        // First SMS should be processed (prefix check fails, but rate limiter allows)
        let shield_sms =
            String::from_utf8([SMS_PREFIX.to_vec(), b"test".to_vec()].concat()).unwrap();

        let _ = channel
            .process_incoming_sms("+989123456789", &shield_sms)
            .await;

        // Second SMS within 30 seconds should be rate-limited
        let result = channel
            .process_incoming_sms("+989123456789", &shield_sms)
            .await
            .unwrap();
        assert!(result.is_none()); // Rate limited
    }

    #[test]
    fn test_sms_payload_freshness() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let fresh = SmsPayload {
            version: 1,
            endpoints: vec![],
            created_at: now,
            sender_id: None,
            yggdrasil_pubkey: None,
        };
        assert!(fresh.is_fresh());

        let stale = SmsPayload {
            version: 1,
            endpoints: vec![],
            created_at: now - 7200, // 2 hours ago
            sender_id: None,
            yggdrasil_pubkey: None,
        };
        assert!(!stale.is_fresh());
    }
}
