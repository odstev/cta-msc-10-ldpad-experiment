import csv,importlib.util,tempfile,unittest
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('ticks',ROOT/'scripts/summarize_ticks.py')
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
class CaptureChecks(unittest.TestCase):
 def test_archived_trace_matches_reported_counts(self):
  s=m.summarize(ROOT/'ldpad/evidence/seeded_ticks.csv')
  self.assertEqual((s['rows'],s['tp'],s['fn'],s['eligible_non_injection_ticks'],s['alerted_non_injection_ticks']),(15000,300,0,13800,198))
 def test_duplicate_row_rejected(self):
  with (ROOT/'ldpad/evidence/seeded_ticks.csv').open() as f:rows=list(csv.DictReader(f))
  rows[-1]=rows[0]
  with tempfile.TemporaryDirectory() as d:
   p=Path(d)/'bad.csv'
   with p.open('w',newline='') as f:
    w=csv.DictWriter(f,fieldnames=rows[0].keys());w.writeheader();w.writerows(rows)
   with self.assertRaises(ValueError):m.summarize(p)
 def test_truncated_trace_rejected(self):
  with tempfile.TemporaryDirectory() as d:
   p=Path(d)/'short.csv';p.write_text('seed,tick\n0,1\n')
   with self.assertRaises(ValueError):m.summarize(p)
if __name__=='__main__':unittest.main()
