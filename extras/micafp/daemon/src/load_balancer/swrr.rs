// Smooth Weighted Round Robin implementation.

use std::collections::HashMap;
use tokio::sync::Mutex;
use tracing::debug;

use super::TransportWeight;

const EWMA_ALPHA: f64 = 0.3;

pub struct SmoothedWeightedRoundRobin {
    weights: Mutex<Vec<TransportWeight>>,
}

impl SmoothedWeightedRoundRobin {
    pub fn new(transports: Vec<String>) -> Self {
        let weights = transports
            .into_iter()
            .map(|name| TransportWeight {
                name,
                weight: 100,
                current_weight: 0.0,
                effective_weight: 100.0,
                enabled: true,
            })
            .collect();
        Self {
            weights: Mutex::new(weights),
        }
    }

    /// Select the next transport using SWRR.
    pub async fn select(&self) -> Option<String> {
        let mut weights = self.weights.lock().await;
        let active: Vec<&mut TransportWeight> = weights.iter_mut().filter(|w| w.enabled).collect();

        if active.is_empty() {
            return None;
        }

        // Increase each current_weight by effective_weight
        let total: f64 = active.iter().map(|w| w.effective_weight).sum();

        // Reacquire since we need mutable references
        drop(weights);
        let mut weights = self.weights.lock().await;

        let mut best_idx = 0;
        let mut best_weight = f64::NEG_INFINITY;

        for (i, w) in weights.iter_mut().enumerate() {
            if !w.enabled {
                continue;
            }
            w.current_weight += w.effective_weight;
            if w.current_weight > best_weight {
                best_weight = w.current_weight;
                best_idx = i;
            }
        }

        if let Some(w) = weights.get_mut(best_idx) {
            w.current_weight -= total;
            debug!(transport = %w.name, weight = w.effective_weight, "SWRR selected");
            Some(w.name.clone())
        } else {
            None
        }
    }

    /// Update effective weight for a transport based on a probe score (0.0–1.0).
    pub async fn update_score(&self, transport: &str, score: f64) {
        let mut weights = self.weights.lock().await;
        if let Some(w) = weights.iter_mut().find(|w| w.name == transport) {
            let new_weight = score * 100.0;
            w.effective_weight = EWMA_ALPHA * new_weight + (1.0 - EWMA_ALPHA) * w.effective_weight;
            w.effective_weight = w.effective_weight.max(1.0);
        }
    }
}
