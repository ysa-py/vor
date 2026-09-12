//! Offline Ed25519 license-token verification (see `license/SPEC.md`).
//!
//! Token: `VOR1.<base64url(payload_json)>.<base64url(signature)>`.
//! Verification never touches the network and needs only the public key.

use crate::b64;
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

/// License status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LicenseStatus {
    /// Signature valid and not expired.
    Valid,
    /// Signature valid but `now >= expires_at`.
    Expired,
    /// Malformed, bad signature, wrong product or version.
    Invalid,
}

impl LicenseStatus {
    /// Wire name (stable across the Go / Kotlin / Python ports).
    pub fn as_str(&self) -> &'static str {
        match self {
            LicenseStatus::Valid => "VALID",
            LicenseStatus::Expired => "EXPIRED",
            LicenseStatus::Invalid => "INVALID",
        }
    }
}

/// Result of verification: status plus the decoded payload (when it parsed).
#[derive(Debug, Clone)]
pub struct LicenseResult {
    /// Verification status.
    pub status: LicenseStatus,
    /// Parsed payload JSON, when the envelope decoded at all.
    pub payload: Option<Value>,
}

/// Verify a license token against a base64url Ed25519 public key at
/// unix-epoch `now` (seconds).
pub fn verify(public_key_b64: &str, token: &str, now_epoch_secs: i64) -> LicenseResult {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    let invalid = || LicenseResult {
        status: LicenseStatus::Invalid,
        payload: None,
    };

    // Key
    let key_bytes = match b64::decode(public_key_b64.trim()) {
        Some(bytes) if bytes.len() == 32 => bytes,
        _ => return invalid(),
    };
    let key_bytes: [u8; 32] = match key_bytes.as_slice().try_into() {
        Ok(array) => array,
        Err(_) => return invalid(),
    };
    let verifying_key = match VerifyingKey::from_bytes(&key_bytes) {
        Ok(key) => key,
        Err(_) => return invalid(),
    };

    // Envelope
    let parts: Vec<&str> = token.trim().split('.').collect();
    if parts.len() != 3 || parts[0] != "VOR1" {
        return invalid();
    }
    let payload_bytes = match b64::decode(parts[1]) {
        Some(bytes) => bytes,
        None => return invalid(),
    };
    let signature_bytes = match b64::decode(parts[2]) {
        Some(bytes) if bytes.len() == 64 => bytes,
        _ => return invalid(),
    };
    let signature = match Signature::from_slice(&signature_bytes) {
        Ok(signature) => signature,
        Err(_) => return invalid(),
    };

    // Signature
    if verifying_key.verify(&payload_bytes, &signature).is_err() {
        return invalid();
    }

    // Payload semantics
    let payload: Value = match serde_json::from_slice(&payload_bytes) {
        Ok(value) => value,
        Err(_) => return invalid(),
    };
    if payload.get("v").and_then(Value::as_i64) != Some(1)
        || payload.get("product").and_then(Value::as_str) != Some("vor")
        || payload.get("id").is_none()
        || payload.get("issued_at").is_none()
    {
        return LicenseResult {
            status: LicenseStatus::Invalid,
            payload: Some(payload),
        };
    }
    let expires_at = match payload.get("expires_at").and_then(Value::as_str) {
        Some(text) => text.to_string(),
        None => {
            return LicenseResult {
                status: LicenseStatus::Invalid,
                payload: Some(payload),
            }
        }
    };
    let expiry_epoch = match parse_rfc3339_epoch(&expires_at) {
        Some(epoch) => epoch,
        None => {
            return LicenseResult {
                status: LicenseStatus::Invalid,
                payload: Some(payload),
            }
        }
    };

    let status = if now_epoch_secs >= expiry_epoch {
        LicenseStatus::Expired
    } else {
        LicenseStatus::Valid
    };
    LicenseResult {
        status,
        payload: Some(payload),
    }
}

/// Current unix time (seconds).
pub fn now_epoch_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Parse an RFC 3339 timestamp ("2027-01-01T00:00:00Z") to unix seconds.
/// Supports the Z suffix, fractional seconds and +hh:mm/-hh:mm offsets —
/// the forms the issuer emits.
pub fn parse_rfc3339_epoch(text: &str) -> Option<i64> {
    let text = text.trim();
    let (date_part, time_part) = text.split_once('T')?;
    let mut body = time_part;
    let mut offset: i64 = 0;
    if let Some(stripped) = body.strip_suffix('Z') {
        body = stripped;
    } else if body.len() > 6 {
        let sign: i64 = match body.as_bytes()[body.len() - 6] {
            b'+' => 1,
            b'-' => -1,
            _ => 0,
        };
        if sign != 0 {
            let hh: i64 = body[body.len() - 5..body.len() - 3].parse().ok()?;
            let mm: i64 = body[body.len() - 2..].parse().ok()?;
            offset = sign * (hh * 3600 + mm * 60);
            body = &body[..body.len() - 6];
        }
    }
    date_time_to_epoch(date_part, body).map(|epoch| epoch - offset)
}

fn date_time_to_epoch(date_part: &str, time_part: &str) -> Option<i64> {
    let (year, month, day) = parse_date(date_part)?;
    let (hour, minute, second) = parse_time(time_part)?;
    Some(days_from_civil(year, month, day)? * 86400 + hour * 3600 + minute * 60 + second)
}

fn parse_date(text: &str) -> Option<(i64, i64, i64)> {
    let mut parts = text.split('-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: i64 = parts.next()?.parse().ok()?;
    let day: i64 = parts.next()?.parse().ok()?;
    if !(1..=12).contains(&month) || !(1970..=9999).contains(&year) {
        return None;
    }
    // Real calendar validation (matches the Python/Go/iOS ports and the
    // shared `impossible_day` conformance vector in license/vectors.json).
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let day_cap = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        _ => return None,
    };
    if !(1..=day_cap).contains(&day) {
        return None;
    }
    Some((year, month, day))
}

fn parse_time(text: &str) -> Option<(i64, i64, i64)> {
    // Drop fractional seconds (".123") if present.
    let text = text.split('.').next().unwrap_or(text);
    let mut parts = text.split(':');
    let hour: i64 = parts.next()?.parse().ok()?;
    let minute: i64 = parts.next()?.parse().ok()?;
    let second: i64 = parts.next().unwrap_or("0").parse().ok()?;
    if !(0..=23).contains(&hour) || !(0..=59).contains(&minute) || !(0..=60).contains(&second) {
        return None;
    }
    Some((hour, minute, second))
}

/// Days since unix epoch from a civil date (Howard Hinnant's algorithm).
fn days_from_civil(year: i64, month: i64, day: i64) -> Option<i64> {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146097 + doe - 719468)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rfc3339() {
        assert_eq!(parse_rfc3339_epoch("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(
            parse_rfc3339_epoch("2024-01-01T00:00:00Z"),
            Some(1704067200)
        );
        assert_eq!(
            parse_rfc3339_epoch("2027-01-01T00:00:00Z"),
            Some(1798761600)
        );
        assert_eq!(
            parse_rfc3339_epoch("2026-09-10T12:00:00Z"),
            Some(1789041600)
        );
        assert_eq!(
            parse_rfc3339_epoch("2026-09-10T12:00:00+02:00"),
            Some(1789034400)
        );
        assert_eq!(
            parse_rfc3339_epoch("2026-09-10T12:00:00.123Z"),
            Some(1789041600)
        );
        assert_eq!(parse_rfc3339_epoch("garbage"), None);
        assert_eq!(parse_rfc3339_epoch("2026-13-01T00:00:00Z"), None);
        // Impossible calendar days (shared `impossible_day` vector).
        assert_eq!(parse_rfc3339_epoch("2026-09-31T00:00:00Z"), None);
        assert_eq!(parse_rfc3339_epoch("2023-02-29T00:00:00Z"), None);
        assert_eq!(parse_rfc3339_epoch("2024-02-30T00:00:00Z"), None);
        // Real leap days still parse.
        assert_eq!(
            parse_rfc3339_epoch("2024-02-29T00:00:00Z"),
            Some(1709164800)
        );
    }

    #[test]
    fn rejects_malformed_envelopes() {
        // A dummy 32-byte key (all zeros is a valid curve point encoding-wise;
        // verification must simply fail on the signature).
        let key = crate::b64::encode(&[0u8; 32]);
        assert_eq!(verify(&key, "", 0).status, LicenseStatus::Invalid);
        assert_eq!(
            verify(&key, "VOR1.only-two", 0).status,
            LicenseStatus::Invalid
        );
        assert_eq!(verify(&key, "VORX.a.b", 0).status, LicenseStatus::Invalid);
        assert_eq!(
            verify(&key, "VOR1.!!!.AAAA", 0).status,
            LicenseStatus::Invalid
        );
    }
}
