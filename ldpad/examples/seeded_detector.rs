//! Evaluation driver for the unchanged LDPAD Rust anomaly detector.
//! Synthetic CPU spikes, fixed event labels, 30 explicitly seeded runs.
//! This measures a component in an in-memory harness, not a deployment.
use std::io::{self, BufWriter, Write};
use ldpad::anomaly::{AnomalyDetector, MetricKind, MetricSample};
use rand::{rngs::StdRng, Rng, SeedableRng};
use rand_distr::{Distribution, Normal};

fn main() -> io::Result<()> {
    let mut out = BufWriter::new(io::stdout().lock());
    writeln!(out, "seed,tick,injected_cpu_spike,cpu_alert,any_alert,score,cpu,memory,request_rate,error_rate,network_io")?;
    let cpu_noise = Normal::<f64>::new(0.0, 2.0).unwrap();
    let memory_noise = Normal::<f64>::new(0.0, 3.0).unwrap();
    let request_noise = Normal::<f64>::new(0.0, 8.0).unwrap();
    let error_noise = Normal::<f64>::new(0.0, 0.3).unwrap();
    let network_noise = Normal::<f64>::new(0.0, 50.0).unwrap();
    for seed in 0_u64..30 {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut detector = AnomalyDetector::new(100, 3.0, 30);
        for tick in 1..=500 {
            let spike = tick % 50 == 0;
            // These are the same distributions and one-tick spike used by
            // src/metrics.rs, except that the RNG is explicitly seeded.
            let cpu = if spike { 95.0 } else { (20.0 + cpu_noise.sample(&mut rng)).max(0.0) };
            let memory = (45.0 + memory_noise.sample(&mut rng)).max(0.0);
            let request_rate = (120.0 + request_noise.sample(&mut rng)).max(0.0);
            let error_rate = (1.0 + error_noise.sample(&mut rng)).max(0.0);
            let network_io = (2048.0 + network_noise.sample(&mut rng)).max(0.0);
            let _: f64 = rng.gen(); // as in src/metrics.rs
            let sample = MetricSample { cpu, memory, request_rate, error_rate, network_io };
            let result = detector.observe(&sample);
            let cpu_alert = result.triggered.iter().any(|(kind, _)| *kind == MetricKind::Cpu);
            writeln!(out, "{seed},{tick},{},{},{},{},{},{},{},{},{}",
                u8::from(spike), u8::from(cpu_alert), u8::from(result.is_anomaly),
                result.score, sample.cpu, sample.memory, sample.request_rate,
                sample.error_rate, sample.network_io)?;
        }
    }
    out.flush()
}
