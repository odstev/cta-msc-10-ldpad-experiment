#!/usr/bin/env python3
"""Rebuild the terminal-transcription summary; this does not rerun K3s."""
from pathlib import Path
import csv,json
ROOT=Path(__file__).resolve().parents[1]
s=json.loads((ROOT/'evidence/k3s/terminal_summary.json').read_text())
with (ROOT/'evidence/k3s/node_snapshots.csv').open() as f:rows=list(csv.DictReader(f))
groups={}
for group in sorted({r['capture_group'] for r in rows}):
    sample=[r for r in rows if r['capture_group']==group]
    groups[group]={'samples':len(sample),'cpu_millicores_range':[min(float(r['cpu_millicores']) for r in sample),max(float(r['cpu_millicores']) for r in sample)],'memory_mib_range':[min(float(r['memory_mib']) for r in sample),max(float(r['memory_mib']) for r in sample)]}
t=s['seeded'];s['non_injection_alert_percent']=100*t['alerted_non_injection_ticks']/t['eligible_non_injection_ticks']
s['node_snapshots']=groups
s['resource_scope']='Whole-node snapshots, not isolated agent overhead; later excerpt is incomplete'
(ROOT/'evidence_outputs/k3s_summary.json').write_text(json.dumps(s,indent=2)+'\n')
print('K3s terminal transcription summarised; baseline memory',groups['baseline']['memory_mib_range'],'MiB')
