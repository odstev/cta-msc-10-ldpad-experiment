#!/bin/bash
set -e
cd ~/ldpad

echo "=== Writing Cargo.toml ==="
cat > Cargo.toml << 'ENDOFFILE'
[package]
name = "ldpad"
version = "0.1.0"
edition = "2021"
description = "Lightweight Dynamic Posture and Anomaly Detection Framework (LDPAD)"
authors = ["Semunyu Aloyzious <semunyu.aloyzious@students.mak.ac.ug>", "Odoi Stephen <odoi.stephen@students.mak.ac.ug>"]

[lib]
name = "ldpad"
path = "src/lib.rs"

[[bin]]
name = "ldpad"
path = "src/main.rs"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.10"
hex = "0.4"
rand = "0.8"
rand_distr = "0.4"
anyhow = "1"
clap = { version = "4", features = ["derive"] }
hdrhistogram = "7"

[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "evaluate_tick_bench"
harness = false

[profile.release]
opt-level = 3
lto = true
ENDOFFILE

echo "=== Writing src/lib.rs ==="
cat > src/lib.rs << 'ENDOFFILE'
pub mod anomaly;
pub mod drift;
pub mod latency;
pub mod metrics;
pub mod risk;
pub mod static_posture;
pub mod stats;
pub mod threat_intel;
ENDOFFILE

echo "=== Writing src/static_posture.rs ==="
cat > src/static_posture.rs << 'ENDOFFILE'
//! Layer 4: Static Posture Auditing
//! Audits Kubernetes-style resource configurations against a fixed
//! rule set derived from CIS Benchmark controls.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sResource {
    pub kind: String,
    pub name: String,
    pub namespace: String,
    pub privileged: bool,
    pub run_as_root: bool,
    pub read_only_root_fs: bool,
    pub has_network_policy: bool,
    pub rbac_least_privilege: bool,
    pub image_tag_is_latest: bool,
    pub has_resource_limits: bool,
}

/// Rule contains a function pointer, which serde cannot serialize.
/// Keep only Debug + Clone on this struct.
#[derive(Debug, Clone)]
pub struct Rule {
    pub name: String,
    pub severity: f64,
    pub check: fn(&K8sResource) -> bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub resource: String,
    pub rule: String,
    pub passed: bool,
    pub severity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticAuditReport {
    pub findings: Vec<Finding>,
    pub static_risk: f64,
    pub rules_evaluated: usize,
}

pub struct Auditor {
    rules: Vec<Rule>,
}

impl Auditor {
    pub fn new() -> Self {
        Self {
            rules: vec![
                Rule { name: "non_privileged".into(),         severity: 30.0, check: |r| !r.privileged },
                Rule { name: "non_root".into(),               severity: 20.0, check: |r| !r.run_as_root },
                Rule { name: "read_only_rootfs".into(),       severity: 15.0, check: |r| r.read_only_root_fs },
                Rule { name: "network_policy_present".into(), severity: 20.0, check: |r| r.has_network_policy },
                Rule { name: "rbac_least_privilege".into(),   severity: 15.0, check: |r| r.rbac_least_privilege },
                Rule { name: "image_not_latest".into(),       severity: 10.0, check: |r| !r.image_tag_is_latest },
                Rule { name: "resource_limits_set".into(),    severity: 10.0, check: |r| r.has_resource_limits },
            ],
        }
    }

    pub fn rule_names(&self) -> Vec<String> {
        self.rules.iter().map(|r| r.name.clone()).collect()
    }

    pub fn rule_count(&self) -> usize { self.rules.len() }

    pub fn audit(&self, resources: &[K8sResource]) -> StaticAuditReport {
        let mut findings = Vec::new();
        let mut total_weight = 0.0_f64;
        let mut failed_weight = 0.0_f64;

        for r in resources {
            for rule in &self.rules {
                let passed = (rule.check)(r);
                total_weight += rule.severity;
                if !passed {
                    failed_weight += rule.severity;
                }
                findings.push(Finding {
                    resource: format!("{}/{}/{}", r.kind, r.namespace, r.name),
                    rule: rule.name.clone(),
                    passed,
                    severity: rule.severity,
                });
            }
        }

        let static_risk = if total_weight > 0.0 {
            (failed_weight / total_weight * 100.0).min(100.0)
        } else {
            0.0
        };

        StaticAuditReport {
            findings,
            static_risk,
            rules_evaluated: self.rules.len(),
        }
    }
}

impl Default for Auditor {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bad() -> K8sResource {
        K8sResource {
            kind: "Deployment".into(),
            name: "emr".into(),
            namespace: "healthcare".into(),
            privileged: true,
            run_as_root: true,
            read_only_root_fs: false,
            has_network_policy: false,
            rbac_least_privilege: false,
            image_tag_is_latest: true,
            has_resource_limits: false,
        }
    }

    fn good() -> K8sResource {
        K8sResource {
            kind: "Deployment".into(),
            name: "emr".into(),
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

    #[test]
    fn bad_resource_scores_high() {
        let a = Auditor::new();
        let report = a.audit(&[bad()]);
        assert!(report.static_risk > 80.0);
    }

    #[test]
    fn good_resource_scores_zero() {
        let a = Auditor::new();
        let report = a.audit(&[good()]);
        assert!(report.static_risk < 1.0);
    }
}
ENDOFFILE

echo "=== Writing src/drift.rs ==="
cat > src/drift.rs << 'ENDOFFILE'
//! Layer 5a: Configuration Drift Detection

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSnapshot {
    pub kind: String,
    pub name: String,
    pub namespace: String,
    pub spec_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftEvent {
    pub resource: String,
    pub old_hash: String,
    pub new_hash: String,
}

pub struct DriftDetector {
    baseline: HashMap<String, String>,
}

impl DriftDetector {
    pub fn new() -> Self { Self { baseline: HashMap::new() } }

    fn key(r: &ResourceSnapshot) -> String {
        format!("{}/{}/{}", r.kind, r.namespace, r.name)
    }

    fn hash(spec: &str) -> String {
        let mut h = Sha256::new();
        h.update(spec.as_bytes());
        hex::encode(h.finalize())
    }

    pub fn set_baseline(&mut self, resources: &[ResourceSnapshot]) {
        self.baseline.clear();
        for r in resources {
            self.baseline.insert(Self::key(r), Self::hash(&r.spec_json));
        }
    }

    pub fn evaluate(&self, current: &[ResourceSnapshot]) -> (f64, Vec<DriftEvent>) {
        if self.baseline.is_empty() || current.is_empty() {
            return (0.0, Vec::new());
        }

        let mut events = Vec::new();
        let mut drifted = 0usize;

        for r in current {
            let key = Self::key(r);
            let new_hash = Self::hash(&r.spec_json);
            match self.baseline.get(&key) {
                Some(old) if old != &new_hash => {
                    drifted += 1;
                    events.push(DriftEvent {
                        resource: key.clone(),
                        old_hash: old.clone(),
                        new_hash,
                    });
                }
                None => {
                    drifted += 1;
                    events.push(DriftEvent {
                        resource: key.clone(),
                        old_hash: "<absent>".into(),
                        new_hash,
                    });
                }
                _ => {}
            }
        }

        let score = (drifted as f64 / current.len() as f64) * 100.0;
        (score.min(100.0), events)
    }
}

impl Default for DriftDetector {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(name: &str, spec: &str) -> ResourceSnapshot {
        ResourceSnapshot {
            kind: "Deployment".into(),
            name: name.into(),
            namespace: "hc".into(),
            spec_json: spec.into(),
        }
    }

    #[test]
    fn detects_drift() {
        let mut d = DriftDetector::new();
        d.set_baseline(&[snap("emr", "{\"replicas\":1}")]);
        let (score, events) = d.evaluate(&[snap("emr", "{\"replicas\":5}")]);
        assert!(score > 50.0);
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn no_drift_no_event() {
        let mut d = DriftDetector::new();
        d.set_baseline(&[snap("emr", "{\"replicas\":1}")]);
        let (score, events) = d.evaluate(&[snap("emr", "{\"replicas\":1}")]);
        assert_eq!(score, 0.0);
        assert!(events.is_empty());
    }
}
ENDOFFILE

echo "=== Writing src/threat_intel.rs ==="
cat > src/threat_intel.rs << 'ENDOFFILE'
//! Layer 5b: Threat Intelligence Integration

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatEntry {
    pub cve_id: String,
    pub severity: f64,
    pub likelihood: f64,
    pub signature: String,
}

pub struct ThreatIntelStore {
    entries: HashMap<String, ThreatEntry>,
    last_sync_ms: u128,
}

impl ThreatIntelStore {
    pub fn new() -> Self {
        Self { entries: HashMap::new(), last_sync_ms: 0 }
    }

    pub fn upsert(&mut self, entry: ThreatEntry) {
        self.entries.insert(entry.cve_id.clone(), entry);
    }

    pub fn score(&self, signatures_seen: &[String]) -> f64 {
        let mut max = 0.0_f64;
        for sig in signatures_seen {
            for entry in self.entries.values() {
                if sig.contains(&entry.signature) {
                    let s = entry.severity * entry.likelihood;
                    if s > max { max = s; }
                }
            }
        }
        max.min(100.0)
    }

    pub fn last_sync_ms(&self) -> u128 { self.last_sync_ms }
    pub fn count(&self) -> usize { self.entries.len() }

    pub fn snapshot_for_offline(&self) -> Vec<ThreatEntry> {
        self.entries.values().cloned().collect()
    }

    pub fn merge_offline_snapshot(&mut self, snap: Vec<ThreatEntry>) {
        for e in snap {
            self.entries.entry(e.cve_id.clone()).or_insert(e);
        }
        self.last_sync_ms = 1;
    }
}

impl Default for ThreatIntelStore {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scores_matched_signature() {
        let mut s = ThreatIntelStore::new();
        s.upsert(ThreatEntry {
            cve_id: "CVE-2024-0001".into(),
            severity: 9.0,
            likelihood: 8.0,
            signature: "ransomware".into(),
        });
        let score = s.score(&["observed: ransomware payload".into()]);
        assert_eq!(score, 72.0);
    }

    #[test]
    fn no_match_no_score() {
        let mut s = ThreatIntelStore::new();
        s.upsert(ThreatEntry {
            cve_id: "CVE-2024-0001".into(),
            severity: 9.0,
            likelihood: 8.0,
            signature: "ransomware".into(),
        });
        assert_eq!(s.score(&["benign".into()]), 0.0);
    }
}
ENDOFFILE

echo "=== Writing src/anomaly.rs ==="
cat > src/anomaly.rs << 'ENDOFFILE'
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
ENDOFFILE

echo "=== Writing src/risk.rs ==="
cat > src/risk.rs << 'ENDOFFILE'
//! Layer 5d: Unified Risk Scoring

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyMode { Normal, Moderate, Strict }

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RiskBreakdown {
    pub static_risk: f64,
    pub drift_risk: f64,
    pub threat_risk: f64,
    pub anomaly_risk: f64,
    pub unified: f64,
    pub mode: PolicyMode,
}

pub struct RiskScorer {
    pub alpha: f64,
    pub beta: f64,
    pub gamma: f64,
    pub delta: f64,
    pub strict_threshold: f64,
    pub moderate_threshold: f64,
}

impl RiskScorer {
    pub fn new() -> Self {
        Self {
            alpha: 0.30,
            beta: 0.25,
            gamma: 0.25,
            delta: 0.20,
            strict_threshold: 80.0,
            moderate_threshold: 50.0,
        }
    }

    pub fn score(&self, s: f64, d: f64, t: f64, a: f64) -> RiskBreakdown {
        let unified = self.alpha * s + self.beta * d + self.gamma * t + self.delta * a;
        let mode = if unified > self.strict_threshold {
            PolicyMode::Strict
        } else if unified > self.moderate_threshold {
            PolicyMode::Moderate
        } else {
            PolicyMode::Normal
        };
        RiskBreakdown {
            static_risk: s,
            drift_risk: d,
            threat_risk: t,
            anomaly_risk: a,
            unified,
            mode,
        }
    }
}

impl Default for RiskScorer {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weights_sum_to_one() {
        let r = RiskScorer::new();
        let sum = r.alpha + r.beta + r.gamma + r.delta;
        assert!((sum - 1.0).abs() < 1e-9);
    }

    #[test]
    fn high_risk_yields_strict_mode() {
        let r = RiskScorer::new();
        let b = r.score(95.0, 90.0, 85.0, 92.0);
        assert_eq!(b.mode, PolicyMode::Strict);
    }

    #[test]
    fn low_risk_yields_normal_mode() {
        let r = RiskScorer::new();
        let b = r.score(5.0, 10.0, 5.0, 8.0);
        assert_eq!(b.mode, PolicyMode::Normal);
    }
}
ENDOFFILE

echo "=== Writing src/latency.rs ==="
cat > src/latency.rs << 'ENDOFFILE'
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
ENDOFFILE

echo "=== Writing src/stats.rs ==="
cat > src/stats.rs << 'ENDOFFILE'
//! Statistical reporting utilities.

use rand::seq::SliceRandom;
use rand::thread_rng;

pub fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() { return 0.0; }
    xs.iter().sum::<f64>() / xs.len() as f64
}

pub fn std_dev(xs: &[f64]) -> f64 {
    if xs.len() < 2 { return 0.0; }
    let m = mean(xs);
    let var = xs.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (xs.len() as f64 - 1.0);
    var.sqrt()
}

pub fn bootstrap_ci_95(samples: &[f64], iterations: usize) -> (f64, f64) {
    if samples.is_empty() { return (0.0, 0.0); }
    let mut rng = thread_rng();
    let mut means = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let resample: Vec<f64> = (0..samples.len())
            .map(|_| *samples.choose(&mut rng).unwrap())
            .collect();
        means.push(mean(&resample));
    }
    means.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let lo = means[(iterations as f64 * 0.025) as usize];
    let hi = means[(iterations as f64 * 0.975) as usize];
    (lo, hi)
}

pub fn cohen_d(a: &[f64], b: &[f64]) -> f64 {
    let ma = mean(a);
    let mb = mean(b);
    let sa = std_dev(a);
    let sb = std_dev(b);
    let pooled = (((sa.powi(2) + sb.powi(2)) / 2.0)).sqrt();
    if pooled.abs() < 1e-12 { return 0.0; }
    (ma - mb) / pooled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mean_of_uniform() {
        let xs = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((mean(&xs) - 3.0).abs() < 1e-9);
    }

    #[test]
    fn ci_brackets_mean() {
        let xs: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let (lo, hi) = bootstrap_ci_95(&xs, 500);
        assert!(lo < mean(&xs));
        assert!(hi > mean(&xs));
    }
}
ENDOFFILE

echo "=== Writing src/metrics.rs ==="
cat > src/metrics.rs << 'ENDOFFILE'
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
ENDOFFILE

echo "=== Writing src/main.rs ==="
cat > src/main.rs << 'ENDOFFILE'
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
ENDOFFILE

echo "=== Writing benches/evaluate_tick_bench.rs ==="
cat > benches/evaluate_tick_bench.rs << 'ENDOFFILE'
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
ENDOFFILE

echo "=== Writing Makefile ==="
cat > Makefile << 'ENDOFFILE'
.PHONY: build test bench reproduce clean fmt clippy

build:
	cargo build --release

test:
	cargo test --all-features

bench:
	cargo bench

reproduce:
	@echo "=== LDPAD Reproducibility Run ==="
	@rustc --version
	@cargo --version
	cargo test --release --all-features -- --nocapture
	cargo run --release -- --experiment-only --iterations 500
	cargo run --release -- --latency-bench
	cargo bench

fmt:
	cargo fmt --all

clippy:
	cargo clippy --all-targets -- -D warnings

clean:
	cargo clean
ENDOFFILE

echo
echo "============================================================"
echo "All files written."
echo
echo "Now running:"
echo "  cargo build --release"
echo "  cargo test"
echo "============================================================"
echo

cargo build --release
cargo test
