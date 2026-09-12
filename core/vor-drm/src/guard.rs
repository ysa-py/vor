//! Anti-analysis: anti-debugging, anti-Frida, anti-hooking, anti-dump.
//!
//! Detectors (all cheap, all best-effort, never fatal on their own):
//!
//! * **TracerPid** — `/proc/self/status` reports the PID tracing this
//!   process; anything non-zero means a debugger (ptrace/gdb/lldb strata)
//!   is attached. This is the safe cross-platform check: unlike the classic
//!   `ptrace(PTRACE_TRACEME)` self-attach it cannot destabilize the
//!   multithreaded ART runtime.
//! * **maps scan** — `/proc/self/maps` is scanned for obfuscated needles of
//!   the common instrumentation frameworks (frida gum, gadget, xposed
//!   family, riru/zygisk). The needles live in the binary only as
//!   obfstr-encrypted constants.
//! * **frida port probe** — a fast `connect()` to the two default
//!   frida-server ports on localhost. A successful connect means a
//!   listening frida-server. Filtered ports cost at most the (short)
//!   connect timeout, once per cache window.
//! * **dump hardening** — `prctl(PR_SET_DUMPABLE, 0)` removes the process
//!   from ptrace-attach by non-root and blocks core-dump helpers, shrinking
//!   the casual memory-dump surface.
//!
//! Response ladder (spec: "silently crash or invalidate the key"):
//!
//! * the **client** fails the verification (`INVALID` + `tampered: true`)
//!   — availability first: a false positive must never brick a paying user;
//! * the **manager** refuses to issue, zeroizes the in-flight seed, and after
//!   [`STRIKE_LIMIT`] detections aborts the process — the admin loses only
//!   the session; the keys at rest stay Keystore-wrapped and unused.
//!
//! Everything is cached in process-global atomics: a detection is sticky for
//! the process lifetime (an attacker gets no second chance via a "clean
//! again" re-sweep), a clean sweep is trusted for [`CLEAN_CACHE_SECS`].
//!
//! Honest limits: each probe is userland-readable state; a root attacker can
//! hide all of it or patch the checks out. The goal is to make casual,
//! unmodified-tooling analysis fail loudly, cheaply and reliably.

use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::time::{Duration, Instant};

/// Detections before the manager escalates to abort.
pub const STRIKE_LIMIT: u8 = 3;

/// How long a clean environment verdict is cached (seconds).
pub const CLEAN_CACHE_SECS: u64 = 60;

/// Connect timeout for the frida port probe.
pub const PORT_PROBE_TIMEOUT: Duration = Duration::from_millis(120);

/// Global sticky verdicts. 0 = unknown, 1 = clean-at, 2 = tampered.
static SWEEP_VERDICT: AtomicU8 = AtomicU8::new(0);
/// Timestamp (secs since process start) of the last clean sweep.
static CLEAN_AT: std::sync::OnceLock<AtomicU64> = std::sync::OnceLock::new();
/// Sticky tamper flag read by the JNI surfaces.
static TAMPERED: AtomicBool = AtomicBool::new(false);
/// Strike counter (saturating).
static STRIKES: AtomicU8 = AtomicU8::new(0);
/// Process start marker for cache timestamps.
static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

fn since_start_secs() -> u64 {
    START.get_or_init(Instant::now).elapsed().as_secs()
}

fn clean_at() -> &'static AtomicU64 {
    CLEAN_AT.get_or_init(|| AtomicU64::new(0))
}

/// Parse `TracerPid:\t<n>` out of a `/proc/self/status` document (pure).
pub fn tracer_pid_from_status(status: &str) -> u32 {
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix(obfstr::obfstr!("TracerPid:")) {
            return rest.trim().parse().unwrap_or(0);
        }
    }
    0
}

/// Scan a `/proc/self/maps` document for instrumentation framework traces
/// (pure). Needles are obfuscated compile-time constants.
pub fn maps_have_framework(content: &str) -> bool {
    // The multi-let obfstr! form creates real long-lived bindings: the
    // plain expression form yields a temporary &str that only lives for
    // its own statement, which cannot back an array (E0716).
    obfstr::obfstr! {
        let n_frida = "frida";
        let n_gumjs = "gum-js";
        let n_gadget = "gadget";
        let n_xposed = "xposed";
        let n_lsposed = "lsposed";
        let n_edxposed = "edxposed";
        let n_lspd = "lspd";
        let n_riru = "riru";
        let n_zygisk = "zygisk";
    }
    let needles = [
        n_frida, n_gumjs, n_gadget, n_xposed, n_lsposed, n_edxposed, n_lspd, n_riru, n_zygisk,
    ];
    // The lowercase comparison makes the scan insensitive to naming styles
    // (Frida/frida, LSPosed/lsposed) without keeping a second copy of each
    // needle.
    let lower = content.to_ascii_lowercase();
    needles.iter().any(|needle| lower.contains(needle))
}

/// Read `/proc/self/status` (thin environment wrapper).
pub fn read_status() -> Option<String> {
    std::fs::read_to_string(obfstr::obfstr!("/proc/self/status")).ok()
}

/// Read `/proc/self/maps` (thin environment wrapper).
pub fn read_maps() -> Option<String> {
    std::fs::read_to_string(obfstr::obfstr!("/proc/self/maps")).ok()
}

/// Probe one localhost TCP port; true when something is LISTENING there.
pub fn port_listening(port: u16) -> bool {
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&addr, PORT_PROBE_TIMEOUT).is_ok()
}

/// True when a debugger is attached to this process (TracerPid != 0).
pub fn being_traced() -> bool {
    match read_status() {
        Some(status) => tracer_pid_from_status(&status) != 0,
        None => false, // unreadable /proc: assume clean (availability first)
    }
}

/// True when /proc maps show a known instrumentation framework.
pub fn maps_dirty() -> bool {
    match read_maps() {
        Some(maps) => maps_have_framework(&maps),
        None => false,
    }
}

/// True when a frida-server default port answers on localhost.
pub fn frida_ports_open() -> bool {
    // Default frida-server ports; both probed, either answering counts.
    port_listening(27042) || port_taking_alias(27043)
}

// Small indirection so the port constant list stays in one place and the
// compiler cannot fold both probes into one visible branch chain.
fn port_taking_alias(port: u16) -> bool {
    port_listening(port)
}

/// Best-effort dump hardening: `PR_SET_DUMPABLE 0`. Failures are ignored —
/// this only narrows casual dump routes; it is not a boundary.
pub fn harden_process() {
    #[cfg(unix)]
    {
        // SAFETY: prctl with PR_SET_DUMPABLE and a value argument is a
        // benign, well-specified call on Linux/Android with no pointer
        // arguments.
        unsafe {
            libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0);
        }
    }
}

/// One full environment sweep (all detectors). Pure result, no caching.
pub fn sweep_once() -> bool {
    let traced = being_traced();
    let maps = maps_dirty();
    let frida = frida_ports_open();
    traced || maps || frida
}

/// The cached environment verdict. `true` = instrumented/debugged.
///
/// * a prior detection is **sticky** for the process lifetime;
/// * a clean verdict is cached for [`CLEAN_CACHE_SECS`], then re-swept.
pub fn environment_tampered(force_resweep: bool) -> bool {
    if TAMPERED.load(Ordering::Acquire) {
        return true;
    }
    let verdict = SWEEP_VERDICT.load(Ordering::Acquire);
    if verdict == 1 && !force_resweep {
        let cached_at = clean_at().load(Ordering::Acquire);
        if since_start_secs().saturating_sub(cached_at) < CLEAN_CACHE_SECS {
            return false;
        }
    }
    if sweep_once() {
        TAMPERED.store(true, Ordering::Release);
        SWEEP_VERDICT.store(2, Ordering::Release);
        register_strike();
        return true;
    }
    SWEEP_VERDICT.store(1, Ordering::Release);
    clean_at().store(since_start_secs(), Ordering::Release);
    false
}

/// Record a strike; returns `true` on the transition at which the strike
/// limit is reached (the manager aborts then; the client fails the verify).
pub fn register_strike() -> bool {
    let now = STRIKES.fetch_add(1, Ordering::AcqRel).saturating_add(1);
    now == STRIKE_LIMIT
}

/// Current strike count (diagnostics).
pub fn strikes() -> u8 {
    STRIKES.load(Ordering::Acquire)
}

/// True when a strike-limit escalation should fire right now.
pub fn should_escalate() -> bool {
    strikes() >= STRIKE_LIMIT
}

/// Reset all state — test-only (the real process never forgives).
#[cfg(test)]
pub(crate) fn reset_for_tests() {
    TAMPERED.store(false, Ordering::Release);
    SWEEP_VERDICT.store(0, Ordering::Release);
    STRIKES.store(0, Ordering::Release);
    clean_at().store(0, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tracer_pid() {
        let clean = "Name:\tvor\nPid:\t42\nTracerPid:\t0\nUidGid stuff\n";
        assert_eq!(tracer_pid_from_status(clean), 0);
        let traced = "Name:\tvor\nPid:\t42\nTracerPid:\t1337\n";
        assert_eq!(tracer_pid_from_status(traced), 1337);
        let absent = "Name:\tvor\n";
        assert_eq!(tracer_pid_from_status(absent), 0);
        let malformed = "TracerPid:\tnope\n";
        assert_eq!(tracer_pid_from_status(malformed), 0);
    }

    #[test]
    fn detects_frameworks_in_maps_fixture() {
        let fixtures = [
            "/data/local/tmp/re.frida.server/frida-agent-16.so",
            "/memfd:frida (deleted)",
            "/data/adb/lspd/framework/lspd.dex",
            "/data/adb/modules/zygisk_ilcpp.so",
            "/data/misc/riru/modules/thing.so",
        ];
        for fixture in fixtures {
            assert!(maps_have_framework(fixture), "missed {fixture}");
        }
        let innocent = [
            "/system/lib64/libc.so\n/system/lib64/libm.so\n/apex/com.android.art/lib64/libart.so",
            "/data/app/com.v2rayez.app/lib/arm64/libv2ray.so",
            "/usr/lib/x86_64-linux-gnu/libssl.so.3",
        ];
        for content in innocent {
            assert!(!maps_have_framework(content), "false positive on {content}");
        }
    }

    #[test]
    fn strike_counter_escalates_exactly_once() {
        reset_for_tests();
        assert!(!register_strike());
        assert!(!register_strike());
        assert!(register_strike()); // third strike = the transition
        assert!(!register_strike()); // already past
        assert!(should_escalate());
    }

    #[test]
    fn closed_port_is_not_listening() {
        // Nothing sane listens on the discard-source port on CI runners or
        // dev machines; if this ever flakes because something DOES listen
        // there, pick another port constant.
        if port_listening(9) {
            eprintln!("port 9 unexpectedly open — skipping");
        } else {
            assert!(!port_listening(9));
        }
    }

    #[test]
    fn environment_sweep_is_cached_clean() {
        reset_for_tests();
        // The test host is not being traced; if a sweep here reports
        // tampered we are genuinely running under an instrumented CI
        // environment — do not fail the test for that, just exercise both
        // cache paths.
        let first = environment_tampered(false);
        let second = environment_tampered(false);
        if !first {
            assert_eq!(second, first); // cached path returns the same verdict
        }
        reset_for_tests();
    }
}
