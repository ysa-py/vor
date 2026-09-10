//! Rolling-window traffic feature extraction.
//!
//! Ported from MICAFP `daemon/src/ai/feature_extractor.rs` (the one *real*
//! piece of upstream AI code — 15 features over a 100-packet window). These
//! features feed the threat heuristic and the shaper; they are deliberately
//! cheap so any platform can compute them per connection.

use serde::{Deserialize, Serialize};

/// Maximum packets kept in the rolling window.
pub const WINDOW: usize = 100;

/// Rolling traffic statistics for one connection direction.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrafficWindow {
    /// Timestamps (ms, monotonic per connection) of observed packets.
    timestamps_ms: Vec<f64>,
    /// Payload sizes (bytes) of observed packets.
    sizes: Vec<f64>,
    /// Count of TLS handshake records (0x16) observed.
    pub tls_handshake_records: u32,
    /// Count of TLS app-data records (0x17) observed.
    pub tls_appdata_records: u32,
    /// Count of TLS alert/change-cipher records (0x15/0x14).
    pub tls_other_records: u32,
}

impl TrafficWindow {
    /// Record one packet. `first_byte` is the first payload byte (used for
    /// TLS record-type ratios when available, 0 otherwise).
    pub fn push(&mut self, timestamp_ms: f64, size: usize, first_byte: u8) {
        if self.sizes.len() >= WINDOW {
            self.timestamps_ms.remove(0);
            self.sizes.remove(0);
        }
        self.timestamps_ms.push(timestamp_ms);
        self.sizes.push(size as f64);
        match first_byte {
            0x16 => self.tls_handshake_records += 1,
            0x17 => self.tls_appdata_records += 1,
            0x14 | 0x15 => self.tls_other_records += 1,
            _ => {}
        }
    }

    /// Number of packets in the window.
    pub fn len(&self) -> usize {
        self.sizes.len()
    }

    /// Whether the window is empty.
    pub fn is_empty(&self) -> bool {
        self.sizes.is_empty()
    }

    fn mean(data: &[f64]) -> f64 {
        if data.is_empty() {
            0.0
        } else {
            data.iter().sum::<f64>() / data.len() as f64
        }
    }

    fn stddev(data: &[f64]) -> f64 {
        if data.len() < 2 {
            return 0.0;
        }
        let m = Self::mean(data);
        let variance =
            data.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / (data.len() - 1) as f64;
        variance.sqrt()
    }

    /// Inter-arrival times (ms) between consecutive packets.
    fn iats(&self) -> Vec<f64> {
        let mut out = Vec::with_capacity(self.timestamps_ms.len().saturating_sub(1));
        for pair in self.timestamps_ms.windows(2) {
            out.push(pair[1] - pair[0]);
        }
        out
    }

    /// Number of bursts (gaps > 10 ms) plus one.
    fn burst_count(&self) -> u32 {
        let iats = self.iats();
        1 + iats.iter().filter(|gap| **gap > 10.0).count() as u32
    }

    /// Shannon entropy of the packet-size histogram (4 buckets).
    fn size_entropy(&self) -> f64 {
        if self.sizes.is_empty() {
            return 0.0;
        }
        let mut counts = [0usize; 4];
        for size in &self.sizes {
            let bucket = match *size as usize {
                0..=199 => 0,
                200..=599 => 1,
                600..=1199 => 2,
                _ => 3,
            };
            counts[bucket] += 1;
        }
        let total = self.sizes.len() as f64;
        let mut entropy = 0.0;
        for count in counts {
            if count > 0 {
                let p = count as f64 / total;
                entropy -= p * p.log2();
            }
        }
        entropy
    }

    /// The 15 extracted features (names stable across platforms).
    pub fn features(&self) -> Features {
        let iats = self.iats();
        Features {
            packet_count: self.len() as u32,
            size_mean: Self::mean(&self.sizes),
            size_stddev: Self::stddev(&self.sizes),
            size_min: if self.sizes.is_empty() {
                0.0
            } else {
                self.sizes.iter().cloned().fold(f64::INFINITY, f64::min)
            },
            size_max: self.sizes.iter().cloned().fold(0.0, f64::max),
            iat_mean_ms: Self::mean(&iats),
            iat_stddev_ms: Self::stddev(&iats),
            burst_count: self.burst_count(),
            size_entropy_bits: self.size_entropy(),
            tls_handshake_ratio: self.tls_handshake_records as f64 / self.len().max(1) as f64,
            tls_appdata_ratio: self.tls_appdata_records as f64 / self.len().max(1) as f64,
            tls_other_ratio: self.tls_other_records as f64 / self.len().max(1) as f64,
            bytes_total: self.sizes.iter().sum::<f64>() as u64,
            duration_ms: self
                .timestamps_ms
                .last()
                .zip(self.timestamps_ms.first())
                .map(|(last, first)| last - first)
                .unwrap_or(0.0),
            packets_per_second: if self.is_empty() {
                0.0
            } else {
                let duration = self
                    .timestamps_ms
                    .last()
                    .zip(self.timestamps_ms.first())
                    .map(|(last, first)| (last - first).max(1.0))
                    .unwrap_or(1.0);
                self.len() as f64 / (duration / 1000.0)
            },
        }
    }
}

/// Extracted feature vector (all platforms agree on these field names —
/// they appear verbatim in the Kotlin and Go ports).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Features {
    /// Packets in the current window.
    pub packet_count: u32,
    /// Mean packet size (bytes).
    pub size_mean: f64,
    /// Stddev of packet size.
    pub size_stddev: f64,
    /// Minimum packet size (0 when empty).
    pub size_min: f64,
    /// Maximum packet size.
    pub size_max: f64,
    /// Mean inter-arrival time (ms).
    pub iat_mean_ms: f64,
    /// Stddev of inter-arrival time.
    pub iat_stddev_ms: f64,
    /// Burst count (gaps > 10 ms).
    pub burst_count: u32,
    /// Shannon entropy of the size histogram (bits).
    pub size_entropy_bits: f64,
    /// TLS handshake record ratio.
    pub tls_handshake_ratio: f64,
    /// TLS app-data record ratio.
    pub tls_appdata_ratio: f64,
    /// TLS alert/CCS record ratio.
    pub tls_other_ratio: f64,
    /// Total bytes observed.
    pub bytes_total: u64,
    /// Window duration (ms).
    pub duration_ms: f64,
    /// Packets per second.
    pub packets_per_second: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_window_is_zeroed() {
        let window = TrafficWindow::default();
        let features = window.features();
        assert_eq!(features.packet_count, 0);
        assert_eq!(features.size_mean, 0.0);
    }

    #[test]
    fn stats_over_uniform_traffic() {
        let mut window = TrafficWindow::default();
        for i in 0..50 {
            window.push(i as f64 * 10.0, 500, 0x17);
        }
        let features = window.features();
        assert_eq!(features.packet_count, 50);
        assert!((features.size_mean - 500.0).abs() < 1e-9);
        assert!((features.iat_mean_ms - 10.0).abs() < 1e-9);
        assert_eq!(features.burst_count, 1); // no gaps > 10ms
        assert!((features.tls_appdata_ratio - 1.0).abs() < 1e-9);
        // 50 packets over 49 * 10ms window = 102.04 pps
        assert!((features.packets_per_second - 102.04).abs() < 0.1);
    }

    #[test]
    fn window_is_bounded() {
        let mut window = TrafficWindow::default();
        for i in 0..500 {
            window.push(i as f64, 100, 0x00);
        }
        assert_eq!(window.len(), WINDOW);
    }

    #[test]
    fn bursts_counted() {
        let mut window = TrafficWindow::default();
        // two packets, 100 ms apart -> 2 bursts
        window.push(0.0, 100, 0x17);
        window.push(100.0, 100, 0x17);
        assert_eq!(window.features().burst_count, 2);
    }
}
