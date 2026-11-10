# Prospective analysis plan: LDPAD laptop-VM comparison

Status: AI-assisted working protocol. This plan is for future, as-yet-unrun
K3s/VM, CRANE Cloud and DDIL comparisons. It was created after the existing
exploratory x86 detector, latency and MadSim studies and is not a
pre-registration of those studies. An author must review it before use. A
local Git commit establishes a traceable repository object and local time,
not independent prior publication or a trusted timestamp.

## Research questions and hypotheses

The target is single-node K3s in an Ubuntu VM on the user's laptop; the actual
CPU, memory, hypervisor, storage and guest allocations will be recorded. The
common synthetic registry workload, event classes, labels and matching window
must be fixed in an addendum before collection. A 30-seed list is included as
the candidate list; each event class and condition requires at least 100
labelled events in total, with their distribution across seeds disclosed.
Use the same event identities per matched LDPAD and CRANE run, randomized
interleaved order and a cryptographic digest of each event list.

- H1a (RQ1/RQ3/RQ4): event-level TPR has a two-sided 95% Clopper--Pearson
  lower bound at least 90% for each labelled class in D0, D1 and D2 below.
  The binomial interval assumes independent events, so also give a bootstrap
  sensitivity interval resampling whole seeds; neither resolves clustering
  perfectly when all outcomes are the same. Count every missed event and
  use a fixed one-to-one window. Report false alerts per node-day separately.
- H1b (RQ2/RQ5): the seed-bootstrap 95% upper bound for P99 request arrival to
  AdmissionReview response after durable SQLite commit is below 50 ms at
  each of 1, 10 and 100 requests/s on the target VM. Thirty seeds, 1--50 KiB
  objects, sustained per-rate run durations and failure handling must be
  fixed before data collection.
- H1c (RQ2/RQ5): for the full deployed agent, the 24-hour mean CPU time
  divided by elapsed time is below 10% of one core, from cgroup cpu.stat;
  report K3s and background processes separately.
- H1d (RQ2/RQ5): the deployed agent's peak cgroup memory.current sampling
  over 24 hours is below 200,000,000 bytes. Report the sampling interval and
  that unobserved spikes between samples are possible; this is cgroup memory
  usage, not process RSS. Report K3s separately.
- H1e (RQ3): for every predeclared class and D0--D2 cell, the mean paired
  seed-level TPR difference (LDPAD minus CRANE), in percentage points, is
  equivalent within [-5, +5] using paired TOST at alpha 0.05. Both one-sided
  nulls must be rejected. An unadjusted 90% paired-t CI must lie inside the
  margins; Holm-adjust TOST p-values across the predeclared H1e cell family,
  and require each adjusted p < 0.05. A 90% unadjusted CI inside the margin
  alone is insufficient after Holm. An inconclusive test does not establish
  equivalence or non-equivalence. Per-seed counts and labels must match.
- H1f (RQ4): during a 5-minute complete remote-link partition and after
  reconnection, locally observable labels yield a 95% exact TPR lower bound
  of at least 90%, and every expected locally recorded event, checked against the independent
  label ledger, arrives authenticated, in per-source sequence order, exactly
  once and without missing entries. State the
  fixed delivery observation window in an addendum. This is a bounded test,
  not a proof for all possible partitions. The local API and local durable
  log remain reachable; otherwise it is a different failure condition.

D0: no imposed delay or loss. D1: 300 ms one-way egress delay and 20% independent
packet loss. D2: 300 ms one-way egress delay and Gilbert--Elliott burst loss,
with p=2.5%, r=10%, bad-state loss=100%, good-state loss=0%. The ideal
stationary loss is p/(p+r) = 20%, but finite-run loss and delay must be
measured, not assumed. Use dedicated test interfaces/namespaces, record both
directions, packet counts, qdisc stats and routing; 300 ms each way would
add roughly 600 ms to RTT. The labelled event path must traverse the impaired
link for H1a, rather than silently detecting an unaffected local event. D3 is
the H1f full remote-link partition. No tc/netem command has been run here.

## Descriptive and comparison statistics

For each independent seed-run unit, report n, median, Q1, Q3 and IQR (linear
quantile interpolation), and a 95% percentile bootstrap interval using 10,000
resamples of independent seed/run units. Use paired resampling of differences
for paired contrasts. Specify outcome, direction, missingness, family and
seed before running. A one-sided tolerance-based detection equivalence test
is not a test of superiority; TOST is the two-one-sided procedure above.

For genuinely independent samples, report two-sided asymptotic Mann--Whitney U
with tie correction and continuity correction, and Cliff's delta as
P(X>Y)-P(X<Y). Matched seeds require paired methods, not Mann--Whitney U or
independent-sample Cliff's delta: report the mean paired difference, 95%
bootstrap interval and Hedges g_z = J(n-1) times mean difference divided by
the sample standard deviation of paired differences. g_z is a standardized
paired effect size, not a significance test or a replacement for TOST.
Zero paired variance is flagged as indeterminate by the analysis code.
Use Holm adjustment across the prespecified family of TOST cell-level
p-values; other hypothesis tests need their own declared families.

## Reproducibility and known limitations

The candidate seeds 0--29 are in `seeds.txt`. The fixed detector generator
parameters reflect the exploratory driver, not real attack traffic. MadSim
used seed 0 and fixed labelled scenarios; it was not 30 random repetitions.
The original unseeded CLI used thread_rng and cannot reproduce identical
new trajectories. Recorded logs can be analyzed again, not rerun exactly.
No operational DDIL, matched CRANE comparator, H1e or H1f results are present.
