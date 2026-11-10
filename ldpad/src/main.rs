//! LDPAD CLI — Layers 4, 5, 6 end-to-end.

use anyhow::Result;
use clap::Parser;
use std::time::Instant;

use ldpad::anomaly::AnomalyDetector;
use ldpad::drift::{DriftDetector, ResourceSnapshot};
use ldpad::latency::LatencyRecorder;
use ldpad::metrics::MetricGenerator;
use ldpad::risk::{PolicyMode, RiskScorer};
use ldpad::static_posture::{Auditor, K8sResource};

#[derive(Parser, Debug)]
#[command(name = "ldpad", version, about = "Lightweight Dynamic Posture and Anomaly Detection Framework")]
struct Cli {
    #[arg(long, default_value_t = false)]
    experiment_only: bool,

    #[arg(long, default_value_t = false)]
    latency_bench: bool,

    #[arg(long, default_value_t = 500)]
    iterations: usize,

    #[arg(long, default_value_t = 50)]
    spike_every: u64,
}

fn make_baseline() -> Vec<ResourceSnapshot> {
    vec![ResourceSnapshot {
        kind: "Deployment".into(),
        name: "emr-registry".into(),
        namespace: "healthcare".into(),
        spec_json: r#"{"replicas":1,"image":"emr:v1"}"#.into(),
    }]
}

fn make_drifted() -> Vec<ResourceSnapshot> {
    vec![ResourceSnapshot {
        kind: "Deployment".into(),
        name: "emr-registry".into(),
        namespace: "healthcare".into(),
        spec_json: r#"{"replicas":5,"image":"emr:latest","privileged":true}"#.into(),
    }]
}

fn make_good_resource() -> K8sResource {
    K8sResource {
        kind: "Deployment".into(),
        name: "emr-registry".into(),
        namespace: "healthcare".into(),
        privileged: false,
        run_as_root: false,
        read_only_root_fs: true,
        has_network_policy: true,
        rbac_least_privilege: true,
        image_tag_is_latest: false,
        has_resource_limits: true,
    }
}

fn run_latency_bench() -> Result<()> {
    println!("=== Latency Microbenchmark (AnomalyDetector::observe) ===");
    let mut det = AnomalyDetector::new(100, 3.0, 30);
    let rec = LatencyRecorder::new();
    let mut gen = MetricGenerator::new(0);

    for _ in 0..1000 { det.observe(&gen.next_sample()); }

    for _ in 0..100_000 {
        let s = gen.next_sample();
        let t0 = Instant::now();
        let _ = det.observe(&s);
        rec.record(t0.elapsed().as_micros() as u64);
    }

    println!("samples   : {}", rec.count());
    println!("p50 (us)  : {}", rec.p50());
    println!("p95 (us)  : {}", rec.p95());
    println!("p99 (us)  : {}", rec.p99());
    Ok(())
}

fn run_experiment(iterations: usize, spike_every: u64) -> Result<()> {
    println!("=== LDPAD End-to-End Experiment ===");
    println!("iterations : {}", iterations);
    println!("spike_every: {}", spike_every);

    let auditor = Auditor::new();
    let resource = make_good_resource();
    let audit = auditor.audit(&[resource]);
    println!("static rules evaluated : {}", audit.rules_evaluated);
    println!("static risk            : {:.2}", audit.static_risk);

    let mut drift = DriftDetector::new();
    drift.set_baseline(&make_baseline());

    let mut det = AnomalyDetector::new(100, 3.0, 30);
    let scorer = RiskScorer::new();
    let mut gen = MetricGenerator::new(spike_every);

    let mut anomalies = 0usize;
    let mut strict_ticks = 0usize;
    let mut moderate_ticks = 0usize;
    let mut normal_ticks = 0usize;
    let mut peak_risk = 0.0_f64;

    for _ in 0..iterations {
        let sample = gen.next_sample();
        let anomaly_result = det.observe(&sample);
        if anomaly_result.is_anomaly { anomalies += 1; }

        let (drift_score, _) = drift.evaluate(&make_drifted());

        let breakdown = scorer.score(
            audit.static_risk,
            drift_score,
            0.0,
            anomaly_result.score,
        );

        if breakdown.unified > peak_risk { peak_risk = breakdown.unified; }
        match breakdown.mode {
            PolicyMode::Strict => strict_ticks += 1,
            PolicyMode::Moderate => moderate_ticks += 1,
            PolicyMode::Normal => normal_ticks += 1,
        }
    }

    println!();
    println!("--- Results ---");
    println!("anomalies detected : {}", anomalies);
    println!("normal ticks       : {}", normal_ticks);
    println!("moderate ticks     : {}", moderate_ticks);
    println!("strict ticks       : {}", strict_ticks);
    println!("peak unified risk  : {:.2}", peak_risk);
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.latency_bench {
        return run_latency_bench();
    }

    let _ = cli.experiment_only;
    run_experiment(cli.iterations, cli.spike_every)
}
