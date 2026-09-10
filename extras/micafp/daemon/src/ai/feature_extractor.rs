use std::collections::VecDeque;

pub const NUM_FEATURES: usize = 47;

pub struct FeatureExtractor {
    packet_window: VecDeque<PacketInfo>,
    window_size: usize,
}

struct PacketInfo {
    timestamp_us: u64,
    size: usize,
    is_outgoing: bool,
    tls_record_type: u8,
}

impl FeatureExtractor {
    pub fn new() -> Self {
        Self {
            packet_window: VecDeque::with_capacity(100),
            window_size: 100,
        }
    }

    pub fn add_packet(
        &mut self,
        timestamp_us: u64,
        size: usize,
        is_outgoing: bool,
        tls_record_type: u8,
    ) {
        if self.packet_window.len() >= self.window_size {
            self.packet_window.pop_front();
        }
        self.packet_window.push_back(PacketInfo {
            timestamp_us,
            size,
            is_outgoing,
            tls_record_type,
        });
    }

    pub fn extract_features(&self) -> [f32; NUM_FEATURES] {
        let mut features = [0.0f32; NUM_FEATURES];
        if self.packet_window.len() < 2 {
            return features;
        }
        let iats: Vec<f64> = self
            .packet_window
            .iter()
            .skip(1)
            .zip(self.packet_window.iter())
            .map(|(a, b)| (a.timestamp_us - b.timestamp_us) as f64 / 1000.0)
            .collect();
        let sizes: Vec<f64> = self.packet_window.iter().map(|p| p.size as f64).collect();
        features[0] = mean(&iats) as f32;
        features[1] = std_dev(&iats) as f32;
        features[2] = skewness(&iats) as f32;
        features[3] = kurtosis(&iats) as f32;
        features[4] = mean(&sizes) as f32;
        features[5] = std_dev(&sizes) as f32;
        let total = sizes.len() as f64;
        features[6] = (sizes.iter().filter(|&&s| s < 100.0).count() as f64 / total) as f32;
        features[7] =
            (sizes.iter().filter(|&&s| s >= 100.0 && s < 500.0).count() as f64 / total) as f32;
        features[8] =
            (sizes.iter().filter(|&&s| s >= 500.0 && s < 1000.0).count() as f64 / total) as f32;
        features[9] =
            (sizes.iter().filter(|&&s| s >= 1000.0 && s < 1500.0).count() as f64 / total) as f32;
        features[10] = burst_count(&iats, 5.0) as f32;
        features[11] = mean_burst_size(&iats, 5.0) as f32;
        let tls_types: Vec<u8> = self
            .packet_window
            .iter()
            .map(|p| p.tls_record_type)
            .collect();
        features[12] = (tls_types.iter().filter(|&&t| t == 0x16).count() as f64 / total) as f32;
        features[13] = (tls_types.iter().filter(|&&t| t == 0x17).count() as f64 / total) as f32;
        features[14] = (tls_types.iter().filter(|&&t| t == 0x14).count() as f64 / total) as f32;
        for i in 16..NUM_FEATURES {
            features[i] = 0.0;
        }
        features
    }
}

fn mean(data: &[f64]) -> f64 {
    if data.is_empty() {
        0.0
    } else {
        data.iter().sum::<f64>() / data.len() as f64
    }
}
fn std_dev(data: &[f64]) -> f64 {
    let m = mean(data);
    (data.iter().map(|x| (x - m).powi(2)).sum::<f64>() / data.len().max(1) as f64).sqrt()
}
fn skewness(data: &[f64]) -> f64 {
    let m = mean(data);
    let s = std_dev(data);
    if s == 0.0 {
        0.0
    } else {
        data.iter().map(|x| ((x - m) / s).powi(3)).sum::<f64>() / data.len() as f64
    }
}
fn kurtosis(data: &[f64]) -> f64 {
    let m = mean(data);
    let s = std_dev(data);
    if s == 0.0 {
        0.0
    } else {
        data.iter().map(|x| ((x - m) / s).powi(4)).sum::<f64>() / data.len() as f64 - 3.0
    }
}
fn burst_count(iats: &[f64], threshold_ms: f64) -> f64 {
    let mut count = 0.0;
    let mut in_burst = false;
    for &iat in iats {
        if iat < threshold_ms {
            if !in_burst {
                count += 1.0;
                in_burst = true;
            }
        } else {
            in_burst = false;
        }
    }
    count
}
fn mean_burst_size(iats: &[f64], threshold_ms: f64) -> f64 {
    let mut bursts = Vec::new();
    let mut current = 0.0;
    for &iat in iats {
        if iat < threshold_ms {
            current += 1.0;
        } else if current > 0.0 {
            bursts.push(current);
            current = 0.0;
        }
    }
    if current > 0.0 {
        bursts.push(current);
    }
    if bursts.is_empty() {
        0.0
    } else {
        bursts.iter().sum::<f64>() / bursts.len() as f64
    }
}
