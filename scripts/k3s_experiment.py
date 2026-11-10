#!/usr/bin/env python3
"""Create a separate K3s test pod, copy the actual Rust binaries, and capture a new run."""
import argparse,datetime,hashlib,json,shutil,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--output',required=True)
    p.add_argument('--namespace',default='default')
    p.add_argument('--image',default='ubuntu:24.04',help='Use a digest for a fixed image; resolved digest is captured')
    p.add_argument('--kubectl',action='store_true',help='Use configured kubectl instead of sudo k3s kubectl')
    a=p.parse_args();out=Path(a.output).expanduser().resolve()
    if out.exists():p.error('Choose a new output directory')
    if not shutil.which('cargo'):p.error('Cargo is required; run this on your Ubuntu VM')
    out.mkdir(parents=True)
    base=['kubectl'] if a.kubectl else ['sudo','k3s','kubectl']
    kc=base+['-n',a.namespace]
    subprocess.run(['cargo','build','--locked','--release','--bin','ldpad','--example','seeded_detector'],cwd=ROOT/'ldpad',check=True)
    metadata=json.loads(subprocess.check_output(['cargo','metadata','--locked','--format-version','1','--no-deps'],cwd=ROOT/'ldpad'))
    target=Path(metadata['target_directory'])/'release'
    pod='ldpad-repro-'+datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%d%H%M%S%f')
    spec={'apiVersion':'v1','kind':'Pod','metadata':{'name':pod,'namespace':a.namespace},'spec':{
        'restartPolicy':'Never','automountServiceAccountToken':False,
        'containers':[{'name':'experiment','image':a.image,'command':['sleep','86400']} ]}}
    (out/'pod.json').write_text(json.dumps(spec,indent=2)+'\n')
    subprocess.run(kc+['create','-f',str(out/'pod.json')],check=True)
    print('Created',pod,'; cleanup after review:',' '.join(kc+['delete','pod',pod]),flush=True)
    subprocess.run(kc+['wait','--for=condition=Ready','pod/'+pod,'--timeout=180s'],check=True)
    binaries={}
    for name,path in [('ldpad',target/'ldpad'),('seeded_detector',target/'examples/seeded_detector')]:
        dest='/tmp/'+name;binaries[name]=hashlib.sha256(path.read_bytes()).hexdigest()
        subprocess.run(kc+['cp',str(path),pod+':'+dest],check=True)
        subprocess.run(kc+['exec',pod,'--','chmod','755',dest],check=True)
    for name,cmd in [('unseeded.log',['/tmp/ldpad']),('ticks.csv',['/tmp/seeded_detector'])]:
        with (out/name).open('wb') as f,(out/(name+'.stderr')).open('wb') as err:
            subprocess.run(kc+['exec',pod,'--']+cmd,stdout=f,stderr=err,check=True)
    for name,cmd in [('pod_status.json',kc+['get','pod',pod,'-o','json']),('cluster_version.json',base+['version','-o','json'])]:
        (out/name).write_bytes(subprocess.check_output(cmd))
    (out/'binary_sha256.json').write_text(json.dumps(binaries,indent=2)+'\n')
    subprocess.run([sys.executable,str(ROOT/'scripts/summarize_ticks.py'),str(out/'ticks.csv'),'--output',str(out/'summary.json')],check=True)
    print('Saved',out,'; this is a synthetic pod run, not admission integration.')
if __name__=='__main__':main()
