//! Synthetic metric stream generator.

use rand::Rng;
use rand_distr::{Distribution, Normal};
use crate::anomaly::MetricSample;

pub struct MetricGenerator {
    tick: u64,
    spike_every: u64,
}

impl MetricGenerator {
    pub fn new(spike_every: u64) -> Self {
        Self { tick: 0, spike_every }
    }

    pub fn next_sample(&mut self) -> MetricSample {
        let mut rng = rand::thread_rng();
        self.tick += 1;

        let is_spike = self.spike_every > 0 && self.tick % self.spike_every == 0;

        let cpu = if is_spike {
            95.0
        } else {
            (20.0 + Normal::<f64>::new(0.0, 2.0).unwrap().sample(&mut rng)).max(0.0)
        };

        let memory = (45.0 + Normal::<f64>::new(0.0, 3.0).unwrap().sample(&mut rng)).max(0.0);
        let request_rate = (120.0 + Normal::<f64>::new(0.0, 8.0).unwrap().sample(&mut rng)).max(0.0);
        let error_rate = (1.0 + Normal::<f64>::new(0.0, 0.3).unwrap().sample(&mut rng)).max(0.0);
        let network_io = (2048.0 + Normal::<f64>::new(0.0, 50.0).unwrap().sample(&mut rng)).max(0.0);

        let _ = rng.gen::<f64>();
        MetricSample { cpu, memory, request_rate, error_rate, network_io }
    }
}
