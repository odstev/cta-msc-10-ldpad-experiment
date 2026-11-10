#!/usr/bin/env python3
"""Rerun in an isolated source copy so archived measurements cannot be overwritten."""
import argparse,datetime,hashlib,json,os,shutil,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('experiment',choices=['seeded','fusion','latency','unseeded','tests','node-replay'])
    p.add_argument('--output',required=True,help='New directory; existing directories are rejected')
    p.add_argument('--input',help='Complete 40-sample node capture for node-replay')
    a=p.parse_args()
    if not shutil.which('cargo'):p.error('Install Rust/Cargo first; this command performs a real build')
    out=Path(a.output).expanduser().resolve()
    if out.exists():p.error('Output already exists; choose a new directory')
    if a.experiment=='node-replay' and (not a.input or not Path(a.input).is_file()):p.error('--input must be a complete recorded node trace')
    out.mkdir(parents=True); project=out/'source'
    shutil.copytree(ROOT/'ldpad',project,ignore=shutil.ignore_patterns('target','.git','evidence','latency_results','__pycache__'))
    env=os.environ.copy();env['CARGO_TARGET_DIR']=str(out/'target')
    manifest={'experiment':a.experiment,'started_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),
        'source_sha256':{str(p.relative_to(project)):hashlib.sha256(p.read_bytes()).hexdigest() for p in project.rglob('*') if p.is_file()},
        'rustc':subprocess.check_output(['rustc','--version'],text=True).strip(),'status':'started'}
    meta=out/'run_metadata.json';meta.write_text(json.dumps(manifest,indent=2)+'\n')
    try:
        with (out/'execution.log').open('wb') as log:
            def run(cmd,cwd=project,stdout=None,stdin=None):
                print('Running:', ' '.join(map(str,cmd)),flush=True)
                subprocess.run(cmd,cwd=cwd,env=env,stdout=stdout or log,stderr=log,stdin=stdin,check=True)
            if a.experiment=='latency':
                run([sys.executable,'scripts/reproduce_latency.py','--run-dir',str(out/'results'),'--storage-label','current host; record VM/storage details separately'])
            elif a.experiment=='unseeded':
                run([sys.executable,'scripts/capture_original.py','--project',str(project),'--runs','30'])
            elif a.experiment=='fusion':run([sys.executable,'scripts/reproduce_fusion.py'])
            elif a.experiment=='seeded':run([sys.executable,'scripts/reproduce_seeded.py'])
            elif a.experiment=='tests':run(['cargo','test','--locked','--release','--all-targets'])
            else:
                with Path(a.input).open('rb') as inp,(out/'node_replay.csv').open('wb') as csvout:
                    run(['cargo','run','--locked','--release','--example','k3s_node_replay'],stdin=inp,stdout=csvout)
        manifest['status']='completed'
    except Exception:
        manifest['status']='failed';raise
    finally:meta.write_text(json.dumps(manifest,indent=2)+'\n')
    print('New output:',out,'; archived evidence was not replaced.')
if __name__=='__main__':main()
