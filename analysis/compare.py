#!/usr/bin/env python3
"""Paired TOST across prespecified cells from recorded seed-level TPR data.

CSV columns: cell,seed,arm,tp,events,event_set_sha256. arm must be LDPAD or
CRANE. The event hash must identify the same labelled event list for each
matched seed and cell. No sample outcomes are included in this package.
"""
from pathlib import Path
import argparse,csv,json,re,sys
import numpy as np
sys.path.insert(0,str(Path(__file__).resolve().parent))
from ldpad_stats import describe,paired_tost,holm

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('input_csv',type=Path);p.add_argument('--output',type=Path,required=True)
    p.add_argument('--margin',type=float,default=5.0)
    p.add_argument('--expected-cells',type=Path,required=True,help='predeclared one cell per line')
    p.add_argument('--seeds',type=Path,default=Path(__file__).resolve().parents[1]/'protocol'/'seeds.txt')
    args=p.parse_args()
    if args.margin!=5.0:raise ValueError('H1e protocol fixes the margin at 5 percentage points; revise the protocol before changing it')
    with args.input_csv.open(newline='',encoding='utf-8') as f:rows=list(csv.DictReader(f))
    if not rows:raise ValueError('No data rows; no comparison can be made')
    required={'cell','seed','arm','tp','events','event_set_sha256'}
    if not required.issubset(rows[0]):raise ValueError('Missing required columns: '+str(required-set(rows[0])))
    expected=[line.strip() for line in args.expected_cells.read_text().splitlines() if line.strip() and not line.startswith('#')]
    seeds_expected=[line.strip() for line in args.seeds.read_text().splitlines() if line.strip() and not line.startswith('#')]
    if not expected or len(set(expected))!=len(expected):raise ValueError('Expected cells must be specified uniquely')
    committed_seeds=(Path(__file__).resolve().parents[1]/'protocol'/'seeds.txt').read_text().splitlines()
    if len(seeds_expected)!=30 or len(set(seeds_expected))!=30 or seeds_expected!=committed_seeds:raise ValueError('Seeds must match the 30 committed protocol seeds')
    groups={}
    for r in rows:
        cell,seed,arm=r['cell'].strip(),r['seed'].strip(),r['arm'].strip()
        if not cell or not seed or arm not in ('LDPAD','CRANE'):raise ValueError('Invalid cell, seed or arm')
        tp,events=int(r['tp']),int(r['events'])
        if events<1 or not 0<=tp<=events:raise ValueError('Invalid TP/event count')
        digest=r['event_set_sha256'].strip().lower()
        if not re.fullmatch('[0-9a-f]{64}',digest):raise ValueError('Event set digest must be SHA-256 hex')
        key=(cell,seed,arm)
        if key in groups:raise ValueError('Duplicate cell, seed and arm')
        groups[key]=(tp,events,digest)
    if {k[0] for k in groups}!=set(expected):raise ValueError('Observed cells do not match the predeclared family')
    output=[]
    for cell in sorted({k[0] for k in groups}):
        seeds=sorted({k[1] for k in groups if k[0]==cell})
        if set(seeds)!=set(seeds_expected):raise ValueError(f'Seed set mismatch for {cell}')
        x=[];y=[];event_total=0
        for seed in seeds:
            try:a=groups[cell,seed,'LDPAD'];b=groups[cell,seed,'CRANE']
            except KeyError:raise ValueError(f'Missing matched arm for {cell}, seed {seed}')
            if a[1:]!=b[1:]:raise ValueError(f'Event counts or hashes differ for {cell}, seed {seed}')
            x.append(100*a[0]/a[1]);y.append(100*b[0]/b[1]);event_total+=a[1]
        result=paired_tost(x,y,args.margin)
        output.append({'cell':cell,'event_count':event_total,'seeds':seeds,
                       'event_minimum_100_met':event_total>=100,'ldpad':describe(x),
                       'crane':describe(y),'paired':result})
    adjusted=holm([o['paired']['p_tost'] for o in output])
    for item,p_adj in zip(output,adjusted):
        item['paired']['p_tost_holm']=float(p_adj)
        item['paired']['equivalence_supported']=bool(p_adj<.05 and item['event_minimum_100_met'])
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps({'family':'all cells in input file','results':output},indent=2)+'\n',encoding='utf-8')
    print('Wrote',args.output,'for',len(output),'cells; data hashes and source provenance still require review.')
if __name__=='__main__':main()
