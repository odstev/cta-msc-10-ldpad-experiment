use criterion::{black_box, criterion_group, criterion_main, Criterion};

use ldpad::anomaly::{AnomalyDetector, MetricSample};
use ldpad::drift::{DriftDetector, ResourceSnapshot};
use ldpad::risk::RiskScorer;

fn bench_anomaly_observe(c: &mut Criterion) {
    let mut det = AnomalyDetector::new(100, 3.0, 30);
    for _ in 0..200 {
        det.observe(&MetricSample {
            cpu: 20.0, memory: 45.0, request_rate: 120.0, error_rate: 1.0, network_io: 2048.0,
        });
    }
    c.bench_function("anomaly_observe", |b| {
        b.iter(|| {
            black_box(det.observe(&MetricSample {
                cpu: 21.0, memory: 46.0, request_rate: 121.0, error_rate: 1.1, network_io: 2050.0,
            }));
        });
    });
}

fn bench_drift_evaluate(c: &mut Criterion) {
    let mut d = DriftDetector::new();
    let baseline = vec![ResourceSnapshot {
        kind: "Deployment".into(), name: "emr".into(), namespace: "hc".into(),
        spec_json: r#"{"replicas":1}"#.into(),
    }];
    d.set_baseline(&baseline);
    let current = vec![ResourceSnapshot {
        kind: "Deployment".into(), name: "emr".into(), namespace: "hc".into(),
        spec_json: r#"{"replicas":5}"#.into(),
    }];
    c.bench_function("drift_evaluate", |b| {
        b.iter(|| black_box(d.evaluate(&current)));
    });
}

fn bench_risk_score(c: &mut Criterion) {
    let r = RiskScorer::new();
    c.bench_function("risk_score", |b| {
        b.iter(|| black_box(r.score(50.0, 40.0, 30.0, 20.0)));
    });
}

criterion_group!(benches, bench_anomaly_observe, bench_drift_evaluate, bench_risk_score);
criterion_main!(benches);
