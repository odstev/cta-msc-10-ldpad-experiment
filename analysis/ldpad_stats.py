"""Statistical helpers for prospective comparisons; AI-assisted review code.

Unit of analysis is an independent seed/run. Functions reject NaN/infinite
values. No function creates outcomes when comparison data are absent.
"""
from __future__ import annotations
from math import exp, lgamma, log, sqrt
import numpy as np
from scipy import stats

DEFAULT_SEED = 20261101

def vector(values, minimum=1):
    x=np.asarray(values,dtype=float)
    if x.ndim!=1 or len(x)<minimum or not np.isfinite(x).all():
        raise ValueError(f'Expected a finite one-dimensional vector of length >= {minimum}')
    return x

def bootstrap_interval(values, statistic='median', *, seed=DEFAULT_SEED, resamples=10000):
    x=vector(values,2)
    if resamples<2:raise ValueError('Need at least two resamples')
    rng=np.random.default_rng(seed)
    idx=rng.integers(0,len(x),size=(resamples,len(x)))
    selected=x[idx]
    if statistic=='median': sample=np.median(selected,axis=1)
    elif statistic=='mean': sample=np.mean(selected,axis=1)
    else:raise ValueError('statistic must be mean or median')
    return tuple(float(v) for v in np.quantile(sample,[.025,.975],method='linear'))

def describe(values, *, seed=DEFAULT_SEED):
    x=vector(values,2)
    q1,median,q3=np.quantile(x,[.25,.5,.75],method='linear')
    return {'n':len(x),'median':float(median),'q1':float(q1),'q3':float(q3),
            'iqr':float(q3-q1),'median_bootstrap_95':bootstrap_interval(x,seed=seed)}

def independent_rank_test(x,y):
    """Two-sided asymptotic Mann--Whitney U and Cliff delta for independent groups.

    Tie-corrected variance, continuity correction and midranks match SciPy's
    mannwhitneyu(...,method='asymptotic',alternative='two-sided').
    Delta = P(X>Y)-P(X<Y); ties count as zero.
    """
    x,y=vector(x,2),vector(y,2)
    n,m=len(x),len(y);total=n+m
    combined=np.concatenate([x,y]);order=np.argsort(combined,kind='stable')
    ranks=np.empty(total,dtype=float);tie_term=0;pos=0
    while pos<total:
        end=pos+1
        while end<total and combined[order[end]]==combined[order[pos]]:end+=1
        ranks[order[pos:end]]=(pos+1+end)/2
        count=end-pos;tie_term+=count**3-count;pos=end
    u=float(ranks[:n].sum()-n*(n+1)/2)
    variance=n*m/12*(total+1-tie_term/(total*(total-1)))
    if variance<=0:p=1.0
    else:p=min(1.0,float(2*stats.norm.sf((abs(u-n*m/2)-.5)/sqrt(variance))))
    comparisons=x[:,None]-y[None,:]
    delta=float((np.count_nonzero(comparisons>0)-np.count_nonzero(comparisons<0))/(n*m))
    return {'u_x':u,'p_two_sided':p,'cliffs_delta':delta}

def hedges_gz(differences):
    d=vector(differences,3);df=len(d)-1;sd=float(np.std(d,ddof=1))
    if sd==0:raise ValueError('Hedges g_z undefined for zero paired standard deviation')
    correction=exp(lgamma(df/2)-.5*log(df/2)-lgamma((df-1)/2))
    return float(correction*np.mean(d)/sd)

def paired_tost(x,y,margin=5.0):
    """Paired t-TOST: margins are in the same units as x and y.

    Here TPRs are expressed in percentage points. A zero sample variance is
    flagged rather than used to manufacture a finite or decisive interval.
    """
    x,y=vector(x,3),vector(y,3)
    if len(x)!=len(y):raise ValueError('Expected a complete pair for every seed')
    if margin<=0:raise ValueError('Margin must be positive')
    d=x-y;n=len(d);df=n-1;mean=float(np.mean(d));sd=float(np.std(d,ddof=1))
    if sd==0:raise ValueError('TOST indeterminate: paired difference has zero sample variance')
    se=sd/sqrt(n)
    p_lower=float(stats.t.sf((mean+margin)/se,df))
    p_upper=float(stats.t.cdf((mean-margin)/se,df))
    critical=float(stats.t.ppf(.95,df))
    return {'n_pairs':n,'mean_difference':mean,'margin':float(margin),
            'ci90':(mean-critical*se,mean+critical*se),
            'p_lower':p_lower,'p_upper':p_upper,'p_tost':max(p_lower,p_upper),
            'hedges_gz':hedges_gz(d),
            'mean_difference_bootstrap_95':bootstrap_interval(d,'mean')}

def holm(p_values):
    p=vector(p_values)
    if (p<0).any() or (p>1).any():raise ValueError('p-values must lie in [0,1]')
    order=np.argsort(p,kind='stable');m=len(p);adjusted=np.empty(m,dtype=float);running=0.0
    for j,i in enumerate(order):
        running=max(running,(m-j)*float(p[i]));adjusted[i]=min(1.0,running)
    return adjusted
