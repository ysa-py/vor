//! Anti-clock-rollback: the monotonic watch.
//!
//! The signed expiry is absolute; a wall clock the user can wind backward
//! would make it decorative. This module composes FOUR time sources into one
//! monotonic, tamper-aware "effective now":
//!
//! 1. **device wall clock** — what the OS says right now (spoofable);
//! 2. **BOOTTIME** — `clock_gettime(CLOCK_BOOTTIME)`, the same counter behind
//!    Android's `SystemClock.elapsedRealtime()`: monotonic, survives deep
//!    sleep, CANNOT be set by the user. Rust reads it directly via libc —
//!    no JNI round-trip, no Kotlin reflection to hook;
//! 3. **in-session delta watch** — every observation pairs (wall, boot).
//!    `expected_wall = last_wall + (boot - last_boot)`; a wall clock that
//!    fell behind its own uptime progression by more than
//!    [`BACKWARD_TOLERANCE_SECS`] was rolled back, and the projected honest
//!    time becomes a floor for the effective now;
//! 4. **persisted ratchet** — the highest time this installation EVER
//!    observed (kept by the caller in its DataStore-backed store; survives
//!    reboots and app restarts, which BOOTTIME alone cannot).
//!
//! Plus a **trusted sample** input (the Kotlin layer's majority vote over
//! HTTPS `Date` headers, when connectivity exists) and the **signed issued_at**
//! floor: even with a wiped ratchet, "now" can never drop below the moment the
//! license was issued.
//!
//! The composition rule: `now = max(device, expected_wall_if_rollback,
//! ratchet, trusted, issued_at_floor)`. Monotonic by construction — winding
//! the clock back mid-session or across reboots cannot resurrect a license.
//!
//! Honest limits: a rooted attacker can patch this function or freeze the
//! ratchet store. That requires code modification on a rooted device — the
//! documented boundary of client-side enforcement.

use std::sync::Mutex;

/// How far the wall clock may drift below its own uptime projection before
/// the delta watch calls it a rollback (NTP slewing is gradual and small;
/// manual date changes are not).
pub const BACKWARD_TOLERANCE_SECS: i64 = 2;

/// In-session watch state.
#[derive(Debug, Default)]
struct WatchState {
    last_wall: i64,
    last_boot: u64,
    initialized: bool,
}

impl WatchState {
    /// Observe a (wall, boot) pair; returns the projected honest wall time
    /// when a rollback is detected, `None` otherwise. Always advances the
    /// watch.
    fn observe(&mut self, wall: i64, boot: u64) -> Option<i64> {
        let verdict = if self.initialized {
            let uptime_progress = (boot.saturating_sub(self.last_boot)) as i64;
            let expected = self.last_wall.saturating_add(uptime_progress);
            if wall < expected.saturating_sub(BACKWARD_TOLERANCE_SECS) {
                Some(expected)
            } else {
                None
            }
        } else {
            None
        };
        // The watch itself is monotonic: never adopt a lower pair.
        if !self.initialized || wall >= self.last_wall || boot >= self.last_boot {
            self.last_wall = wall;
            self.last_boot = boot;
            self.initialized = true;
        }
        verdict
    }
}

/// Process-global watch (the JNI surface is stateless; this survives across
/// calls within the process).
static WATCH: Mutex<WatchState> = Mutex::new(WatchState {
    last_wall: 0,
    last_boot: 0,
    initialized: false,
});

/// Read CLOCK_BOOTTIME in whole seconds (0 on non-unix targets / failures).
#[cfg(unix)]
pub fn boottime_secs() -> u64 {
    // SAFETY: clock_gettime with a valid timespec pointer is a benign,
    // allocation-free syscall on Linux/Android.
    unsafe {
        let mut ts = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        if libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut ts) == 0 {
            ts.tv_sec as u64
        } else {
            0
        }
    }
}

/// Non-unix fallback (builds/docs only — the shipped targets are Android).
#[cfg(not(unix))]
pub fn boottime_secs() -> u64 {
    0
}

/// The composed time verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveNow {
    /// The monotonic effective now (epoch seconds, never below the floors).
    pub now: i64,
    /// The new ratchet value the caller should persist (>= its previous).
    pub new_ratchet: i64,
    /// True when the wall clock fell behind its own uptime progression.
    pub rolled_back: bool,
}

/// Compose the effective now from all sources.
///
/// * `device_wall` — the OS wall clock (seconds);
/// * `persisted_ratchet` — the caller's stored high-water mark (0 if none);
/// * `trusted` — an optional trusted-time sample (0 if none);
/// * `issued_at_floor` — the license's signed issuance time (0 if unknown);
/// * `boot` — the BOOTTIME sample paired with `device_wall`.
///
/// Pure of I/O: the caller reads the clocks (JNI layer), this function only
/// decides — so every rule is unit-testable with injected values.
pub fn effective_now(
    device_wall: i64,
    persisted_ratchet: i64,
    trusted: i64,
    issued_at_floor: i64,
    boot: u64,
) -> EffectiveNow {
    let projected = WATCH
        .lock()
        .map(|mut watch| watch.observe(device_wall, boot))
        .unwrap_or(None);
    let rolled_back = projected.is_some();

    let candidates = [
        device_wall.max(0),
        persisted_ratchet.max(0),
        trusted.max(0),
        issued_at_floor.max(0),
        projected.unwrap_or(0).max(0),
    ];
    let now = candidates.into_iter().max().unwrap_or(0);
    EffectiveNow {
        now,
        new_ratchet: now,
        rolled_back,
    }
}

/// Test-only: reset the in-process watch between scenarios.
#[cfg(test)]
pub(crate) fn reset_watch_for_tests() {
    if let Ok(mut watch) = WATCH.lock() {
        *watch = WatchState {
            last_wall: 0,
            last_boot: 0,
            initialized: false,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn normal_progression_is_never_a_rollback() {
        reset_watch_for_tests();
        let first = effective_now(1_000_000, 0, 0, 0, 500);
        assert!(!first.rolled_back);
        let second = effective_now(1_000_060, 0, 0, 0, 560);
        assert!(!second.rolled_back);
        assert_eq!(second.now, 1_000_060);
    }

    #[test]
    fn mid_session_rollback_is_caught_and_projected() {
        reset_watch_for_tests();
        effective_now(1_000_000, 0, 0, 0, 500);
        // 60 uptime seconds later the wall claims it went BACK 30 days.
        let verdict = effective_now(1_000_060 - 30 * 86_400, 0, 0, 0, 560);
        assert!(verdict.rolled_back);
        // The projected honest time is a floor: now >= 1_000_060.
        assert_eq!(verdict.now, 1_000_060);
        assert_eq!(verdict.new_ratchet, 1_000_060);
    }

    #[test]
    fn small_backward_drift_within_tolerance_is_forgiven() {
        reset_watch_for_tests();
        effective_now(1_000_000, 0, 0, 0, 500);
        // NTP-style slewing: -1s with +60s uptime — inside tolerance.
        let verdict = effective_now(1_000_059, 0, 0, 0, 560);
        assert!(!verdict.rolled_back);
    }

    #[test]
    fn reboot_rollback_is_caught_by_the_persisted_ratchet() {
        reset_watch_for_tests();
        // Session before reboot observed time 2_000_000 and persisted it.
        effective_now(2_000_000, 0, 0, 0, 900);
        // After reboot: fresh boot=10, wall claims 1 month earlier.
        let verdict = effective_now(2_000_000 - 30 * 86_400, 2_000_000, 0, 0, 10);
        // Boottime reset means the delta watch starts fresh (no projection),
        // but the persisted ratchet holds the floor.
        assert_eq!(verdict.now, 2_000_000);
        assert_eq!(verdict.new_ratchet, 2_000_000);
    }

    #[test]
    fn issued_at_is_a_floor_even_with_a_wiped_ratchet() {
        reset_watch_for_tests();
        // "Clear app data + restore token + wind the clock back" attack:
        // ratchet gone, but the signed issued_at pins the floor.
        let verdict = effective_now(500_000, 0, 0, 1_000_000, 42);
        assert_eq!(verdict.now, 1_000_000);
    }

    #[test]
    fn trusted_sample_outranks_a_lying_device_clock() {
        reset_watch_for_tests();
        let verdict = effective_now(500_000, 0, 700_000, 0, 10);
        assert_eq!(verdict.now, 700_000);
    }

    #[test]
    fn negative_inputs_clamp_to_zero() {
        reset_watch_for_tests();
        let verdict = effective_now(-5, -10, -1, 0, 0);
        assert_eq!(verdict.now, 0);
    }

    #[test]
    fn boottime_reads_plausibly_on_unix() {
        reset_watch_for_tests();
        #[cfg(unix)]
        {
            let first = boottime_secs();
            std::thread::sleep(Duration::from_millis(50));
            let second = boottime_secs();
            assert!(second >= first, "BOOTTIME must not run backward");
        }
    }
}
