#!/usr/bin/env python3
"""Rebuild figures from captured operating points and literature working data."""
from pathlib import Path
import shutil,subprocess,sys,tempfile
ROOT=Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix='ldpad-figure-') as d:
 t=Path(d);(t/'scripts').mkdir();(t/'evidence').mkdir()
 shutil.copy2(ROOT/'ldpad'/'scripts'/'plot_seeded_roc.py',t/'scripts'/'plot_seeded_roc.py')
 source=ROOT/'evidence_outputs'/'seeded_roc.csv'
 if not source.exists():source=ROOT/'ldpad'/'evidence'/'seeded_roc.csv'
 shutil.copy2(source,t/'evidence'/'seeded_roc.csv')
 subprocess.run([sys.executable,str(t/'scripts'/'plot_seeded_roc.py')],cwd=t,check=True)
 shutil.copy2(t/'evidence'/'seeded_roc.png',ROOT/'CTA'/'figures'/'seeded_roc.png')
subprocess.run([sys.executable,str(ROOT/'CTA'/'literature'/'rebuild.py')],cwd=ROOT,check=True)
subprocess.run([sys.executable,str(ROOT/'CTA'/'figures'/'create_architecture.py')],cwd=ROOT,check=True)
print('Rebuilt the ROC plot, screening flow, and single-column architecture.')
