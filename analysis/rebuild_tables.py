#!/usr/bin/env python3
"""Rebuild current manuscript tables from captured data and Rust source."""
from pathlib import Path
import csv,io,json,random,re,statistics,sys,tomllib,zipfile
import numpy as np
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'analysis'))
from ldpad_stats import describe
OUT=ROOT/'CTA'/'generated';OUT.mkdir(parents=True,exist_ok=True)
RUST=ROOT/'ldpad'

def tex(s):return s.replace('_',r'\_').replace('%',r'\%')
def write(name,lines):(OUT/name).write_text('\n'.join(lines)+'\n',encoding='utf-8')

# Stable manuscript Table 2: assert listed used crates are direct dependencies.
data=tomllib.loads((RUST/'Cargo.toml').read_text())
deps=data['dependencies'];dev=data['dev-dependencies']
fusion=tomllib.loads((RUST/'fusion_study'/'Cargo.toml').read_text())['dependencies']
rows=[('Serialisation',['serde','serde_json'],'In use'),('Hashing',['sha2','hex'],'In use'),
      ('Randomness',['rand','rand_distr'],'In use'),('CLI',['clap'],'In use'),
      ('Tail latency',['hdrhistogram'],'Pilot'),('Benchmarking',['criterion'],'Dev dependency'),
      ('Async runtime',['tokio'],'Latency pilot'),('Policy and log',['regorus','rusqlite'],'Latency pilot')]
for _,crates,_ in rows:
 for name in crates:assert name in deps or name in dev,name
assert 'madsim' in fusion and (RUST/'src'/'stats.rs').is_file()
lines=[r'\begin{tabular}{|p{55pt}|p{85pt}|p{60pt}|}',r'\hline',r'\textbf{Layer} & \textbf{Crate} & \textbf{Status} \\',r'\hline']
for label,crates,status in rows:
 lines.append(label+' & '+', '.join(r'\texttt{'+tex(c)+'}' for c in crates)+' & '+status+r' \\')
lines += [r'Statistics & in-house \texttt{stats.rs} & In use \\',
          r'Deterministic simulation & \texttt{madsim} & Separate prototype \\',
          r'Other planned services & \texttt{turmoil}, \texttt{openraft}, \texttt{tonic} & Not direct dependencies \\',
          r'\hline',r'\end{tabular}']
write('toolchain.tex',lines)

# Stable manuscript Table 3: summaries of the captured original unseeded run.
with zipfile.ZipFile(RUST/'evidence'/'original_unseeded_capture.zip') as z:
 summary_name=next(n for n in z.namelist() if n.endswith('/run_summary.csv'))
 manifest_name=next(n for n in z.namelist() if n.endswith('/manifest.json'))
 with z.open(summary_name) as b:rows=list(csv.DictReader(io.TextIOWrapper(b,encoding='utf-8')))
 manifest=json.loads(z.read(manifest_name))
assert len(rows)==manifest['repetitions_completed']==30
assert [int(r['run']) for r in rows]==list(range(1,31))
assert all(int(r['normal_ticks'])==500 and int(r['moderate_ticks'])==0 and int(r['strict_ticks'])==0 for r in rows)
values=[int(r['alerts']) for r in rows]
rng=random.Random(20261101)
means=sorted(statistics.mean(rng.choices(values,k=len(values))) for _ in range(10000))
def percentile(x,p):
 pos=(len(x)-1)*p;lo=int(pos);hi=min(lo+1,len(x)-1);return x[lo]+(x[hi]-x[lo])*(pos-lo)
lo,hi=percentile(means,.025),percentile(means,.975)
assert abs(lo-manifest['alerts_summary']['bootstrap_95_ci_for_mean'][0])<1e-12
assert abs(hi-manifest['alerts_summary']['bootstrap_95_ci_for_mean'][1])<1e-12
med=describe(values)
lines=[r'\begin{tabular}{@{}lr@{}}',r'\toprule',r'\textbf{Measure} & \textbf{Observed} \\',r'\midrule',
 f'CLI repetitions & {len(rows)} '+r'\\',
 f"Samples / scheduled spikes per run & {manifest['iterations']} / {manifest['iterations']//manifest['spike_every']} "+r'\\',
 f'Alert ticks/run, mean (95\\% CI) & {statistics.mean(values):.2f} ({lo:.2f}--{hi:.2f}) '+r'\\',
 f"Alert ticks/run, median (Q1--Q3) & {med['median']:g} ({med['q1']:g}--{med['q3']:g}) "+r'\\',
 f"IQR (Q3--Q1) & {med['iqr']:g} "+r'\\',
 f"Median, 95\\% bootstrap CI & {med['median_bootstrap_95'][0]:g}--{med['median_bootstrap_95'][1]:g} "+r'\\',
 f'Alert ticks/run, range & {min(values)}--{max(values)} '+r'\\',
 r'Normal ticks/run & 500 \\',r'\bottomrule',r'\end{tabular}']
write('unseeded_capture.tex',lines)
(ROOT/'evidence_outputs'/'unseeded_descriptive.json').write_text(json.dumps({'description':'30 captured unseeded exploratory CLI runs; original data cannot be newly reproduced exactly','mean':statistics.mean(values),'mean_bootstrap_95':[lo,hi],'median_iqr':med,'raw_capture_manifest':manifest_name},indent=2)+'\n')

# Stable manuscript Table 10: rule identifiers and severity weights come
# directly from the supplied Rust auditor, not the prior seven-rule proposal.
src=(RUST/'src'/'static_posture.rs').read_text()
pairs=re.findall(r'Rule\s*\{\s*name:\s*"([a-z_]+)"\.into\(\),\s*severity:\s*([0-9.]+)',src)
assert len(pairs)==7 and len(set(n for n,_ in pairs))==7,pairs
names=['non_privileged','non_root','read_only_rootfs','network_policy_present','rbac_least_privilege','image_not_latest','resource_limits_set']
assert [n for n,_ in pairs]==names
weights=[float(w) for _,w in pairs];assert sum(weights)==120
lines=[r'\begin{tabular}{|c|p{0.60\columnwidth}|r|}',r'\hline',r'\textbf{ID} & \textbf{Rust auditor rule} & \textbf{Weight} \\',r'\hline']
for i,(name,weight) in enumerate(pairs,1):
 lines += [f'R{i} & '+r'\texttt{'+tex(name)+'} & '+f'{float(weight):g} '+r'\\',r'\hline']
lines.append(r'\end{tabular}')
write('rust_static_rules.tex',lines)
(OUT/'static_weight_sum.tex').write_text(f'{sum(weights):g}',encoding='utf-8')
print('Rebuilt toolchain, 30-run captured summary, and seven Rust auditor rules (weight sum 120).')
