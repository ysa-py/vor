//! Minimal base64url (RFC 4648 §5, unpadded) codec.
//!
//! Hand-rolled to keep the dependency surface of the shared library tiny —
//! the same 30 lines of logic are mirrored by every platform port and locked
//! by the shared conformance vectors.

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// Encode bytes to unpadded base64url.
pub fn encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[((triple >> 18) & 63) as usize] as char);
        out.push(ALPHABET[((triple >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(ALPHABET[((triple >> 6) & 63) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(ALPHABET[(triple & 63) as usize] as char);
        }
    }
    out
}

/// Decode unpadded (or padded) base64url. Returns `None` on invalid input.
pub fn decode(text: &str) -> Option<Vec<u8>> {
    let text = text.trim_end_matches('=');
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let mut buffer: u32 = 0;
    let mut bits: u32 = 0;
    for ch in text.bytes() {
        let value = match ch {
            b'A'..=b'Z' => (ch - b'A') as u32,
            b'a'..=b'z' => (ch - b'a' + 26) as u32,
            b'0'..=b'9' => (ch - b'0' + 52) as u32,
            b'-' => 62,
            b'_' => 63,
            _ => return None,
        };
        buffer = (buffer << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buffer >> bits) & 0xFF) as u8);
        }
    }
    // Leftover bits must be zero for canonical encodings; tolerate silently
    // (mirrors Go/Kotlin ports, which reject invalid characters only).
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        for len in 0..40usize {
            let data: Vec<u8> = (0..len as u8).collect();
            let encoded = encode(&data);
            assert_eq!(decode(&encoded).unwrap(), data, "len={len}");
        }
    }

    #[test]
    fn known_vectors() {
        assert_eq!(encode(b""), "");
        assert_eq!(encode(b"f"), "Zg");
        assert_eq!(encode(b"fo"), "Zm8");
        assert_eq!(encode(b"foo"), "Zm9v");
        assert_eq!(encode(b"foob"), "Zm9vYg");
        assert_eq!(encode(b"fooba"), "Zm9vYmE");
        assert_eq!(encode(b"foobar"), "Zm9vYmFy");
        assert_eq!(decode("Zm9vYmFy").unwrap(), b"foobar");
        assert_eq!(decode("Zm9vYmFy==").unwrap(), b"foobar");
    }

    #[test]
    fn rejects_invalid() {
        assert!(decode("!!").is_none());
        assert!(decode("ab!c").is_none());
    }
}
