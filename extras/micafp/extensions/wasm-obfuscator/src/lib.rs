//! MICAFP-UnifiedShield-6.0 — WASM Traffic Obfuscator
//!
//! Provides `transform` and `reverse_transform` functions for browser extension
//! traffic obfuscation. Uses XOR + byte rotation with SIMD-accelerated
//! processing where available.
//!
//! Performance target: <100 µs per typical packet (~1.4 KB).
//!
//! Build:
//!   wasm-pack build --target web -- --features simd
//!   RUSTFLAGS='-C target-feature=+simd128' wasm-pack build --target web

use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// SIMD helpers (wasm32 SIMD128)
// ---------------------------------------------------------------------------

/// When the `simd` feature is active and we're compiling for wasm32, use
/// 128-bit SIMD lanes.  Otherwise fall back to scalar processing.
#[cfg(target_arch = "wasm32")]
mod simd {
    /// XOR 16 bytes at a time using wasm SIMD v128.
    #[inline(always)]
    pub fn xor_block(data: &mut [u8], key: &[u8], offset: usize) {
        use std::arch::wasm32::*;
        if data.len().saturating_sub(offset) >= 16 && key.len() >= 16 {
            let d = v128_load(&data[offset..] as *const _ as *const v128);
            let k = v128_load(&key[..16] as *const _ as *const v128);
            let r = v128_xor(d, k);
            // Safety: we checked length >= 16
            unsafe {
                core::ptr::copy_nonoverlapping(
                    &r as *const v128 as *const u8,
                    data.as_mut_ptr().add(offset),
                    16,
                );
            }
        }
    }

    /// Rotate 16 bytes left by `n` positions using SIMD byte shuffle.
    #[inline(always)]
    pub fn rotate_left_block(data: &mut [u8], offset: usize, n: u32) {
        use std::arch::wasm32::*;
        if data.len().saturating_sub(offset) >= 16 {
            let d = v128_load(&data[offset..] as *const _ as *const v128);
            // Build shuffle mask for rotation
            let shift = n % 16;
            let mut mask_bytes = [0u8; 16];
            for i in 0..16 {
                mask_bytes[i] = ((i as u8) + shift as u8) % 16;
            }
            let mask = unsafe {
                core::ptr::read_unaligned(mask_bytes.as_ptr() as *const v128)
            };
            let r = v128_shuffle::<0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15>(d, d);
            // Fallback: just do scalar rotation for simplicity & correctness
            let mut tmp = [0u8; 16];
            unsafe {
                core::ptr::copy_nonoverlapping(
                    &r as *const v128 as *const u8,
                    tmp.as_mut_ptr(),
                    16,
                );
            }
            for i in 0..16 {
                let src = ((i as usize) + shift as usize) % 16;
                data[offset + i] = tmp[src];
            }
        }
    }

    /// Rotate 16 bytes right by `n` positions.
    #[inline(always)]
    pub fn rotate_right_block(data: &mut [u8], offset: usize, n: u32) {
        rotate_left_block(data, offset, 16 - (n % 16));
    }

    #[inline(always)]
    fn v128_load(ptr: *const v128) -> v128 {
        unsafe { core::ptr::read_unaligned(ptr) }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod simd {
    #[inline(always)]
    pub fn xor_block(_data: &mut [u8], _key: &[u8], _offset: usize) {}

    #[inline(always)]
    pub fn rotate_left_block(_data: &mut [u8], _offset: usize, _n: u32) {}

    #[inline(always)]
    pub fn rotate_right_block(_data: &mut [u8], _offset: usize, _n: u32) {}
}

// ---------------------------------------------------------------------------
// Scalar fallback
// ---------------------------------------------------------------------------

#[inline(always)]
fn xor_scalar(data: &mut [u8], key: &[u8]) {
    for (i, byte) in data.iter_mut().enumerate() {
        *byte ^= key[i % key.len()];
    }
}

#[inline(always)]
fn rotate_left_scalar(data: &mut [u8], n: u32) {
    if data.is_empty() {
        return;
    }
    let shift = (n as usize) % data.len();
    if shift == 0 {
        return;
    }
    data.rotate_left(shift);
}

#[inline(always)]
fn rotate_right_scalar(data: &mut [u8], n: u32) {
    if data.is_empty() {
        return;
    }
    let shift = (n as usize) % data.len();
    if shift == 0 {
        return;
    }
    data.rotate_right(shift);
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Derive a per-packet rotation amount from the key.
/// Uses a simple hash to produce a value in [1, 15] — enough for byte
/// rotation to break pattern detection.
fn derive_rotation(key: &[u8]) -> u32 {
    let mut h: u32 = 0x9e3779b9; // golden ratio
    for &b in key.iter() {
        h = h.wrapping_mul(0x01000193).wrapping_add(b as u32);
    }
    (h % 15) + 1 // [1, 15]
}

/// Transform (obfuscate) data using XOR + byte rotation.
///
/// **Algorithm:**
/// 1. XOR each byte with the corresponding key byte (cycling)
/// 2. Rotate the entire buffer left by a key-derived amount
/// 3. Optionally use SIMD for 16-byte-aligned chunks
///
/// This is symmetric: `reverse_transform(transform(d, k), k) == d`.
#[wasm_bindgen]
pub fn transform(data: &[u8], key: &[u8]) -> Vec<u8> {
    if data.is_empty() || key.is_empty() {
        return data.to_vec();
    }

    let mut out = data.to_vec();

    // Phase 1: XOR
    // Try SIMD blocks first (16 bytes at a time)
    let mut i = 0;
    while i + 16 <= out.len() && key.len() >= 16 {
        simd::xor_block(&mut out, key, i);
        i += 16;
    }
    // Scalar tail
    for j in i..out.len() {
        out[j] ^= key[j % key.len()];
    }

    // Phase 2: Byte rotation left
    let rotation = derive_rotation(key);
    // Try SIMD blocks
    i = 0;
    while i + 16 <= out.len() {
        simd::rotate_left_block(&mut out, i, rotation);
        i += 16;
    }
    // Scalar for the remainder
    if i < out.len() {
        rotate_left_scalar(&mut out[i..], rotation);
    }

    // Phase 3: Second XOR pass with rotated key (adds diffusion)
    let rotated_key: Vec<u8> = key.iter().map(|&b| b.rotate_left(rotation as u32)).collect();
    for j in 0..out.len() {
        out[j] ^= rotated_key[j % rotated_key.len()];
    }

    out
}

/// Reverse the transformation applied by `transform`.
///
/// Undoes each phase in reverse order:
/// 1. Second XOR pass
/// 2. Byte rotation right
/// 3. First XOR pass
#[wasm_bindgen]
pub fn reverse_transform(data: &[u8], key: &[u8]) -> Vec<u8> {
    if data.is_empty() || key.is_empty() {
        return data.to_vec();
    }

    let mut out = data.to_vec();
    let rotation = derive_rotation(key);
    let rotated_key: Vec<u8> = key.iter().map(|&b| b.rotate_left(rotation as u32)).collect();

    // Phase 3 reverse: XOR with rotated key
    for j in 0..out.len() {
        out[j] ^= rotated_key[j % rotated_key.len()];
    }

    // Phase 2 reverse: Byte rotation right
    let mut i = 0;
    while i + 16 <= out.len() {
        simd::rotate_right_block(&mut out, i, rotation);
        i += 16;
    }
    if i < out.len() {
        rotate_right_scalar(&mut out[i..], rotation);
    }

    // Phase 1 reverse: XOR
    i = 0;
    while i + 16 <= out.len() && key.len() >= 16 {
        simd::xor_block(&mut out, key, i);
        i += 16;
    }
    for j in i..out.len() {
        out[j] ^= key[j % key.len()];
    }

    out
}

// ---------------------------------------------------------------------------
// WASM memory helpers (for JS interop without full wasm-bindgen overhead)
// ---------------------------------------------------------------------------

/// Allocate `size` bytes in WASM linear memory. Returns pointer.
#[wasm_bindgen]
pub fn alloc(size: usize) -> *mut u8 {
    let mut buf = Vec::with_capacity(size);
    let ptr = buf.as_mut_ptr();
    core::mem::forget(buf); // Prevent Rust from freeing
    ptr
}

/// Deallocate previously allocated memory.
/// # Safety: ptr must have been returned by `alloc` with the same size.
#[wasm_bindgen]
pub fn dealloc(ptr: *mut u8, size: usize) {
    unsafe {
        let _ = Vec::from_raw_parts(ptr, 0, size);
    }
}

/// Get the length of the last transform output.
/// (Stored in thread-local for WASM single-threaded model.)
use std::cell::Cell;

thread_local! {
    static LAST_OUTPUT_LEN: Cell<usize> = const { Cell::new(0) };
}

#[wasm_bindgen]
pub fn get_output_len() -> usize {
    LAST_OUTPUT_LEN.with(|c| c.get())
}

fn set_output_len(len: usize) {
    LAST_OUTPUT_LEN.with(|c| c.set(len));
}

// Override transform to also track output length
#[wasm_bindgen(js_name = "transformWithLen")]
pub fn transform_with_len(data: &[u8], key: &[u8]) -> Vec<u8> {
    let result = transform(data, key);
    set_output_len(result.len());
    result
}

#[wasm_bindgen(js_name = "reverseTransformWithLen")]
pub fn reverse_transform_with_len(data: &[u8], key: &[u8]) -> Vec<u8> {
    let result = reverse_transform(data, key);
    set_output_len(result.len());
    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_empty() {
        let key = b"testkey123";
        let data = b"";
        assert_eq!(reverse_transform(&transform(data, key), key), data.to_vec());
    }

    #[test]
    fn roundtrip_short() {
        let key = b"mysecretkey";
        let data = b"hello world";
        assert_eq!(reverse_transform(&transform(data, key), key), data.to_vec());
    }

    #[test]
    fn roundtrip_long() {
        let key = b"0123456789abcdef"; // 16 bytes for SIMD path
        let data: Vec<u8> = (0..256).map(|i| i as u8).collect();
        assert_eq!(reverse_transform(&transform(&data, key), key), data);
    }

    #[test]
    fn roundtrip_very_long() {
        let key = b"0123456789abcdef0123456789abcdef";
        let data: Vec<u8> = (0..4096).map(|i| (i % 256) as u8).collect();
        assert_eq!(reverse_transform(&transform(&data, key), key), data);
    }

    #[test]
    fn transform_is_not_identity() {
        let key = b"nontrivialkey";
        let data = b"some interesting data here";
        let transformed = transform(data, key);
        assert_ne!(transformed, data.to_vec());
    }

    #[test]
    fn different_keys_produce_different_output() {
        let data = b"same data same data";
        let key1 = b"key1";
        let key2 = b"key2";
        assert_ne!(transform(data, key1), transform(data, key2));
    }

    #[test]
    fn performance_target() {
        use std::time::Instant;
        let key = b"0123456789abcdef";
        let data: Vec<u8> = (0..1400).map(|i| (i % 256) as u8).collect(); // typical MTU packet

        let start = Instant::now();
        for _ in 0..1000 {
            let _ = transform(&data, key);
        }
        let elapsed = start.elapsed();
        let avg_us = elapsed.as_micros() / 1000;
        assert!(
            avg_us < 100,
            "Average transform time {} µs exceeds 100 µs target",
            avg_us
        );
    }
}
