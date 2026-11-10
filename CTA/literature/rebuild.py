#!/usr/bin/env python3
"""Rebuild the review copy's literature matrix, counts and historical flow.
This is AI-assisted working code for author review, not original screening data.
"""
from pathlib import Path
import csv
from datetime import date
import json
import sys

HERE=Path(__file__).resolve().parent
ROOT=HERE.parent

def read(name):
    with (HERE/name).open(newline='',encoding='utf-8') as f:
        return list(csv.DictReader(f))

def need_evidence(row,prefix=''):
    return all(row.get(prefix+k,'').strip() for k in ('source_url','locator','quote'))

def esc(s):
    special={'\\':r'\textbackslash{}','{':r'\{','}':r'\}','$':r'\$','&':r'\&','#':r'\#','_':r'\_','%':r'\%','~':r'\textasciitilde{}','^':r'\textasciicircum{}'}
    return ''.join(special.get(c,c) for c in s)

papers=read('papers.csv')
codes=read('coding.csv')
protocol=json.loads((HERE/'protocol.json').read_text())
start=date.fromisoformat(protocol['search_start_date'])
cutoff=date.fromisoformat(protocol['search_cutoff_date'])
assert start<=cutoff
assert len(papers)==25 and len({p['ref'] for p in papers})==25, 'Expected 25 unique references'
assert {p['ref'] for p in papers}=={'b'+str(i) for i in range(1,26)}
assert len(codes)==175 and len({(c['ref'],c['criterion']) for c in codes})==175
matrix={(c['ref'],c['criterion']):c for c in codes}
for p in papers:
    assert p['implementation'] in ('Y','N','U')
    assert p['date_screen'] in ('I','E','U')
    assert p['available_by_search_date'] in ('Y','N','U')
    if p['implementation']!='U':
        assert need_evidence(p,'implementation_'), p['ref']+': implementation evidence incomplete'
    if p['date_screen']!='U':
        assert p['publication_date'] and p['date_source_url'] and p['date_locator'], p['ref']+': date evidence incomplete'
        d=date.fromisoformat(p['publication_date'])
        if p['date_screen']=='I':
            assert start<=d<=cutoff and p['available_by_search_date']=='Y', p['ref']+': inconsistent date inclusion'
        else:
            assert d<start or d>cutoff or p['available_by_search_date']=='N', p['ref']+': date exclusion not justified'
    for i in range(1,8):
        c=matrix[p['ref'],'G'+str(i)]
        assert c['value'] in ('Y','N','U')
        if c['value']!='U': assert need_evidence(c),p['ref']+' '+c['criterion']+': coding evidence incomplete'

out=[]
out.append(r'\begin{table*}[t]')
out.append(r'\caption{Literature matrix: coding status for 25 references and seven criteria}')
out.append(r'\label{tab:litmatrix}')
out.append(r'\centering\small\setlength{\tabcolsep}{4pt}\renewcommand{\arraystretch}{1.15}')
out.append(r'\begin{tabular}{@{}llccccccccc@{}}\toprule')
out.append(r'\textbf{Ref.} & \textbf{Venue, year} & \textbf{Date} & \textbf{Impl.} & \textbf{G1} & \textbf{G2} & \textbf{G3} & \textbf{G4} & \textbf{G5} & \textbf{G6} & \textbf{G7} \\ \midrule')
def s(v):return '?' if v=='U' else v
for p in papers:
    cells=[r'\cite{'+p['ref']+'}',esc(p['venue_as_listed'])+', '+p['year_as_listed'],s(p['date_screen']),s(p['implementation'])]+[s(matrix[p['ref'],'G'+str(i)]['value']) for i in range(1,8)]
    out.append(' & '.join(cells)+r' \\')
out.append(r'\bottomrule\end{tabular}')
out.append(r'\par\smallskip\parbox{0.96\textwidth}{\scriptsize Year and venue are transcribed from the manuscript bibliography. Date: I=in window and available by the reported search date, E=excluded on date, ?=not verified. Impl.: Y=implemented system, N=not an implemented system, ?=unassessed. G1--G7 are the positive criteria in Table~\ref{tab:gapcounts}; Y/N require a source, locator and quotation in the coding file. ? means not assessed, never absence. A full evidence audit remains necessary.}')
out.append(r'\end{table*}')
(HERE/'generated_matrix.tex').write_text('\n'.join(out)+'\n',encoding='utf-8')

# Each decided cell is accompanied by the source excerpt and locator supplied
# by the researcher. Validation checks completeness, not source correctness.
decided=[c for c in codes if c['value']!='U']
evidence=[r'\noindent\textbf{Per-cell source excerpts.}']
if not decided:
    evidence.append('No criterion cells have yet been evidence-coded in the supplied files.')
else:
    evidence.append(r'\begin{description}')
    for c in decided:
        assert not any(ch in c['source_url'] for ch in ' {}\n\r'), 'Unsafe source URL'
        evidence.append(r'\item[\cite{'+c['ref']+'}, '+c['criterion']+' ('+c['value']+r')] ``'+esc(c['quote'])+r"'' ("+esc(c['locator'])+r'; \url{'+c['source_url']+r'}).')
    evidence.append(r'\end{description}')
(HERE/'generated_evidence.tex').write_text('\n'.join(evidence)+'\n',encoding='utf-8')


labels=['Integrated controls','Offline operation','Dynamic fusion','Constrained hardware','Deep learning / transformer','P95 or P99 reported','Closed-loop remediation']
out=[r'\begin{table}[t]',r'\caption{Evidence-coded counts for seven gap criteria}',r'\label{tab:gapcounts}',r'\centering\footnotesize\setlength{\tabcolsep}{3pt}',r'\begin{tabular}{@{}p{0.48\columnwidth}cc@{}}\toprule',r'\textbf{Criterion} & \textbf{Yes/coded} & \textbf{Pending}$^\dagger$ \\ \midrule']
eligible=[p for p in papers if p['date_screen']=='I' and p['implementation']=='Y']
unknown_selection=sum(p['date_screen']=='U' or (p['date_screen']=='I' and p['implementation']=='U') for p in papers)
for i,label in enumerate(labels,1):
    vals=[matrix[p['ref'],'G'+str(i)]['value'] for p in eligible]
    known=[v for v in vals if v!='U'];yes=known.count('Y')
    count=f'{yes}/{len(known)}' if known else r'\textemdash'
    pending=unknown_selection+sum(v=='U' for v in vals)
    out.append('G'+str(i)+' '+label+' & '+count+' & '+str(pending)+r' \\')
out.extend([r'\bottomrule\end{tabular}',r'\par\smallskip\parbox{\columnwidth}{\scriptsize $^\dagger$ Papers with unassessed date or implementation status, plus eligible implementations with this criterion unassessed. Yes/coded refers only to evidence-coded eligible implementations; it is not a complete-corpus rate while any are pending. A dash indicates no coded denominator.}',r'\end{table}'])
(HERE/'generated_counts.tex').write_text('\n'.join(out)+'\n',encoding='utf-8')

# A compact reconstruction of historical reported numbers; explicitly incomplete.
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.patches import FancyBboxPatch
fig,ax=plt.subplots(figsize=(7.1,5.4))
ax.set_xlim(0,10);ax.set_ylim(0,10);ax.axis('off')
def box(x,y,w,h,title,detail,face='#ffffff'):
    patch=FancyBboxPatch((x,y),w,h,boxstyle='round,pad=0.08,rounding_size=0.08',facecolor=face,edgecolor='#000000',linewidth=.9)
    ax.add_patch(patch)
    ax.text(x+w/2,y+h*.67,title,ha='center',va='center',fontsize=8.6,fontweight='normal',color='#000000')
    ax.text(x+w/2,y+h*.30,detail,ha='center',va='center',fontsize=7.4,color='#000000',linespacing=1.15)
def arrow(x1,y1,x2,y2):
    ax.annotate('',xy=(x2,y2),xytext=(x1,y1),arrowprops=dict(arrowstyle='-|>',color='#000000',lw=1.0,mutation_scale=9))
box(.45,8.6,4.35,1.05,'Database records identified','127 reported; per-database totals unknown')
box(5.75,8.6,3.8,1.05,'Removed before screening','38 duplicates reported')
box(.45,6.9,4.35,1.05,'Records screened','89 after duplicate removal (127 - 38)')
box(5.75,6.9,3.8,1.05,'Records excluded','20 inferred (89 - 69)')
box(.45,5.2,4.35,1.05,'Records passing initial screening','69 reported')
box(.45,3.5,4.35,1.05,'Reports sought / retrieved','Numbers not recorded in available files')
box(5.75,3.5,3.8,1.05,'Full-text exclusions','44 reported; reasons unverified')
box(.45,1.8,4.35,1.05,'Studies included','25 reported; coding not validated')
for y1,y2 in [(8.6,7.95),(6.9,6.25),(5.2,4.55),(3.5,2.85)]:arrow(2.625,y1,2.625,y2)
arrow(4.8,9.125,5.75,9.125);arrow(4.8,7.425,5.75,7.425);arrow(4.8,4.025,5.75,4.025)
ax.text(5,0.5,'Reported counts; screening records and complete PRISMA stages remain unavailable.',ha='center',va='center',fontsize=7.1,color='#000000')
fig.tight_layout(pad=.3)
fig.savefig(ROOT/'figures'/'reported_prisma_flow.png',dpi=220,bbox_inches='tight',facecolor='white')
plt.close(fig)
print(f'Indexed: {len(papers)}; confirmed eligible implementations: {len(eligible)}; unresolved date/implementation: {unknown_selection}')
print('Regenerated literature/generated_matrix.tex, literature/generated_counts.tex, literature/generated_evidence.tex, and figures/reported_prisma_flow.png')
