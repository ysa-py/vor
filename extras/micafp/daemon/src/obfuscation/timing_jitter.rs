use std::time::Duration;
use rand_distr::{Exp, Normal, Uniform, Distribution};

pub fn exponential_jitter(mean_ms: f64) -> Duration {
    let exp = Exp::new(1.0 / mean_ms).unwrap();
    let val = exp.sample(&mut rand::thread_rng());
    Duration::from_micros((val.max(0.1) * 1000.0) as u64)
}

pub fn gaussian_jitter(mean_ms: f64, std_dev_ms: f64) -> Duration {
    let normal = Normal::new(mean_ms, std_dev_ms).unwrap();
    let val = normal.sample(&mut rand::thread_rng());
    Duration::from_micros((val.max(0.1) * 1000.0) as u64)
}

pub fn uniform_jitter(min_ms: f64, max_ms: f64) -> Duration {
    let uniform = Uniform::new(min_ms, max_ms);
    let val = uniform.sample(&mut rand::thread_rng());
    Duration::from_micros((val * 1000.0) as u64)
}

pub struct AntiCorrelationJitter {
    last_delay_ms: f64,
    min_diff_ms: f64,
}

impl AntiCorrelationJitter {
    pub fn new(min_diff_ms: f64) -> Self {
        Self { last_delay_ms: 0.0, min_diff_ms }
    }

    pub fn next_jitter(&mut self, base_ms: f64, std_dev_ms: f64) -> Duration {
        let normal = Normal::new(base_ms, std_dev_ms).unwrap();
        let mut delay = normal.sample(&mut rand::thread_rng()).max(0.1);
        if (delay - self.last_delay_ms).abs() < self.min_diff_ms {
            delay = self.last_delay_ms + if rand::random() { self.min_diff_ms } else { -self.min_diff_ms };
            delay = delay.max(0.1);
        }
        self.last_delay_ms = delay;
        Duration::from_micros((delay * 1000.0) as u64)
    }
}
