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
