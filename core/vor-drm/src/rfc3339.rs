//! RFC 3339 timestamp codec — exact port of the shared Vor time rules.
//!
//! `parse_epoch` mirrors the Rust/Kotlin/Go/Python reference implementations
//! bit-for-bit (UTC "Z" suffix, fractional seconds, ±hh:mm offsets), so a
//! VOR1 token expires at the same instant on every platform.
//!
//! `format_utc` is the inverse (whole-second precision, always "Z") used by
//! the manager to render issuance/expiry strings and by the client to map
//! VOR2 epochs back onto the VOR1 string shape the UI already displays.

/// Parse an RFC 3339 timestamp to unix epoch seconds.
///
/// Accepts: `2027-01-01T00:00:00Z`, fractional seconds, `+hh:mm` / `-hh:mm`
/// offsets. Rejects everything else (month 13, hour 25, pre-1970, garbage).
pub fn parse_epoch(text: &str) -> Option<i64> {
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
            if !(0..=23).contains(&hh) || !(0..=59).contains(&mm) {
                return None;
            }
            offset = sign * (hh * 3600 + mm * 60);
            body = &body[..body.len() - 6];
        }
    }
    date_time_to_epoch(date_part, body).map(|epoch| epoch - offset)
}

fn date_time_to_epoch(date_part: &str, time_part: &str) -> Option<i64> {
    let (year, month, day) = parse_date(date_part)?;
    let (hour, minute, second) = parse_time(time_part)?;
    Some(days_from_civil(year, month, day) * 86400 + hour * 3600 + minute * 60 + second)
}

fn parse_date(text: &str) -> Option<(i64, i64, i64)> {
    let mut parts = text.split('-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: i64 = parts.next()?.parse().ok()?;
    let day: i64 = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    if !(1..=12).contains(&month) || !(1970..=9999).contains(&year) {
        return None;
    }
    // Real calendar validation: September has 30 days, February 28/29.
    // (Proleptic Gregorian leap rule — the same rule days_from_civil uses.)
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
    if parts.next().is_some() {
        return None;
    }
    if !(0..=23).contains(&hour) || !(0..=59).contains(&minute) || !(0..=60).contains(&second) {
        return None;
    }
    Some((hour, minute, second))
}

/// Days since unix epoch from a civil date (Howard Hinnant's algorithm).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Inverse: civil date from days since epoch (Howard Hinnant).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// Format a unix epoch (seconds) as `YYYY-MM-DDTHH:MM:SSZ` (UTC).
pub fn format_utc(epoch: i64) -> String {
    let epoch = epoch.max(0);
    let days = epoch.div_euclid(86400);
    let seconds_of_day = epoch.rem_euclid(86400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3600;
    let minute = (seconds_of_day % 3600) / 60;
    let second = seconds_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rfc3339() {
        assert_eq!(parse_epoch("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_epoch("2024-01-01T00:00:00Z"), Some(1_704_067_200));
        assert_eq!(parse_epoch("2027-01-01T00:00:00Z"), Some(1_798_761_600));
        assert_eq!(parse_epoch("2026-09-10T12:00:00Z"), Some(1_789_041_600));
        assert_eq!(
            parse_epoch("2026-09-10T12:00:00+02:00"),
            Some(1_789_034_400)
        );
        assert_eq!(parse_epoch("2026-09-10T12:00:00.123Z"), Some(1_789_041_600));
        assert_eq!(
            parse_epoch("2026-09-10T12:00:00-05:30"),
            Some(1_789_061_400)
        );
    }

    #[test]
    fn rejects_garbage() {
        for bad in [
            "garbage",
            "",
            "2026-13-01T00:00:00Z",
            "2026-09-31T00:00:00Z",
            "2026-02-30T00:00:00Z",
            "2023-02-29T00:00:00Z", // 2023 is not a leap year
            "2024-02-30T00:00:00Z", // 2024 IS a leap year, but still no Feb 30
            "2026-04-31T00:00:00Z",
            "2026-09-10",
            "2026-09-10T25:00:00Z",
            "2026-09-10T12:61:00Z",
            "1969-12-31T23:59:59Z",
            "2026-09-10T12:00:00+99:00",
        ] {
            assert!(parse_epoch(bad).is_none(), "accepted {bad:?}");
        }
        // Leap-day acceptance is equally strict the other way.
        assert!(parse_epoch("2024-02-29T00:00:00Z").is_some());
        assert!(parse_epoch("2000-02-29T00:00:00Z").is_some()); // 400-rule
        assert!(parse_epoch("1900-02-29T00:00:00Z").is_none()); // century, not 400
    }

    #[test]
    fn format_and_parse_roundtrip() {
        for epoch in [0i64, 1, 86_399, 1_704_067_200, 1_789_041_600, 4_102_444_800] {
            let formatted = format_utc(epoch);
            assert_eq!(
                parse_epoch(&formatted),
                Some(epoch),
                "roundtrip {formatted}"
            );
        }
    }

    #[test]
    fn formats_known_instants() {
        assert_eq!(format_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_utc(1_789_041_600), "2026-09-10T12:00:00Z");
        assert_eq!(format_utc(1_798_761_600), "2027-01-01T00:00:00Z");
        // Leap-year day boundary.
        assert_eq!(format_utc(1_709_164_800), "2024-02-29T00:00:00Z");
        // Negative epochs clamp to the epoch itself (licenses are never
        // pre-1970; clamping keeps the output well-formed).
        assert_eq!(format_utc(-1), "1970-01-01T00:00:00Z");
    }
}
