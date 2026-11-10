#!/usr/bin/env python3
"""Generate the committed synthetic scenario fixtures and their design labels."""
from pathlib import Path
import json

ROOT = Path(__file__).resolve().parent
OUT = ROOT / 'scenarios'
OUT.mkdir(exist_ok=True)
Z = [0, 0, 0, 0]

def seg(start, end, risks, expected):
    return {'start_s': start, 'end_s': end, 'risks': risks, 'expected_mode': expected}

def make(name, description, segments, snapshots, core=True):
    assert segments[0]['start_s'] == 0 and segments[-1]['end_s'] == 3600
    assert all(a['end_s'] == b['start_s'] for a, b in zip(segments, segments[1:]))
    data = {'id': name, 'description': description, 'duration_s': 3600,
            'risk_order': ['static', 'drift', 'threat', 'runtime'],
            'grid_core': core, 'segments': segments, 'snapshots': snapshots}
    (OUT / (name + '.json')).write_text(json.dumps(data, indent=2) + '\n', encoding='utf-8')

make('01_clean', 'Clean conditions throughout.', [seg(0, 3600, Z, 'Normal')],
     [{'label':'Clean', 'time_s':1000}])
make('02_config_only', 'Sustained high static configuration risk.',
     [seg(0,300,Z,'Normal'), seg(300,1800,[90,0,0,0],'Moderate'), seg(1800,3600,Z,'Normal')],
     [{'label':'Config only', 'time_s':1000}])
make('03_cve_only', 'Sustained high threat/CVE risk.',
     [seg(0,300,Z,'Normal'), seg(300,1800,[0,0,90,0],'Moderate'), seg(1800,3600,Z,'Normal')],
     [{'label':'CVE only', 'time_s':1000}])
make('04_runtime_only', 'Sustained high runtime risk.',
     [seg(0,300,Z,'Normal'), seg(300,1800,[0,0,0,95],'Strict'), seg(1800,3600,Z,'Normal')],
     [{'label':'Runtime only', 'time_s':1000}])
make('05_ramp', 'Expected target progresses from Normal to Moderate to Strict and recovers.',
     [seg(0,300,Z,'Normal'), seg(300,900,[45,35,35,35],'Normal'),
      seg(900,1800,[70,60,60,60],'Moderate'), seg(1800,2700,[95,95,95,95],'Strict'),
      seg(2700,3600,Z,'Normal')],
     [{'label':'Ramp: medium', 'time_s':1400}, {'label':'Ramp: high', 'time_s':2300}])
make('06_combined', 'Several elevated sources produce a Strict target.',
     [seg(0,300,Z,'Normal'), seg(300,1800,[85,80,85,70],'Strict'), seg(1800,3600,Z,'Normal')],
     [{'label':'Combined', 'time_s':1000}])

noise = [seg(0,300,Z,'Normal')]
for start in range(300, 1500, 30):
    level = 48 if ((start - 300) // 30) % 2 == 0 else 52
    noise.append(seg(start,start+30,[level]*4,'Normal'))
noise.append(seg(1500,2100,[65]*4,'Moderate'))
for start in range(2100, 3000, 30):
    level = 78 if ((start - 2100) // 30) % 2 == 0 else 82
    noise.append(seg(start,start+30,[level]*4,'Moderate'))
noise.append(seg(3000,3600,Z,'Normal'))
make('07_threshold_noise', 'Repeated 30-second threshold oscillation; labels encode the no-escalation noise assumption.',
     noise, [], core=False)
for seconds in (2,3):
    make(f'0{6+seconds}_runtime_pulse_{seconds}s',
         f'One isolated {seconds}-second high runtime pulse treated as nuisance noise by the design label.',
         [seg(0,600,Z,'Normal'), seg(600,600+seconds,[0,0,0,95],'Normal'),
          seg(600+seconds,3600,Z,'Normal')], [], core=False)

print('Wrote', len(list(OUT.glob('*.json'))), 'labelled scenarios in', OUT)
