import csv
import json
from math import exp,lgamma,log,sqrt
from pathlib import Path
import subprocess,sys,tempfile,unittest
import numpy as np
from scipy import stats,special

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'analysis'))
from ldpad_stats import bootstrap_interval,describe,independent_rank_test,hedges_gz,paired_tost,holm

class StatisticalChecks(unittest.TestCase):
    def test_mann_whitney_and_cliff_against_scipy(self):
        rng=np.random.default_rng(4810)
        cases=[(rng.normal(size=17),rng.normal(size=21)),
               (np.ones(8),np.ones(9))]
        cases.extend((rng.integers(0,7,size=13),rng.integers(0,7,size=14)) for _ in range(100))
        for x,y in cases:
            ours=independent_rank_test(x,y)
            ref=stats.mannwhitneyu(x,y,method='asymptotic',use_continuity=True,alternative='two-sided')
            self.assertAlmostEqual(ours['u_x'],float(ref.statistic),delta=1e-9)
            self.assertAlmostEqual(ours['p_two_sided'],float(ref.pvalue),delta=1e-9)
            self.assertAlmostEqual(ours['cliffs_delta'],2*float(ref.statistic)/(len(x)*len(y))-1,delta=1e-9)
    def test_quantiles_and_bootstrap_against_scipy(self):
        x=np.random.default_rng(17).normal(size=30)
        ours=describe(x)
        self.assertAlmostEqual(ours['iqr'],float(stats.iqr(x,interpolation='linear')),delta=1e-9)
        ref=stats.bootstrap((x,),np.median,confidence_level=.95,n_resamples=10000,method='percentile',vectorized=False,rng=np.random.default_rng(20261101))
        for a,b in zip(ours['median_bootstrap_95'],ref.confidence_interval):self.assertAlmostEqual(a,float(b),delta=1e-9)
    def test_paired_tost_and_hedges_gz_against_scipy(self):
        rng=np.random.default_rng(7)
        for shift in (-7,-3,0,3,7):
            y=rng.uniform(70,95,size=30);x=y+rng.normal(shift,2,size=30)
            result=paired_tost(x,y)
            lower=stats.ttest_1samp(x-y,popmean=-5,alternative='greater')
            upper=stats.ttest_1samp(x-y,popmean=5,alternative='less')
            interval=stats.ttest_1samp(x-y,popmean=0).confidence_interval(confidence_level=.90)
            self.assertAlmostEqual(result['p_lower'],float(lower.pvalue),delta=1e-9)
            self.assertAlmostEqual(result['p_upper'],float(upper.pvalue),delta=1e-9)
            for a,b in zip(result['ci90'],interval):self.assertAlmostEqual(a,float(b),delta=1e-9)
            df=len(x)-1;J=special.gamma(df/2)/(np.sqrt(df/2)*special.gamma((df-1)/2))
            ref=float(J*stats.ttest_rel(x,y).statistic/np.sqrt(len(x)))
            self.assertAlmostEqual(result['hedges_gz'],ref,delta=1e-9)
    def test_holm_known_adjustments_and_invalid_input(self):
        np.testing.assert_allclose(holm([.01,.015,.03,.2]),[.04,.045,.06,.2],atol=1e-12,rtol=0)
        with self.assertRaises(ValueError):holm([-.1,.4])
        with self.assertRaises(ValueError):paired_tost([2,2,2],[1,1,1])
    def test_cli_requires_complete_matching_and_reports_no_fabricated_data(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);expected=root/'cells.txt';expected.write_text('D0:synthetic\n')
            inp=root/'example.csv';out=root/'out.json';fields=['cell','seed','arm','tp','events','event_set_sha256']
            with inp.open('w',newline='') as f:
                w=csv.DictWriter(f,fieldnames=fields);w.writeheader()
                for i in range(30):
                    for arm in ('LDPAD','CRANE'):
                        w.writerow(dict(cell='D0:synthetic',seed=i,arm=arm,tp=8+int(i%3==0 and arm=='LDPAD'),events=10,event_set_sha256=f'{i:064x}'))
            p=subprocess.run([sys.executable,str(ROOT/'analysis'/'compare.py'),str(inp),'--output',str(out),'--expected-cells',str(expected)],capture_output=True,text=True)
            self.assertEqual(p.returncode,0,p.stderr)
            result=json.loads(out.read_text())['results'][0]
            self.assertEqual(result['event_count'],300)
            self.assertTrue(result['event_minimum_100_met'])
            # A missing predeclared cell cannot silently disappear from Holm.
            expected.write_text('D0:synthetic\nD1:synthetic\n')
            p=subprocess.run([sys.executable,str(ROOT/'analysis'/'compare.py'),str(inp),'--output',str(root/'bad.json'),'--expected-cells',str(expected)],capture_output=True,text=True)
            self.assertNotEqual(p.returncode,0)
            self.assertIn('predeclared family',p.stderr)

if __name__=='__main__':unittest.main()
