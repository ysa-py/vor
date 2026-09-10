//! Byte-level TLS ClientHello fragmentation planner.
//!
//! Strategies and byte-level rules are ported from:
//! * MICAFP `daemon/src/obfuscation/tls_fragment.rs` — `SNI_SPLIT` ("most
//!   effective against FAVA v2"), `RECORD_SPLIT` (FAVA v1 reassembly-buffer
//!   exploit), `RANDOM_SPLIT`;
//! * UAC `MciFragmenter` strategy taxonomy — `FULLn`, `SNI_BOUNDARY`,
//!   `TLS_RECORD_FRAG`, `TLS_SNI_RECORDS`, `HALF`, plus the "external
//!   finalmask" 5-byte-prefix + remainder split used by the Edge Bridge.
//!
//! This module never touches a socket: it turns a ClientHello record into a
//! **plan** (ordered byte ranges + inter-write delays), and each platform
//! applies the plan to its own write path (Kotlin socket writes, Go
//! `net.Buffers`, C `send(MSG_MORE)`). The bytes of the record are never
//! modified — only the write boundaries change, so the peer sees a valid TLS
//! stream and the DPI's reassembly window is what breaks.

use serde::{Deserialize, Serialize};

/// TLS constants.
pub const TLS_CONTENT_HANDSHAKE: u8 = 0x16;
/// Handshake type: ClientHello.
pub const TLS_HANDSHAKE_CLIENT_HELLO: u8 = 0x01;
/// server_name extension type.
pub const TLS_EXT_SERVER_NAME: u16 = 0x0000;

/// Fragmentation strategies (union of MICAFP + UAC taxonomies).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Strategy {
    /// Split at exact SNI start and end (FAVA v2 killer, ~60% effective).
    SniSplit,
    /// Split after the 5-byte TLS record header (FAVA v1 exploit).
    RecordSplit,
    /// 2–4 deterministic-ish random split points (entropy fallback).
    RandomSplit,
    /// Split into `chunk`-byte pieces across the whole record.
    Full(usize),
    /// Split exactly at the SNI start boundary only.
    SniBoundary,
    /// Split before SNI and after SNI, keeping SNI intact in one segment.
    SniSplitKeep,
    /// External-finalmask: 5-byte TLS record prefix, then the remainder
    /// rewritten as a second well-formed record (UAC Edge Bridge form).
    TlsRecords,
    /// Split the record in half.
    Half,
    /// No fragmentation (baseline / control candidate).
    Raw,
}

impl Strategy {
    /// Canonical strategy name used in JSON state and every port.
    pub fn name(&self) -> &'static str {
        match self {
            Strategy::SniSplit => "sni_split",
            Strategy::RecordSplit => "record_split",
            Strategy::RandomSplit => "random_split",
            Strategy::Full(5) => "full5",
            Strategy::Full(10) => "full10",
            Strategy::Full(20) => "full20",
            Strategy::Full(64) => "multi64",
            Strategy::Full(_) => "full",
            Strategy::SniBoundary => "sni_boundary",
            Strategy::SniSplitKeep => "tls_sni_records",
            Strategy::TlsRecords => "tls_record_frag",
            Strategy::Half => "half",
            Strategy::Raw => "raw",
        }
    }

    /// Parse a strategy name (unknown -> `Raw`).
    pub fn from_name(name: &str) -> Strategy {
        match name {
            "sni_split" | "SNI_SPLIT" => Strategy::SniSplit,
            "record_split" | "RECORD_SPLIT" => Strategy::RecordSplit,
            "random_split" | "RANDOM_SPLIT" => Strategy::RandomSplit,
            "full5" => Strategy::Full(5),
            "full10" => Strategy::Full(10),
            "full20" => Strategy::Full(20),
            "multi64" => Strategy::Full(64),
            "sni_boundary" => Strategy::SniBoundary,
            "tls_sni_records" => Strategy::SniSplitKeep,
            "tls_record_frag" => Strategy::TlsRecords,
            "half" => Strategy::Half,
            _ => Strategy::Raw,
        }
    }
}

/// Location of the SNI extension inside a TLS record, in absolute byte
/// offsets from the start of the record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SniLocation {
    /// Offset of the first byte of the SNI extension (type+length header).
    pub extension_start: usize,
    /// Offset one past the last byte of the SNI extension.
    pub extension_end: usize,
    /// Offset of the hostname bytes inside the extension.
    pub hostname_start: usize,
    /// Length of the hostname bytes.
    pub hostname_len: usize,
}

/// A single planned write.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedWrite {
    /// Byte offset (inclusive) of this write.
    pub start: usize,
    /// Byte offset (exclusive) of this write.
    pub end: usize,
    /// Delay in **milliseconds** before performing this write (applies to
    /// every write except the first).
    pub delay_ms: u32,
    /// Whether the writer should hint "more data follows"
    /// (`MSG_MORE` / `Cork` / partial write batching).
    pub more_hint: bool,
}

/// The fragmentation plan for one ClientHello record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FragmentPlan {
    /// Strategy used.
    pub strategy: String,
    /// Total record length.
    pub record_len: usize,
    /// Planned writes in order.
    pub writes: Vec<PlannedWrite>,
}

/// Is this buffer a TLS handshake record containing a ClientHello?
pub fn is_tls_client_hello(data: &[u8]) -> bool {
    data.len() >= 9
        && data[0] == TLS_CONTENT_HANDSHAKE
        && data[1] == 0x03 // TLS 1.0+ record version
        && data[5] == TLS_HANDSHAKE_CLIENT_HELLO
}

/// Locate the SNI (server_name) extension in a ClientHello record.
///
/// Parses: record header (5) | handshake header (4) | client version (2) |
/// random (32) | session-id len (1) + session-id | cipher len (2) + suites |
/// comp len (1) + methods | extensions len (2) | extensions… and returns the
/// absolute offsets of the first `server_name` extension and the hostname
/// inside it. Returns `None` when not parseable or no SNI present.
pub fn find_sni(data: &[u8]) -> Option<SniLocation> {
    if !is_tls_client_hello(data) {
        return None;
    }
    let mut pos = 5 + 4; // record header + handshake header
    pos += 2 + 32; // client version + random
                   // session id
    let session_len = *data.get(pos)? as usize;
    pos += 1 + session_len;
    // cipher suites
    let cipher_len = u16::from_be_bytes([*data.get(pos)?, *data.get(pos + 1)?]) as usize;
    pos += 2 + cipher_len;
    // compression methods
    let comp_len = *data.get(pos)? as usize;
    pos += 1 + comp_len;
    // extensions length
    if pos + 2 > data.len() {
        return None;
    }
    let extensions_len = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
    pos += 2;
    let extensions_end = (pos + extensions_len).min(data.len());

    while pos + 4 <= extensions_end {
        let ext_type = u16::from_be_bytes([data[pos], data[pos + 1]]);
        let ext_len = u16::from_be_bytes([data[pos + 2], data[pos + 3]]) as usize;
        let ext_start = pos;
        let ext_end = (pos + 4 + ext_len).min(extensions_end);
        if ext_type == TLS_EXT_SERVER_NAME {
            // extension payload: list len (2) | type=0 (1) | len (2) | name
            let mut p = ext_start + 4;
            p += 2; // server_name list length
            if p < ext_end && data[p] == 0x00 {
                p += 1;
                if p + 2 <= ext_end {
                    let name_len = u16::from_be_bytes([data[p], data[p + 1]]) as usize;
                    p += 2;
                    let hostname_start = p;
                    let hostname_len = name_len.min(ext_end.saturating_sub(p));
                    return Some(SniLocation {
                        extension_start: ext_start,
                        extension_end: ext_end,
                        hostname_start,
                        hostname_len,
                    });
                }
            }
            // Malformed SNI — still report the extension boundaries.
            return Some(SniLocation {
                extension_start: ext_start,
                extension_end: ext_end,
                hostname_start: ext_start + 4,
                hostname_len: 0,
            });
        }
        pos = ext_end;
    }
    None
}

/// Deterministic pseudo-random split points (xorshift over the record bytes
/// — identical results on every platform, which matters for the conformance
/// vectors; using a seeded PRNG rather than reading OS entropy keeps the
/// plans reproducible).
fn pseudo_random_splits(data: &[u8], min: usize, max: usize) -> Vec<usize> {
    let mut seed: u32 = data
        .iter()
        .fold(0x9E3779B9u32, |acc, b| acc.rotate_left(5) ^ (*b as u32));
    if seed == 0 {
        seed = 0x12345678;
    }
    let count = min.saturating_add((seed as usize) % (max - min + 1));
    let mut points: Vec<usize> = Vec::with_capacity(count);
    let mut value = seed;
    for _ in 0..count {
        value ^= value << 13;
        value ^= value >> 17;
        value ^= value << 5;
        let point = 1 + (value as usize) % (data.len().saturating_sub(1));
        points.push(point);
    }
    points.sort_unstable();
    points.dedup();
    points
}

/// Build the write plan for `data` under `strategy`.
///
/// `inter_write_delay_ms` is applied between consecutive writes (0 on the
/// first). Unparseable or non-ClientHello input yields a single whole-record
/// write (passthrough) — matching MICAFP semantics.
pub fn plan(data: &[u8], strategy: Strategy, inter_write_delay_ms: u32) -> FragmentPlan {
    let record_len = data.len();
    let single = |strategy: &str| FragmentPlan {
        strategy: strategy.to_string(),
        record_len,
        writes: vec![PlannedWrite {
            start: 0,
            end: record_len,
            delay_ms: 0,
            more_hint: false,
        }],
    };

    if !is_tls_client_hello(data) || data.len() < 10 {
        return single("passthrough");
    }

    let sni = find_sni(data);
    let mut points: Vec<usize> = Vec::new();

    match strategy {
        Strategy::Raw => return single(strategy.name()),
        Strategy::Half => points.push(data.len() / 2),
        Strategy::RecordSplit => points.push(5),
        Strategy::TlsRecords => {
            // 5-byte record prefix, then the rest as the second record body
            points.push(5);
        }
        Strategy::Full(chunk) => {
            let chunk = chunk.max(1);
            let mut p = chunk;
            while p < data.len() {
                points.push(p);
                p += chunk;
            }
        }
        Strategy::SniBoundary => {
            if let Some(sni) = sni {
                points.push(sni.extension_start);
            } else {
                points.push(data.len() / 2);
            }
        }
        Strategy::SniSplit | Strategy::SniSplitKeep => {
            if let Some(sni) = sni {
                points.push(sni.extension_start);
                points.push(sni.extension_end);
            } else {
                // No SNI: fall back to a mid-record split (MICAFP behavior)
                points.push(data.len() / 2);
            }
        }
        Strategy::RandomSplit => {
            let sni_end = sni.map(|s| s.extension_end).unwrap_or(data.len() / 2);
            points = pseudo_random_splits(data, 2, 4)
                .into_iter()
                .filter(|p| *p < sni_end)
                .collect();
            points.push(sni_end);
        }
    }

    points.sort_unstable();
    points.dedup();

    // Note: every split point is strictly inside the record (points >= len
    // were never pushed), so a non-empty tail write always follows — which
    // means every split-point write can safely carry more_hint = true.
    let mut writes = Vec::with_capacity(points.len() + 1);
    let mut prev = 0usize;
    for point in points {
        if point <= prev || point >= data.len() {
            continue;
        }
        writes.push(PlannedWrite {
            start: prev,
            end: point,
            delay_ms: if prev == 0 { 0 } else { inter_write_delay_ms },
            more_hint: true,
        });
        prev = point;
    }
    if prev < data.len() {
        writes.push(PlannedWrite {
            start: prev,
            end: data.len(),
            delay_ms: if prev == 0 { 0 } else { inter_write_delay_ms },
            more_hint: false,
        });
    }
    if writes.len() <= 1 {
        return single(strategy.name());
    }

    FragmentPlan {
        strategy: strategy.name().to_string(),
        record_len,
        writes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal but structurally valid ClientHello with SNI "example.com"
    /// followed by a dummy supported_versions extension (real hellos always
    /// carry extensions after the SNI, which the split-at-SNI-end strategies
    /// rely on).
    fn client_hello_with_sni(hostname: &str) -> Vec<u8> {
        let name = hostname.as_bytes();
        // server_name extension payload: list_len(2) type(1) name_len(2) name
        let mut ext = Vec::new();
        let list_len = (1 + 2 + name.len()) as u16;
        ext.extend_from_slice(&list_len.to_be_bytes());
        ext.push(0x00);
        ext.extend_from_slice(&(name.len() as u16).to_be_bytes());
        ext.extend_from_slice(name);

        // dummy supported_versions extension (type 0x002B, 2 bytes)
        let mut trailing = Vec::new();
        trailing.extend_from_slice(&0x002Bu16.to_be_bytes());
        trailing.extend_from_slice(&2u16.to_be_bytes());
        trailing.extend_from_slice(&[0x03, 0x04]);

        let ext_total = ext.len() + 4 + trailing.len();

        let mut handshake = Vec::new();
        handshake.extend_from_slice(&[0x03, 0x03]); // client version TLS1.2
        handshake.extend_from_slice(&[0u8; 32]); // random
        handshake.push(0); // session id len
        handshake.extend_from_slice(&2u16.to_be_bytes()); // cipher len
        handshake.extend_from_slice(&[0x13, 0x01]); // one suite
        handshake.push(1); // comp len
        handshake.push(0x00); // null comp
        handshake.extend_from_slice(&(ext_total as u16).to_be_bytes()); // ext total len
        handshake.extend_from_slice(&TLS_EXT_SERVER_NAME.to_be_bytes());
        handshake.extend_from_slice(&(ext.len() as u16).to_be_bytes());
        handshake.extend_from_slice(&ext);
        handshake.extend_from_slice(&trailing);

        let mut record = Vec::new();
        record.push(TLS_CONTENT_HANDSHAKE);
        record.extend_from_slice(&[0x03, 0x01]);
        record.extend_from_slice(&((handshake.len() + 4) as u16).to_be_bytes());
        record.push(TLS_HANDSHAKE_CLIENT_HELLO);
        record.extend_from_slice(&((handshake.len()) as u32).to_be_bytes()[1..4]); // 3-byte len
        record.extend_from_slice(&handshake);
        record
    }

    #[test]
    fn detects_client_hello_and_finds_sni() {
        let record = client_hello_with_sni("example.com");
        assert!(is_tls_client_hello(&record));
        let sni = find_sni(&record).expect("sni");
        let name = &record[sni.hostname_start..sni.hostname_start + sni.hostname_len];
        assert_eq!(name, b"example.com");
        assert!(sni.extension_start > 5);
        assert!(sni.extension_end <= record.len());
    }

    #[test]
    fn rejects_non_client_hello() {
        assert!(!is_tls_client_hello(&[
            0x17, 0x03, 0x03, 0x00, 0x02, 0x01, 0x02
        ]));
        assert!(find_sni(&[0u8; 64]).is_none());
    }

    #[test]
    fn sni_split_brackets_the_sni() {
        let record = client_hello_with_sni("hcaptcha.com");
        let sni = find_sni(&record).unwrap();
        let frag = plan(&record, Strategy::SniSplit, 0);
        assert!(frag.writes.len() >= 3);
        // first write must end exactly at the SNI extension start
        assert_eq!(frag.writes[0].end, sni.extension_start);
        // some middle write ends exactly at the SNI extension end
        assert!(frag.writes.iter().any(|w| w.end == sni.extension_end));
        // plan covers the whole record with no gaps
        let mut cursor = 0;
        for w in &frag.writes {
            assert_eq!(w.start, cursor);
            cursor = w.end;
        }
        assert_eq!(cursor, record.len());
    }

    #[test]
    fn record_split_is_at_byte_five() {
        let record = client_hello_with_sni("a.com");
        let frag = plan(&record, Strategy::RecordSplit, 20);
        assert_eq!(frag.writes[0].end, 5);
        assert_eq!(frag.writes[1].delay_ms, 20);
        assert!(frag.writes[0].more_hint);
    }

    #[test]
    fn full5_chunks_every_five_bytes() {
        let record = client_hello_with_sni("hcaptcha.com"); // len not multiple of 5
        let frag = plan(&record, Strategy::Full(5), 0);
        for w in &frag.writes[..frag.writes.len() - 1] {
            assert_eq!(w.end - w.start, 5);
        }
        let mut cursor = 0;
        for w in &frag.writes {
            assert_eq!(w.start, cursor);
            cursor = w.end;
        }
        assert_eq!(cursor, record.len());
    }

    #[test]
    fn half_splits_in_middle() {
        let record = client_hello_with_sni("a.com");
        let frag = plan(&record, Strategy::Half, 0);
        assert_eq!(frag.writes.len(), 2);
        assert_eq!(frag.writes[0].end, record.len() / 2);
    }

    #[test]
    fn raw_and_passthrough_are_single_writes() {
        let record = client_hello_with_sni("a.com");
        assert_eq!(plan(&record, Strategy::Raw, 0).writes.len(), 1);
        assert_eq!(plan(&[0u8; 8], Strategy::SniSplit, 0).writes.len(), 1);
    }

    #[test]
    fn random_split_is_deterministic() {
        let record = client_hello_with_sni("example.com");
        let a = plan(&record, Strategy::RandomSplit, 0);
        let b = plan(&record, Strategy::RandomSplit, 0);
        assert_eq!(a.writes, b.writes);
        assert!(a.writes.len() >= 2);
    }

    #[test]
    fn strategy_names_roundtrip() {
        for name in [
            "sni_split",
            "record_split",
            "random_split",
            "full5",
            "full10",
            "full20",
            "multi64",
            "sni_boundary",
            "tls_sni_records",
            "tls_record_frag",
            "half",
            "raw",
        ] {
            assert_eq!(Strategy::from_name(name).name(), name);
        }
    }
}
