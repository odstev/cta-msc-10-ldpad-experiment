# LDPAD hypotheses and reproducibility — working package

This package carries forward the correction 7 manuscript with a formatting pass. Empty headings have introductory text, overflow has been corrected, and unassigned publisher metadata is suppressed. It is AI-assisted
working material for author review. Its new prospective plan does not
pre-register the existing x86 component, loopback latency or fusion studies.

## Build the recorded results

Requirements: Python 3.11 or 3.12, `make`, and the packages listed in
`requirements.txt` (tested with Python 3.12). From this directory (or use the delegated `make` targets in `ldpad/`):

```bash
python3 -m pip install -r requirements.txt
make reproduce
make pdf
```

`make tables` regenerates the tables now labelled 2, 3 and 10 in this version,
the synthetic 1--10 sigma replay, and the captured latency and fusion table
fragments. The labels are `tab:rusttoolchain`, `tab:ci` and `tab:rules`; use
labels, since future manuscript edits can change table numbers. Table 3 is
recomputed from the original 30 unseeded captured CLI logs. A new unseeded
run would have different results and a new latency run would make new timing
measurements. `make figures` regenerates the seeded ROC, the black-and-white
screening diagram, and the single-column architecture figure. `make reproduce` includes these commands and statistical
tests; it reanalyses the captured laptop-VM loopback pilot but does not deploy K3s or run a new network experiment. The literature
matrix still contains unverified cells and does not recreate missing papers.

`make verify-rust` is optional and needs Rust, Cargo and dependencies in the
two lockfiles. It compiles the Rust project in an isolated temporary target,
reruns the seeded detector and MadSim experiment, and compares the resulting
seeded trace, scenario trace, transitions and 1,771-vector weight grid with
captured files. It does not run real attacks or the target VM. Locked crates
must be available in the local Cargo cache or downloadable from crates.io.

The pinned Python library versions were used for this review; run `make test`
to check the independent rank test, paired TOST, Hedges g_z and bootstrap
calculations against SciPy at absolute tolerance 1e-9. Holm's procedure is
checked against known examples, because SciPy does not implement Holm as a
public function. Synthetic test inputs are created only in temporary files.

## Prospective protocol and statistics

`protocol/seeds.txt` and the identical root `seeds.txt` list seeds 0--29.
`protocol/generator_parameters.json` describes the **existing synthetic**
detector and loopback pilots. `protocol/analysis_plan.md` and
`protocol/netem_planned.txt` describe a future VM study. The local prospective
protocol commit and date are in `protocol/prospective_commit.json`; verify
its bundled history with:

```bash
git clone protocol/prospective_protocol.bundle protocol-history
cd protocol-history
git log --format=fuller
```

The local Git timestamp is not independent registration or proof of a study's
first run. Event classes, attack workload, matching window, comparison family
and observation duration still need author review, completion and a dated
addendum before prospective collection. The prior fusion-only bundle in
`ldpad/` is a different commit and does not cover the future comparison.

For a future paired comparison, `analysis/compare.py` expects real seed-level
CSV records with columns `cell,seed,arm,tp,events,event_set_sha256`, one row
per arm (`LDPAD` or `CRANE`), seed and cell. A separate text file must list the
entire predeclared H1e cell family, one cell per line. The script checks the
30 seeds, matching event count and event-list digest before calculating paired
TOST and Holm-adjusted p-values. It flags zero variance as indeterminate.

```bash
python3 analysis/compare.py paired_data.csv --expected-cells cells.txt --output results.json
```

Do not create or populate paired outcomes without the actual comparison. The
input SHA-256 inventory in `protocol/input_sha256.txt` was made when this
package was assembled; it protects against accidental changes but does not
prove pre-registration.

## Important limits

No DDIL, live CRANE comparator, true network partition, 24-hour CPU/RAM trial,
clinical workload or laptop-VM H1 verdict is reported. H1e uses a 5-percentage-
point TPR margin and is not the separate 20% simulation--deployment agreement
rule. The proposed failover features are not established by the component
replay. The old seven-rule manuscript table differed from the actual Rust
predicates and weights; the new table is generated from `src/static_posture.rs`.

## Laptop-VM latency capture, 2 November 2026

The manuscript latency table is rebuilt from `ldpad/latency_results/laptop_vm_20261102`; the original uploaded ZIP is retained beside it. The earlier shared-host pilot remains in `ldpad/latency_results/main`. The new capture is a short-burst localhost pilot, not a deployed K3s admission path. The 95% P99 upper bound at 10 requests/s was 56.075 ms, so this pilot did not meet the 50 ms bound for all rates.

## Manuscript consistency review

The former per-node budget table was removed and remaining tables renumber automatically. The appendix now includes the 25-reference coding inventory, with unassessed entries marked `?`. The historical EWMA prototype source and raw traces were not present in this package, so its reported table cannot be reproduced here. The saved laptop-VM latency evidence is unchanged.

## Manuscript scope

The manuscript presents Rust posture and anomaly-detection prototype components intended for K3s edge nodes. The detector was evaluated with synthetic inputs, the guarded fusion component in virtual-time scenarios, and a separate policy/SQLite loopback harness on a laptop VM. K3s deployment and physical edge-node performance have not been measured.
