//! Experimental fusion rules and a policy adapter for the scenario study.
//!
//! The original `RiskScorer` and the default CLI retain their original
//! behaviour. Risk inputs and output scores are on a 0--100 scale.

use crate::risk::{PolicyMode, RiskScorer};

pub const DEFAULT_WEIGHTS: [f64; 4] = [0.30, 0.25, 0.25, 0.20];
const EPS: f64 = 1e-9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FusionRule { Linear, NoisyOr, MaxWithFloor }

impl FusionRule {
    pub fn name(self) -> &'static str {
        match self {
            Self::Linear => "linear",
            Self::NoisyOr => "noisy_or",
            Self::MaxWithFloor => "max_with_floor",
        }
    }

    pub fn score(self, risks: [u8; 4], weights: [f64; 4]) -> f64 {
        assert!(risks.iter().all(|&x| x <= 100));
        assert!(weights.iter().all(|&w| w >= 0.0 && w <= 1.0));
        assert!((weights.iter().sum::<f64>() - 1.0).abs() < 1e-8);
        let scorer = RiskScorer {
            alpha: weights[0], beta: weights[1], gamma: weights[2], delta: weights[3],
            moderate_threshold: 50.0, strict_threshold: 80.0,
        };
        let linear = scorer.score(risks[0] as f64, risks[1] as f64,
                                  risks[2] as f64, risks[3] as f64).unified;
        match self {
            Self::Linear => linear,
            Self::NoisyOr => {
                let raw = 1.0 - (0..4).map(|i| 1.0 - weights[i] * risks[i] as f64 / 100.0).product::<f64>();
                let maximum = 1.0 - weights.iter().map(|w| 1.0 - w).product::<f64>();
                100.0 * raw / maximum
            }
            Self::MaxWithFloor => {
                let structural_floor: f64 = if risks[..3].iter().any(|&x| x >= 90) { 51.0 } else { 0.0 };
                let runtime_floor: f64 = if risks[3] >= 90 { 81.0 } else { 0.0 };
                linear.max(structural_floor).max(runtime_floor)
            }
        }
    }
}

pub fn mode_name(mode: PolicyMode) -> &'static str {
    match mode { PolicyMode::Normal => "Normal", PolicyMode::Moderate => "Moderate", PolicyMode::Strict => "Strict" }
}

pub fn mode_rank(mode: PolicyMode) -> u8 {
    match mode { PolicyMode::Normal => 0, PolicyMode::Moderate => 1, PolicyMode::Strict => 2 }
}

/// One-second observations with ordinary mode checks each 30 seconds.
/// Guarded fusion adds the documented three-sample runtime fast path.
pub struct PolicyAdapter {
    pub mode: PolicyMode,
    dwell: u32,
    pending: Option<PolicyMode>,
    runtime_streak: u32,
    last_change_s: Option<u64>,
    strict_hold_until_s: u64,
}

impl PolicyAdapter {
    pub fn new() -> Self {
        Self { mode: PolicyMode::Normal, dwell: 0, pending: None, runtime_streak: 0,
               last_change_s: None, strict_hold_until_s: 0 }
    }

    pub fn step(&mut self, t: u64, score: f64, runtime_risk: u8,
                use_runtime_fast_path: bool) -> Option<(PolicyMode, PolicyMode, &'static str)> {
        if use_runtime_fast_path && runtime_risk >= 90 {
            self.runtime_streak = self.runtime_streak.saturating_add(1);
        } else {
            self.runtime_streak = 0;
        }
        if use_runtime_fast_path && self.runtime_streak >= 3 && self.mode != PolicyMode::Strict {
            return Some(self.change(t, PolicyMode::Strict, "runtime_fast_path"));
        }
        if t % 30 != 0 { return None; }
        let (eligible, count, next) = match self.mode {
            PolicyMode::Normal => (score > 50.0 + EPS, 2, PolicyMode::Moderate),
            PolicyMode::Moderate if score > 80.0 + EPS => (true, 3, PolicyMode::Strict),
            PolicyMode::Moderate if score < 45.0 - EPS => (true, 4, PolicyMode::Normal),
            PolicyMode::Moderate => (false, 0, PolicyMode::Moderate),
            PolicyMode::Strict => (score < 72.0 - EPS, 5, PolicyMode::Moderate),
        };
        if eligible {
            if self.pending != Some(next) { self.dwell = 0; self.pending = Some(next); }
            self.dwell = self.dwell.saturating_add(1);
        } else {
            self.dwell = 0;
            self.pending = None;
        }
        let rate_limit_ok = self.last_change_s.map(|last| t - last >= 60).unwrap_or(true);
        let hold_ok = self.mode != PolicyMode::Strict || t >= self.strict_hold_until_s;
        if eligible && self.dwell >= count && rate_limit_ok && hold_ok {
            return Some(self.change(t, next, "dwell"));
        }
        None
    }

    fn change(&mut self, t: u64, next: PolicyMode, reason: &'static str)
              -> (PolicyMode, PolicyMode, &'static str) {
        let previous = self.mode;
        self.mode = next;
        self.last_change_s = Some(t);
        self.dwell = 0;
        self.pending = None;
        if next == PolicyMode::Strict { self.strict_hold_until_s = t + 60; }
        (previous, next, reason)
    }
}

impl Default for PolicyAdapter { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isolated_risks_and_floor_match_the_documented_equations() {
        let config = [90, 0, 0, 0];
        let cve = [0, 0, 90, 0];
        assert!((FusionRule::Linear.score(config, DEFAULT_WEIGHTS) - 27.0).abs() < 1e-9);
        assert!((FusionRule::Linear.score(cve, DEFAULT_WEIGHTS) - 22.5).abs() < 1e-9);
        assert!((FusionRule::MaxWithFloor.score(config, DEFAULT_WEIGHTS) - 51.0).abs() < 1e-9);
        assert!((FusionRule::MaxWithFloor.score([0, 0, 0, 95], DEFAULT_WEIGHTS) - 81.0).abs() < 1e-9);
        assert!((FusionRule::NoisyOr.score([100; 4], DEFAULT_WEIGHTS) - 100.0).abs() < 1e-9);
    }

    #[test]
    fn fast_path_requires_three_consecutive_seconds() {
        let mut a = PolicyAdapter::new();
        assert!(a.step(1, 81.0, 95, true).is_none());
        assert!(a.step(2, 81.0, 95, true).is_none());
        assert_eq!(a.step(3, 81.0, 95, true).unwrap().1, PolicyMode::Strict);
        let mut b = PolicyAdapter::new();
        b.step(1, 81.0, 95, true);
        b.step(2, 81.0, 95, true);
        assert!(b.step(3, 0.0, 0, true).is_none());
        assert_eq!(b.mode, PolicyMode::Normal);
    }

    #[test]
    fn hysteresis_counts_new_direction_separately() {
        let mut a = PolicyAdapter::new();
        for t in 1..=60 { a.step(t, 60.0, 0, false); }
        assert_eq!(a.mode, PolicyMode::Moderate);
        for t in 61..=120 { a.step(t, 90.0, 0, false); }
        for t in 121..=210 { a.step(t, 0.0, 0, false); }
        assert_eq!(a.mode, PolicyMode::Moderate);
        for t in 211..=240 { a.step(t, 0.0, 0, false); }
        assert_eq!(a.mode, PolicyMode::Normal);
    }
}
