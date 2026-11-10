#!/usr/bin/env python3
"""Regression assertions over regenerated outputs, never a result generator."""
from pathlib import Path
import csv,json
ROOT=Path(__file__).resolve().parents[1];OUT=ROOT/'evidence_outputs'
s=json.loads((OUT/'seeded_summary.json').read_text())
assert (s['events'],s['tp_cpu_same_tick'],s['clean_eligible_ticks'],s['unmatched_clean_alert_ticks'])==(300,300,13800,198)
with (OUT/'latency_summary.csv').open() as f:lat={int(r['rate_hz']):r for r in csv.DictReader(f)}
expected={1:(13.341685,14.736510),10:(29.811595,56.074556),100:(15.880800,25.415196)}
for rate,(p99,upper) in expected.items():
 assert abs(float(lat[rate]['p99_ms'])-p99)<1e-6
 assert abs(float(lat[rate]['p99_ci_high_ms'])-upper)<1e-6
f=json.loads((OUT/'fusion_summary.json').read_text())
assert [f['methods'][r]['grid_pass'] for r in ('linear','noisy_or','max_with_floor')]==[0,0,939]
assert f['methods']['max_with_floor']['grid_total']==1771
assert f['methods']['max_with_floor']['all_flaps_300s']==2
runtime=next(r for r in f['scenario_metrics'] if r['rule']=='max_with_floor' and r['scenario']=='04_runtime_only')
assert runtime['time_to_strict_s']==3 and runtime['flaps_300s']==0
k=json.loads((OUT/'k3s_summary.json').read_text())
assert k['node_snapshots']['baseline']['memory_mib_range']==[3970,3987]
print('PASS: rebuilt detector, VM latency, fusion and transcribed node ranges match the reported values.')
