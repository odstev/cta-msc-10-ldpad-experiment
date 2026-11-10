#!/usr/bin/env python3
"""Rerun deterministic Rust components in a temporary checkout; compare raw evidence."""
from pathlib import Path
import hashlib,json,os,shutil,subprocess,tempfile,datetime
ROOT=Path(__file__).resolve().parents[1]
source=ROOT/'ldpad';cargo=shutil.which('cargo');rustc=shutil.which('rustc')
if not cargo or not rustc:raise SystemExit('Rust and Cargo must be installed and on PATH.')

def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def run(cmd,cwd,env=None,output=None):
    print('Running',' '.join(cmd),flush=True)
    if output:
        with output.open('wb') as f:p=subprocess.run(cmd,cwd=cwd,env=env,stdout=f,stderr=subprocess.PIPE)
    else:p=subprocess.run(cmd,cwd=cwd,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
    if p.returncode:raise RuntimeError((p.stderr or p.stdout).decode(errors='replace')[-6000:])
    if not output:print(p.stdout.decode(errors='replace')[-600:],flush=True)

with tempfile.TemporaryDirectory(prefix='ldpad-rust-recheck-') as d:
    p=Path(d)/'ldpad'
    shutil.copytree(source,p,ignore=shutil.ignore_patterns('target','.git','evidence','latency_results','__pycache__'))
    (p/'evidence'/'fusion').mkdir(parents=True)
    base_env=os.environ.copy();base_env['CARGO_TARGET_DIR']=str(Path(d)/'target')
    run([cargo,'test','--locked','--release','--all-features'],p,env=base_env)
    run([cargo,'run','--locked','--release','--example','seeded_detector'],p,env=base_env,output=p/'evidence'/'seeded_ticks.csv')
    files=[('seeded_ticks.csv',p/'evidence'/'seeded_ticks.csv',source/'evidence'/'seeded_ticks.csv')]
    env=base_env.copy();env['RUSTFLAGS']=(env.get('RUSTFLAGS','')+' --cfg madsim').strip()
    study=p/'fusion_study'
    run([cargo,'test','--locked','--release'],study,env=env)
    run([cargo,'run','--locked','--release'],study,env=env)
    for name in ['trace.csv','transitions.csv','weight_grid.csv']:
        files.append((name,p/'evidence'/'fusion'/name,source/'evidence'/'fusion'/name))
    result={}
    for name,actual,expected in files:
        actual_hash=sha(actual);expected_hash=sha(expected)
        if actual_hash!=expected_hash:
            raise RuntimeError(f'{name}: rerun differs from recorded evidence; do not replace either file')
        result[name]={'recorded_sha256':expected_hash,'rerun_sha256':actual_hash,'identical':True}
    report={'description':'Deterministic Rust/MadSim component replay; no VM, network, clinical or comparator run.',
            'checked_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),
            'rustc':subprocess.check_output([rustc,'--version'],text=True).strip(),
            'cargo':subprocess.check_output([cargo,'--version'],text=True).strip(),'files':result}
    out=ROOT/'evidence_outputs'/'rust_recheck.json';out.parent.mkdir(exist_ok=True)
    out.write_text(json.dumps(report,indent=2)+'\n')
    print('All deterministic seeded trace and MadSim/grid files match the retained captures. Report:',out,flush=True)
