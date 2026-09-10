use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicU64, Ordering};

pub static BYTES_SENT: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));
pub static BYTES_RECV: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));
pub static RECONNECTS: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));
pub static FAILOVERS: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));
pub static ACTIVE_TUNNEL: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));

pub fn inc_bytes_sent(n: u64) {
    BYTES_SENT.fetch_add(n, Ordering::Relaxed);
}
pub fn inc_bytes_recv(n: u64) {
    BYTES_RECV.fetch_add(n, Ordering::Relaxed);
}
pub fn inc_reconnects() {
    RECONNECTS.fetch_add(1, Ordering::Relaxed);
}
pub fn inc_failovers() {
    FAILOVERS.fetch_add(1, Ordering::Relaxed);
}
