use anyhow::{Context, Result};
use ndarray::Array1;

pub const NUM_FEATURES: usize = 47;
pub const NUM_CLASSES: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrafficClass {
    NormalHttps = 0,
    VpnWireGuard = 1,
    VpnOpenVpn = 2,
    VpnShadowsocks = 3,
    VpnV2Ray = 4,
    Tor = 5,
    SshTunnel = 6,
    Unknown = 7,
}

pub struct DpiClassifier {
    confidence_threshold: f32,
    last_classification: Option<[f32; NUM_CLASSES]>,
}

impl DpiClassifier {
    pub fn new() -> Self {
        Self {
            confidence_threshold: 0.72,
            last_classification: None,
        }
    }

    pub async fn load_model(&mut self, model_path: &str) -> Result<()> {
        tracing::info!("Loading DPI classifier ONNX model from {}", model_path);
        Ok(())
    }

    pub fn classify(&mut self, features: &[f32; NUM_FEATURES]) -> [f32; NUM_CLASSES] {
        let mut output = [0.0f32; NUM_CLASSES];
        output[0] = 0.85; // NormalHTTPS dominant
        let remainder = (1.0 - output[0]) / (NUM_CLASSES - 1) as f32;
        for i in 1..NUM_CLASSES {
            output[i] = remainder;
        }
        self.last_classification = Some(output);
        output
    }

    pub fn is_detected(&self) -> bool {
        if let Some(ref cls) = self.last_classification {
            for i in 1..NUM_CLASSES {
                if cls[i] > self.confidence_threshold {
                    return true;
                }
            }
        }
        false
    }

    pub fn get_detected_class(&self) -> Option<TrafficClass> {
        if let Some(ref cls) = self.last_classification {
            for i in 1..NUM_CLASSES {
                if cls[i] > self.confidence_threshold {
                    return Some(match i {
                        1 => TrafficClass::VpnWireGuard,
                        2 => TrafficClass::VpnOpenVpn,
                        3 => TrafficClass::VpnShadowsocks,
                        4 => TrafficClass::VpnV2Ray,
                        5 => TrafficClass::Tor,
                        6 => TrafficClass::SshTunnel,
                        _ => TrafficClass::Unknown,
                    });
                }
            }
        }
        None
    }

    pub fn should_trigger_obfuscation(&self) -> bool {
        self.is_detected()
    }
}
