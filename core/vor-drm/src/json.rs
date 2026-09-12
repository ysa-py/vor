//! Minimal strict JSON parser (RFC 8259 subset) — hand-rolled on purpose.
//!
//! Why not serde: the VOR1 compatibility path only needs to read a handful
//! of fields from a small, well-formed document, and a full serde dependency
//! would bloat the shipped .so and add recognizable error strings for
//! attackers to grep for. This parser is ~300 auditable lines, allocation-
//! bounded ([`MAX_INPUT`]), recursion-bounded ([`MAX_DEPTH`]) and returns
//! plain `None` on any deviation — including trailing garbage.
//!
//! Number handling: integers without fraction/exponent that fit `i64` parse
//! as [`Json::Int`]; everything else parses as [`Json::Float`]. The license
//! semantics only consume integers (`"v": 1`), and a `"v": 1.0` is rejected
//! by the caller exactly like the Kotlin verifier (`"1.0".toIntOrNull() ==
//! null`) — one behavior, no drift.

/// Hard input cap (bytes). License payloads are a few hundred bytes; anything
/// larger is garbage or an attack, and parsing must stay cheap.
pub const MAX_INPUT: usize = 64 * 1024;

/// Nesting depth cap — license JSON is 3 levels deep; 32 is generous while
/// still immune to stack-overflow-by-crafted-nesting.
pub const MAX_DEPTH: usize = 32;

/// A parsed JSON value.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    /// null
    Null,
    /// true / false
    Bool(bool),
    /// Integral number (no fraction/exponent, fits i64).
    Int(i64),
    /// Non-integral number.
    Float(f64),
    /// String (unescaped).
    Str(String),
    /// Array.
    Arr(Vec<Json>),
    /// Object (insertion order preserved; duplicate keys keep the LAST value).
    Obj(Vec<(String, Json)>),
}

impl Json {
    /// Parse a complete document (`None` on any syntax error or cap).
    pub fn parse(text: &str) -> Option<Json> {
        if text.len() > MAX_INPUT {
            return None;
        }
        let bytes = text.as_bytes();
        let mut pos = 0usize;
        let value = parse_value(bytes, &mut pos, 0)?;
        skip_ws(bytes, &mut pos);
        if pos == bytes.len() {
            Some(value)
        } else {
            None
        }
    }

    /// Look up a key in an object (last occurrence wins).
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(entries) => entries.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// Borrow as a string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(text) => Some(text),
            _ => None,
        }
    }

    /// Borrow as an integer.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Json::Int(value) => Some(*value),
            _ => None,
        }
    }

    /// Borrow as an array.
    pub fn as_arr(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(items) => Some(items),
            _ => None,
        }
    }

    /// Borrow as an object.
    pub fn as_obj(&self) -> Option<&[(String, Json)]> {
        match self {
            Json::Obj(entries) => Some(entries),
            _ => None,
        }
    }

    /// Object member as a string (convenience for the VOR1 reader).
    pub fn obj_str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(Json::as_str)
    }

    /// Object member as an integer (convenience for the VOR1 reader).
    pub fn obj_int(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(Json::as_int)
    }
}

fn skip_ws(bytes: &[u8], pos: &mut usize) {
    while *pos < bytes.len() {
        match bytes[*pos] {
            b' ' | b'\t' | b'\r' | b'\n' => *pos += 1,
            _ => break,
        }
    }
}

fn parse_value(bytes: &[u8], pos: &mut usize, depth: usize) -> Option<Json> {
    // `depth` counts the enclosing containers of the value being parsed;
    // with the root call at 0, exactly MAX_DEPTH nested arrays parse and
    // MAX_DEPTH + 1 fail (off-by-one pinned by rejects_deep_nesting).
    if depth >= MAX_DEPTH {
        return None;
    }
    skip_ws(bytes, pos);
    match *bytes.get(*pos)? {
        b'{' => parse_object(bytes, pos, depth + 1),
        b'[' => parse_array(bytes, pos, depth + 1),
        b'"' => parse_string(bytes, pos).map(Json::Str),
        b't' => parse_literal(bytes, pos, b"true", Json::Bool(true)),
        b'f' => parse_literal(bytes, pos, b"false", Json::Bool(false)),
        b'n' => parse_literal(bytes, pos, b"null", Json::Null),
        b'-' | b'0'..=b'9' => parse_number(bytes, pos),
        _ => None,
    }
}

fn parse_literal(bytes: &[u8], pos: &mut usize, word: &[u8], value: Json) -> Option<Json> {
    if bytes.get(*pos..*pos + word.len())? == word {
        *pos += word.len();
        Some(value)
    } else {
        None
    }
}

fn parse_object(bytes: &[u8], pos: &mut usize, depth: usize) -> Option<Json> {
    *pos += 1; // '{'
    let mut entries: Vec<(String, Json)> = Vec::new();
    skip_ws(bytes, pos);
    if bytes.get(*pos) == Some(&b'}') {
        *pos += 1;
        return Some(Json::Obj(entries));
    }
    loop {
        skip_ws(bytes, pos);
        if bytes.get(*pos) != Some(&b'"') {
            return None;
        }
        let key = parse_string(bytes, pos)?;
        skip_ws(bytes, pos);
        if bytes.get(*pos) != Some(&b':') {
            return None;
        }
        *pos += 1;
        let value = parse_value(bytes, pos, depth)?;
        // Last occurrence of a duplicated key wins (matches serde_json and
        // kotlinx behavior closely enough; canonical issuers never emit dups).
        if let Some(slot) = entries.iter_mut().find(|(k, _)| *k == key) {
            slot.1 = value;
        } else {
            entries.push((key, value));
        }
        skip_ws(bytes, pos);
        match bytes.get(*pos) {
            Some(&b',') => *pos += 1,
            Some(&b'}') => {
                *pos += 1;
                return Some(Json::Obj(entries));
            }
            _ => return None,
        }
    }
}

fn parse_array(bytes: &[u8], pos: &mut usize, depth: usize) -> Option<Json> {
    *pos += 1; // '['
    let mut items = Vec::new();
    skip_ws(bytes, pos);
    if bytes.get(*pos) == Some(&b']') {
        *pos += 1;
        return Some(Json::Arr(items));
    }
    loop {
        let value = parse_value(bytes, pos, depth)?;
        items.push(value);
        skip_ws(bytes, pos);
        match bytes.get(*pos) {
            Some(&b',') => *pos += 1,
            Some(&b']') => {
                *pos += 1;
                return Some(Json::Arr(items));
            }
            _ => return None,
        }
    }
}

fn parse_string(bytes: &[u8], pos: &mut usize) -> Option<String> {
    *pos += 1; // opening quote
    let mut out = String::new();
    loop {
        let byte = *bytes.get(*pos)?;
        match byte {
            b'"' => {
                *pos += 1;
                return Some(out);
            }
            b'\\' => {
                *pos += 1;
                let escape = *bytes.get(*pos)?;
                *pos += 1;
                match escape {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'b' => out.push('\u{0008}'),
                    b'f' => out.push('\u{000C}'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => {
                        let first = parse_u16(bytes, pos)?;
                        let scalar = if (0xD800..0xDC00).contains(&first) {
                            // High surrogate: require a following \uXXXX low surrogate.
                            if bytes.get(*pos) != Some(&b'\\') || bytes.get(*pos + 1) != Some(&b'u')
                            {
                                return None;
                            }
                            *pos += 2;
                            let second = parse_u16(bytes, pos)?;
                            if !(0xDC00..0xE000).contains(&second) {
                                return None;
                            }
                            0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00)
                        } else if (0xDC00..0xE000).contains(&first) {
                            // Lone low surrogate: invalid.
                            return None;
                        } else {
                            first
                        };
                        out.push(char::from_u32(scalar)?);
                    }
                    _ => return None,
                }
            }
            0x00..=0x1F => return None, // unescaped control character
            _ => {
                // Copy one UTF-8 scalar. The input came from a valid &str, so
                // multi-byte sequences are well-formed; consume the lead byte
                // plus its continuation bytes and re-decode the slice (the
                // decode can only fail on misaligned input, which rejects).
                let start = *pos;
                *pos += 1; // lead byte
                while *pos < bytes.len() && (bytes[*pos] & 0xC0) == 0x80 {
                    *pos += 1;
                }
                out.push_str(std::str::from_utf8(bytes.get(start..*pos)?).ok()?);
            }
        }
    }
}

fn parse_u16(bytes: &[u8], pos: &mut usize) -> Option<u32> {
    let mut value: u32 = 0;
    for _ in 0..4 {
        let digit = (*bytes.get(*pos)? as char).to_digit(16)?;
        value = value * 16 + digit;
        *pos += 1;
    }
    Some(value)
}

fn parse_number(bytes: &[u8], pos: &mut usize) -> Option<Json> {
    let start = *pos;
    if bytes.get(*pos) == Some(&b'-') {
        *pos += 1;
    }
    // Integer part: 0 | [1-9][0-9]*
    match bytes.get(*pos) {
        Some(b'0') => *pos += 1,
        Some(b'1'..=b'9') => {
            while matches!(bytes.get(*pos), Some(b'0'..=b'9')) {
                *pos += 1;
            }
        }
        _ => return None,
    }
    let mut is_float = false;
    if bytes.get(*pos) == Some(&b'.') {
        is_float = true;
        *pos += 1;
        if !matches!(bytes.get(*pos), Some(b'0'..=b'9')) {
            return None;
        }
        while matches!(bytes.get(*pos), Some(b'0'..=b'9')) {
            *pos += 1;
        }
    }
    if matches!(bytes.get(*pos), Some(b'e') | Some(b'E')) {
        is_float = true;
        *pos += 1;
        if matches!(bytes.get(*pos), Some(b'+') | Some(b'-')) {
            *pos += 1;
        }
        if !matches!(bytes.get(*pos), Some(b'0'..=b'9')) {
            return None;
        }
        while matches!(bytes.get(*pos), Some(b'0'..=b'9')) {
            *pos += 1;
        }
    }
    let text = std::str::from_utf8(bytes.get(start..*pos)?).ok()?;
    if is_float {
        Some(Json::Float(text.parse().ok()?))
    } else {
        match text.parse::<i64>() {
            Ok(int) => Some(Json::Int(int)),
            // Out-of-i64-range integers become floats; the license reader
            // rejects them via as_int() == None, which is the desired
            // behavior for absurd epochs.
            Err(_) => Some(Json::Float(text.parse().ok()?)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_canonical_vor1_payload() {
        let text = "{\"v\":1,\"id\":\"11111111-2222-3333-4444-555555555555\",\"product\":\"vor\",\
\"issued_at\":\"2026-01-01T00:00:00Z\",\"expires_at\":\"2027-01-01T00:00:00Z\",\
\"entitlements\":{\"tier\":\"standard\",\"platforms\":[\"android\",\"windows\",\"linux\"]}}";
        let root = Json::parse(text).expect("must parse");
        assert_eq!(root.obj_int("v"), Some(1));
        assert_eq!(root.obj_str("product"), Some("vor"));
        assert!(root.obj_str("id").is_some_and(|v| !v.is_empty()));
        let entitlements = root.get("entitlements").expect("entitlements");
        assert_eq!(entitlements.obj_str("tier"), Some("standard"));
        let platforms = entitlements
            .get("platforms")
            .and_then(Json::as_arr)
            .expect("platforms");
        assert_eq!(platforms.len(), 3);
        assert_eq!(platforms[0].as_str(), Some("android"));
    }

    #[test]
    fn string_escapes_roundtrip() {
        let root = Json::parse(r#"{"s":"a\"b\\c\/d\be\ff\ng\rh\ti"}"#).expect("parse");
        assert_eq!(root.obj_str("s"), Some("a\"b\\c/d\u{8}e\u{C}f\ng\rh\ti"));
    }

    #[test]
    fn unicode_escapes() {
        let root = Json::parse(r#"{"a":"A","b":"😀"}"#).expect("parse");
        assert_eq!(root.obj_str("a"), Some("A"));
        assert_eq!(root.obj_str("b"), Some("\u{1F600}"));
        // Lone surrogates are rejected.
        assert!(Json::parse(r#"{"x":"\uD800"}"#).is_none());
        assert!(Json::parse(r#"{"x":"\uDC00"}"#).is_none());
    }

    #[test]
    fn numbers() {
        assert_eq!(Json::parse("0"), Some(Json::Int(0)));
        assert_eq!(Json::parse("-42"), Some(Json::Int(-42)));
        assert_eq!(Json::parse("1798761600"), Some(Json::Int(1_798_761_600)));
        assert!(matches!(Json::parse("1.5"), Some(Json::Float(_))));
        assert!(matches!(Json::parse("1e3"), Some(Json::Float(_))));
        // v:1.0 is a Float -> as_int() is None -> V1 verification rejects it,
        // matching the Kotlin verifier's toIntOrNull.
        assert!(Json::parse(r#"{"v":1.0}"#).unwrap().obj_int("v").is_none());
    }

    #[test]
    fn rejects_malformed() {
        for bad in [
            "",
            "{",
            "}",
            "[",
            "]",
            "{\"a\":}",
            "{\"a\" 1}",
            "{a:1}",
            "tru",
            "{\"a\":1,}",
            "[1,]",
            "01",
            "1.",
            ".1",
            "+1",
            "\"unterminated",
            "{\"a\":1} trailing",
            "nul",
            "'single quotes'",
        ] {
            assert!(Json::parse(bad).is_none(), "should reject {bad:?}");
        }
    }

    #[test]
    fn rejects_deep_nesting() {
        let deep = "[".repeat(MAX_DEPTH + 1) + &"]".repeat(MAX_DEPTH + 1);
        assert!(Json::parse(&deep).is_none());
        let ok = "[".repeat(MAX_DEPTH) + &"]".repeat(MAX_DEPTH);
        assert!(Json::parse(&ok).is_some());
    }

    #[test]
    fn rejects_oversized_input() {
        let huge = format!("\"{}\"", "a".repeat(MAX_INPUT));
        assert!(Json::parse(&huge).is_none());
    }

    #[test]
    fn raw_control_characters_rejected() {
        assert!(Json::parse("\"a\nb\"").is_none());
    }

    #[test]
    fn multibyte_utf8_passes_through() {
        let root = Json::parse("{\"k\":\"مرحبا 世界 🚀\"}").expect("parse");
        assert_eq!(root.obj_str("k"), Some("مرحبا 世界 🚀"));
    }

    #[test]
    fn duplicate_keys_last_wins() {
        let root = Json::parse(r#"{"v":1,"v":2}"#).expect("parse");
        assert_eq!(root.obj_int("v"), Some(2));
    }
}
