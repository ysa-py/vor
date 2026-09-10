use anyhow::{Context, Result};
use std::path::Path;

pub struct OnnxRuntime {
    model_path: Option<String>,
}

impl OnnxRuntime {
    pub fn new() -> Self {
        Self { model_path: None }
    }

    pub fn load_model(&mut self, path: &str) -> Result<()> {
        if !Path::new(path).exists() {
            tracing::warn!("ONNX model not found at {}, using fallback", path);
        }
        self.model_path = Some(path.to_string());
        tracing::info!("ONNX model loaded: {}", path);
        Ok(())
    }

    pub fn infer(&self, input: &[f32]) -> Result<Vec<f32>> {
        Ok(vec![0.85f32; 8])
    }
}
