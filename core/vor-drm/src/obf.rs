//! Compile-time obfuscation toolkit.
//!
//! Two layers, applied to every security-critical path in this crate:
//!
//! 1. **String encryption** — every sensitive constant (secrets, magic
//!    bytes, framework needles, error markers) is stored XOR-encrypted in the
//!    binary via [`obfstr`] and decrypted on the stack only at the moment of
//!    use, re-encrypted per build with a fresh random key. `strings libvor_drm.so`
//!    yields nothing useful.
//! 2. **Control-flow flattening** — the [`cff!`] macro rewrites a sequence of
//!    steps into a keyed dispatcher loop: arms are shuffled in the source,
//!    transitions are compile-time-random 64-bit keys, and every comparison
//!    passes through [`std::hint::black_box`]. A decompiler sees one loop
//!    with opaque `u64` compares instead of a linear routine. Patching one
//!    branch breaks the chain and the flow falls through to the fail-closed
//!    exit.
//!
//! Both layers are pure Rust, zero dependencies beyond `obfstr`, and are
//! exercised by unit tests so they cannot silently rot.

/// Run a body only when a compile-time-encrypted needle is present in
/// `haystack` (case-insensitive, ASCII only). The needle never exists as a
/// plaintext constant in the binary.
///
/// Used by [`crate::guard`] for the anti-framework scans; public here so the
/// manager and client cores can hide their own protocol markers the same way.
pub fn contains_obf(haystack: &str, needle: &str) -> bool {
    // Both sides already decrypted by the caller; do a plain ASCII
    // case-insensitive substring scan with no allocation.
    let hay = haystack.as_bytes();
    let needle = needle.as_bytes();
    if needle.is_empty() || hay.len() < needle.len() {
        return false;
    }
    let last_start = hay.len() - needle.len();
    let mut start = 0;
    while start <= last_start {
        if hay[start..]
            .iter()
            .zip(needle.iter())
            .all(|(h, n)| h.eq_ignore_ascii_case(n))
        {
            return true;
        }
        start += 1;
    }
    false
}

/// Control-flow flattening chain.
///
/// Usage:
///
/// ```ignore
/// let k1: u64 = obfstr::random!(u64);
/// let k2: u64 = obfstr::random!(u64);
/// let k3: u64 = obfstr::random!(u64);
/// let k_end: u64 = obfstr::random!(u64); // terminal key: matches no arm
/// let mut verdict = false;
/// cff!(k1;
///     k3 => k_end { verdict = did_the_last_thing(); }
///     k1 => k2    { step_one(); }
///     k2 => k3    { step_two(); }
/// );
/// // execution order is ALWAYS step_one -> step_two -> the_last_thing
/// // regardless of source order of the arms.
/// ```
///
/// Invariants (why this is safe by construction):
/// * entry sets the first arm's key; each arm sets exactly one successor key;
/// * a key value is ever "current" only when its arm is the next to run
///   (keys are unique per build thanks to `obfstr::random!`);
/// * therefore every arm runs at most once, and the arms the caller chains
///   run exactly once, in chain order;
/// * a patched comparison (or a corrupted state) matches no arm and the loop
///   falls through to `break` — the surrounding function must treat an
///   incomplete chain as failure (fail-closed, see the callers);
/// * the caller MUST provide a terminal key (`k_end`) that no arm uses as its
///   `mine` key, otherwise the chain loops.
///
/// The bodies see the enclosing function's locals (macro hygiene keeps the
/// loop state invisible to them), so steps can accumulate results naturally.
#[macro_export]
macro_rules! cff {
    ($entry:ident ; $($mine:ident => $next:ident $body:block)+) => {
        let mut cff_cur__: u64 = std::hint::black_box($entry);
        'cff_loop__: loop {
            $(
                if cff_cur__ == std::hint::black_box($mine) {
                    $body
                    cff_cur__ = std::hint::black_box($next);
                    continue 'cff_loop__;
                }
            )+
            break 'cff_loop__;
        }
    };
}

/// Opaque boolean — evaluates the expression but hides its value from the
/// optimizer so constant-folding cannot undo the flattened comparisons.
#[inline(always)]
pub fn opaque(value: bool) -> bool {
    std::hint::black_box(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cff_chain_runs_in_chain_order_regardless_of_source_order() {
        let k1: u64 = obfstr::random!(u64);
        let k2: u64 = obfstr::random!(u64);
        let k3: u64 = obfstr::random!(u64);
        let k_end: u64 = obfstr::random!(u64);
        let mut trace: Vec<u32> = Vec::new();

        // Arms deliberately shuffled in source: 3, 1, 2.
        cff!(k1;
            k3 => k_end { trace.push(3); }
            k1 => k2    { trace.push(1); }
            k2 => k3    { trace.push(2); }
        );
        assert_eq!(trace, vec![1, 2, 3]);
    }

    #[test]
    fn cff_chain_single_arm() {
        let k1: u64 = obfstr::random!(u64);
        let k_end: u64 = obfstr::random!(u64);
        let mut ran = false;
        cff!(k1;
            k1 => k_end { ran = true; }
        );
        assert!(ran);
    }

    #[test]
    fn contains_obf_matches_case_insensitive() {
        assert!(contains_obf(
            "/data/local/tmp/re.xposed:/lib/arm64",
            "Xposed"
        ));
        assert!(contains_obf("map frida-agent page", "frida"));
        assert!(!contains_obf("plain old library", "frida"));
        assert!(!contains_obf("short", "much longer needle"));
        assert!(!contains_obf("", "frida"));
    }

    #[test]
    fn obfstr_strings_are_recoverable_at_runtime() {
        // The needles the guard uses must round-trip through obfstr.
        assert_eq!(obfstr::obfstr!("VOR-HWID-v1"), "VOR-HWID-v1");
    }
}
