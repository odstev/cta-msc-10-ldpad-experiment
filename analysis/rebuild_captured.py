#!/usr/bin/env python3
"""Reanalyse the recorded raw captures in temporary directories.

This does not rerun measurements, simulate DDIL, or manufacture comparisons.
"""
from pathlib import Path
import csv,json,shutil,subprocess,sys,tempfile
ROOT=Path(__file__).resolve().parents[1];ORIG=ROOT/'ldpad';OUT=ROOT/'CTA'/'generated';EOUT=ROOT/'evidence_outputs'
OUT.mkdir(parents=True,exist_ok=True);EOUT.mkdir(parents=True,exist_ok=True)
sys.path.insert(0,str(ROOT/'analysis'))
from ldpad_stats import describe

def copy(src,dst):dst.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(src,dst)
def run(script,cwd,*args):
    p=subprocess.run([sys.executable,str(script),*args],cwd=cwd,capture_output=True,text=True)
    if p.returncode:raise RuntimeError(p.stdout+'\n'+p.stderr)
def csv_rows(path):
    with path.open(newline='',encoding='utf-8') as f:return list(csv.DictReader(f))

with tempfile.TemporaryDirectory(prefix='ldpad-captured-') as d:
    tmp=Path(d)
    # Seeded trace: rerun the actual ten-threshold replay, not a hard-coded table.
    copy(ORIG/'scripts'/'analyse_seeded.py',tmp/'scripts'/'analyse_seeded.py')
    copy(ORIG/'evidence'/'seeded_ticks.csv',tmp/'evidence'/'seeded_ticks.csv')
    run(tmp/'scripts'/'analyse_seeded.py',tmp)
    regenerated=csv_rows(tmp/'evidence'/'seeded_roc.csv');recorded=csv_rows(ORIG/'evidence'/'seeded_roc.csv')
    assert regenerated==recorded, 'Replayed 1--10 sigma sweep differs from recorded capture'
    per=csv_rows(tmp/'evidence'/'seeded_per_run.csv');assert len(per)==30
    stats=describe([int(x['false_alert_ticks']) for x in per])
    summary=json.loads((tmp/'evidence'/'seeded_summary.json').read_text())
    assert summary['events']==300 and summary['tp_cpu_same_tick']==300 and summary['unmatched_clean_alert_ticks']==198
    assert all(int(x['total_alert_ticks']) == int(x['tp']) + int(x['false_alert_ticks']) for x in per)
    false_ticks = sum(int(x['false_alert_ticks']) for x in per)
    alert_ticks = sum(int(x['total_alert_ticks']) for x in per)
    assert false_ticks == summary['unmatched_clean_alert_ticks'] and alert_ticks > 0
    (EOUT/'seeded_descriptive.json').write_text(json.dumps({'scope':'Synthetic CPU-spike component; one seed is the run unit','false_alert_ticks_per_seed':stats,'false_alert_ticks':false_ticks,'all_alert_bearing_ticks':alert_ticks,'false_share_of_alert_bearing_ticks_percent':100*false_ticks/alert_ticks},indent=2)+'\n')
    for name in ['seeded_roc.csv','seeded_per_run.csv','seeded_summary.json']:copy(tmp/'evidence'/name,EOUT/name)
    copy(tmp/'evidence'/'seeded_roc_table.tex',OUT/'seeded_roc_table.tex')
    ci=summary['clopper_pearson_95_ci_tpr_percent'];fp=summary['clopper_pearson_95_ci_clean_alert_percent']
    lines=[r'\begin{tabular}{@{}p{0.24\columnwidth}p{0.27\columnwidth}p{0.39\columnwidth}@{}}',r'\toprule',r'\textbf{Measure} & \textbf{Criterion} & \textbf{Synthetic result} \\',r'\midrule',
     f"CPU-spike TPR & CI lower $\\geq$90\\% & {summary['tp_cpu_same_tick']}/{summary['events']}; {summary['tpr_percent']:.2f}\\% [{ci[0]:.2f}, {ci[1]:.2f}]\\% "+r'\\',
     f"Clean-tick FP rate & $\\leq$5\\% & {summary['unmatched_clean_alert_ticks']:,}/{summary['clean_eligible_ticks']:,}; {summary['unmatched_clean_alert_percent']:.2f}\\% [{fp[0]:.2f}, {fp[1]:.2f}]\\% "+r'\\',
     f"Clean share of alert ticks & No target & {false_ticks}/{alert_ticks}; {100*false_ticks/alert_ticks:.1f}\\% "+r'\\',
     f"FP ticks/seed & Median (Q1--Q3) & {stats['median']:g} ({stats['q1']:g}--{stats['q3']:g}); IQR {stats['iqr']:g}; median 95\\% bootstrap CI {stats['median_bootstrap_95'][0]:g}--{stats['median_bootstrap_95'][1]:g} "+r'\\',
     r'Detection lag & Same-tick match & 0 ticks; wall-clock time unmeasured \\',r'\bottomrule',r'\end{tabular}']
    (OUT/'seeded_results.tex').write_text('\n'.join(lines)+'\n')
    print('Replayed seeded trace, 1--10 sigma sweep and run-level statistics.',flush=True)

    # Latency: uses saved raw records, histogram and original SQLite oplog.
    lat=tmp/'latency';lat.mkdir()
    for name in ('raw.csv','hist.csv','oplog.sqlite'):copy(ORIG/'latency_results'/'laptop_vm_20261102'/name,lat/name)
    copy(ORIG/'scripts'/'analyse_latency.py',tmp/'scripts'/'analyse_latency.py')
    run(tmp/'scripts'/'analyse_latency.py',tmp,'--run-dir',str(lat))
    for n in ('latency_table.tex','latency_ci_table.tex'):
        assert (lat/n).read_bytes()==(ORIG/'latency_results'/'laptop_vm_20261102'/n).read_bytes(),n+' capture differs'
        copy(lat/n,OUT/n)
    copy(lat/'summary.csv',EOUT/'latency_summary.csv')
    print('Recomputed laptop-VM latency tables from saved request records and SQLite log.',flush=True)

    # MadSim: reanalyse recorded risk trace and exhaustive saved grid.
    for n in ('trace.csv','transitions.csv','weight_grid.csv','runtime_metadata.json'):
        copy(ORIG/'evidence'/'fusion'/n,tmp/'evidence'/'fusion'/n)
    for p in (ORIG/'fusion_study'/'scenarios').glob('*.json'):
        copy(p,tmp/'fusion_study'/'scenarios'/p.name)
    copy(ORIG/'scripts'/'analyse_fusion.py',tmp/'scripts'/'analyse_fusion.py')
    run(tmp/'scripts'/'analyse_fusion.py',tmp)
    for n in ('scenario_table.tex','summary_table.tex','latency_table.tex','runtime_log.tex'):
        assert (tmp/'evidence'/'fusion'/n).read_bytes()==(ORIG/'evidence'/'fusion'/n).read_bytes(),n+' fusion capture differs'
        copy(tmp/'evidence'/'fusion'/n,OUT/('fusion_'+n))
    copy(tmp/'evidence'/'fusion'/'analysis_summary.json',EOUT/'fusion_summary.json')
    print('Reanalysed recorded MadSim trace and all 1,771 weight-grid rows.',flush=True)
