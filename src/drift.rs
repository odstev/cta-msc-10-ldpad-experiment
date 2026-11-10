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
