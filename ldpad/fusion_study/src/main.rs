#![allow(unexpected_cfgs)]
#[cfg(not(madsim))]
compile_error!("Run with RUSTFLAGS='--cfg madsim' to use virtual time.");

#[path = "../../src/risk.rs"]
mod risk;
#[path = "../../src/fusion.rs"]
mod fusion;

use fusion::{mode_name, FusionRule, PolicyAdapter, DEFAULT_WEIGHTS};
use madsim::time::{self, Duration, TimeHandle};
use risk::PolicyMode;
use serde::Deserialize;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::PathBuf;

const RULES: [FusionRule; 3] = [FusionRule::Linear, FusionRule::NoisyOr, FusionRule::MaxWithFloor];
const SETTLE_S: u64 = 300;

#[derive(Debug, Clone, Deserialize)]
struct Segment { start_s: u64, end_s: u64, risks: [u8; 4], expected_mode: PolicyMode }

#[derive(Debug, Clone, Deserialize)]
struct Scenario { id: String, duration_s: u64, grid_core: bool, segments: Vec<Segment> }

fn load_scenarios(root: &PathBuf) -> Vec<Scenario> {
    let mut paths: Vec<_> = fs::read_dir(root.join("scenarios")).unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("json"))
        .collect();
    paths.sort();
    let mut scenarios = Vec::new();
    for path in paths {
        let scenario: Scenario = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert_eq!(scenario.duration_s, 3600);
        let mut prev = 0;
        for segment in &scenario.segments {
            assert_eq!(segment.start_s, prev, "scenario {} has a gap", scenario.id);
            assert!(segment.end_s > segment.start_s);
            assert!(segment.risks.iter().all(|&x| x <= 100));
            prev = segment.end_s;
        }
        assert_eq!(prev, scenario.duration_s);
        scenarios.push(scenario);
    }
    assert_eq!(scenarios.len(), 9);
    assert_eq!(scenarios.iter().filter(|s| s.grid_core).count(), 6);
    scenarios
}

struct VirtualOutput { trace: String, transitions: String, elapsed_s: u64 }

async fn run_virtual(scenarios: &[Scenario]) -> VirtualOutput {
    let mut trace = String::from("rule,scenario,time_s,static,drift,threat,runtime,score,expected_mode,actual_mode\n");
    let mut transitions = String::from("rule,scenario,time_s,from_mode,to_mode,reason,score,runtime\n");
    let clock = TimeHandle::current();
    let whole_start = clock.elapsed();
    for rule in RULES {
        for scenario in scenarios {
            let start = clock.elapsed();
            let mut policy = PolicyAdapter::new();
            for segment in &scenario.segments {
                let score = rule.score(segment.risks, DEFAULT_WEIGHTS);
                for t in segment.start_s + 1..=segment.end_s {
                    time::sleep(Duration::from_secs(1)).await;
                    assert_eq!((clock.elapsed() - start).as_secs(), t, "virtual clock mismatch");
                    if let Some((old, new, reason)) = policy.step(
                        t, score, segment.risks[3], rule == FusionRule::MaxWithFloor
                    ) {
                        writeln!(transitions, "{},{},{},{},{},{},{:.6},{}",
                            rule.name(), scenario.id, t, mode_name(old), mode_name(new),
                            reason, score, segment.risks[3]).unwrap();
                    }
                    writeln!(trace, "{},{},{},{},{},{},{},{:.6},{},{}",
                        rule.name(), scenario.id, t, segment.risks[0], segment.risks[1],
                        segment.risks[2], segment.risks[3], score,
                        mode_name(segment.expected_mode), mode_name(policy.mode)).unwrap();
                }
            }
        }
    }
    VirtualOutput { trace, transitions, elapsed_s: (clock.elapsed() - whole_start).as_secs() }
}

// The exhaustive grid replays the very same one-second controller with the
// virtual timestamps from the fixtures. It does not create 1,771 MadSim runtimes.
fn passes_grid(rule: FusionRule, weights: [f64; 4], scenarios: &[Scenario]) -> bool {
    for scenario in scenarios.iter().filter(|s| s.grid_core) {
        let mut policy = PolicyAdapter::new();
        let mut previous = scenario.segments[0].expected_mode;
        let mut since = 0;
        for segment in &scenario.segments {
            if segment.expected_mode != previous {
                previous = segment.expected_mode;
                since = segment.start_s;
            }
            let score = rule.score(segment.risks, weights);
            for t in segment.start_s + 1..=segment.end_s {
                policy.step(t, score, segment.risks[3], rule == FusionRule::MaxWithFloor);
                if t >= since + SETTLE_S && policy.mode != segment.expected_mode { return false; }
            }
        }
    }
    true
}

fn run_grid(scenarios: &[Scenario]) -> (String, [usize; 3], usize) {
    let mut output = String::from("static_weight,drift_weight,threat_weight,runtime_weight,linear_pass,noisy_or_pass,max_with_floor_pass\n");
    let mut passes = [0; 3];
    let mut count = 0;
    for a in 0..=20 {
        for b in 0..=20-a {
            for c in 0..=20-a-b {
                let d = 20-a-b-c;
                let w = [a as f64/20.0,b as f64/20.0,c as f64/20.0,d as f64/20.0];
                let mut correct = [false; 3];
                for (i, rule) in RULES.iter().enumerate() {
                    correct[i] = passes_grid(*rule, w, scenarios);
                    passes[i] += usize::from(correct[i]);
                }
                writeln!(output, "{:.2},{:.2},{:.2},{:.2},{},{},{}", w[0],w[1],w[2],w[3],
                    u8::from(correct[0]),u8::from(correct[1]),u8::from(correct[2])).unwrap();
                count += 1;
            }
        }
    }
    assert_eq!(count, 1771);
    (output, passes, count)
}

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let scenarios = load_scenarios(&root);
    let destination = root.parent().unwrap().join("evidence").join("fusion");
    fs::create_dir_all(&destination).unwrap();
    let runtime = madsim::runtime::Runtime::new();
    let observed = runtime.block_on(run_virtual(&scenarios));
    assert_eq!(observed.elapsed_s, 9 * 3 * 3600);
    fs::write(destination.join("trace.csv"), observed.trace).unwrap();
    fs::write(destination.join("transitions.csv"), observed.transitions).unwrap();
    let (grid, passed, count) = run_grid(&scenarios);
    fs::write(destination.join("weight_grid.csv"), grid).unwrap();
    let metadata = serde_json::json!({
        "simulator":"madsim 0.2.34", "cfg_madsim":true, "madsim_seed":0,
        "scenarios":scenarios.len(), "core_scenarios":6, "stress_scenarios":3,
        "virtual_seconds_per_scenario":3600, "rules":3, "virtual_seconds_total":observed.elapsed_s,
        "grid_vectors":count, "grid_pass_counts":passed,
        "grid_method":"one-second replay of the same adapter and six core fixtures; 300-second grace after an expected-mode change",
        "warning":"Synthetic, design-labelled component simulation; not deployment or independently verified attack truth."
    });
    fs::write(destination.join("runtime_metadata.json"), serde_json::to_vec_pretty(&metadata).unwrap()).unwrap();
    println!("Virtual seconds: {}; grid pass counts: {:?} / {}", observed.elapsed_s, passed, count);
    println!("Wrote raw observations and transitions to {}", destination.display());
}
