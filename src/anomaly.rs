//! Layer 6: Runtime Anomaly Detection (3-sigma statistical bounds).

use std::collections::VecDeque;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MetricSample {
    pub cpu: f64,
    pub memory: f64,
    pub request_rate: f64,
    pub error_rate: f64,
    pub network_io: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricKind { Cpu, Memory, RequestRate, ErrorRate, NetworkIo }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyResult {
    pub is_anomaly: bool,
    pub score: f64,
    pub triggered: Vec<(MetricKind, f64)>,
}

pub struct AnomalyDetector {
    window_size: usize,
    min_samples: usize,
    sigma_threshold: f64,
    windows: Vec<(MetricKind, VecDeque<f64>)>,
}

impl AnomalyDetector {
    pub fn new(window_size: usize, sigma_threshold: f64, min_samples: usize) -> Self {
        Self {
            window_size,
            min_samples,
            sigma_threshold,
            windows: vec![
                (MetricKind::Cpu, VecDeque::with_capacity(window_size)),
                (MetricKind::Memory, VecDeque::with_capacity(window_size)),
                (MetricKind::RequestRate, VecDeque::with_capacity(window_size)),
                (MetricKind::ErrorRate, VecDeque::with_capacity(window_size)),
                (MetricKind::NetworkIo, VecDeque::with_capacity(window_size)),
            ],
        }
    }

    fn sample_for(kind: MetricKind, s: &MetricSample) -> f64 {
        match kind {
            MetricKind::Cpu => s.cpu,
            MetricKind::Memory => s.memory,
            MetricKind::RequestRate => s.request_rate,
            MetricKind::ErrorRate => s.error_rate,
            MetricKind::NetworkIo => s.network_io,
        }
    }

    pub fn observe(&mut self, sample: &MetricSample) -> AnomalyResult {
        let mut triggered = Vec::new();
        let mut max_z = 0.0_f64;

        for (kind, window) in self.windows.iter_mut() {
            let value = Self::sample_for(*kind, sample);

            if window.len() >= self.min_samples {
                let n = window.len() as f64;
                let mean = window.iter().sum::<f64>() / n;
                let var = window.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
                let std = var.sqrt().max(1e-9);
                let z = (value - mean).abs() / std;
                if z > self.sigma_threshold {
                    triggered.push((*kind, z));
                }
                if z > max_z { max_z = z; }
            }

            if window.len() == self.window_size { window.pop_front(); }
            window.push_back(value);
        }

        let score = (max_z / (self.sigma_threshold * 2.0) * 100.0).min(100.0);
        AnomalyResult {
            is_anomaly: !triggered.is_empty(),
            score,
            triggered,
        }
    }

    pub fn baseline(&self) -> Vec<(MetricKind, f64, f64)> {
        self.windows.iter().map(|(k, w)| {
            if w.is_empty() { return (*k, 0.0, 0.0); }
            let n = w.len() as f64;
            let mean = w.iter().sum::<f64>() / n;
            let var = w.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
            (*k, mean, var.sqrt())
        }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(cpu: f64) -> MetricSample {
        MetricSample { cpu, memory: 50.0, request_rate: 100.0, error_rate: 0.01, network_io: 1000.0 }
    }

    #[test]
    fn detects_large_outlier() {
        let mut d = AnomalyDetector::new(50, 3.0, 20);
        for _ in 0..40 { d.observe(&s(20.0)); }
        let r = d.observe(&s(99.0));
        assert!(r.is_anomaly);
    }

    #[test]
    fn stable_baseline_no_false_positive() {
        // Use realistic jitter, not zero-variance data (which is a
        // degenerate case for the 3-sigma rule: std -> 0, z -> infinity).
        let mut d = AnomalyDetector::new(50, 3.0, 20);
        let mut rng = rand::thread_rng();
        for _ in 0..40 {
            let jitter: f64 = rand::Rng::gen_range(&mut rng, -0.5..0.5);
            d.observe(&s(20.0 + jitter));
        }
        let r = d.observe(&s(20.1));
        assert!(!r.is_anomaly, "unexpected anomaly, score={}", r.score);
    }
}
