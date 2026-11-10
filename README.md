# CTA: LDPAD experiments for K3s edge nodes

Rust source, locked Cargo dependencies, experiment drivers, saved observations,
and analysis for LDPAD .

## Start here: reproduce the reported numbers

On Ubuntu, with Python 3.12 and make installed:

```bash
python3 -m venv .venv
source .venv/bin/activate
python -m pip install -r requirements.txt
make reproduce
```

This analyses the archived observations, regenerates tables and figures, runs
statistical tests, and checks the reported detector/latency/fusion results.
It does not run new attacks or claim to have collected new VM measurements.
Read `evidence_outputs/latency_summary.csv`, `seeded_summary.json`,
`fusion_summary.json`, and `k3s_summary.json` after completion.
`make pdf` optionally builds the paper with pdfLaTeX.

### Expected archived outputs

| Experiment | Result reconstructed from retained observations |
|---|---|
| Seeded Rust detector | 15,000 rows; 300/300 spike matches; 198/13,800 eligible non-injection alert ticks |
| VM loopback, 1 request/s | P99 13.341685 ms; upper 95% bound 14.736510 ms |
| VM loopback, 10 requests/s | P99 29.811595 ms; upper 95% bound 56.074556 ms |
| VM loopback, 100 requests/s | P99 15.880800 ms; upper 95% bound 25.415196 ms |
| Guarded fusion, sustained runtime | Strict after 3 virtual seconds; zero flaps in that scenario |
| Guarded fusion, all scenarios | Two flaps; 939/1,771 grid vectors pass |
| K3s node baseline transcription | Four samples: 3,970–3,987 MiB and 34–269 millicores |

The 10 requests/s latency condition does not meet the 50 ms upper-bound rule.
It is a loopback pilot, not Kubernetes admission-path latency. The four resource
samples are whole-node readings and cannot establish LDPAD-only overhead.

## What can have the same output?

- **Saved-data reanalysis:** the reported numeric tables are regression-checked.
  Dates in logs or plot metadata need not be byte-identical.
- **Seeded Rust and fixed MadSim scenarios:** `make verify-rust` builds and reruns
  them in a temporary copy, then checks their raw CSV SHA-256 hashes against the
  archived files. A mismatch fails visibly; it never substitutes archived rows
  for the new run. Use the recorded toolchain and lockfiles for closest reproduction.
- **Fresh timings, memory, CPU and unseeded runs:** outputs legitimately vary with
  scheduling, storage, background processes and randomness. Exact equality is
  neither promised nor forced.
- **K3s screenshots:** historical aggregates are transcribed and labelled as such.
  The original VM `ticks.csv` has not been supplied. A new run cannot recover it.

## Fresh experiment commands

Install Rust/Cargo and Linux build tools (`cc`, a linker and libc development
headers) before these commands. The historical manifests record Rust/Cargo
1.99.1; retain both Cargo.lock files. No toolchain is silently installed here.
The package was reanalysed with Python 3.12 and the versions in requirements.txt.
The original VM environment recorded NumPy 1.26.4; the retained rounded latency
outputs were also checked with the pinned analysis environment.

Every command below refuses an existing output directory. The isolated source,
build logs and new results remain under `runs/`; historical evidence is preserved.

```bash
# Rebuild and test the Rust project, including the node-replay example.
python3 scripts/run_experiment.py tests --output runs/tests-01

# Run the 30-seed detector, then analyse its labels and threshold sweep.
python3 scripts/run_experiment.py seeded --output runs/seeded-01

# Run the nine virtual-time fusion scenarios and 1,771-vector sweep.
python3 scripts/run_experiment.py fusion --output runs/fusion-01

# Run the original 30 unseeded CLI repetitions and component timing probe.
python3 scripts/run_experiment.py unseeded --output runs/unseeded-01

# Run the same short-burst loopback latency protocol on your Ubuntu VM.
# Includes Regorus, SQLite WAL/FULL commit, sync-call probe and bootstrap CIs.
python3 scripts/run_experiment.py latency --output runs/latency-01

# Stronger deterministic replay check: compare newly generated CSV hashes.
make verify-rust
```

The latency protocol is 30 seeds, 1/10/100 requests/s, ten requests per seed/rate,
and the 1/4/10/25/50 KiB object mix. It is intentionally the recorded pilot, not
an unperformed 30-minute stress test. `ldpad/scripts/reproduce_latency.py --help`
exposes duration-related request counts and storage placement for future studies.

## K3s on the existing Ubuntu VM

K3s must already be running. Do not reinstall it. Run these commands from the
repository root, with Cargo as your normal user. The helper uses sudo only for
K3s commands. It creates a uniquely named temporary pod, copies the actual
compiled executables, and saves both outputs locally.

```bash
python3 scripts/k3s_experiment.py --output runs/k3s-01
```

It records the pod description/image ID, cluster version, executable hashes,
raw `ticks.csv`, default CLI output and calculated counts. The image defaults
to Ubuntu 24.04; pass `--image ubuntu@sha256:YOUR_VERIFIED_DIGEST` to pin it.
Use the same CPU architecture and a compatible Ubuntu runtime as the build VM.
The helper prints the exact cleanup command. The pod has no service-account
token and sleeps for at most 24 hours; delete it after inspecting the capture.
Use `--kubectl` only if your configured standalone kubectl points at the intended cluster.

Validate a copied VM CSV without modifying it:

```bash
python3 scripts/summarize_ticks.py /home/user/k3s-seeded-run1/ticks.csv
```

Capture four whole-node readings, matching the baseline sampling method:

```bash
python3 scripts/capture_node.py --node user --samples 4 --interval 15 --output runs/node-baseline-01.txt
```

Replace `user` if `sudo k3s kubectl get nodes` shows another node name. The metrics
service must be ready. Run during the condition you intend to describe and
record whether the detector was active; the script does not impose a workload.

For a new 40-sample capture and the node-replay example:

```bash
python3 scripts/capture_node.py --node user --samples 40 --interval 15 --output runs/node-40.txt
python3 scripts/run_experiment.py node-replay --input runs/node-40.txt --output runs/replay-01
```

Only three node-replay unit tests were observed in the earlier session. No
completed replay result is claimed. The eight later screenshot samples are
retained as an incomplete excerpt, separate from the four baseline samples.

## Repository map

| Path | Purpose |
|---|---|
| `ldpad/src/` | Original Rust detector, posture, drift, risk and fusion code |
| `ldpad/examples/` | Seeded detector, loopback latency, node replay |
| `ldpad/fusion_study/` | MadSim source, lockfile, nine scenario JSON files |
| `ldpad/native/sync_probe.c` | Actual fsync/fdatasync timing probe |
| `ldpad/evidence/` | Original host detector and simulated fusion captures |
| `ldpad/latency_results/laptop_vm_20261102/` | Uploaded laptop-VM records and analysis |
| `ldpad/latency_results/main/` | Separate earlier host pilot |
| `evidence/k3s/` | Explicit screenshot transcriptions, not manufactured raw traces |
| `analysis/` | Reanalysis, statistics, hash verification and regression checks |
| `scripts/` | New isolated-run, K3s and resource-capture commands |
| `protocol/` | Existing seed list, parameters and historical prospective plan |
| `CTA/` | Morning manuscript and literature working files |
| `docs/VALIDATION.md` | Checks actually executed while packaging |

## Scope and provenance

No CRANE/OpenStack/Falco/Kubescape/Gatekeeper matched trials, DDIL trial,
seven-day clean trace, real-attack dataset or complete K3s admission path is
included as a completed experiment. The old EWMA Python model's source/raw
trace was missing from the recovered package and is not reconstructed as if
it were original evidence. Literature coding remains incomplete.

The original prospective plan and Git bundles are retained without rewriting
their dates. New repository scripts are retrospective packaging, not
preregistration. See `docs/PROVENANCE.md` for sources and `docs/GITHUB.md` for upload steps.

AI assistance: ChatGPT assisted with the evaluation code, analysis and this
repository packaging; the manuscript includes the broader tool disclosure.
Original authorship and any existing license notices remain unchanged. No new
open-source license is assigned on the authors' behalf.
