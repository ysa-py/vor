use rand::Rng;

/// Apply Gaussian noise for differential privacy.
pub fn gaussian_noise(value: f64, sensitivity: f64, epsilon: f64, delta: f64) -> f64 {
    let sigma = (2.0 * (1.25 / delta).ln()).sqrt() * sensitivity / epsilon;
    let noise: f64 = rand::thread_rng().sample(rand_distr::Normal::new(0.0_f64, sigma).unwrap());
    (value + noise).max(0.0)
}

/// Randomized response for boolean values.
pub fn randomized_response(value: bool, epsilon: f64) -> bool {
    use rand::Rng;
    let p = epsilon.exp() / (epsilon.exp() + 1.0);
    if rand::thread_rng().gen::<f64>() < p {
        value
    } else {
        !value
    }
}
