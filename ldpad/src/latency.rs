//! HDR-histogram-backed percentile tracking.

use hdrhistogram::Histogram;
use std::sync::Mutex;

pub struct LatencyRecorder {
    hist: Mutex<Histogram<u64>>,
}

impl LatencyRecorder {
    pub fn new() -> Self {
        Self {
            hist: Mutex::new(
                Histogram::<u64>::new_with_bounds(1, 60_000_000, 3).unwrap()
            ),
        }
    }
    pub fn record(&self, micros: u64) {
        let _ = self.hist.lock().unwrap().record(micros.min(59_999_999));
    }
    pub fn p50(&self) -> u64 { self.hist.lock().unwrap().value_at_quantile(0.50) }
    pub fn p95(&self) -> u64 { self.hist.lock().unwrap().value_at_quantile(0.95) }
    pub fn p99(&self) -> u64 { self.hist.lock().unwrap().value_at_quantile(0.99) }
    pub fn count(&self) -> u64 { self.hist.lock().unwrap().len() }
}

impl Default for LatencyRecorder {
    fn default() -> Self { Self::new() }
}
