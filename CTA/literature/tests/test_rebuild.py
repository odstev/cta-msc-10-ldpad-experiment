"""Structural checks on synthetic temporary copies; no source data are changed."""
import csv
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

SOURCE=Path(__file__).resolve().parents[1]

class RebuildTest(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory()
        self.root=Path(self.temp.name)
        self.lit=self.root/'literature'
        shutil.copytree(SOURCE,self.lit,ignore=shutil.ignore_patterns('tests','__pycache__'))
        (self.root/'figures').mkdir()
    def tearDown(self):self.temp.cleanup()
    def change(self,name,fn):
        path=self.lit/name
        with path.open(newline='') as f:
            reader=csv.DictReader(f); fields=reader.fieldnames;rows=list(reader)
        fn(rows)
        with path.open('w',newline='') as f:
            writer=csv.DictWriter(f,fieldnames=fields);writer.writeheader();writer.writerows(rows)
    def run_tool(self):
        return subprocess.run(['python3',str(self.lit/'rebuild.py')],capture_output=True,text=True)
    def test_unknown_cells_are_not_zero_or_negative(self):
        p=self.run_tool();self.assertEqual(p.returncode,0,p.stderr)
        counts=(self.lit/'generated_counts.tex').read_text()
        self.assertIn(r'G1 Integrated controls & \textemdash & 25',counts)
        self.assertIn('No criterion cells have yet been evidence-coded',(self.lit/'generated_evidence.tex').read_text())
    def test_synthetic_coded_values_update_numerators(self):
        def edit_papers(rows):
            p=next(r for r in rows if r['ref']=='b24')
            p.update(implementation='Y',implementation_source_url='https://example.test',implementation_locator='synthetic',implementation_quote='synthetic test only')
        def edit_codes(rows):
            for code in rows:
                if code['ref']=='b24' and code['criterion'] in ('G1','G3'):
                    code.update(value='Y' if code['criterion']=='G3' else 'N',source_url='https://example.test',locator='synthetic',quote='synthetic test only')
        self.change('papers.csv',edit_papers);self.change('coding.csv',edit_codes)
        p=self.run_tool();self.assertEqual(p.returncode,0,p.stderr)
        counts=(self.lit/'generated_counts.tex').read_text()
        self.assertIn('G1 Integrated controls & 0/1 & 24',counts)
        self.assertIn('G3 Dynamic fusion & 1/1 & 24',counts)
        self.assertIn('synthetic test only',(self.lit/'generated_evidence.tex').read_text())
    def test_unsourced_decision_is_rejected(self):
        def edit(rows):next(r for r in rows if r['ref']=='b1' and r['criterion']=='G1').update(value='Y')
        self.change('coding.csv',edit)
        p=self.run_tool();self.assertNotEqual(p.returncode,0)
        self.assertIn('coding evidence incomplete',p.stderr)
    def test_out_of_window_inclusion_is_rejected(self):
        def edit(rows):next(r for r in rows if r['ref']=='b24').update(publication_date='2027-01-01')
        self.change('papers.csv',edit)
        p=self.run_tool();self.assertNotEqual(p.returncode,0)
        self.assertIn('inconsistent date inclusion',p.stderr)

if __name__=='__main__':unittest.main()
