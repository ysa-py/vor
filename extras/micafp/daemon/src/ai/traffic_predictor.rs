use anyhow::Result;

pub struct TrafficPredictor {
    sequence_length: usize,
}

impl TrafficPredictor {
    pub fn new() -> Self {
        Self {
            sequence_length: 20,
        }
    }

    pub async fn load_model(&mut self, path: &str) -> Result<()> {
        tracing::info!("Loading traffic predictor from {}", path);
        Ok(())
    }

    pub fn predict_blocking_probability(&self, metrics: &[[f32; 4]; 20]) -> [f32; 9] {
        let mut probs = [0.0f32; 9];
        for i in 0..9 {
            probs[i] = 0.05 + (i as f32 * 0.02);
        }
        probs
    }
}
