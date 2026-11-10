#!/usr/bin/env python3
"""Validate a seeded-driver CSV and calculate counts without changing observations."""
import argparse,csv,hashlib,json
from pathlib import Path

def summarize(path):
    with Path(path).open(newline='') as f: rows=list(csv.DictReader(f))
    expected={(s,t) for s in range(30) for t in range(1,501)}
    keys=[(int(r['seed']),int(r['tick'])) for r in rows]
    if len(keys)!=15000 or set(keys)!=expected:
        raise ValueError('Expected exactly one row for each seed 0..29 and tick 1..500')
    for r in rows:
        for k in ('injected_cpu_spike','cpu_alert','any_alert'):
            if r[k] not in ('0','1'): raise ValueError('Non-binary '+k)
        if int(r['injected_cpu_spike'])!=int(int(r['tick'])%50==0):
            raise ValueError('Injection schedule differs from this protocol')
        if int(r['cpu_alert'])>int(r['any_alert']): raise ValueError('CPU alert without any_alert')
    injected=[r for r in rows if r['injected_cpu_spike']=='1']
    clean=[r for r in rows if int(r['tick'])>30 and r['injected_cpu_spike']=='0']
    tp=sum(int(r['cpu_alert']) for r in injected)
    fp=sum(int(r['any_alert']) for r in clean)
    return dict(rows=len(rows),seeds=30,injected_ticks=len(injected),tp=tp,fn=len(injected)-tp,
        eligible_non_injection_ticks=len(clean),alerted_non_injection_ticks=fp,
        tpr_percent=100*tp/len(injected),non_injection_alert_percent=100*fp/len(clean),
        sha256=hashlib.sha256(Path(path).read_bytes()).hexdigest(),
        scope='Synthetic labelled ticks; no wall-clock delay or alerts/day estimate')

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('csv');p.add_argument('--output')
    a=p.parse_args(); data=json.dumps(summarize(a.csv),indent=2)+'\n'
    if a.output:
        with open(a.output,'x') as f:f.write(data)
    print(data,end='')
