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
